//! Implicit generic binders: unbound type names become parameters (#75 / #78).

use crate::ident::Ident;
use crate::item::{Item, ShapeField, TypeBody, TypeDef};
use crate::span::Span;
use crate::ty::{ErrorType, Type, TypeBound, TypeKind};
use std::collections::HashSet;

/// Prelude / builtin names that are never implicit parameters.
pub const PRELUDE_TYPE_NAMES: &[&str] = &[
    "int", "uint", "float", "bool", "char", "str", "Never", "vec", "map", "set", "Self",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericShadow {
    pub name: String,
    pub span: Span,
}

pub fn prelude_type_set() -> HashSet<String> {
    PRELUDE_TYPE_NAMES
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Type, shape, and trait names defined in these items.
pub fn defined_type_names(items: &[Item]) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in items {
        match item {
            Item::TypeDef(t) => {
                names.insert(t.name.name.clone());
            }
            Item::ShapeDef(s) => {
                names.insert(s.name.name.clone());
            }
            Item::TraitDef(t) => {
                names.insert(t.name.name.clone());
            }
            _ => {}
        }
    }
    names
}

pub fn collect_type_idents(ty: &Type, out: &mut Vec<Ident>) {
    match &ty.kind {
        TypeKind::Named(id) => out.push(id.clone()),
        TypeKind::Option(inner) | TypeKind::Slice(inner) | TypeKind::Ref { inner, .. } => {
            collect_type_idents(inner, out);
        }
        TypeKind::Tuple(ts) | TypeKind::Fn { params: ts, .. } => {
            for t in ts {
                collect_type_idents(t, out);
            }
            if let TypeKind::Fn { ret, .. } = &ty.kind {
                collect_type_idents(ret, out);
            }
        }
        TypeKind::Array { elem, .. } => collect_type_idents(elem, out),
        TypeKind::Constrained { inner, bounds } => {
            collect_type_idents(inner, out);
            for b in bounds {
                match b {
                    TypeBound::Shape(id) | TypeBound::Trait(id) => out.push(id.clone()),
                }
            }
        }
        TypeKind::Generic { base, args } => {
            collect_type_idents(base, out);
            for a in args {
                collect_type_idents(a, out);
            }
        }
        TypeKind::Never | TypeKind::Unit => {}
    }
}

fn collect_error_type_idents(err: &ErrorType, out: &mut Vec<Ident>) {
    for ty in &err.variants {
        collect_type_idents(ty, out);
    }
}

fn collect_type_def_idents(td: &TypeDef, out: &mut Vec<Ident>) {
    match &td.body {
        TypeBody::Struct(fields) => {
            for f in fields {
                collect_type_idents(&f.ty, out);
            }
        }
        TypeBody::Enum(variants) => {
            for v in variants {
                for ty in &v.fields {
                    collect_type_idents(ty, out);
                }
            }
        }
        TypeBody::Alias(ty) => collect_type_idents(ty, out),
    }
}

/// Type-position names on an item (annotations only, not expression bodies).
pub fn item_type_idents(item: &Item) -> Vec<Ident> {
    let mut out = Vec::new();
    match item {
        Item::Function(f) => {
            for p in &f.params {
                if let Some(ty) = &p.ty {
                    collect_type_idents(ty, &mut out);
                }
            }
            if let Some(ty) = &f.ret_type {
                collect_type_idents(ty, &mut out);
            }
            if let Some(err) = &f.error_type {
                collect_error_type_idents(err, &mut out);
            }
        }
        Item::TypeDef(td) => collect_type_def_idents(td, &mut out),
        Item::TraitDef(t) => {
            for m in &t.items {
                for p in &m.params {
                    if let Some(ty) = &p.ty {
                        collect_type_idents(ty, &mut out);
                    }
                }
                if let Some(ty) = &m.ret_type {
                    collect_type_idents(ty, &mut out);
                }
            }
        }
        Item::ShapeDef(s) => {
            for f in &s.fields {
                match f {
                    ShapeField::Data { ty, .. } => collect_type_idents(ty, &mut out),
                    ShapeField::Method {
                        params, ret_type, ..
                    } => {
                        for p in params {
                            if let Some(ty) = &p.ty {
                                collect_type_idents(ty, &mut out);
                            }
                        }
                        collect_type_idents(ret_type, &mut out);
                    }
                }
            }
        }
        Item::Impl(ib) => {
            collect_type_idents(&ib.ty, &mut out);
            for arg in &ib.trait_args {
                collect_type_idents(arg, &mut out);
            }
            for f in &ib.items {
                for p in &f.params {
                    if let Some(ty) = &p.ty {
                        collect_type_idents(ty, &mut out);
                    }
                }
                if let Some(ty) = &f.ret_type {
                    collect_type_idents(ty, &mut out);
                }
            }
        }
        Item::Use(_)
        | Item::Const(_)
        | Item::Extern(_)
        | Item::Test(_)
        | Item::TestCompileFail(_) => {}
    }
    out
}

/// Explicit binders first; then unbound type names in first-seen order.
/// An explicit binder that is also a known type is a collision (#78).
pub fn merge_implicit_generics(
    explicit: &[Ident],
    type_idents: &[Ident],
    known_types: &HashSet<String>,
) -> Result<Vec<Ident>, GenericShadow> {
    for g in explicit {
        if known_types.contains(&g.name) {
            return Err(GenericShadow {
                name: g.name.clone(),
                span: g.span,
            });
        }
    }
    let mut out = explicit.to_vec();
    let mut seen: HashSet<String> = out.iter().map(|g| g.name.clone()).collect();
    for id in type_idents {
        if known_types.contains(&id.name) || !seen.insert(id.name.clone()) {
            continue;
        }
        out.push(id.clone());
    }
    Ok(out)
}

pub fn apply_implicit_generics(
    items: &mut [Item],
    known_types: &HashSet<String>,
) -> Result<(), GenericShadow> {
    for item in items {
        let idents = item_type_idents(item);
        match item {
            Item::Function(f) => {
                f.generics = merge_implicit_generics(&f.generics, &idents, known_types)?;
            }
            Item::TypeDef(t) => {
                t.generics = merge_implicit_generics(&t.generics, &idents, known_types)?;
            }
            Item::TraitDef(t) => {
                t.generics = merge_implicit_generics(&t.generics, &idents, known_types)?;
            }
            Item::ShapeDef(s) => {
                s.generics = merge_implicit_generics(&s.generics, &idents, known_types)?;
            }
            Item::Impl(ib) => {
                for f in &mut ib.items {
                    let mut method_idents = Vec::new();
                    for p in &f.params {
                        if let Some(ty) = &p.ty {
                            collect_type_idents(ty, &mut method_idents);
                        }
                    }
                    if let Some(ty) = &f.ret_type {
                        collect_type_idents(ty, &mut method_idents);
                    }
                    f.generics = merge_implicit_generics(&f.generics, &method_idents, known_types)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn id(name: &str) -> Ident {
        Ident::new(name, Span::new(0, 1))
    }

    #[test]
    fn merge_adds_unbound_names() {
        let known = prelude_type_set();
        let out = merge_implicit_generics(&[], &[id("T"), id("int"), id("T"), id("A")], &known)
            .expect("merge");
        let names: Vec<_> = out.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["T", "A"]);
    }

    #[test]
    fn merge_rejects_explicit_shadow() {
        let mut known = prelude_type_set();
        known.insert("T".into());
        let err = merge_implicit_generics(&[id("T")], &[id("T")], &known).unwrap_err();
        assert_eq!(err.name, "T");
    }
}
