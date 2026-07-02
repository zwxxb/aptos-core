# Move-flow Audit Fixes Implementation Plan

> Executed with subagent-driven development on branch `move-flow-facts-soundness`
> (worktree `.worktrees/pr-20174`). Fixes the verified findings from the
> 2026-07-02 correctness audit of move-flow's Move-code analysis.

**Goal:** Fix the five verified correctness defects plus the verified
inconsistencies found by the post-facts audit (query tools, edit hooks, facts
residuals).

**Design calls (lead decisions):**
- Move 2 access specifiers surface as new per-function facts fields
  `readsDeclared` / `writesDeclared` (`AccessSpecifierKind::Reads`/`Writes`);
  `acquiresDeclared` keeps `LegacyAcquires` and now preserves declared type
  arguments. `ResourceSpecifier::Any` renders `*`, `DeclaredAtAddress` renders
  `0xADDR::*`, `DeclaredInModule` renders `addr::module::*`; negated specifiers
  get a `!` prefix. Address-clause constraints (`reads R(addr)`) are not
  represented — documented.
- `call_graph` keeps its adjacency-map wire shape, documented as static call
  edges only; `function_usage` gains `invokes_function_values` and
  `creates_closures`; both queries gain the compile-error gate facts already
  has. `dep_graph`/`module_summary` stay ungated (structural).
- `module_summary` builds its signature string locally (qualified types,
  `package` visibility, `, ` separator) instead of `get_header_string`.

## Tasks

1. **Facts: declared access specifiers** — `readsDeclared`/`writesDeclared`
   fields, `acquiresDeclared` type args, doc-contract note that
   `#[verify_only]` items appear (build runs verify mode). Golden test
   `facts_access_specifiers` (a `writes Config` body-pure function must no
   longer read as fully pure+complete).
2. **Hooks: text checks + manifest parsing** — identifier/receiver boundary
   for `borrow_global<`/`borrow_global_mut<` patterns; accept `>` before
   `acquires`; rewrite `read_package_name` (section-aware, exact key, comment
   stripping). Edit-hook + unit tests.
3. **Hooks: spec walker** — classify `invariant` as loop-invariant only within
   loop context; add missing traversal arms (struct/vector literals, match)
   to both walkers. Edit-hook tests.
4. **Queries: soundness signals** — compile-error gate for
   `call_graph`/`function_usage`; `resolve_function` ambiguity error + no
   `expect_numerical` panic; `function_usage` unknown-dispatch fields;
   `call_graph`/`FunctionUsage` doc notes; `is_view` requires `Apply`-form
   attribute. Tests + baseline updates (incl. `list_tools`).
5. **module_summary local signatures** — qualified types, package visibility,
   comma spacing. Baseline regen.
6. **Hardening + verification** — `lifted_acquires` non-panicking lookups;
   hook language version derived from latest instead of pinned `V2_5` (or a
   commented single-point constant if no conversion exists); full suite,
   clippy, fmt, final adversarial review gate.

Out of scope (documented, not fixed): script pseudo-module address rendering,
`Value::AddressArray`/`Tuple` Debug fallback, dep_graph friend semantics
(doc note only), `defined_in` single-creator invariant (correct today, tested).
