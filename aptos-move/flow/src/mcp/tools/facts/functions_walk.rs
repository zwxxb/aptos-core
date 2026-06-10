// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-function walk: emits [`Function`] including the unified
//! `resourceAccess` set. Constant references are intentionally not emitted
//! (named constants are folded before the model is built).

use super::types::Function;
use move_model::model::{GlobalEnv, ModuleEnv};

/// Emit a [`Function`] for every function declared in `m`.
pub(crate) fn walk_functions(_env: &GlobalEnv, _m: &ModuleEnv<'_>, _file: &str) -> Vec<Function> {
    // Filled in by Task 4.
    Vec::new()
}
