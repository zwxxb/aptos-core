---
name: add-or-update-edit-hook-and-tests
description: Workflow command scaffold for add-or-update-edit-hook-and-tests in aptos-core.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /add-or-update-edit-hook-and-tests

Use this workflow when working on **add-or-update-edit-hook-and-tests** in `aptos-core`.

## Goal

Implements or fixes logic in Move Flow edit hooks (source_check.rs, package_path.rs) and adds or updates corresponding golden tests.

## Common Files

- `aptos-move/flow/src/hooks/source_check.rs`
- `aptos-move/flow/src/hooks/package_path.rs`
- `aptos-move/flow/src/tests/edit_hook/*.exp`
- `aptos-move/flow/src/tests/edit_hook/*.rs`
- `aptos-move/flow/src/tests/edit_hook/mod.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit or extend logic in aptos-move/flow/src/hooks/source_check.rs and/or aptos-move/flow/src/hooks/package_path.rs
- Add or update .exp and .rs golden test files in aptos-move/flow/src/tests/edit_hook/
- Update mod.rs in the same test directory to register new tests

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.