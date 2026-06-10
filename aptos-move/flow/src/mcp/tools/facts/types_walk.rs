// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-struct / per-enum walk: emits [`TypeDef`] with resolved `typeRefs`.

use super::types::TypeDef;
use move_model::model::{GlobalEnv, ModuleEnv};

/// Emit a [`TypeDef`] for every struct and enum declared in `m`.
pub(crate) fn walk_types(_env: &GlobalEnv, _m: &ModuleEnv<'_>, _file: &str) -> Vec<TypeDef> {
    // Filled in by Task 3.
    Vec::new()
}
