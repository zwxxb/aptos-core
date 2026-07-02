# Move-flow Facts Soundness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `facts` query of `move_package_query` sound and internally consistent for acquires, resource reads/writes, closures, function values, lambda-lifted functions, and inline functions.

**Architecture:** Additive schema extension in `package_query.rs` only (spec: `docs/superpowers/specs/2026-07-02-move-flow-facts-soundness-design.md`). One combined AST walk (`scan_body`) replaces `build_resource_access` and additionally yields `createsClosures` + `invokesFunctionValues`; a small module-local fixpoint recomputes acquires for lambda-lifted functions; `acquiresDeclared` comes from access specifiers; the facts query fails fast on compilation errors. No compiler changes.

**Tech Stack:** Rust, move-model AST (`ExpData`, `Operation`), rmcp MCP server, golden `.exp` baseline tests (`UB=1` regenerates).

**Working directory: ALL commands run from `/Users/amine/Desktop/aptos-core/.worktrees/pr-20174`** (the base is PR #20174, commit `f24da52ee9`). The one file modified is `aptos-move/flow/src/mcp/tools/package_query.rs` in that worktree; tests live under `aptos-move/flow/src/tests/move_package_query/`.

**Baseline churn rule:** any task that changes the emitted schema regenerates ALL baselines with `UB=1 cargo test -p aptos-move-flow -- move_package_query`, then reviews `git diff -- '*.exp'` before committing. Never hand-edit `.exp` files.

---

### Task 0: Branch and docs

**Files:**
- Create branch in worktree; copy spec + this plan into it.

- [ ] **Step 1: Create the feature branch**

```bash
cd /Users/amine/Desktop/aptos-core/.worktrees/pr-20174
git switch -c move-flow-facts-soundness
```

- [ ] **Step 2: Copy spec and plan from the main tree and commit**

```bash
mkdir -p docs/superpowers/specs docs/superpowers/plans
cp /Users/amine/Desktop/aptos-core/docs/superpowers/specs/2026-07-02-move-flow-facts-soundness-design.md docs/superpowers/specs/
cp /Users/amine/Desktop/aptos-core/docs/superpowers/plans/2026-07-02-move-flow-facts-soundness.md docs/superpowers/plans/
git add docs/superpowers
git commit -m "[move-flow] Add facts-soundness design spec and plan"
```

- [ ] **Step 3: Sanity-build the crate once (warms the target dir)**

```bash
cargo check -p aptos-move-flow
```
Expected: success (PR base compiles).

---

### Task 1: Fail fast on compilation errors

**Files:**
- Modify: `aptos-move/flow/src/mcp/tools/package_query.rs` (the `QueryType::Facts` arm in `move_package_query_impl`)
- Create: `aptos-move/flow/src/tests/move_package_query/facts_compile_error.rs`
- Modify: `aptos-move/flow/src/tests/move_package_query/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `aptos-move/flow/src/tests/move_package_query/facts_compile_error.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// The facts query must fail on a package with compilation errors instead of
/// silently emitting facts from a partially transformed model (e.g. before
/// lambda lifting), whose effect attribution differs from a clean build.
#[tokio::test]
async fn move_package_query_facts_compile_error() {
    let pkg = common::make_package("facts_compile_error", &[(
        "broken",
        "module 0xCAFE::broken {
    struct R has key { v: u64 }

    // Error: declared acquires is non-empty but misses `R` read via the lambda.
    public fun bad(): u64 acquires SomethingElse {
        borrow_global<R>(@0xCAFE).v
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

Register it in `aptos-move/flow/src/tests/move_package_query/mod.rs` (alphabetical order with the other `mod` lines):

```rust
mod facts_compile_error;
```

Note: before writing the fixture, mirror how `invalid_path.rs` in the same directory calls the tool and formats errors; if error results go through `call_tool_raw`/`format_service_error` there, use the same pattern here.

- [ ] **Step 2: Run to verify current behavior (test creates a baseline showing facts JSON, not an error)**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query_facts_compile_error
cat aptos-move/flow/src/tests/move_package_query/facts_compile_error.exp
```
Expected: the `.exp` shows emitted facts (the bug). Do not commit this baseline.

- [ ] **Step 3: Implement the gate**

In `move_package_query_impl`, change the `QueryType::Facts` arm:

```rust
QueryType::Facts => {
    if data.has_compilation_errors() {
        return Err(mcp_err(
            "package has compilation errors; facts would reflect a partially \
             compiled package. Run move_package_status for diagnostics",
        ));
    }
    let result = try_call("failed to build facts", || build_facts(data.env()))?;
    log::info!("move_package_query facts: {} module(s)", result.len());
    Ok(into_call_tool_result(&result))
},
```

- [ ] **Step 4: Regenerate the baseline and verify it now shows the error**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query_facts_compile_error
cat aptos-move/flow/src/tests/move_package_query/facts_compile_error.exp
cargo test -p aptos-move-flow -- move_package_query
```
Expected: baseline contains the "package has compilation errors" message; all other query tests still pass.

- [ ] **Step 5: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Fail facts query on packages with compile errors"
```

---

### Task 2: Combined body scan — spec-block skip, createsClosures, invokesFunctionValues, effectsComplete

This is the core task: replace `build_resource_access` with a single `scan_body` walk and add the three new fields plus `effectsComplete`.

**Files:**
- Modify: `aptos-move/flow/src/mcp/tools/package_query.rs` (`FunctionFacts`, `build_function_facts`, `build_resource_access` → `scan_body`)
- Create: `aptos-move/flow/src/tests/move_package_query/facts_function_value.rs` + `.exp`
- Create: `aptos-move/flow/src/tests/move_package_query/facts_spec_block.rs` + `.exp`
- Modify: `aptos-move/flow/src/tests/move_package_query/mod.rs`

- [ ] **Step 1: Write the failing tests**

`facts_function_value.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// A function invoking a function-value parameter must not claim purity:
/// `invokesFunctionValues: true`, `effectsComplete: false`.
#[tokio::test]
async fn move_package_query_facts_function_value() {
    let pkg = common::make_package("facts_function_value", &[(
        "fv",
        "module 0xCAFE::fv {
    public fun foo(bar: |u64|u64): u64 {
        bar(1)
    }

    public fun pure_leaf(x: u64): u64 {
        x + 1
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

`facts_spec_block.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Storage operations inside spec blocks are specification-only and must not
/// appear in `resourceAccess`.
#[tokio::test]
async fn move_package_query_facts_spec_block() {
    let pkg = common::make_package("facts_spec_block", &[(
        "specs",
        "module 0xCAFE::specs {
    struct R has key { v: u64 }

    public fun check(addr: address): bool {
        let result = addr == @0xCAFE;
        spec {
            assert exists<R>(addr) ==> result;
        };
        result
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

Register both in `mod.rs`:

```rust
mod facts_function_value;
mod facts_spec_block;
```

- [ ] **Step 2: Generate pre-fix baselines to see the defects**

```bash
UB=1 cargo test -p aptos-move-flow -- facts_function_value facts_spec_block
```
Expected: `foo` shows empty access with no unknown marker; `check` shows `reads: ["0xcafe::specs::R"]` (the spec-block leak). Do not commit these.

- [ ] **Step 3: Implement the scan**

In `package_query.rs`:

3a. Extend `FunctionFacts` (order matters for the wire format — insert exactly here):

```rust
struct FunctionFacts {
    // ... existing fields through `return_types` unchanged ...
    return_types: Vec<String>,
    acquires_inferred: Vec<String>,
    resource_access: ResourceAccessFacts,
    /// Fully-qualified names of functions for which this body creates closures
    /// (`Operation::Closure`). The closure's own effects are on the target
    /// function's facts; creation alone does not execute them.
    creates_closures: Vec<String>,
    /// True if the body invokes a function value whose target is not statically
    /// known. Such calls can have arbitrary resource effects.
    invokes_function_values: bool,
    /// False when `resourceAccess`/`acquiresInferred` are lower bounds: native
    /// functions, functions without a body, or `invokesFunctionValues`.
    /// Effects of *named* callees are intentionally not folded in — compose
    /// with the `call_graph` query.
    effects_complete: bool,
    is_lambda_lifted: bool,
    // ... `defined_in` unchanged ...
}
```

3b. Replace `build_resource_access` with `scan_body` (keep the doc comment style; import `VisitorPosition` from `move_model::ast`):

```rust
/// Direct effects of a function body: storage ops, closure creations, and
/// unknown function-value invocations. Spec blocks are specification-only and
/// skipped; lambda bodies (pre-lifting only, defensive) belong to the closure.
#[derive(Default)]
struct BodyScan {
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    creates_closures: BTreeSet<String>,
    invokes_function_values: bool,
}

fn scan_body(
    env: &GlobalEnv,
    func: &FunctionEnv<'_>,
    type_ctx: &TypeDisplayContext<'_>,
) -> BodyScan {
    let mut scan = BodyScan::default();
    let Some(body) = func.get_def() else {
        return scan;
    };
    let mut spec_depth = 0usize;
    let mut lambda_depth = 0usize;
    body.visit_positions(&mut |pos, e| {
        match e {
            ExpData::SpecBlock(..) => match pos {
                VisitorPosition::Pre => spec_depth += 1,
                VisitorPosition::Post => spec_depth -= 1,
                _ => {},
            },
            ExpData::Lambda(..) => match pos {
                VisitorPosition::Pre => lambda_depth += 1,
                VisitorPosition::Post => lambda_depth -= 1,
                _ => {},
            },
            ExpData::Invoke(_, callee, _)
                if matches!(pos, VisitorPosition::Pre)
                    && spec_depth == 0
                    && lambda_depth == 0 =>
            {
                // Invoking a closure built in place has a known target whose
                // facts carry the effects; anything else is unknown.
                if !matches!(
                    callee.as_ref(),
                    ExpData::Call(_, Operation::Closure(..), _)
                ) {
                    scan.invokes_function_values = true;
                }
            },
            ExpData::Call(node_id, op, _)
                if matches!(pos, VisitorPosition::Pre)
                    && spec_depth == 0
                    && lambda_depth == 0 =>
            {
                if let Operation::Closure(mid, fid, _) = op {
                    scan.creates_closures.insert(
                        env.get_function(mid.qualified(*fid))
                            .get_full_name_with_address(),
                    );
                } else {
                    let (does_read, does_write) = match op {
                        Operation::Exists(_)
                        | Operation::BorrowGlobal(ReferenceKind::Immutable) => (true, false),
                        Operation::BorrowGlobal(ReferenceKind::Mutable)
                        | Operation::MoveFrom => (true, true),
                        Operation::MoveTo => (false, true),
                        _ => (false, false),
                    };
                    if does_read || does_write {
                        let insts = env.get_node_instantiation(*node_id);
                        if let Some(ty) = insts.first() {
                            let ty = ty.display(type_ctx).to_string();
                            if does_read {
                                scan.reads.insert(ty.clone());
                            }
                            if does_write {
                                scan.writes.insert(ty);
                            }
                        }
                    }
                }
            },
            _ => {},
        }
        true
    });
    scan
}
```

3c. In `build_function_facts`, replace the `build_resource_access` call:

```rust
let scan = scan_body(env, func, &type_ctx);
let resource_access = ResourceAccessFacts {
    reads: scan.reads.into_iter().collect(),
    writes: scan.writes.into_iter().collect(),
};
let effects_complete =
    !func.is_native() && func.get_def().is_some() && !scan.invokes_function_values;
```

and populate the struct:

```rust
resource_access,
creates_closures: scan.creates_closures.into_iter().collect(),
invokes_function_values: scan.invokes_function_values,
effects_complete,
```

Delete the old `build_resource_access` function.

- [ ] **Step 4: Regenerate all facts baselines and verify semantics**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query
git diff -- 'aptos-move/flow/src/tests/**/*.exp'
```
Expected in the diff:
- `facts_function_value.exp`: `foo` has `invokesFunctionValues: true`, `effectsComplete: false`; `pure_leaf` has `false`/`true`.
- `facts_spec_block.exp`: `check.resourceAccess.reads` is `[]`.
- `facts_closure.exp`: `run`/`apply` now `invokesFunctionValues: true`, `effectsComplete: false`; `setup` gains `createsClosures: ["0xcafe::closures::__lambda__1__setup"]`.
- `facts.exp`, `facts_nested.exp`: new fields with expected values, no other changes.

```bash
cargo test -p aptos-move-flow -- move_package_query
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Scan bodies for closures, invokes, spec blocks in facts"
```

---

### Task 3: `acquiresDeclared`

**Files:**
- Modify: `aptos-move/flow/src/mcp/tools/package_query.rs`
- Create: `aptos-move/flow/src/tests/move_package_query/facts_acquires_transitive.rs` + `.exp`
- Modify: `aptos-move/flow/src/tests/move_package_query/mod.rs`

- [ ] **Step 1: Write the failing test (also pins spec case 1: transitive acquires without direct reads)**

`facts_acquires_transitive.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Acquires propagation vs direct access: `caller` inherits `acquiresInferred`
/// from `helper` but must NOT report a direct read; `acquiresDeclared` reflects
/// only source annotations. Mutual recursion pins fixpoint stability.
#[tokio::test]
async fn move_package_query_facts_acquires_transitive() {
    let pkg = common::make_package("facts_acquires_transitive", &[(
        "acq",
        "module 0xCAFE::acq {
    struct R has key { v: u64 }
    struct S has key { v: u64 }

    public fun helper(): u64 acquires R {
        borrow_global<R>(@0xCAFE).v
    }

    public fun caller(): u64 acquires R {
        helper()
    }

    public fun undeclared_caller(): u64 {
        helper()
    }

    fun ping(n: u64): u64 acquires R, S {
        if (n == 0) { borrow_global<R>(@0xCAFE).v } else { pong(n - 1) }
    }

    fun pong(n: u64): u64 acquires R, S {
        if (n == 0) { borrow_global<S>(@0xCAFE).v } else { ping(n - 1) }
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

Register in `mod.rs`: `mod facts_acquires_transitive;`

- [ ] **Step 2: Implement `acquiresDeclared`**

In `package_query.rs`, extend imports:

```rust
use move_model::ast::{AccessSpecifierKind, ResourceSpecifier};
```

Add to `FunctionFacts` directly BEFORE `acquires_inferred`:

```rust
/// Resources named in a source-level `acquires` annotation.
acquires_declared: Vec<String>,
```

Add the helper next to `build_function_facts`:

```rust
/// Resources listed in the function's source `acquires` annotation
/// (legacy access specifiers), as fully-qualified names without type args,
/// matching the `acquiresInferred` rendering.
fn declared_acquires(func: &FunctionEnv<'_>) -> Vec<String> {
    let env = func.module_env.env;
    let mut result = BTreeSet::new();
    for spec in func.get_access_specifiers().unwrap_or(&[]) {
        if spec.kind != AccessSpecifierKind::LegacyAcquires {
            continue;
        }
        if let ResourceSpecifier::Resource(qid) = &spec.resource.1 {
            result.insert(
                env.get_struct(qid.to_qualified_id())
                    .get_full_name_with_address(),
            );
        }
    }
    result.into_iter().collect()
}
```

(If `get_access_specifiers` returns `Option<&Vec<_>>` rather than `Option<&[_]>`, adapt with `.map(|v| v.as_slice()).unwrap_or(&[])`.)

Populate in `build_function_facts`:

```rust
acquires_declared: declared_acquires(func),
```

- [ ] **Step 3: Regenerate baselines, verify, run**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query
git diff -- 'aptos-move/flow/src/tests/**/*.exp'
```
Expected in `facts_acquires_transitive.exp`:
- `helper`: `acquiresDeclared: ["0xcafe::acq::R"]`, `acquiresInferred: ["0xcafe::acq::R"]`, `reads: ["0xcafe::acq::R"]`.
- `caller`: same acquires, `resourceAccess: { reads: [], writes: [] }`, `effectsComplete: true`.
- `undeclared_caller`: `acquiresDeclared: []`, `acquiresInferred: ["0xcafe::acq::R"]`, empty access.
- `ping`/`pong`: both `acquiresInferred: [R, S]`.

```bash
cargo test -p aptos-move-flow -- move_package_query
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Emit acquiresDeclared alongside acquiresInferred"
```

---

### Task 4: Recompute acquires for lambda-lifted functions

**Files:**
- Modify: `aptos-move/flow/src/mcp/tools/package_query.rs` (`build_module_facts`, `build_function_facts`, new helper)
- Create: `aptos-move/flow/src/tests/move_package_query/facts_closure_stored.rs` + `.exp`
- Modify: `aptos-move/flow/src/tests/move_package_query/mod.rs`

- [ ] **Step 1: Write the failing test (spec case 3: stored callback, never invoked)**

`facts_closure_stored.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Stored callbacks: the creator records `createsClosures` but no direct
/// access; the lifted closure carries its own reads AND a recomputed
/// `acquiresInferred` (the compiler's acquires pass ran before lifting).
/// `setup.acquiresInferred` still contains `Config` — the language attributes
/// lambda-body acquires to the enclosing function; consumers distinguish
/// storage from execution via `createsClosures` + the lifted function's facts.
#[tokio::test]
async fn move_package_query_facts_closure_stored() {
    let pkg = common::make_package("facts_closure_stored", &[(
        "stored",
        "module 0xCAFE::stored {
    struct Config has key { v: u64 }
    struct Holder has key { cb: |u64|u64 has store + copy }

    fun register_callback(cb: |u64|u64 has store + copy) {
        borrow_global_mut<Holder>(@0xCAFE).cb = cb;
    }

    public fun setup() {
        register_callback(|x| x + borrow_global<Config>(@0xCAFE).v)
    }

    public fun lifted_calls_helper() {
        register_callback(|x| x + read_config())
    }

    fun read_config(): u64 {
        borrow_global<Config>(@0xCAFE).v
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

Register in `mod.rs`: `mod facts_closure_stored;`

- [ ] **Step 2: Generate pre-fix baseline showing the defect**

```bash
UB=1 cargo test -p aptos-move-flow -- facts_closure_stored
```
Expected: lifted `__lambda__` functions show `reads: [Config]` but `acquiresInferred: []`. Do not commit.

- [ ] **Step 3: Implement the recompute**

Add imports: `StructId` to the `move_model::model` import list; `Type` via existing `move_model::ty` (import as `use move_model::ty::Type;` if not already available through the module path used in the file).

Add the helper:

```rust
/// The compiler's acquires pass runs before lambda lifting, so lifted
/// functions carry no acquires info. Recompute it with the same rule:
/// direct borrow_global/borrow_global_mut/move_from of same-module structs,
/// joined with same-module static callees, to a fixpoint across the module's
/// lifted functions (non-lifted callees use their compiler-stored value).
fn lifted_acquires(
    env: &GlobalEnv,
    module: &ModuleEnv<'_>,
) -> BTreeMap<FunId, BTreeSet<StructId>> {
    let mid = module.get_id();
    let mut result: BTreeMap<FunId, BTreeSet<StructId>> = BTreeMap::new();
    let mut callees: BTreeMap<FunId, BTreeSet<FunId>> = BTreeMap::new();
    for f in module.get_functions() {
        if !is_lambda_lifted(&f) {
            continue;
        }
        let mut direct = BTreeSet::new();
        let mut calls = BTreeSet::new();
        if let Some(body) = f.get_def() {
            body.visit_pre_order(&mut |e| {
                if let ExpData::Call(node_id, op, _) = e {
                    match op {
                        Operation::MoveFrom | Operation::BorrowGlobal(..) => {
                            if let Some(Type::Struct(s_mid, sid, _)) =
                                env.get_node_instantiation(*node_id).first()
                            {
                                if *s_mid == mid {
                                    direct.insert(*sid);
                                }
                            }
                        },
                        Operation::MoveFunction(c_mid, c_fid) if *c_mid == mid => {
                            calls.insert(*c_fid);
                        },
                        _ => {},
                    }
                }
                true
            });
        }
        result.insert(f.get_id(), direct);
        callees.insert(f.get_id(), calls);
    }
    loop {
        let mut changed = false;
        for (fid, calls) in &callees {
            let mut acc = result[fid].clone();
            for callee in calls {
                match result.get(callee) {
                    Some(lifted) => acc.extend(lifted.iter().copied()),
                    None => {
                        if let Some(stored) =
                            module.get_function(*callee).get_acquired_structs()
                        {
                            acc.extend(stored.iter().copied());
                        }
                    },
                }
            }
            if acc.len() != result[fid].len() {
                result.insert(*fid, acc);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    result
}
```

In `build_module_facts`, compute it once and thread it through (same pattern as `defined_in`):

```rust
let lifted_acq = lifted_acquires(env, module);
let functions: Vec<FunctionFacts> = module
    .get_functions()
    .map(|f| build_function_facts(env, &f, &defined_in, &lifted_acq))
    .collect();
```

In `build_function_facts` (add the parameter `lifted_acq: &BTreeMap<FunId, BTreeSet<StructId>>`), replace the `acquires_inferred` computation:

```rust
let acquired: Option<BTreeSet<StructId>> = if is_lambda_lifted(func) {
    lifted_acq.get(&func.get_id()).cloned()
} else {
    func.get_acquired_structs().cloned()
};
let acquires_inferred: Vec<String> = acquired
    .map(|set| {
        set.iter()
            .map(|sid| {
                func.module_env
                    .get_struct(*sid)
                    .get_full_name_with_address()
            })
            .collect()
    })
    .unwrap_or_default();
```

- [ ] **Step 4: Regenerate baselines, verify, run**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query
git diff -- 'aptos-move/flow/src/tests/**/*.exp'
```
Expected:
- `facts_closure_stored.exp`: both lifted functions have `acquiresInferred: ["0xcafe::stored::Config"]` (one via direct read, one via the `read_config` callee); `setup`/`lifted_calls_helper` have `createsClosures` with the lifted names and empty `resourceAccess`.
- `facts_closure.exp`: `__lambda__1__setup` now `acquiresInferred: ["0xcafe::closures::Config"]`.

```bash
cargo test -p aptos-move-flow -- move_package_query
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Recompute acquires for lambda-lifted functions"
```

---

### Task 5: Remaining golden coverage — invoked closures, inline functions, nested closures, native

**Files:**
- Create: `aptos-move/flow/src/tests/move_package_query/facts_closure_invoked.rs` + `.exp`
- Create: `aptos-move/flow/src/tests/move_package_query/facts_inline.rs` + `.exp`
- Create: `aptos-move/flow/src/tests/move_package_query/facts_native.rs` + `.exp`
- Modify: `aptos-move/flow/src/tests/move_package_query/facts_nested.rs` (extend fixture with a storage op)
- Modify: `aptos-move/flow/src/tests/move_package_query/mod.rs`

- [ ] **Step 1: Write the tests**

`facts_closure_invoked.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Closure invocation vs creation: `run` invokes an unknown function value
/// (flagged incomplete); `immediate` creates and invokes its own closure
/// (known target, complete); `chain` passes a closure to a same-module
/// invoker (creation recorded on `chain`, unknown invoke stays on `run`).
#[tokio::test]
async fn move_package_query_facts_closure_invoked() {
    let pkg = common::make_package("facts_closure_invoked", &[(
        "invoked",
        "module 0xCAFE::invoked {
    struct R has key { v: u64 }

    fun run(f: ||u64): u64 {
        f()
    }

    public fun immediate(): u64 {
        (|| borrow_global<R>(@0xCAFE).v)()
    }

    public fun chain(): u64 {
        run(|| borrow_global<R>(@0xCAFE).v)
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

`facts_inline.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Inline functions: the definition keeps its own body facts (`isInline`),
/// and callers absorb the inlined effects (reads AND acquires) because
/// inlining happens before the acquires pass. Consumers must not
/// double-count; see the FunctionFacts docs.
#[tokio::test]
async fn move_package_query_facts_inline() {
    let pkg = common::make_package("facts_inline", &[(
        "inl",
        "module 0xCAFE::inl {
    struct Config has key { v: u64 }

    inline fun inline_read(): u64 {
        borrow_global<Config>(@0xCAFE).v
    }

    public fun uses_inline(): u64 {
        inline_read()
    }

    public fun uses_inline_with_lambda(): u64 {
        apply_inline(|x| x + borrow_global<Config>(@0xCAFE).v)
    }

    inline fun apply_inline(f: |u64|u64): u64 {
        f(1)
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

`facts_native.rs`:

```rust
// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Native functions have no analyzable body: `effectsComplete: false`.
#[tokio::test]
async fn move_package_query_facts_native() {
    let pkg = common::make_package("facts_native", &[(
        "nat",
        "module 0xCAFE::nat {
    native fun ffi(x: u64): u64;

    public fun wrapper(x: u64): u64 {
        ffi(x)
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
```

(If the native fixture fails to build the model, the test output will show the build error via the tool result — in that case declare the native inside a `#[test_only]`-free module is not an option; instead drop `facts_native.rs` and note in the PR that native coverage comes from the `effects_complete` expression `func.get_def().is_some()` being exercised by every native-less baseline. Check the result before committing.)

Extend `facts_nested.rs` fixture: change the `outer` function body so the innermost lambda reads a resource. Replace the module source with:

```move
module 0xCAFE::nested {
    struct R has key { v: u64 }

    fun apply(f: |u64|u64, x: u64): u64 { f(x) }

    public fun outer(c: u64): u64 {
        apply(|x| apply(|y| x + y + c + borrow_global<R>(@0xCAFE).v, x), 10)
    }
}
```

Register new mods in `mod.rs`:

```rust
mod facts_closure_invoked;
mod facts_inline;
mod facts_native;
```

- [ ] **Step 2: Regenerate all baselines and inspect semantics**

```bash
UB=1 cargo test -p aptos-move-flow -- move_package_query
git diff -- 'aptos-move/flow/src/tests/**/*.exp'
```
Expected:
- `facts_closure_invoked.exp`: `run` → `invokesFunctionValues: true`, `effectsComplete: false`; `immediate` → `createsClosures: [..__lambda__..]`, `invokesFunctionValues: false` (callee is a known closure), `effectsComplete: true`; `chain` → `createsClosures`, complete; lifted fns → `reads: [R]`, `acquiresInferred: [R]`.
- `facts_inline.exp`: `inline_read` → `isInline: true`, `reads/acquires [Config]`; `uses_inline` → `reads/acquires [Config]` (absorbed), `effectsComplete: true`; `uses_inline_with_lambda` → lambda body inlined into the caller, so `reads: [Config]` directly (lambdas passed to inline functions are beta-reduced by the inliner, not lifted).
- `facts_nested.exp`: innermost lifted fn carries `reads: [R]` + recomputed acquires; `outer` only `createsClosures`.
- `facts_native.exp`: `ffi` → `isNative: true`, `effectsComplete: false`, empty access.

If any expectation does NOT hold, STOP and investigate before committing — the baseline must never encode behavior that contradicts the spec without an explicit decision.

- [ ] **Step 3: Run the full move_package_query suite**

```bash
cargo test -p aptos-move-flow -- move_package_query
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Add golden tests for closure, inline, native facts"
```

---

### Task 6: Documentation contract and final verification

**Files:**
- Modify: `aptos-move/flow/src/mcp/tools/package_query.rs` (doc comments only)

- [ ] **Step 1: Add the documentation contract**

On `FunctionFacts` (struct-level doc comment; NOT on the `QueryType::Facts` variant, whose doc feeds the tool JSON schema and would churn `list_tools/success.exp`):

```rust
/// Per-function facts.
///
/// Effect-model contract:
/// - `acquiresInferred` is the compiler-checked value: post-inlining,
///   pre-lambda-lifting, same-module resources only, no type arguments.
///   Lambda bodies count toward the enclosing function (language rule).
///   For lambda-lifted functions it is recomputed with the same rule.
/// - `resourceAccess` covers this body only, fully qualified with type
///   arguments; `exists<T>` is a read but never an acquire. Cross-module
///   resources appear here but never in `acquiresInferred`.
/// - Inline function bodies are expanded into callers before analysis:
///   callers legitimately absorb the effects, and the inline definition
///   also reports its own. Do not double-count.
/// - When `effectsComplete` is false, all effect fields are lower bounds.
struct FunctionFacts {
```

- [ ] **Step 2: Verify no baseline churn from docs, then run the full crate suite**

```bash
cargo test -p aptos-move-flow
```
Expected: PASS, no `.exp` diffs (`git status --short` clean except intended files).

- [ ] **Step 3: Lint and format**

```bash
cargo xclippy -p aptos-move-flow
cargo +nightly fmt -p aptos-move-flow
git diff --stat
```
Expected: clippy clean; fmt produces no or trivial diffs (include them in the commit).

- [ ] **Step 4: Commit**

```bash
git add aptos-move/flow/src
git commit -m "[move-flow] Document the facts effect-model contract"
```

---

### Task 7: Completion gate

- [ ] **Step 1: Verify all spec requirements have landed**

Walk the spec's test-plan list (items 1-10) against `aptos-move/flow/src/tests/move_package_query/`; every item maps to a committed test or a recorded decision (native fallback).

- [ ] **Step 2: Independent review gate**

Run a code review (deep-reasoner or Codex) over `git diff main...HEAD -- aptos-move/flow` in the worktree, focused on: soundness of `scan_body` guards, fixpoint termination in `lifted_acquires`, and baseline semantics. Fix findings before declaring done.

- [ ] **Step 3: Report**

Summarize commits, test counts, and any deviations from the spec to the user. Do not push or open a PR without explicit instruction.
