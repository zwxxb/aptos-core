// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Drives the single facts pass: package header, per-module facts, explicit
//! `use` statements, and structured diagnostics, assembled into the v2 payload.
//!
//! Best-effort partial output falls out naturally: modules that failed to
//! parse never enter the `GlobalEnv`, so iterating the target modules yields
//! only the ones that compiled. Whole-package failures are surfaced as MCP
//! errors upstream in `resolve_package`, before this runs.

use super::convert::{
    attribute_names, constant_qname, is_error_code, module_display_name, module_qname, span_of,
};
use super::types::*;
use super::{calls_walk, functions_walk, types_walk};
use codespan::{ByteIndex, FileId, Span as CsSpan};
use codespan_reporting::diagnostic::{Diagnostic as CsDiagnostic, Severity as CsSeverity};
use move_model::model::{GlobalEnv, Loc, ModuleEnv};
use std::collections::BTreeMap;
use std::path::Path;

/// Build the full v2 facts payload for an already-compiled package.
pub(crate) fn build_facts(env: &GlobalEnv, package_path: &str) -> MovePackageFacts {
    let mut modules = BTreeMap::new();
    for m in env.get_primary_target_modules() {
        modules.insert(module_qname(env, &m), build_module(env, &m, package_path));
    }
    MovePackageFacts {
        schema_version: SCHEMA_VERSION,
        package: build_package_header(package_path),
        diagnostics: collect_diagnostics(env, package_path),
        modules,
        call_graph: calls_walk::walk_calls(env),
    }
}

fn build_module(env: &GlobalEnv, m: &ModuleEnv<'_>, package_path: &str) -> Module {
    let file = repo_relative(&m.get_source_path().to_string_lossy(), package_path);
    let constants = m
        .get_named_constants()
        .map(|c| {
            let ctx = c.get_type_display_ctx();
            let name = c.get_name().display(env.symbol_pool()).to_string();
            Constant {
                qualified_name: constant_qname(env, &c),
                is_error_code: is_error_code(&name),
                type_: c.get_type().display(&ctx).to_string(),
                value: env.display(&c.get_value()).to_string(),
                span: span_of(env, &c.get_loc()),
                name,
            }
        })
        .collect();
    let friends = m
        .get_friend_modules()
        .iter()
        .map(|mid| module_qname(env, &env.get_module(*mid)))
        .collect();
    Module {
        address: super::convert::module_address_str(m),
        name: m.get_name().name().display(env.symbol_pool()).to_string(),
        qualified_name: module_qname(env, m),
        display_name: module_display_name(env, m),
        span: span_of(env, &m.get_loc()),
        attributes: attribute_names(env, m.get_attributes()),
        has_spec: !m.get_spec().is_empty(),
        friends,
        uses: walk_uses(env, m),
        constants,
        types: types_walk::walk_types(env, m, &file),
        functions: functions_walk::walk_functions(env, m, &file),
        file,
    }
}

/// One [`Use`] per source-level import, expanding member uses into one entry
/// per imported member. Unresolved imports (no `module_id`) are skipped.
fn walk_uses(env: &GlobalEnv, m: &ModuleEnv<'_>) -> Vec<Use> {
    let mut out = Vec::new();
    for decl in m.get_use_decls() {
        let Some(module_id) = decl.module_id else {
            continue;
        };
        let module_qn = module_qname(env, &env.get_module(module_id));
        if decl.members.is_empty() {
            out.push(Use {
                target: module_qn,
                alias: decl.alias.map(|s| s.display(env.symbol_pool()).to_string()),
                span: span_of(env, &decl.loc),
            });
        } else {
            for (member_loc, member, member_alias) in &decl.members {
                out.push(Use {
                    target: format!("{}::{}", module_qn, member.display(env.symbol_pool())),
                    alias: member_alias.map(|s| s.display(env.symbol_pool()).to_string()),
                    span: span_of(env, member_loc),
                });
            }
        }
    }
    out
}

fn build_package_header(package_path: &str) -> PackageHeader {
    let raw = std::fs::read_to_string(Path::new(package_path).join("Move.toml")).unwrap_or_default();
    let parsed: toml::Value = raw.parse().unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or_default()
        .to_string();
    PackageHeader {
        name,
        addresses: read_addresses(&parsed, "addresses"),
        dev_addresses: read_addresses(&parsed, "dev-addresses"),
        root_file: "Move.toml".to_string(),
    }
}

fn read_addresses(parsed: &toml::Value, table: &str) -> BTreeMap<String, String> {
    parsed
        .get(table)
        .and_then(|a| a.as_table())
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), normalize_addr(s))))
                .collect()
        })
        .unwrap_or_default()
}

/// Canonical short hex for a manifest address literal. Placeholders (`_`) and
/// non-hex values normalize to `0x0`.
fn normalize_addr(s: &str) -> String {
    let Some(hex) = s.strip_prefix("0x") else {
        return "0x0".to_string();
    };
    let trimmed = hex.trim_start_matches('0').to_ascii_lowercase();
    if trimmed.is_empty() {
        "0x0".to_string()
    } else {
        format!("0x{}", trimmed)
    }
}

/// Collect structured diagnostics from the compiler's reporting channel.
/// Warnings and above are included; `[]` on a clean compile.
fn collect_diagnostics(env: &GlobalEnv, package_path: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    env.report_diag_with_filter(
        |files, diag| out.push(to_wire_diagnostic(env, files, diag, package_path)),
        |d| d.severity >= CsSeverity::Warning,
    );
    out
}

fn to_wire_diagnostic(
    env: &GlobalEnv,
    _files: &codespan::Files<String>,
    diag: &CsDiagnostic<FileId>,
    package_path: &str,
) -> Diagnostic {
    let primary = diag.labels.first();
    let file = primary.map(|l| repo_relative(&env.get_file(l.file_id).to_string_lossy(), package_path));
    let span = primary.map(|l| {
        let loc = Loc::new(
            l.file_id,
            CsSpan::new(ByteIndex(l.range.start as u32), ByteIndex(l.range.end as u32)),
        );
        span_of(env, &loc)
    });
    Diagnostic {
        file,
        span,
        severity: match diag.severity {
            CsSeverity::Bug | CsSeverity::Error => Severity::Error,
            CsSeverity::Warning => Severity::Warning,
            CsSeverity::Note | CsSeverity::Help => Severity::Note,
        },
        code: diag.code.clone(),
        message: diag.message.clone(),
    }
}

/// Strip the package root prefix to produce a package-relative path.
fn repo_relative(absolute: &str, package_root: &str) -> String {
    absolute
        .strip_prefix(package_root)
        .map(|s| s.trim_start_matches('/').to_string())
        .unwrap_or_else(|| absolute.to_string())
}
