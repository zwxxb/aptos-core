```markdown
# aptos-core Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill introduces the core development patterns and workflows used in the `aptos-core` repository, a Rust codebase focused on blockchain infrastructure. It covers coding conventions, common workflows for extending Move Flow features and tests, and documentation practices. By following these patterns, contributors can ensure consistency and quality across the project.

## Coding Conventions

### File Naming
- **Style:** `snake_case`
- **Example:**  
  ```text
  package_query.rs
  source_check.rs
  mod.rs
  ```

### Imports
- **Style:** Relative imports are preferred.
- **Example:**
  ```rust
  use crate::mcp::tools::package_query;
  use super::source_check;
  ```

### Exports
- **Style:** Named exports.
- **Example:**
  ```rust
  pub fn run_query() { ... }
  pub struct PackageInfo { ... }
  ```

## Workflows

### Extend Move Flow Package Query and Update Golden Tests
**Trigger:** When adding or changing facts, effect models, or related features in Move Flow's `package_query`, requiring new or updated golden tests.  
**Command:** `/update-move-flow-package-query-tests`

1. Edit or extend logic in `aptos-move/flow/src/mcp/tools/package_query.rs`.
2. Add or update `.exp` and `.rs` golden test files in `aptos-move/flow/src/tests/move_package_query/`.
3. Update `mod.rs` in the same test directory to register new tests.

**Example:**
```rust
// In package_query.rs
pub fn new_feature() {
    // implementation
}
```
```rust
// In move_package_query/test_new_feature.rs
#[test]
fn test_new_feature() {
    // test logic
}
```
```rust
// In move_package_query/mod.rs
mod test_new_feature;
```

---

### Add or Update Edit Hook and Tests
**Trigger:** When adding or fixing edit-hook logic for Move source or package files, requiring new or updated golden tests.  
**Command:** `/update-edit-hook-tests`

1. Edit or extend logic in `aptos-move/flow/src/hooks/source_check.rs` and/or `aptos-move/flow/src/hooks/package_path.rs`.
2. Add or update `.exp` and `.rs` golden test files in `aptos-move/flow/src/tests/edit_hook/`.
3. Update `mod.rs` in the same test directory to register new tests.

**Example:**
```rust
// In source_check.rs
pub fn validate_source() {
    // implementation
}
```
```rust
// In edit_hook/test_validate_source.rs
#[test]
fn test_validate_source() {
    // test logic
}
```
```rust
// In edit_hook/mod.rs
mod test_validate_source;
```

---

### Add Design Spec or Plan Documentation
**Trigger:** When documenting a new feature, plan, or audit for Move Flow.  
**Command:** `/add-design-spec`

1. Create a new markdown file in `docs/superpowers/specs/` or `docs/superpowers/plans/`.
2. Write the design spec or plan.
3. Commit the new documentation file.

**Example:**
```text
docs/superpowers/specs/my_feature_spec.md
docs/superpowers/plans/my_feature_plan.md
```

## Testing Patterns

- **Framework:** Unknown (Rust standard testing likely)
- **File Pattern:** Tests are typically placed in `.rs` files alongside `.exp` files for golden test outputs.
- **Test Registration:** New test files must be registered in the corresponding `mod.rs`.

**Example:**
```rust
// In tests/move_package_query/test_something.rs
#[test]
fn test_something() {
    // test logic
}
```
```rust
// In tests/move_package_query/mod.rs
mod test_something;
```

## Commands

| Command                                   | Purpose                                                        |
|--------------------------------------------|----------------------------------------------------------------|
| /update-move-flow-package-query-tests      | Update or add Move Flow package_query logic and golden tests    |
| /update-edit-hook-tests                    | Update or add edit hook logic and golden tests                  |
| /add-design-spec                           | Add a new design spec or plan document                         |
```
