//! Synthesis passes: Box insertion, default builders, shape traits (spec §3.3, §3.5).

use crate::node::*;
use crate::ty::CirTy;
use crisp_ast::item::{FieldDef, ShapeDef, ShapeField, TypeBody, TypeDef};
use crisp_ast::ty::{Type, TypeKind};
use crisp_typeck::Ty;
use std::collections::BTreeMap;

pub fn lower_struct(td: &TypeDef, field_types: &BTreeMap<String, Ty>) -> CirStruct {
    let fields: Vec<CirField> = match &td.body {
        TypeBody::Struct(fs) => fs
            .iter()
            .map(|f| lower_field(f, field_types.get(&f.name.name)))
            .collect(),
        _ => vec![],
    };
    let with_fn = synthesize_with_fn(&td.name.name, &fields);
    CirStruct {
        name: td.name.name.clone(),
        is_pub: td.is_pub,
        generics: td.generics.iter().map(|g| g.name.clone()).collect(),
        fields,
        with_fn,
        span: td.span,
    }
}

fn lower_field(f: &FieldDef, inferred: Option<&Ty>) -> CirField {
    let ty = inferred
        .map(CirTy::from_ty)
        .unwrap_or_else(|| ast_type_to_cir(&f.ty));
    let default = f.default.as_ref().map(lower_default_expr);
    CirField {
        name: f.name.name.clone(),
        ty,
        default,
        span: f.span,
    }
}

fn lower_default_expr(expr: &crisp_ast::expr::Expr) -> CirExpr {
    use crisp_ast::expr::{ExprKind, StringPart};
    match &expr.kind {
        ExprKind::Str(parts) => {
            let mut s = String::new();
            for p in &parts.0 {
                if let StringPart::Lit(l) = p {
                    s.push_str(l);
                }
            }
            CirExpr::Str {
                value: s,
                span: expr.span,
            }
        }
        ExprKind::Int(n) => CirExpr::Int {
            value: *n,
            span: expr.span,
        },
        ExprKind::Float(f) => CirExpr::Float {
            value: *f,
            span: expr.span,
        },
        ExprKind::Bool(b) => CirExpr::Ident {
            name: if *b { "true" } else { "false" }.into(),
            ty: CirTy::Bool,
            span: expr.span,
        },
        _ => CirExpr::Unit { span: expr.span },
    }
}

fn ast_type_to_cir(ty: &Type) -> CirTy {
    match &ty.kind {
        TypeKind::Named(id) => match id.name.as_str() {
            "int" => CirTy::Int,
            "uint" => CirTy::UInt,
            "float" => CirTy::Float,
            "bool" => CirTy::Bool,
            "str" => CirTy::Str,
            "char" => CirTy::Char,
            other => CirTy::Named {
                name: other.to_string(),
                args: vec![],
            },
        },
        TypeKind::Generic { base, args } => {
            let mut cir = ast_type_to_cir(base);
            if let CirTy::Named {
                args: ref mut a, ..
            } = cir
            {
                *a = args.iter().map(ast_type_to_cir).collect();
            }
            cir
        }
        TypeKind::Option(inner) => CirTy::Option(Box::new(ast_type_to_cir(inner))),
        TypeKind::Tuple(ts) => CirTy::Tuple(ts.iter().map(ast_type_to_cir).collect()),
        TypeKind::Ref { mutable, inner } => CirTy::Ref {
            mutable: *mutable,
            inner: Box::new(ast_type_to_cir(inner)),
        },
        TypeKind::Never => CirTy::Never,
        TypeKind::Unit => CirTy::Unit,
        _ => CirTy::Error,
    }
}

pub fn synthesize_with_fn(_name: &str, fields: &[CirField]) -> Option<CirWithFn> {
    let has_defaults = fields.iter().any(|f| f.default.is_some());
    if !has_defaults {
        return None;
    }
    let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
    let defaults: Vec<Option<CirExpr>> = fields.iter().map(|f| f.default.clone()).collect();
    Some(CirWithFn {
        fields: field_names,
        defaults,
        span: fields.first().map(|f| f.span).unwrap_or_default(),
    })
}

pub fn lower_enum(td: &TypeDef) -> Option<CirEnum> {
    let TypeBody::Enum(variants) = &td.body else {
        return None;
    };
    let enum_name = td.name.name.clone();
    let cir_variants: Vec<CirVariant> = variants
        .iter()
        .map(|v| {
            let fields: Vec<CirTy> = v
                .fields
                .iter()
                .map(|t| {
                    let mut cir = ast_type_to_cir(t);
                    if type_refs_self(&cir, &enum_name) {
                        cir = CirTy::Boxed(Box::new(cir));
                    }
                    cir
                })
                .collect();
            CirVariant {
                name: v.name.name.clone(),
                fields,
                span: v.span,
            }
        })
        .collect();
    Some(CirEnum {
        name: enum_name,
        is_pub: td.is_pub,
        generics: td.generics.iter().map(|g| g.name.clone()).collect(),
        variants: cir_variants,
        span: td.span,
    })
}

fn type_refs_self(ty: &CirTy, self_name: &str) -> bool {
    match ty {
        CirTy::Named { name, args } if name == self_name => true,
        CirTy::Named { args, .. } => args.iter().any(|a| type_refs_self(a, self_name)),
        CirTy::Tuple(ts) => ts.iter().any(|t| type_refs_self(t, self_name)),
        CirTy::Option(inner) | CirTy::Boxed(inner) | CirTy::Ref { inner, .. } => {
            type_refs_self(inner, self_name)
        }
        _ => false,
    }
}

pub fn synthesize_shape_trait(shape: &ShapeDef, structs: &[(String, CirStruct)]) -> CirShapeTrait {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    for sf in &shape.fields {
        match sf {
            ShapeField::Data { name, ty, .. } => {
                fields.push((name.name.clone(), ast_type_to_cir(ty)));
            }
            ShapeField::Method {
                name,
                params,
                ret_type,
                ..
            } => {
                methods.push(CirTraitMethod {
                    name: name.name.clone(),
                    params: params
                        .iter()
                        .map(|p| {
                            (
                                p.name.name.clone(),
                                p.ty.as_ref().map(ast_type_to_cir).unwrap_or(CirTy::Error),
                            )
                        })
                        .collect(),
                    ret: ast_type_to_cir(ret_type),
                    default_body: None,
                });
            }
        }
    }

    let shape_gens: Vec<String> = shape.generics.iter().map(|g| g.name.clone()).collect();
    let impls: Vec<CirShapeImpl> = structs
        .iter()
        .filter_map(|(_module, s)| infer_shape_impl(s, &fields, &shape_gens))
        .collect();

    CirShapeTrait {
        name: shape.name.name.clone(),
        generics: shape_gens,
        fields,
        methods,
        impls,
        span: shape.span,
    }
}

fn infer_shape_impl(
    s: &CirStruct,
    shape_fields: &[(String, CirTy)],
    shape_gens: &[String],
) -> Option<CirShapeImpl> {
    let mut subst: BTreeMap<String, CirTy> = BTreeMap::new();
    for (name, sty) in shape_fields {
        let field = s.fields.iter().find(|f| &f.name == name)?;
        bind_or_check(&field.ty, sty, shape_gens, &mut subst)?;
    }
    let args: Vec<CirTy> = shape_gens
        .iter()
        .map(|g| subst.get(g).cloned())
        .collect::<Option<Vec<_>>>()?;
    Some(CirShapeImpl {
        ty_name: s.name.clone(),
        ty_generics: s.generics.clone(),
        args,
        span: s.span,
    })
}

fn bind_or_check(
    concrete: &CirTy,
    pattern: &CirTy,
    gens: &[String],
    subst: &mut BTreeMap<String, CirTy>,
) -> Option<()> {
    match pattern {
        CirTy::Named { name, args } if args.is_empty() && gens.iter().any(|g| g == name) => {
            if let Some(prev) = subst.get(name) {
                if types_compatible(prev, concrete) {
                    Some(())
                } else {
                    None
                }
            } else {
                subst.insert(name.clone(), concrete.clone());
                Some(())
            }
        }
        _ => {
            if types_compatible(concrete, pattern) {
                Some(())
            } else {
                None
            }
        }
    }
}

fn types_compatible(a: &CirTy, b: &CirTy) -> bool {
    match (a, b) {
        (CirTy::Named { name: na, args: aa }, CirTy::Named { name: nb, args: ab }) => {
            na == nb
                && aa.len() == ab.len()
                && aa.iter().zip(ab).all(|(x, y)| types_compatible(x, y))
        }
        (CirTy::Int, CirTy::Int)
        | (CirTy::UInt, CirTy::UInt)
        | (CirTy::Float, CirTy::Float)
        | (CirTy::Bool, CirTy::Bool)
        | (CirTy::Str, CirTy::Str)
        | (CirTy::Char, CirTy::Char) => true,
        (CirTy::Option(a), CirTy::Option(b)) => types_compatible(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crisp_ast::ident::Ident;
    use crisp_ast::item::{TypeBody, TypeDef, VariantDef};
    use crisp_ast::span::Span;
    use crisp_ast::ty::Type;

    fn span() -> Span {
        Span::new(0, 1)
    }

    #[test]
    fn box_inserted_at_recursive_enum() {
        let td = TypeDef {
            is_pub: true,
            name: Ident {
                name: "List".into(),
                span: span(),
            },
            generics: vec![Ident {
                name: "T".into(),
                span: span(),
            }],
            body: TypeBody::Enum(vec![
                VariantDef {
                    name: Ident {
                        name: "Nil".into(),
                        span: span(),
                    },
                    fields: vec![],
                    span: span(),
                },
                VariantDef {
                    name: Ident {
                        name: "Cons".into(),
                        span: span(),
                    },
                    fields: vec![
                        Type {
                            kind: TypeKind::Named(Ident {
                                name: "T".into(),
                                span: span(),
                            }),
                            span: span(),
                        },
                        Type {
                            kind: TypeKind::Named(Ident {
                                name: "List".into(),
                                span: span(),
                            }),
                            span: span(),
                        },
                    ],
                    span: span(),
                },
            ]),
            span: span(),
        };
        let en = lower_enum(&td).expect("enum");
        let cons = en.variants.iter().find(|v| v.name == "Cons").unwrap();
        assert!(matches!(cons.fields[1], CirTy::Boxed(_)));
    }

    #[test]
    fn with_fn_synthesized_for_defaults() {
        let fields = vec![
            CirField {
                name: "host".into(),
                ty: CirTy::Str,
                default: Some(CirExpr::Str {
                    value: "localhost".into(),
                    span: span(),
                }),
                span: span(),
            },
            CirField {
                name: "port".into(),
                ty: CirTy::UInt,
                default: Some(CirExpr::Int {
                    value: 8080,
                    span: span(),
                }),
                span: span(),
            },
        ];
        let with_fn = synthesize_with_fn("Config", &fields).expect("with");
        assert_eq!(with_fn.fields, vec!["host", "port"]);
        assert_eq!(with_fn.defaults.len(), 2);
    }
}
