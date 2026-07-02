---
name: extend-move-flow-package-query-and-update-golden-tests
description: Workflow command scaffold for extend-move-flow-package-query-and-update-golden-tests in aptos-core.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /extend-move-flow-package-query-and-update-golden-tests

Use this workflow when working on **extend-move-flow-package-query-and-update-golden-tests** in `aptos-core`.

## Goal

Implements or modifies logic in the Move Flow package_query tool and updates or adds corresponding golden test files to validate the new logic.

## Common Files

- `aptos-move/flow/src/mcp/tools/package_query.rs`
- `aptos-move/flow/src/tests/move_package_query/*.exp`
- `aptos-move/flow/src/tests/move_package_query/*.rs`
- `aptos-move/flow/src/tests/move_package_query/mod.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit or extend logic in aptos-move/flow/src/mcp/tools/package_query.rs
- Add or update .exp and .rs golden test files in aptos-move/flow/src/tests/move_package_query/
- Update mod.rs in the same test directory to register new tests

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.