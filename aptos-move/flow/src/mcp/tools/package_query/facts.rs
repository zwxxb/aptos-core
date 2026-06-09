// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Builder for the `facts` query: structured per-module facts derived from `move_model::GlobalEnv`.

use move_model::{
    ast::{AccessSpecifierKind, Attribute, AttributeValue, ExpData, Operation, ResourceSpecifier},
    model::{
        FieldEnv, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, StructEnv, StructId,
        TypeParameter, Visibility,
    },
    symbol::Symbol,
    ty::{ReferenceKind, Type, TypeDisplayContext},
};
use std::collections::{BTreeMap, BTreeSet};

// ========== Schema types ==========

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ModuleFacts {
    file: String,
    span: (u32, u32),
    friends: Vec<FriendFacts>,
    attributes: Vec<AttributeFacts>,
    has_specs: bool,
    functions: Vec<FunctionFacts>,
    types: Vec<TypeFacts>,
    constants: Vec<ConstantFacts>,
}

/// A `friend` declaration target. Emitted as a record so `friends` matches
/// the shape of every other `Vec<*Facts>` field and stays forward-compatible
/// (future resolution status, alias, etc.) without breaking consumers.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FriendFacts {
    module: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ConstantFacts {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    value: String,
}

/// Mirror of `move_model::ast::Attribute`. `Apply` covers bare flags
/// (`#[test]`) and call-style attributes (`#[expected_failure(abort_code = 42)]`);
/// `Assign` covers `name = value` pairs (`#[max_gas = 1000]`). The `untagged`
/// enum keeps the JSON shape (`{name}` / `{name, args}` / `{name, value}`) and
/// makes invalid mixed states unrepresentable.
#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub(super) enum AttributeFacts {
    Apply {
        name: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        args: Vec<AttributeFacts>,
    },
    Assign {
        name: String,
        value: String,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub(super) enum TypeFacts {
    Struct {
        name: String,
        file: String,
        span: (u32, u32),
        abilities: Vec<String>,
        type_params: Vec<TypeParamFacts>,
        fields: Vec<FieldFacts>,
        attributes: Vec<AttributeFacts>,
        has_spec: bool,
    },
    Enum {
        name: String,
        file: String,
        span: (u32, u32),
        abilities: Vec<String>,
        type_params: Vec<TypeParamFacts>,
        variants: Vec<VariantFacts>,
        attributes: Vec<AttributeFacts>,
        has_spec: bool,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TypeParamFacts {
    name: String,
    abilities: Vec<String>,
    is_phantom: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FieldFacts {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    positional: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct VariantFacts {
    name: String,
    kind: String,
    fields: Vec<FieldFacts>,
    attributes: Vec<AttributeFacts>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FunctionFacts {
    name: String,
    file: String,
    span: (u32, u32),
    visibility: String,
    is_entry: bool,
    is_inline: bool,
    is_native: bool,
    is_view: bool,
    attributes: Vec<AttributeFacts>,
    type_params: Vec<TypeParamFacts>,
    params: Vec<ParameterFacts>,
    return_type: Option<String>,
    declared_access: Vec<AccessSpecFacts>,
    /// `null` whenever the function has no analyzable body (e.g. native).
    acquires_inferred: Option<Vec<String>>,
    /// `null` whenever the function has no analyzable body (e.g. native).
    resource_access: Option<ResourceAccessFacts>,
    has_spec: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ParameterFacts {
    name: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AccessSpecFacts {
    kind: String,
    resource: ResourceSpecFacts,
    negated: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ResourceSpecFacts {
    form: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ResourceAccessFacts {
    reads: Vec<String>,
    writes: Vec<String>,
}

// ========== Builders ==========

pub(super) fn build_facts(env: &GlobalEnv) -> BTreeMap<String, ModuleFacts> {
    env.get_primary_target_modules()
        .iter()
        .map(|module| (module.get_full_name_str(), build_module_facts(env, module)))
        .collect()
}

fn build_module_facts(env: &GlobalEnv, module: &ModuleEnv<'_>) -> ModuleFacts {
    let (file, span) = resolve_loc(env, &module.get_loc());
    ModuleFacts {
        file,
        span,
        // `get_friend_modules()` is the resolved set. `get_friend_decls()`
        // carries source locations but is left empty by some compile paths
        // (notably packages compiled through the MCP flow), so it isn't safe
        // to rely on here.
        friends: module
            .get_friend_modules()
            .iter()
            .map(|mid| FriendFacts {
                module: env.get_module(*mid).get_full_name_str(),
            })
            .collect(),
        attributes: serialize_attributes(env, module.get_attributes()),
        has_specs: module.has_specs(),
        functions: build_functions(env, module),
        types: build_types(env, module),
        constants: build_constants(env, module),
    }
}

fn build_constants(env: &GlobalEnv, module: &ModuleEnv<'_>) -> Vec<ConstantFacts> {
    module
        .get_named_constants()
        .map(|c| {
            let ctx = c.get_type_display_ctx();
            ConstantFacts {
                name: c.get_name().display(env.symbol_pool()).to_string(),
                type_: c.get_type().display(&ctx).to_string(),
                value: env.display(&c.get_value()).to_string(),
            }
        })
        .collect()
}

fn build_types(env: &GlobalEnv, module: &ModuleEnv<'_>) -> Vec<TypeFacts> {
    let type_ctx = module.get_type_display_ctx();
    let dummy = dummy_field_symbol(env);
    module
        .get_structs()
        .map(|s| build_type(env, &type_ctx, dummy, &s))
        .collect()
}

fn build_type(
    env: &GlobalEnv,
    type_ctx: &TypeDisplayContext<'_>,
    dummy: Symbol,
    s: &StructEnv<'_>,
) -> TypeFacts {
    let name = s.get_name().display(env.symbol_pool()).to_string();
    let (file, span) = resolve_loc(env, &s.get_loc());
    let abilities: Vec<String> = s
        .get_abilities()
        .into_iter()
        .map(|a| a.to_string())
        .collect();
    let type_params = serialize_type_params(env, s.get_type_parameters());
    let attributes = serialize_attributes(env, s.get_attributes());
    let has_spec = !s.get_spec().conditions.is_empty();

    if s.has_variants() {
        TypeFacts::Enum {
            name,
            file,
            span,
            abilities,
            type_params,
            variants: build_variants(env, type_ctx, dummy, s),
            attributes,
            has_spec,
        }
    } else {
        TypeFacts::Struct {
            name,
            file,
            span,
            abilities,
            type_params,
            fields: build_fields(env, type_ctx, dummy, s.get_fields()),
            attributes,
            has_spec,
        }
    }
}

fn build_variants(
    env: &GlobalEnv,
    type_ctx: &TypeDisplayContext<'_>,
    dummy: Symbol,
    s: &StructEnv<'_>,
) -> Vec<VariantFacts> {
    s.get_variants()
        .map(|v| {
            let fields = build_fields(env, type_ctx, dummy, s.get_fields_of_variant(v));
            VariantFacts {
                name: v.display(env.symbol_pool()).to_string(),
                kind: variant_kind(&fields),
                fields,
                attributes: serialize_attributes(env, s.get_variant_attributes(v)),
            }
        })
        .collect()
}

fn build_fields<'a>(
    env: &GlobalEnv,
    type_ctx: &TypeDisplayContext<'_>,
    dummy: Symbol,
    fields: impl Iterator<Item = FieldEnv<'a>>,
) -> Vec<FieldFacts> {
    fields
        .filter(|f| !is_dummy_field(f, dummy))
        .map(|f| FieldFacts {
            name: f.get_name().display(env.symbol_pool()).to_string(),
            type_: f.get_type().display(type_ctx).to_string(),
            positional: f.is_positional(),
        })
        .collect()
}

fn variant_kind(fields: &[FieldFacts]) -> String {
    if fields.is_empty() {
        "unit".to_string()
    } else if fields.iter().all(|f| f.positional) {
        "positional".to_string()
    } else {
        "named".to_string()
    }
}

const VIEW_FUN_ATTRIBUTE: &str = "view";

fn build_functions(env: &GlobalEnv, module: &ModuleEnv<'_>) -> Vec<FunctionFacts> {
    let module_id = module.get_id();
    let fq_ctx = fully_qualified_type_ctx(env);
    module
        .get_functions()
        .map(|f| build_function(env, &fq_ctx, module_id, &f))
        .collect()
}

fn build_function(
    env: &GlobalEnv,
    fq_ctx: &TypeDisplayContext<'_>,
    module_id: ModuleId,
    f: &FunctionEnv<'_>,
) -> FunctionFacts {
    let type_ctx = f.get_type_display_ctx();
    let (file, span) = resolve_loc(env, &f.get_loc());
    let result_type = f.get_result_type();
    let view_sym = env.symbol_pool().make(VIEW_FUN_ATTRIBUTE);
    let has_body = f.get_def().is_some();

    FunctionFacts {
        name: f.get_name_str(),
        file,
        span,
        visibility: visibility_str(f.visibility()).to_string(),
        is_entry: f.is_entry(),
        is_inline: f.is_inline(),
        is_native: f.is_native(),
        is_view: f.has_attribute(|a| a.name() == view_sym),
        attributes: serialize_attributes(env, f.get_attributes()),
        type_params: serialize_type_params(env, &f.get_type_parameters()),
        params: f
            .get_parameters()
            .iter()
            .map(|p| ParameterFacts {
                name: env.symbol_pool().string(p.0).to_string(),
                type_: p.1.display(&type_ctx).to_string(),
            })
            .collect(),
        return_type: match &result_type {
            Type::Tuple(ts) if ts.is_empty() => None,
            _ => Some(result_type.display(&type_ctx).to_string()),
        },
        declared_access: serialize_access_specifiers(env, f.get_access_specifiers()),
        // Gate on `get_def()` so `null` consistently means "no body to
        // analyze" — the model can return `Some(empty)` for natives.
        acquires_inferred: has_body
            .then(|| serialize_acquires_inferred(env, module_id, f.get_acquired_structs()))
            .flatten(),
        resource_access: compute_resource_access(env, fq_ctx, f),
        has_spec: !f.get_spec().conditions.is_empty(),
    }
}

fn visibility_str(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "public",
        Visibility::Friend => "friend",
        Visibility::Private => "private",
    }
}

fn serialize_access_specifiers(
    env: &GlobalEnv,
    specs: Option<&[move_model::ast::AccessSpecifier]>,
) -> Vec<AccessSpecFacts> {
    let Some(specs) = specs else { return vec![] };
    specs
        .iter()
        .map(|s| AccessSpecFacts {
            kind: access_kind_str(&s.kind).to_string(),
            resource: resource_specifier_facts(env, &s.resource.1),
            negated: s.negated,
        })
        .collect()
}

fn access_kind_str(kind: &AccessSpecifierKind) -> &'static str {
    match kind {
        AccessSpecifierKind::Reads => "reads",
        AccessSpecifierKind::Writes => "writes",
        AccessSpecifierKind::LegacyAcquires => "legacy_acquires",
    }
}

fn resource_specifier_facts(env: &GlobalEnv, spec: &ResourceSpecifier) -> ResourceSpecFacts {
    match spec {
        ResourceSpecifier::Any => ResourceSpecFacts {
            form: "any".to_string(),
            value: None,
        },
        ResourceSpecifier::DeclaredAtAddress(addr) => ResourceSpecFacts {
            form: "address".to_string(),
            value: Some(env.display(addr).to_string()),
        },
        ResourceSpecifier::DeclaredInModule(mid) => ResourceSpecFacts {
            form: "module".to_string(),
            value: Some(env.get_module(*mid).get_full_name_str()),
        },
        ResourceSpecifier::Resource(qid) => ResourceSpecFacts {
            form: "struct".to_string(),
            value: Some(qualified_struct_name(env, qid.module_id, qid.id)),
        },
    }
}

fn serialize_acquires_inferred(
    env: &GlobalEnv,
    module_id: ModuleId,
    ids: Option<&BTreeSet<StructId>>,
) -> Option<Vec<String>> {
    ids.map(|set| {
        set.iter()
            .map(|sid| qualified_struct_name(env, module_id, *sid))
            .collect()
    })
}

fn compute_resource_access(
    env: &GlobalEnv,
    fq_ctx: &TypeDisplayContext<'_>,
    func: &FunctionEnv<'_>,
) -> Option<ResourceAccessFacts> {
    let body = func.get_def()?;
    let mut reads: BTreeSet<String> = BTreeSet::new();
    let mut writes: BTreeSet<String> = BTreeSet::new();

    body.as_ref().visit_pre_order(&mut |e| {
        let ExpData::Call(node_id, op, _) = e else {
            return true;
        };
        // Match on `op` before fetching the instantiation: `Call` nodes are
        // dominated by non-storage ops (`Add`, `Eq`, `Pack`, …), and
        // `get_node_instantiation` clones a `Vec<Type>` per call.
        let bucket = match op {
            Operation::BorrowGlobal(ReferenceKind::Immutable) | Operation::Exists(_) => &mut reads,
            Operation::BorrowGlobal(ReferenceKind::Mutable)
            | Operation::MoveTo
            | Operation::MoveFrom => &mut writes,
            _ => return true,
        };
        let inst = env.get_node_instantiation(*node_id);
        if let Some(key) = inst.first().and_then(|t| resource_type_for(fq_ctx, t)) {
            bucket.insert(key);
        }
        true
    });

    Some(ResourceAccessFacts {
        reads: reads.into_iter().collect(),
        writes: writes.into_iter().collect(),
    })
}

fn resource_type_for(fq_ctx: &TypeDisplayContext<'_>, ty: &Type) -> Option<String> {
    matches!(ty, Type::Struct(..)).then(|| ty.display(fq_ctx).to_string())
}

// ========== Helpers ==========

/// Resolve a `Loc` to `(file_path, (start_line, end_line))`. Lines are
/// 1-based. Returns `(String::new(), (0, 0))` for unknown locations.
fn resolve_loc(env: &GlobalEnv, loc: &Loc) -> (String, (u32, u32)) {
    let file = env.get_file(loc.file_id()).to_string_lossy().into_owned();
    let start = env.get_location(loc).map(|l| l.line.0 + 1).unwrap_or(0);
    let end = env
        .get_location(&loc.at_end())
        .map(|l| l.line.0 + 1)
        .unwrap_or(start);
    (file, (start, end))
}

fn serialize_attributes(env: &GlobalEnv, attrs: &[Attribute]) -> Vec<AttributeFacts> {
    attrs.iter().map(|a| serialize_attribute(env, a)).collect()
}

fn serialize_attribute(env: &GlobalEnv, attr: &Attribute) -> AttributeFacts {
    match attr {
        Attribute::Apply(_, name_sym, sub) => AttributeFacts::Apply {
            name: env.symbol_pool().string(*name_sym).to_string(),
            args: sub.iter().map(|s| serialize_attribute(env, s)).collect(),
        },
        Attribute::Assign(_, name_sym, value) => AttributeFacts::Assign {
            name: env.symbol_pool().string(*name_sym).to_string(),
            value: render_attribute_value(env, value),
        },
    }
}

fn render_attribute_value(env: &GlobalEnv, value: &AttributeValue) -> String {
    match value {
        AttributeValue::Value(_, v) => env.display(v).to_string(),
        AttributeValue::Name(_, module_name, sym) => {
            let s = env.symbol_pool().string(*sym).to_string();
            match module_name {
                Some(m) => format!("{}::{}", m.display_full(env), s),
                None => s,
            }
        },
    }
}

fn serialize_type_params(env: &GlobalEnv, params: &[TypeParameter]) -> Vec<TypeParamFacts> {
    params
        .iter()
        .map(|tp| TypeParamFacts {
            name: env.symbol_pool().string(tp.0).to_string(),
            abilities: tp.1.abilities.into_iter().map(|a| a.to_string()).collect(),
            is_phantom: tp.1.is_phantom,
        })
        .collect()
}

/// Storage key for a struct: `address::module::name`. Type instantiations are
/// intentionally omitted — storage is keyed by the struct's identity, not by
/// its instantiation.
fn qualified_struct_name(env: &GlobalEnv, mid: ModuleId, sid: StructId) -> String {
    let m = env.get_module(mid);
    let s = m.get_struct(sid);
    format!(
        "{}::{}",
        m.get_full_name_str(),
        s.get_name().display(env.symbol_pool())
    )
}

/// Type display context that always qualifies struct references with both the
/// address and module path. Used for `resourceAccess.reads/writes`, which must
/// remain joinable across modules.
fn fully_qualified_type_ctx(env: &GlobalEnv) -> TypeDisplayContext<'_> {
    TypeDisplayContext {
        display_module_addr: true,
        use_module_qualification: true,
        ..TypeDisplayContext::new(env)
    }
}

/// The legacy Move compiler synthesizes a `dummy_field: bool` for empty
/// structs so they have a non-zero layout. It's invisible in source and would
/// pollute consumer indexes, so we filter it out at the `FieldEnv` level.
/// See `move_model::sourcifier::is_dummy_field`.
pub(super) fn dummy_field_symbol(env: &GlobalEnv) -> Symbol {
    env.symbol_pool().make("dummy_field")
}

pub(super) fn is_dummy_field(field: &FieldEnv<'_>, dummy: Symbol) -> bool {
    field.get_name() == dummy && field.get_type().is_bool()
}
