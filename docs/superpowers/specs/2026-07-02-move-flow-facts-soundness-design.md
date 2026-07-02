# Move-flow facts soundness: resource effects, acquires, closures, function values

**Date:** 2026-07-02
**Status:** Approved
**Scope:** `aptos-move/flow/src/mcp/tools/package_query.rs` (facts query of `move_package_query`) and its tests. No compiler changes.
**Base:** Stacked on PR #20174 (`f24da52ee9`, local worktree `.worktrees/pr-20174`), which adds `returnTypes`, `isLambdaLifted`, `definedIn`, qualified type rendering, and attribute values.

## Problem

The `facts` query emits per-function effect claims that are unsound or internally
inconsistent. Root causes, verified against compiler source and live probes:

1. **Function-value invocation is invisible.** `ExpData::Invoke` (calling a `|T|R`
   value) is counted neither by the compiler's acquires fixpoint (which only sees
   `Operation::MoveFunction`) nor by `build_resource_access`. A function like
   `fun foo(bar: |u64|u64): u64 { bar(1) }` emits
   `acquiresInferred: [], resourceAccess: {reads: [], writes: []}` — a false purity
   claim.
2. **Lambda-lifted functions claim no acquires.** The acquires checker
   (`third_party/move/move-compiler-v2/src/env_pipeline/acquires_checker.rs`) runs
   *before* lambda lifting (`move-compiler-v2/src/lib.rs` pipeline: inlining →
   acquires check → lambda lifting → closure check), so lifted functions have
   `acquired_structs = None`, emitted as `[]` — even when their body directly does
   `borrow_global<Config>`. Inconsistent with their own `resourceAccess.reads`.
3. **Closure creation is indistinguishable from invocation.** Because the acquires
   pass runs pre-lifting, lambda-body acquires are attributed to the *enclosing*
   function (this is the language rule: the compiler demands `acquires Config` on a
   function that merely stores `|| borrow_global<Config>(..)` when any acquires are
   declared). The facts carry no creator→closure link, so a consumer cannot tell
   "stores a callback that could read Config" from "reads Config when called".
4. **Spec-block leakage.** `build_resource_access` walks the whole body with no
   `ExpData::SpecBlock` guard (the acquires checker has one), so `exists<T>` inside
   spec assertions counts as a runtime read.
5. **Facts from broken builds.** When the package has compilation errors the env
   pipeline stops mid-way (e.g. before lambda lifting), yet `facts` silently emits
   data from the partially transformed env with different attribution than a clean
   build. Verified live: an acquires error yields facts with no lifted functions and
   lambda reads attributed to the enclosing function.
6. **Correlation mismatch (documented, not changed).** `acquiresInferred` is
   same-module-only and carries no type instantiation (the compiler stores
   `BTreeSet<StructId>`; indirect entries have no instantiation to recover), while
   `resourceAccess` is fully qualified and instantiated.

Already correct (needs a pinning test only): transitive acquires do not leak into
`resourceAccess` — `caller() { helper() }` where `helper` borrows `R` gets
`acquiresInferred: [R]`, `resourceAccess: {}`.

## Design

Additive schema extension. Keep `acquiresInferred` compiler-faithful; expose the
missing distinctions as new primitive fields consumers compose with `call_graph`.

### Per-function schema deltas (wire names)

| Field | Type | Semantics |
|---|---|---|
| `acquiresInferred` | `string[]` | Unchanged for regular functions: the compiler-checked acquires (post-inlining, pre-lifting, same-module, no type args). **For lambda-lifted functions:** recomputed by move-flow with the same rule — direct `borrow_global`/`borrow_global_mut`/`move_from` on same-module structs, joined with the acquires of same-module static callees, iterated to fixpoint among lifted functions (non-lifted callees use their compiler-stored value). |
| `acquiresDeclared` | `string[]` *(new)* | What the programmer wrote: `get_access_specifiers()` filtered to `AccessSpecifierKind::LegacyAcquires`, rendered as qualified struct names. |
| `resourceAccess` | `{reads, writes}` | Still direct-body-only, but the walk now skips `ExpData::SpecBlock` subtrees and (defensively) `ExpData::Lambda` subtrees. Op mapping unchanged: `exists`/`borrow_global` → read; `borrow_global_mut`/`move_from` → read+write; `move_to` → write. Fully qualified with type args. |
| `createsClosures` | `string[]` *(new)* | Qualified function names of every `Operation::Closure(mid, fid, _)` target in the body, deduplicated and sorted. Complements `definedIn` on the lifted function: creator↔closure is bidirectional. A stored callback's latent effects live on the lifted function's facts, not on the creator's. |
| `invokesFunctionValues` | `bool` *(new)* | True iff the body contains an `ExpData::Invoke` whose callee expression is not statically resolvable to an `Operation::Closure` target. |
| `effectsComplete` | `bool` *(new)* | False iff the function is native, has no AST body, or `invokesFunctionValues` is true. When false, `resourceAccess`/`acquiresInferred` are lower bounds and consumers must widen conservatively. Calling *named* functions does **not** clear this flag: direct facts compose with `call_graph` by design. |

### Behavior change

`move_package_query` with `query: facts` returns a tool error when
`PackageData::has_compilation_errors()` is true, directing the caller to
`move_package_status`. Other query types (structural: `dep_graph`,
`module_summary`, `call_graph`, `function_usage`) are unchanged.

### Documentation contract (doc comments on the emitted structs)

- `acquiresInferred` follows the language's source-syntactic attribution: closure
  bodies count toward the enclosing function; same-module resources only; no type
  arguments.
- Inline functions: their bodies are expanded into callers before the acquires
  pass, so callers legitimately absorb the reads/writes/acquires; the inline
  definition is also kept and emits its own facts. Consumers must not double-count.
- `exists<T>` is a `resourceAccess` read but never an acquire (language rule).
- Cross-module resource access appears in `resourceAccess` (qualified name shows
  the defining module) but never in `acquiresInferred`.

## Not doing

- No structured per-site effect objects (operation kind, viaFunction, latent
  accesses). The additive primitives cover the soundness gap at a fraction of the
  schema churn; revisit if consumers demonstrably need per-site data.
- No `crossModule` boolean: derivable from the qualified resource name.
- No type-instantiation retrofit for `acquiresInferred`: the compiler stores bare
  `StructId`s; partial instantiation (direct only) would be misleadingly mixed.
- No compiler-pass changes.
- No error gating for the structural query types.

## Test plan

Golden `.exp` tests under `aptos-move/flow/src/tests/move_package_query/`
(regenerate with `UB=1 cargo test -p aptos-move-flow`):

1. `facts_acquires_transitive` — helper borrows `R`, caller calls helper: caller
   has `acquiresInferred: [R]`, empty `resourceAccess`, `effectsComplete: true`.
2. `facts_function_value_param` — `fun foo(bar: |u64|u64) { bar(1) }`:
   `invokesFunctionValues: true`, `effectsComplete: false`, empty access lists.
3. `facts_closure_stored` — `register_callback(|x| borrow_global<Config>(..).v + x)`
   stored into a resource, never invoked: creator has `createsClosures:
   [..__lambda__1__setup]`, empty `resourceAccess`; lifted fn has recomputed
   `acquiresInferred: [Config]` and `reads: [Config]`.
4. `facts_closure_invoked` — `run(f: ||u64) { f() }` plus an IIFE
   `(|| borrow_global<R>(..))()`: invoker of unknown value flagged; IIFE body shows
   both `createsClosures` and (if callee resolvable) no unknown flag.
5. `facts_inline` — inline function borrowing `Config` plus its caller: both carry
   `reads/acquires [Config]`; pins the double-count documentation.
6. `facts_nested_closures` — nested lambdas with storage ops: effects attributed to
   the innermost lifted function; `definedIn` chains; creators only link.
7. `facts_spec_block` — `exists<R>` inside a spec block only: excluded from
   `resourceAccess`.
8. `facts_native` — native function: `effectsComplete: false`.
9. `facts_compile_error` — package with an acquires error: tool error, no facts.
10. Baseline updates for existing `facts.exp`, `facts_closure.exp`,
    `facts_nested.exp` (new fields; `facts_closure.exp`'s `run`/`apply` now show
    `invokesFunctionValues: true`, `effectsComplete: false`).

Recursion smoke case (same-module mutual recursion with distinct direct acquires)
is folded into test 1's fixture.

## Verification

`cargo test -p aptos-move-flow -- move_package_query` first, then full
`cargo test -p aptos-move-flow`, then `cargo xclippy -p aptos-move-flow` and
`cargo +nightly fmt`.
