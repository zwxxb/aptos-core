// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Wire-schema types for the v2 `move_package_facts` response.
//!
//! Fields use camelCase via `serde(rename_all)` so the JSON shape matches the
//! v2 contract verbatim. These types are the producer side of the contract:
//! they derive `Serialize` (to emit `structuredContent`) and `JsonSchema` (so
//! the tool can advertise an `outputSchema`). They are intentionally
//! `Deserialize`-free — only the GitNexus consumer decodes them.
//!
//! `REFERENCES` (function -> named constant) is deliberately absent: the Move
//! compiler constant-folds named constants into literal values before the
//! `GlobalEnv` is built, so the constant identity is not recoverable from the
//! expression AST. Constant *declarations* (with `isErrorCode`) are still
//! emitted; only the body references are dropped.

use rmcp::schemars::{self, JsonSchema};
use serde::Serialize;
use std::collections::BTreeMap;

/// Wire-protocol version. Bumped on any breaking change to the shape below.
pub(crate) const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MovePackageFacts {
    pub schema_version: u32,
    pub package: PackageHeader,
    pub diagnostics: Vec<Diagnostic>,
    /// Keyed by canonical module QName (e.g. `0x1::coin`).
    pub modules: BTreeMap<String, Module>,
    pub call_graph: Vec<CallEdge>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageHeader {
    pub name: String,
    /// `displayName -> "0xhex"` (canonical short hex).
    pub addresses: BTreeMap<String, String>,
    pub dev_addresses: BTreeMap<String, String>,
    /// Package-relative path to the manifest, always `"Move.toml"`.
    pub root_file: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Diagnostic {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Span {
    /// Half-open byte range `[start, end)`.
    pub byte_range: [u32; 2],
    /// 1-indexed inclusive line range `[start, end]`.
    pub line_range: [u32; 2],
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Module {
    pub address: String,
    pub name: String,
    pub qualified_name: String,
    /// Present when the source referred to this module via a named address
    /// (e.g. `aptos_framework::coin`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub file: String,
    pub span: Span,
    /// Residual attributes not promoted to typed fields.
    pub attributes: Vec<String>,
    pub has_spec: bool,
    /// Canonical module QNames declared as friends.
    pub friends: Vec<String>,
    pub uses: Vec<Use>,
    pub constants: Vec<Constant>,
    pub types: Vec<TypeDef>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Use {
    /// Module or member QName being imported.
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Constant {
    pub name: String,
    pub qualified_name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub value: String,
    pub is_error_code: bool,
    pub span: Span,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StructDef {
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub span: Span,
    pub abilities: Vec<Ability>,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<Field>,
    pub attributes: Vec<String>,
    pub has_spec: bool,
    /// Union of every field's `typeRefs`, deduped.
    pub type_refs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnumDef {
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub span: Span,
    pub abilities: Vec<Ability>,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<Variant>,
    pub attributes: Vec<String>,
    pub has_spec: bool,
    pub type_refs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Variant {
    pub name: String,
    pub kind: VariantKind,
    pub fields: Vec<Field>,
    pub attributes: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum VariantKind {
    Unit,
    Positional,
    Named,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Field {
    /// Positional fields are named by their index: `"0"`, `"1"`, ...
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    /// Resolved struct/enum QNames referenced in this type expression.
    pub type_refs: Vec<String>,
    pub positional: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TypeParam {
    pub name: String,
    pub abilities: Vec<Ability>,
    pub is_phantom: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Function {
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub span: Span,
    pub visibility: Visibility,
    pub flags: Vec<FunctionFlag>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub resource_access: Vec<ResourceAccess>,
    /// Residual attributes not promoted to typed fields.
    pub attributes: Vec<String>,
    pub has_spec: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Visibility {
    Public,
    Friend,
    Package,
    Private,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FunctionFlag {
    Entry,
    View,
    Inline,
    Native,
    Test,
    TestOnly,
    InitModule,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub type_refs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TypeExpr {
    pub display: String,
    pub type_refs: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceAccess {
    pub kind: ResourceAccessKind,
    /// Always fully qualified.
    pub resource: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ResourceAccessKind {
    Read,
    Write,
    Acquire,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallEdge {
    pub caller: String,
    pub callee: String,
    pub kind: CallEdgeKind,
    pub span: Span,
    /// `1.0` for direct/inline calls, `< 1.0` for statically resolved closures.
    pub confidence: f64,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CallEdgeKind {
    Direct,
    Inline,
    Closure,
}
