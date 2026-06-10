// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `move_package_facts` response builder.
//!
//! A single pass over a compiled `GlobalEnv` produces the v2 contract payload
//! consumed by GitNexus. The wire schema lives in [`types`]; [`convert`] holds
//! the canonicalization/span/type-ref helpers; the `*_walk` modules emit one
//! kind of fact each; [`builder`] orchestrates them and assembles the package
//! header, diagnostics, and per-module facts.

mod builder;
mod calls_walk;
mod convert;
mod functions_walk;
pub(crate) mod types;
mod types_walk;

pub(crate) use builder::build_facts;
pub(crate) use types::MovePackageFacts;
