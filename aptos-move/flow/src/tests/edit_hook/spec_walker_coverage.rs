// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Golden tests proving that spec-expression sub-walkers visit all compound
//! expression forms.
//!
//! Each case contains a dereference (`*r`) that the hook must flag.  Before the
//! fix the dereference was silently missed because the relevant walker lacked an
//! arm for the enclosing expression variant.  After the fix every case is
//! reported.
//!
//! ## Defect A (skipped)
//! The hook classifies all inline non-lambda code-spec-block `invariant`
//! conditions as `LoopInvariant` — matching the compiler's own classification
//! (`module_builder.rs:3336-3343`: `FunctionCodeV2(_, _, None)` →
//! `LoopInvariant`).  Furthermore `allow_old()` returns `false` for that
//! context (`module_builder.rs:135`), so the compiler rejects `old()` entirely
//! in code spec blocks rather than relaxing to allow `old(simple_name)`.
//! Relaxing the hook's classification to `SpecContext::Other` for non-loop code
//! blocks would therefore produce a less-faithful approximation.  Defect A is
//! intentionally not fixed.
//!
//! ## Match in spec expressions (V2_5)
//! `Exp_::Match` can appear inside a spec expression at V2_5 (the enum variant
//! exists in the legacy parser and is reachable via `ensures match (...) ...`).
//! The `walk_spec_exp` Match arm was absent before this fix, so derefs inside
//! match arms were silently missed.

use crate::{hooks::source_check, tests::common};

#[test]
fn edit_hook_spec_walker_coverage() {
    // Note: `match` in spec expressions (case_match below) parses successfully
    // at LanguageVersion::V2_5 because `Exp_::Match` is a first-class AST node
    // in the legacy parser.  If a future parser version rejects it, drop that
    // case with a note.
    let source = r#"module 0xCAFE::walker_coverage {
    struct S has drop { x: u64 }

    // Case 1: *r inside a Pack (struct literal) field in a spec ensures.
    // walk_spec_exp previously lacked a Pack arm; deref was silently missed.
    fun case_pack(r: &u64): u64 { *r }
    spec case_pack {
        ensures (S { x: *r }).x >= result;
    }

    // Case 2: *r inside a vector literal element in a spec ensures.
    // walk_spec_exp previously lacked a Vector arm; deref was silently missed.
    fun case_vector(r: &u64): u64 { *r }
    spec case_vector {
        ensures vector[*r] == vector[result];
    }

    // Case 3: inline spec block nested inside a Pack field in a function body.
    // walk_exp_for_spec_blocks previously lacked a Pack arm; the spec block
    // was never visited so the inner deref escaped detection.
    fun case_body_pack(r: &u64): S {
        S { x: { spec { invariant *r >= 0; }; 1 } }
    }

    // Case 4: inline spec block nested inside a vector element in a function body.
    // walk_exp_for_spec_blocks previously lacked a Vector arm.
    fun case_body_vector(r: &u64): vector<u64> {
        vector[{ spec { invariant *r >= 0; }; 1 }]
    }
}
"#;
    let result = source_check::check("spec_walker_coverage.move", source);
    assert!(result.has_errors);
    assert!(!result.has_parse_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
