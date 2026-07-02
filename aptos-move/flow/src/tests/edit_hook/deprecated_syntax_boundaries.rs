// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Boundary tests for deprecated-syntax detection.
//!
//! Verifies that the `borrow_global<`/`borrow_global_mut<` pattern checks do
//! not fire on user identifiers that merely *contain* the pattern (false
//! positives), and that genuine bare builtin calls and `>acquires` (no space
//! between closing generic and keyword) are still caught.
//!
//! Receiver-style calls (`w.borrow_global<T>(..)`) are covered by the
//! preceding-byte `.` check in `is_bare_builtin_use`; they are not exercised
//! here because the legacy Move 1 parser used by the edit hook does not parse
//! Move 2 receiver syntax, so a fixture with `.borrow_global` in a call
//! position would fail to parse rather than reach `text_checks`.

use crate::{hooks::source_check, tests::common};

/// User functions/structs whose names contain the deprecated pattern substrings
/// must not produce diagnostics.
#[test]
fn edit_hook_deprecated_syntax_boundaries_no_false_positives() {
    // `raw_borrow_global` contains "borrow_global" but is a user identifier.
    let source = r#"module 0xCAFE::boundaries {
    struct BorrowToken has key { value: u64 }

    fun raw_borrow_global<T>(_a: address): u64 {
        0
    }

    fun raw_borrow_global_mut<T>(_a: address): u64 {
        0
    }

    fun call_user_fns(addr: address): u64 {
        raw_borrow_global<u64>(addr) + raw_borrow_global_mut<u64>(addr)
    }
}
"#;
    let result = source_check::check("deprecated_syntax_boundaries_clean.move", source);
    assert!(
        !result.has_errors,
        "unexpected diagnostics on user identifiers:\n{}",
        result.output
    );
    assert!(!result.has_parse_errors);
    assert!(
        result.output.is_empty(),
        "unexpected output:\n{}",
        result.output
    );
}

/// Genuine bare builtin calls and `>acquires` (no space) must still be flagged.
#[test]
fn edit_hook_deprecated_syntax_boundaries_genuine_hits() {
    // Contains:
    //   - bare `borrow_global<u64>` call                  → flagged
    //   - ): &vector<u64>acquires R (no space before acquires) → flagged
    let source = r#"module 0xCAFE::genuine {
    struct R has key { value: u64 }

    fun get(addr: address): u64 acquires R {
        borrow_global<R>(addr).value
    }

    fun get_typed(_x: &vector<u64>): u64 acquires R {
        0
    }
}
"#;
    let result = source_check::check("deprecated_syntax_boundaries_hits.move", source);
    assert!(result.has_errors, "expected diagnostics but got none");
    assert!(!result.has_parse_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
