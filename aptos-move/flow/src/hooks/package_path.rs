// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! UserPromptSubmit hook that detects the current Move package.
//!
//! Walks up from the current working directory looking for `Move.toml`.
//! When found, outputs JSON with `additionalContext` so the AI assistant
//! knows which package the user is working in. Outputs nothing if no
//! package is found. Always exits 0.

use anyhow::Result;
use std::path::Path;

/// Entry point: detect the nearest Move package and emit context.
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    if let Some(pkg_path) = find_package_root(&cwd) {
        let manifest = pkg_path.join("Move.toml");
        let pkg_name = read_package_name(&manifest).unwrap_or_else(|| "(unknown)".to_string());
        let ctx = format!(
            "Current Move package: {} at {}.",
            pkg_name,
            pkg_path.display()
        );
        let output = serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
                "additionalContext": ctx
            }
        });
        println!("{}", output);
    }
    Ok(())
}

/// Read the exact `name` key from the `[package]` section.
pub(crate) fn read_package_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_package_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            let header = trimmed.split('#').next().unwrap_or("").trim();
            in_package_section = header
                .strip_prefix('[')
                .and_then(|h| h.strip_suffix(']'))
                .is_some_and(|h| h.trim() == "package");
            continue;
        }
        if !in_package_section {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let value = value.split('#').next().unwrap_or("").trim();
        let name = value
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('\'');
        return Some(name.to_string());
    }
    None
}

/// Find the Move package root by walking up from the given directory.
fn find_package_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut dir = start;
    loop {
        let manifest = dir.join("Move.toml");
        if manifest.is_file() {
            return Some(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}
