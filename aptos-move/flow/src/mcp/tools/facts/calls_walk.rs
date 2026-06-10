// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-call-site walk: emits [`CallEdge`] tagged `direct | inline | closure`,
//! each with its own span and a confidence weight.

use super::types::CallEdge;
use move_model::model::GlobalEnv;

/// Emit one [`CallEdge`] per resolved call site across all target modules.
pub(crate) fn walk_calls(_env: &GlobalEnv) -> Vec<CallEdge> {
    // Filled in by Task 5.
    Vec::new()
}
