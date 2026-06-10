// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Foundation conversions shared by every facts walk.
//!
//! One canonicalization point for qualified names, one span builder, and the
//! ability / attribute / type-reference helpers. Everything downstream calls
//! into here so the canonical short-hex QName form is enforced in exactly one
//! place.

use super::types::{Ability, Span};
use move_core_types::ability::{Ability as CoreAbility, AbilitySet};
use move_core_types::account_address::AccountAddress;
use move_model::{
    ast::{Address, Attribute},
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, StructEnv},
    ty::Type,
};
use std::collections::BTreeSet;

/// Canonical short-hex form of an account address: lowercase, no leading
/// zeros, `0x`-prefixed. `0x1`, `0xa`, `0xcafe`, `0x0`.
pub(crate) fn canonical_address_str(addr: &AccountAddress) -> String {
    format!("0x{}", addr.short_str_lossless())
}

/// Canonical address of a module. Symbolic (unresolved named) addresses fall
/// back to their numerical resolution; target modules always carry numerical
/// addresses by the time the model is built.
pub(crate) fn module_address_str(m: &ModuleEnv<'_>) -> String {
    canonical_address_str(&m.self_address().expect_numerical())
}

/// `0x1::coin`.
pub(crate) fn module_qname(env: &GlobalEnv, m: &ModuleEnv<'_>) -> String {
    format!(
        "{}::{}",
        module_address_str(m),
        m.get_name().name().display(env.symbol_pool())
    )
}

/// `0x1::coin::transfer`.
pub(crate) fn function_qname(env: &GlobalEnv, f: &FunctionEnv<'_>) -> String {
    format!("{}::{}", module_qname(env, &f.module_env), f.get_name_str())
}

/// `0x1::coin::CoinStore`.
pub(crate) fn struct_qname(env: &GlobalEnv, s: &StructEnv<'_>) -> String {
    format!(
        "{}::{}",
        module_qname(env, &s.module_env),
        s.get_name().display(env.symbol_pool())
    )
}

/// `0x1::coin::E_NOT_OWNER`.
pub(crate) fn constant_qname(env: &GlobalEnv, c: &NamedConstantEnv<'_>) -> String {
    format!(
        "{}::{}",
        module_qname(env, &c.module_env),
        c.get_name().display(env.symbol_pool())
    )
}

/// Display name when the module was declared with a named address. Returns
/// `None` for purely numerical modules so the consumer can fall back to the
/// canonical QName.
pub(crate) fn module_display_name(env: &GlobalEnv, m: &ModuleEnv<'_>) -> Option<String> {
    match m.get_name().addr() {
        Address::Symbolic(sym) => Some(format!(
            "{}::{}",
            sym.display(env.symbol_pool()),
            m.get_name().name().display(env.symbol_pool())
        )),
        Address::Numerical(_) => None,
    }
}

/// Build a [`Span`] from a `Loc`: half-open byte range, 1-indexed inclusive
/// line range. codespan's `Location` is already 1-indexed.
pub(crate) fn span_of(env: &GlobalEnv, loc: &Loc) -> Span {
    let start_byte = loc.span().start().to_usize() as u32;
    let end_byte = loc.span().end().to_usize() as u32;
    // codespan's `Location.line` is a 0-indexed `LineIndex(u32)`; the contract
    // is 1-indexed inclusive lines.
    let start_line = env.get_location(loc).map(|l| l.line.0 + 1).unwrap_or(1);
    let end_line = env
        .get_location(&loc.at_end())
        .map(|l| l.line.0 + 1)
        .unwrap_or(start_line);
    Span {
        byte_range: [start_byte, end_byte],
        line_range: [start_line, end_line],
    }
}

/// Translate an `AbilitySet` into the ordered wire vector.
pub(crate) fn abilities_of(set: AbilitySet) -> Vec<Ability> {
    let mut out = Vec::new();
    for a in set {
        match a {
            CoreAbility::Copy => out.push(Ability::Copy),
            CoreAbility::Drop => out.push(Ability::Drop),
            CoreAbility::Store => out.push(Ability::Store),
            CoreAbility::Key => out.push(Ability::Key),
        }
    }
    out
}

/// Names of the attributes attached to a symbol, in source order.
pub(crate) fn attribute_names(env: &GlobalEnv, attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .map(|a| match a {
            Attribute::Apply(_, name, _) => name.display(env.symbol_pool()).to_string(),
            Attribute::Assign(_, name, _) => name.display(env.symbol_pool()).to_string(),
        })
        .collect()
}

/// True if any attribute on the symbol has the given name.
pub(crate) fn has_attribute(env: &GlobalEnv, attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| match a {
        Attribute::Apply(_, s, _) | Attribute::Assign(_, s, _) => {
            s.display(env.symbol_pool()).to_string() == name
        },
    })
}

/// Deduped, sorted struct/enum QNames referenced anywhere in a type
/// expression. Generic type parameters are not resolved (they live only in the
/// display string).
pub(crate) fn type_refs_of(env: &GlobalEnv, ty: &Type) -> Vec<String> {
    let mut out = BTreeSet::new();
    collect_type_refs(env, ty, &mut out);
    out.into_iter().collect()
}

fn collect_type_refs(env: &GlobalEnv, ty: &Type, out: &mut BTreeSet<String>) {
    match ty {
        Type::Struct(mid, sid, params) => {
            let s = env.get_module(*mid).into_struct(*sid);
            out.insert(struct_qname(env, &s));
            for p in params {
                collect_type_refs(env, p, out);
            }
        },
        Type::Vector(inner) | Type::Reference(_, inner) => collect_type_refs(env, inner, out),
        Type::Tuple(parts) => {
            for p in parts {
                collect_type_refs(env, p, out);
            }
        },
        Type::Fun(args, result, _) => {
            collect_type_refs(env, args, out);
            collect_type_refs(env, result, out);
        },
        Type::TypeParameter(_)
        | Type::Primitive(_)
        | Type::Var(_)
        | Type::Error
        | Type::TypeDomain(_)
        | Type::ResourceDomain(..)
        | Type::StateDomain => {},
    }
}

/// Resolve a `Type::Struct` to its canonical QName, if it is one.
pub(crate) fn struct_qname_of_type(env: &GlobalEnv, ty: &Type) -> Option<String> {
    if let Type::Struct(mid, sid, _) = ty {
        let s = env.get_module(*mid).into_struct(*sid);
        Some(struct_qname(env, &s))
    } else {
        None
    }
}

/// `E_*` / `EUpperCamel` error-code naming convention.
pub(crate) fn is_error_code(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('E'))
        && matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{canonical_address_str, is_error_code};
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn canonical_short_hex() {
        let a = AccountAddress::from_hex_literal("0x1").unwrap();
        assert_eq!(canonical_address_str(&a), "0x1");
    }

    #[test]
    fn canonical_strips_leading_zeros() {
        let a = AccountAddress::from_hex_literal("0x00a").unwrap();
        assert_eq!(canonical_address_str(&a), "0xa");
    }

    #[test]
    fn canonical_lowercases() {
        let a = AccountAddress::from_hex_literal("0xABCDEF").unwrap();
        assert_eq!(canonical_address_str(&a), "0xabcdef");
    }

    #[test]
    fn canonical_zero() {
        let a = AccountAddress::from_hex_literal("0x0").unwrap();
        assert_eq!(canonical_address_str(&a), "0x0");
    }

    #[test]
    fn error_code_detection() {
        assert!(is_error_code("E_NOT_OWNER"));
        assert!(is_error_code("ENotOwner"));
        assert!(!is_error_code("MAX_SUPPLY"));
        assert!(!is_error_code("Edge"));
        assert!(!is_error_code("E"));
    }
}
