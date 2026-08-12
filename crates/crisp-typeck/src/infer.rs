use crate::env::{TypeEnv, generalize, instantiate, scheme};
use crate::types::{InferContext, InferredSig, Ty};
use crate::unify::{UnifyError, unify};
use crisp_ast::Span;
use crisp_ast::expr::{BinaryOp, Block, Expr, ExprKind, FieldInit, Stmt, UnaryOp};
use crisp_ast::ident::Ident;
use crisp_ast::item::{ExternBlock, FunctionDef, ImplBlock, Item, SourceFile, TypeBody};
use crisp_ast::pat::{Pat, PatKind};
use crisp_ast::ty::{Type, TypeKind};
use crisp_resolve::module::load_module_graph;
use crisp_resolve::{ResolvedRustImport, Resolver};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("unification error: {0}")]
    Unify(#[from] UnifyError),
    #[error("[E0040] unknown type `{name}`")]
    UnknownType { name: String, span: Span },
    #[error("[E0041] unknown name `{name}`")]
    UnknownName { name: String, span: Span },
    #[error("[E0042] resolve error: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
    #[error(
        "[E0043] ambiguous field `{field}` on unresolved type; annotate the parameter (candidates: {candidates})"
    )]
    AmbiguousField {
        field: String,
        candidates: String,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct TypedCrate {
    pub signatures: BTreeMap<String, InferredSig>,
    /// `TypeName` → `method` → signature key (`module::Type::method`).
    pub inherent_methods: BTreeMap<String, BTreeMap<String, String>>,
    /// Resolved `use <crate> { … }` / `use rust.<crate> { … }` imports (spec §14.2).
    pub rust_imports: Vec<ResolvedRustImport>,
}

pub struct TypeChecker {
    ctx: InferContext,
    env: TypeEnv,
    structs: BTreeMap<String, BTreeMap<String, Ty>>,
    /// Enum name → variant name → payload field types.
    enums: BTreeMap<String, BTreeMap<String, Vec<Ty>>>,
    signatures: BTreeMap<String, InferredSig>,
    /// `TypeName` → `method` → signature key.
    inherent_methods: BTreeMap<String, BTreeMap<String, String>>,
}

impl TypeChecker {
    pub fn check_crate(crate_root: &Path) -> Result<TypedCrate, TypeError> {
        let resolved = Resolver::resolve_crate(crate_root)?;
        let graph = load_module_graph(crate_root)?;
        let mut checker = Self::new();
        checker.register_prelude();
        checker.register_rust_imports(&resolved.rust_imports);
        for node in graph.modules.values() {
            checker.collect_types(&node.module_path, &node.ast);
        }
        // Stub every function into the env before checking bodies so modules that
        // sort after `main` (and same-module forward refs) are visible (#13).
        for node in graph.modules.values() {
            checker.collect_fn_stubs(&node.module_path, &node.ast)?;
        }
        for node in graph.modules.values() {
            checker.check_module(&node.module_path, &node.ast)?;
        }
        Ok(TypedCrate {
            signatures: checker.signatures,
            inherent_methods: checker.inherent_methods,
            rust_imports: resolved.rust_imports,
        })
    }

    fn new() -> Self {
        Self {
            ctx: InferContext::new(),
            env: TypeEnv::new(),
            structs: BTreeMap::new(),
            enums: BTreeMap::new(),
            signatures: BTreeMap::new(),
            inherent_methods: BTreeMap::new(),
        }
    }

    /// Bind imported Rust crate items into the type env (opaque / known stubs).
    fn register_rust_imports(&mut self, imports: &[ResolvedRustImport]) {
        for imp in imports {
            let (params, ret) = rust_import_fn_type(&imp.crate_name, &imp.item);
            let fn_ty = Ty::Fn {
                params: params.clone(),
                ret: Box::new(ret.clone()),
            };
            self.env.insert(imp.local_name.clone(), scheme(fn_ty));
            let module = format!("rust.{}", imp.crate_name);
            let key = format!("{module}::{}", imp.local_name);
            self.signatures.insert(
                key,
                InferredSig {
                    module,
                    name: imp.local_name.clone(),
                    impl_ty: None,
                    params: params
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| (format!("arg{i}"), t))
                        .collect(),
                    ret,
                    span: Span::new(0, 0),
                },
            );
        }
    }

    fn register_prelude(&mut self) {
        for (name, ty) in [
            ("int", Ty::Int),
            ("uint", Ty::UInt),
            ("float", Ty::Float),
            ("bool", Ty::Bool),
            ("char", Ty::Char),
            ("str", Ty::Str),
            ("Never", Ty::Never),
            (
                "vec",
                Ty::Named {
                    name: "vec".into(),
                    args: vec![],
                },
            ),
            (
                "map",
                Ty::Named {
                    name: "map".into(),
                    args: vec![],
                },
            ),
            (
                "set",
                Ty::Named {
                    name: "set".into(),
                    args: vec![],
                },
            ),
            (
                "log",
                Ty::Fn {
                    params: vec![Ty::StrSlice],
                    ret: Box::new(Ty::Unit),
                },
            ),
            (
                "some",
                Ty::Fn {
                    params: vec![self.ctx.fresh()],
                    ret: Box::new(Ty::Option(Box::new(self.ctx.fresh()))),
                },
            ),
            (
                "none",
                Ty::Fn {
                    params: vec![],
                    ret: Box::new(Ty::Option(Box::new(self.ctx.fresh()))),
                },
            ),
        ] {
            self.env.insert(name, scheme(ty));
        }
        let p = self.ctx.fresh();
        let assert_ty = Ty::Fn {
            params: vec![p.clone(), p],
            ret: Box::new(Ty::Unit),
        };
        let assert_scheme = generalize(&self.env, &mut self.ctx, &assert_ty);
        self.env.insert("assert_eq", assert_scheme);

        let pp = self.ctx.fresh();
        let print_ty = Ty::Fn {
            params: vec![pp.clone()],
            ret: Box::new(Ty::Unit),
        };
        let print_scheme = generalize(&self.env, &mut self.ctx, &print_ty);
        self.env.insert("print", print_scheme);

        for (name, ty) in stdlib_fn_types() {
            self.env.insert(name, scheme(ty));
        }
    }

    fn collect_types(&mut self, module: &str, file: &SourceFile) {
        for item in &file.items {
            if let Item::TypeDef(td) = item {
                if let TypeBody::Struct(fields) = &td.body {
                    let mut field_map = BTreeMap::new();
                    for f in fields {
                        if let Ok(ty) = self.ast_type(&f.ty) {
                            field_map.insert(f.name.name.clone(), self.ctx.apply(&ty));
                        }
                    }
                    self.structs.insert(td.name.name.clone(), field_map);
                    self.env.insert(
                        td.name.name.clone(),
                        scheme(Ty::Named {
                            name: td.name.name.clone(),
                            args: vec![],
                        }),
                    );
                } else if let TypeBody::Enum(variants) = &td.body {
                    let mut variant_map = BTreeMap::new();
                    for v in variants {
                        let mut fields = Vec::new();
                        for t in &v.fields {
                            if let Ok(ty) = self.ast_type(t) {
                                fields.push(self.ctx.apply(&ty));
                            }
                        }
                        variant_map.insert(v.name.name.clone(), fields);
                    }
                    self.enums.insert(td.name.name.clone(), variant_map);
                    self.env.insert(
                        td.name.name.clone(),
                        scheme(Ty::Named {
                            name: td.name.name.clone(),
                            args: vec![],
                        }),
                    );
                } else if let TypeBody::Alias(ty) = &td.body
                    && let Ok(t) = self.ast_type(ty)
                {
                    self.env.insert(td.name.name.clone(), scheme(t));
                }
            }
        }
        let _ = module;
    }

    fn collect_fn_stubs(&mut self, module: &str, file: &SourceFile) -> Result<(), TypeError> {
        for item in &file.items {
            match item {
                Item::Extern(ext) => {
                    // Externs are fully known; register immediately as stubs.
                    self.check_extern(module, ext)?;
                }
                Item::Function(f) => {
                    let mut params = Vec::new();
                    for p in &f.params {
                        let ty = if let Some(ast_ty) = &p.ty {
                            self.ast_type(ast_ty)?
                        } else {
                            self.ctx.fresh()
                        };
                        params.push(ty);
                    }
                    let ret = if let Some(t) = &f.ret_type {
                        self.ast_type(t)?
                    } else {
                        self.ctx.fresh()
                    };
                    self.env.insert(
                        f.name.name.clone(),
                        scheme(Ty::Fn {
                            params,
                            ret: Box::new(ret),
                        }),
                    );
                }
                Item::Impl(ib) => {
                    self.collect_impl_stubs(module, ib)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_impl_stubs(&mut self, module: &str, ib: &ImplBlock) -> Result<(), TypeError> {
        let ty_name = match &ib.ty.kind {
            TypeKind::Named(id) => id.name.clone(),
            _ => {
                return Err(TypeError::UnknownType {
                    name: "impl".into(),
                    span: ib.span,
                });
            }
        };
        let self_ty = Ty::Named {
            name: ty_name.clone(),
            args: vec![],
        };
        for f in &ib.items {
            let mut params = Vec::new();
            for p in &f.params {
                let ty = if p.name.name == "self" && p.ty.is_none() {
                    self_ty.clone()
                } else if let Some(ast_ty) = &p.ty {
                    self.ast_type(ast_ty)?
                } else {
                    self.ctx.fresh()
                };
                params.push(ty);
            }
            let ret = if let Some(t) = &f.ret_type {
                self.ast_type(t)?
            } else {
                self.ctx.fresh()
            };
            let key = format!("{module}::{ty_name}::{}", f.name.name);
            self.inherent_methods
                .entry(ty_name.clone())
                .or_default()
                .insert(f.name.name.clone(), key.clone());
            // Placeholder signature so later passes can see the key early.
            self.signatures.insert(
                key,
                InferredSig {
                    module: module.to_string(),
                    name: f.name.name.clone(),
                    impl_ty: Some(ty_name.clone()),
                    params: f
                        .params
                        .iter()
                        .zip(params.iter())
                        .map(|(p, t)| (p.name.name.clone(), t.clone()))
                        .collect(),
                    ret,
                    span: f.span,
                },
            );
        }
        Ok(())
    }

    fn check_module(&mut self, module: &str, file: &SourceFile) -> Result<(), TypeError> {
        for item in &file.items {
            match item {
                Item::Function(f) => self.check_function(module, f)?,
                Item::Impl(ib) => self.check_impl(module, ib)?,
                Item::Test(t) => self.check_test_block(module, &t.name, &t.body)?,
                Item::TestCompileFail(_) | Item::Extern(_) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn check_impl(&mut self, module: &str, ib: &ImplBlock) -> Result<(), TypeError> {
        let ty_name = match &ib.ty.kind {
            TypeKind::Named(id) => id.name.clone(),
            _ => {
                return Err(TypeError::UnknownType {
                    name: "impl".into(),
                    span: ib.span,
                });
            }
        };
        for f in &ib.items {
            self.check_impl_method(module, &ty_name, f)?;
        }
        Ok(())
    }

    fn check_impl_method(
        &mut self,
        module: &str,
        ty_name: &str,
        f: &FunctionDef,
    ) -> Result<(), TypeError> {
        let key = format!("{module}::{ty_name}::{}", f.name.name);
        let stub = self.signatures.get(&key).cloned();
        let (stub_params, stub_ret) = match stub {
            Some(s) => (
                s.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
                Some(s.ret),
            ),
            None => (Vec::new(), None),
        };
        let self_ty = Ty::Named {
            name: ty_name.to_string(),
            args: vec![],
        };

        let mut local = self.env.clone();
        let mut param_vars = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            let mut ty = stub_params
                .get(i)
                .cloned()
                .unwrap_or_else(|| self.ctx.fresh());
            if p.name.name == "self" && p.ty.is_none() {
                unify(&mut self.ctx, &ty, &self_ty)?;
                ty = self.ctx.apply(&self_ty);
            } else if let Some(ast_ty) = &p.ty {
                let ann = self.ast_type(ast_ty)?;
                unify(&mut self.ctx, &ty, &ann)?;
                ty = self.ctx.apply(&ann);
            }
            param_vars.push((p.name.name.clone(), ty.clone()));
            local.insert(p.name.name.clone(), scheme(ty));
        }
        let ret_ann = f.ret_type.as_ref().map(|t| self.ast_type(t)).transpose()?;
        let body_ty = self.infer_expr(&mut local, &f.body)?;
        let param_types: Vec<(String, Ty)> = param_vars
            .iter()
            .map(|(n, t)| (n.clone(), self.ctx.apply(t)))
            .collect();
        let ret = if let Some(ann) = ret_ann {
            unify(&mut self.ctx, &body_ty, &ann)?;
            if let Some(stub_r) = &stub_ret {
                unify(&mut self.ctx, &body_ty, stub_r)?;
            }
            self.ctx.apply(&ann)
        } else if let Some(stub_r) = &stub_ret {
            unify(&mut self.ctx, &body_ty, stub_r)?;
            self.ctx.apply(&body_ty)
        } else {
            self.ctx.apply(&body_ty)
        };
        self.signatures.insert(
            key.clone(),
            InferredSig {
                module: module.to_string(),
                name: f.name.name.clone(),
                impl_ty: Some(ty_name.to_string()),
                params: param_types,
                ret,
                span: f.span,
            },
        );
        self.inherent_methods
            .entry(ty_name.to_string())
            .or_default()
            .insert(f.name.name.clone(), key);
        Ok(())
    }

    fn check_extern(&mut self, module: &str, ext: &ExternBlock) -> Result<(), TypeError> {
        for f in &ext.functions {
            let mut param_tys = Vec::new();
            for p in &f.params {
                let ty = if let Some(ast_ty) = &p.ty {
                    self.ast_type(ast_ty)?
                } else {
                    Ty::Int
                };
                param_tys.push(ty);
            }
            let ret = if let Some(ast_ty) = &f.ret_type {
                self.ast_type(ast_ty)?
            } else {
                Ty::Unit
            };
            let fn_ty = Ty::Fn {
                params: param_tys.clone(),
                ret: Box::new(ret.clone()),
            };
            self.env.insert(f.name.name.clone(), scheme(fn_ty));
            let key = format!("{}::{}", module, f.name.name);
            self.signatures.insert(
                key,
                InferredSig {
                    module: module.to_string(),
                    name: f.name.name.clone(),
                    impl_ty: None,
                    params: f
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (p.name.name.clone(), param_tys[i].clone()))
                        .collect(),
                    ret,
                    span: f.span,
                },
            );
        }
        let _ = ext;
        Ok(())
    }

    fn check_test_block(
        &mut self,
        module: &str,
        name: &str,
        body: &Block,
    ) -> Result<(), TypeError> {
        let mut local = self.env.clone();
        let body_ty = self.infer_block(&mut local, body)?;
        unify(&mut self.ctx, &body_ty, &Ty::Unit)?;
        let key = format!("{module}::test::{name}");
        self.signatures.insert(
            key,
            InferredSig {
                module: module.to_string(),
                name: format!("test::{name}"),
                impl_ty: None,
                params: vec![],
                ret: Ty::Unit,
                span: body.span,
            },
        );
        Ok(())
    }

    fn check_function(&mut self, module: &str, f: &FunctionDef) -> Result<(), TypeError> {
        // Reuse stub param/ret vars so call-site unifications from earlier modules stick.
        let stub = self
            .env
            .get(&f.name.name)
            .map(|s| instantiate(&mut self.ctx, s))
            .map(|t| self.ctx.apply(&t));
        let (stub_params, stub_ret) = match stub {
            Some(Ty::Fn { params, ret }) => (params, Some(*ret)),
            _ => (Vec::new(), None),
        };

        let mut local = self.env.clone();
        let mut param_vars = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            let mut ty = stub_params
                .get(i)
                .cloned()
                .unwrap_or_else(|| self.ctx.fresh());
            if let Some(ast_ty) = &p.ty {
                let ann = self.ast_type(ast_ty)?;
                unify(&mut self.ctx, &ty, &ann)?;
                ty = self.ctx.apply(&ann);
            }
            param_vars.push((p.name.name.clone(), ty.clone()));
            local.insert(p.name.name.clone(), scheme(ty));
        }
        let ret_ann = f.ret_type.as_ref().map(|t| self.ast_type(t)).transpose()?;
        let body_ty = self.infer_expr(&mut local, &f.body)?;
        let param_types: Vec<(String, Ty)> = param_vars
            .iter()
            .map(|(n, t)| (n.clone(), self.ctx.apply(t)))
            .collect();
        let ret = if let Some(ann) = ret_ann {
            unify(&mut self.ctx, &body_ty, &ann)?;
            if let Some(stub_r) = &stub_ret {
                unify(&mut self.ctx, &body_ty, stub_r)?;
            }
            self.ctx.apply(&ann)
        } else if let Some(stub_r) = &stub_ret {
            unify(&mut self.ctx, &body_ty, stub_r)?;
            self.ctx.apply(&body_ty)
        } else {
            self.ctx.apply(&body_ty)
        };
        let fn_params: Vec<Ty> = param_types.iter().map(|(_, t)| t.clone()).collect();
        let key = format!("{module}::{}", f.name.name);
        self.signatures.insert(
            key,
            InferredSig {
                module: module.to_string(),
                name: f.name.name.clone(),
                impl_ty: None,
                params: param_types,
                ret: ret.clone(),
                span: f.span,
            },
        );
        self.env.insert(
            f.name.name.clone(),
            scheme(Ty::Fn {
                params: fn_params,
                ret: Box::new(ret),
            }),
        );
        Ok(())
    }

    fn infer_expr(&mut self, env: &mut TypeEnv, expr: &Expr) -> Result<Ty, TypeError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Ty::Int),
            ExprKind::Float(_) => Ok(Ty::Float),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Char(_) => Ok(Ty::Char),
            ExprKind::Str(_) => Ok(Ty::Str),
            ExprKind::Unit => Ok(Ty::Unit),
            ExprKind::Ident(id) => self.lookup(env, &id.name, id.span),
            ExprKind::Block(b) => self.infer_block(env, b),
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cty = self.infer_expr(env, cond)?;
                unify(&mut self.ctx, &cty, &Ty::Bool)?;
                let t = self.infer_expr(env, then_branch)?;
                if let Some(e) = else_branch {
                    let e_ty = self.infer_expr(env, e)?;
                    unify(&mut self.ctx, &t, &e_ty)?;
                }
                Ok(self.ctx.apply(&t))
            }
            ExprKind::Match { scrutinee, arms } => {
                let scrut = self.infer_expr(env, scrutinee)?;
                let mut local = env.clone();
                let mut result = None;
                for arm in arms {
                    self.infer_pat(&mut local, &arm.pat, &scrut)?;
                    if let Some(g) = &arm.guard {
                        let gty = self.infer_expr(&mut local, g)?;
                        unify(&mut self.ctx, &gty, &Ty::Bool)?;
                    }
                    let body = self.infer_expr(&mut local, &arm.body)?;
                    result = Some(match result {
                        None => body,
                        Some(prev) => {
                            unify(&mut self.ctx, &prev, &body)?;
                            prev
                        }
                    });
                }
                Ok(result.unwrap_or(Ty::Unit))
            }
            ExprKind::Lambda { params, body } => {
                let mut local = env.clone();
                let mut ptys = Vec::new();
                for p in params {
                    let ty = if let Some(ast_ty) = &p.ty {
                        self.ast_type(ast_ty)?
                    } else {
                        self.ctx.fresh()
                    };
                    ptys.push(self.ctx.apply(&ty));
                    local.insert(p.name.name.clone(), scheme(ty));
                }
                let ret = self.infer_expr(&mut local, body)?;
                Ok(Ty::Fn {
                    params: ptys,
                    ret: Box::new(self.ctx.apply(&ret)),
                })
            }
            ExprKind::Call { func, args } => {
                // Inherent methods parse as Call(Field(...), args) — not MethodCall.
                if let ExprKind::Field { base, field } = &func.kind
                    && let Some(ret) = self.try_infer_method_call(env, base, field, args)?
                {
                    return Ok(ret);
                }
                let ft = self.infer_expr(env, func)?;
                let ft = self.ctx.apply(&ft);
                let (params, ret) = match ft {
                    Ty::Fn { params, ret } => (params, ret),
                    Ty::Var(v) => {
                        let ps: Vec<_> = (0..args.len()).map(|_| self.ctx.fresh()).collect();
                        let ret = self.ctx.fresh();
                        unify(
                            &mut self.ctx,
                            &Ty::Var(v),
                            &Ty::Fn {
                                params: ps.clone(),
                                ret: Box::new(ret.clone()),
                            },
                        )?;
                        (ps, Box::new(ret))
                    }
                    other => {
                        return Err(TypeError::Unify(UnifyError::Mismatch {
                            expected: "function".into(),
                            found: format!("{other:?}"),
                        }));
                    }
                };
                if args.len() != params.len() {
                    return Err(TypeError::Unify(UnifyError::Mismatch {
                        expected: format!("{} arguments", params.len()),
                        found: format!("{} arguments", args.len()),
                    }));
                }
                for (arg, pty) in args.iter().zip(params) {
                    let aty = self.infer_expr(env, arg)?;
                    unify(&mut self.ctx, &aty, &pty)?;
                }
                Ok(self.ctx.apply(&ret))
            }
            ExprKind::Field { base, field } => {
                // Enum variants: Color.Red (unit) / Color.Custom (ctor fn type)
                if let ExprKind::Ident(id) = &base.kind
                    && let Some(variants) = self.enums.get(&id.name)
                {
                    return match variants.get(&field.name) {
                        Some(payload) if payload.is_empty() => Ok(Ty::Named {
                            name: id.name.clone(),
                            args: vec![],
                        }),
                        Some(payload) => Ok(Ty::Fn {
                            params: payload.clone(),
                            ret: Box::new(Ty::Named {
                                name: id.name.clone(),
                                args: vec![],
                            }),
                        }),
                        None => Err(TypeError::UnknownName {
                            name: format!("{}.{}", id.name, field.name),
                            span: field.span,
                        }),
                    };
                }
                // Associated inherent method as a value: `Vec2.new` → fn type (no self).
                if let ExprKind::Ident(id) = &base.kind
                    && self.structs.contains_key(&id.name)
                    && let Some(sig) = self.method_sig(&id.name, &field.name)
                {
                    let has_self = sig
                        .params
                        .first()
                        .map(|(n, _)| n == "self")
                        .unwrap_or(false);
                    if !has_self {
                        let params: Vec<Ty> = sig.params.iter().map(|(_, t)| t.clone()).collect();
                        return Ok(Ty::Fn {
                            params,
                            ret: Box::new(sig.ret.clone()),
                        });
                    }
                }
                let base_ty = self.infer_expr(env, base)?;
                self.field_type(&base_ty, &field.name, field.span)
            }
            ExprKind::Unary { op, expr } => match op {
                UnaryOp::Not => {
                    let t = self.infer_expr(env, expr)?;
                    unify(&mut self.ctx, &t, &Ty::Bool)?;
                    Ok(Ty::Bool)
                }
                UnaryOp::Neg => {
                    let t = self.infer_expr(env, expr)?;
                    unify(&mut self.ctx, &t, &Ty::Int)?;
                    Ok(Ty::Int)
                }
            },
            ExprKind::Binary { op, left, right } => self.infer_binary(env, *op, left, right),
            ExprKind::StructLit { name, fields } => {
                self.check_struct_lit(env, name, fields)?;
                Ok(Ty::Named {
                    name: name.name.clone(),
                    args: vec![],
                })
            }
            ExprKind::Bind { pat, value, .. } => {
                let ty = self.infer_expr(env, value)?;
                let mut local = env.clone();
                self.infer_pat(&mut local, pat, &ty)?;
                Ok(Ty::Unit)
            }
            ExprKind::Pipe { left, right } => {
                let lt = self.infer_expr(env, left)?;
                let mut local = env.clone();
                let v = self.ctx.fresh();
                local.insert("_pipe".to_string(), scheme(v.clone()));
                unify(&mut self.ctx, &lt, &v)?;
                self.infer_expr(&mut local, right)
            }
            ExprKind::Return(Some(e)) => {
                self.infer_expr(env, e)?;
                Ok(Ty::Never)
            }
            ExprKind::Return(None) => Ok(Ty::Never),
            ExprKind::Try(inner) => {
                let t = self.infer_expr(env, inner)?;
                match self.ctx.apply(&t) {
                    Ty::Option(inner) => Ok(*inner),
                    other => {
                        let fresh = self.ctx.fresh();
                        unify(&mut self.ctx, &other, &Ty::Option(Box::new(fresh.clone())))?;
                        Ok(self.ctx.apply(&fresh))
                    }
                }
            }
            ExprKind::Catch { body, arms } => {
                let _ = self.infer_expr(env, body)?;
                let mut result = None;
                for arm in arms {
                    let body_ty = self.infer_expr(env, &arm.body)?;
                    result = Some(match result {
                        None => body_ty,
                        Some(prev) => {
                            unify(&mut self.ctx, &prev, &body_ty)?;
                            prev
                        }
                    });
                }
                Ok(result.unwrap_or(Ty::Unit))
            }
            ExprKind::Async(inner) => {
                let inner_ty = self.infer_expr(env, inner)?;
                Ok(Ty::Named {
                    name: "Future".into(),
                    args: vec![self.ctx.apply(&inner_ty)],
                })
            }
            ExprKind::Await(inner) => {
                let t = self.infer_expr(env, inner)?;
                match self.ctx.apply(&t) {
                    Ty::Named { name, args } if name == "Future" && args.len() == 1 => {
                        Ok(args[0].clone())
                    }
                    other => {
                        let fresh = self.ctx.fresh();
                        unify(
                            &mut self.ctx,
                            &other,
                            &Ty::Named {
                                name: "Future".into(),
                                args: vec![fresh.clone()],
                            },
                        )?;
                        Ok(self.ctx.apply(&fresh))
                    }
                }
            }
            ExprKind::Unsafe(inner) => self.infer_expr(env, inner),
            ExprKind::Spawn(inner) => {
                self.infer_expr(env, inner)?;
                Ok(Ty::Named {
                    name: "JoinHandle".into(),
                    args: vec![],
                })
            }
            _ => Ok(self.ctx.fresh()),
        }
    }

    fn infer_binary(
        &mut self,
        env: &mut TypeEnv,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<Ty, TypeError> {
        let lt = self.infer_expr(env, left)?;
        let rt = self.infer_expr(env, right)?;
        match op {
            BinaryOp::Concat => {
                unify(&mut self.ctx, &lt, &Ty::Str)?;
                unify(&mut self.ctx, &rt, &Ty::StrSlice)?;
                Ok(Ty::Str)
            }
            BinaryOp::Mod => {
                unify(&mut self.ctx, &lt, &rt)?;
                unify(&mut self.ctx, &lt, &Ty::Int)?;
                Ok(Ty::Int)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                unify(&mut self.ctx, &lt, &rt)?;
                if matches!(&lt, Ty::Float) || matches!(&rt, Ty::Float) {
                    unify(&mut self.ctx, &lt, &Ty::Float)?;
                    Ok(Ty::Float)
                } else {
                    unify(&mut self.ctx, &lt, &Ty::Int)?;
                    Ok(Ty::Int)
                }
            }
            BinaryOp::Pow => {
                unify(&mut self.ctx, &lt, &Ty::Float)?;
                unify(&mut self.ctx, &rt, &Ty::Float)?;
                Ok(Ty::Float)
            }
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                unify(&mut self.ctx, &lt, &rt)?;
                Ok(Ty::Bool)
            }
            BinaryOp::And | BinaryOp::Or => {
                unify(&mut self.ctx, &lt, &Ty::Bool)?;
                unify(&mut self.ctx, &rt, &Ty::Bool)?;
                Ok(Ty::Bool)
            }
            _ => Ok(self.ctx.fresh()),
        }
    }

    fn infer_block(&mut self, env: &mut TypeEnv, block: &Block) -> Result<Ty, TypeError> {
        let mut local = env.clone();
        for stmt in &block.stmts {
            match stmt {
                Stmt::Bind { pat, value, .. } => {
                    let ty = self.infer_expr(&mut local, value)?;
                    self.infer_pat(&mut local, pat, &ty)?;
                }
                Stmt::Assign { target, value } => {
                    let expected = self.lookup(&local, &target.name, target.span)?;
                    let got = self.infer_expr(&mut local, value)?;
                    unify(&mut self.ctx, &got, &expected)?;
                }
                Stmt::Expr(e) => {
                    self.infer_expr(&mut local, e)?;
                }
            }
        }
        if let Some(tail) = &block.tail {
            self.infer_expr(&mut local, tail)
        } else {
            Ok(Ty::Unit)
        }
    }

    fn infer_pat(&mut self, env: &mut TypeEnv, pat: &Pat, ty: &Ty) -> Result<(), TypeError> {
        match &pat.kind {
            PatKind::Wildcard => Ok(()),
            PatKind::Ident(id) => {
                env.insert(id.name.clone(), scheme(self.ctx.apply(ty)));
                Ok(())
            }
            PatKind::Tuple(pats) => {
                if let Ty::Tuple(ts) = self.ctx.apply(ty) {
                    for (p, t) in pats.iter().zip(ts) {
                        self.infer_pat(env, p, &t)?;
                    }
                    Ok(())
                } else {
                    let vars: Vec<_> = (0..pats.len()).map(|_| self.ctx.fresh()).collect();
                    unify(&mut self.ctx, ty, &Ty::Tuple(vars.clone()))?;
                    for (p, t) in pats.iter().zip(vars) {
                        self.infer_pat(env, p, &t)?;
                    }
                    Ok(())
                }
            }
            PatKind::Enum {
                name,
                variant,
                args,
            } => {
                let enum_ty = Ty::Named {
                    name: name.name.clone(),
                    args: vec![],
                };
                unify(&mut self.ctx, ty, &enum_ty)?;
                let Some(variants) = self.enums.get(&name.name) else {
                    return Err(TypeError::UnknownType {
                        name: name.name.clone(),
                        span: name.span,
                    });
                };
                let Some(payload) = variants.get(&variant.name) else {
                    return Err(TypeError::UnknownName {
                        name: format!("{}.{}", name.name, variant.name),
                        span: variant.span,
                    });
                };
                if args.len() != payload.len() {
                    return Err(TypeError::Unify(UnifyError::Mismatch {
                        expected: format!("{} payload fields", payload.len()),
                        found: format!("{} pattern args", args.len()),
                    }));
                }
                let payload = payload.clone();
                for (arg, field_ty) in args.iter().zip(payload) {
                    self.infer_pat(env, arg, &field_ty)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_struct_lit(
        &mut self,
        env: &mut TypeEnv,
        name: &Ident,
        fields: &[FieldInit],
    ) -> Result<(), TypeError> {
        let schema = self
            .structs
            .get(&name.name)
            .ok_or_else(|| TypeError::UnknownType {
                name: name.name.clone(),
                span: name.span,
            })?;
        let field_types: Vec<_> = fields
            .iter()
            .map(|field| {
                schema
                    .get(&field.name.name)
                    .cloned()
                    .ok_or_else(|| TypeError::UnknownType {
                        name: field.name.name.clone(),
                        span: field.name.span,
                    })
            })
            .collect::<Result<_, _>>()?;
        for (field, expected) in fields.iter().zip(field_types) {
            let got = self.infer_expr(env, &field.value)?;
            unify(&mut self.ctx, &got, &expected)?;
        }
        Ok(())
    }

    fn method_sig(&self, ty_name: &str, method: &str) -> Option<&InferredSig> {
        let key = self.inherent_methods.get(ty_name)?.get(method)?;
        self.signatures.get(key)
    }

    /// Resolve `Type.assoc(args)` / `recv.method(args)` for inherent impls (§5.4).
    /// Returns `Ok(None)` when this is not an inherent-method call (caller falls through).
    fn try_infer_method_call(
        &mut self,
        env: &mut TypeEnv,
        base: &Expr,
        field: &Ident,
        args: &[Expr],
    ) -> Result<Option<Ty>, TypeError> {
        // Associated: `Vec2.new(x, y)` — base is a known struct type name, not an enum.
        if let ExprKind::Ident(id) = &base.kind
            && self.structs.contains_key(&id.name)
            && !self.enums.contains_key(&id.name)
            && let Some(sig) = self.method_sig(&id.name, &field.name).cloned()
        {
            let has_self = sig
                .params
                .first()
                .map(|(n, _)| n == "self")
                .unwrap_or(false);
            if has_self {
                return Err(TypeError::Unify(UnifyError::Mismatch {
                    expected: format!("instance method `{}.{}(self, …)`", id.name, field.name),
                    found: "associated call on type name".into(),
                }));
            }
            if args.len() != sig.params.len() {
                return Err(TypeError::Unify(UnifyError::Mismatch {
                    expected: format!("{} arguments", sig.params.len()),
                    found: format!("{} arguments", args.len()),
                }));
            }
            for (arg, (_, pty)) in args.iter().zip(sig.params.iter()) {
                let aty = self.infer_expr(env, arg)?;
                unify(&mut self.ctx, &aty, pty)?;
            }
            return Ok(Some(self.ctx.apply(&sig.ret)));
        }

        // Instance: `v.magnitude()` / `v.scale(2.0)`
        // Peek: only attempt when some inherent method with this name exists.
        let candidate_tys: Vec<String> = self
            .inherent_methods
            .iter()
            .filter_map(|(ty, methods)| {
                if methods.contains_key(&field.name) {
                    Some(ty.clone())
                } else {
                    None
                }
            })
            .collect();
        if candidate_tys.is_empty() {
            return Ok(None);
        }

        let base_ty = self.infer_expr(env, base)?;
        let base_ty = self.ctx.apply(&base_ty);
        let ty_name = match &base_ty {
            Ty::Named { name, .. } => name.clone(),
            Ty::Var(v) if candidate_tys.len() == 1 => {
                let name = candidate_tys[0].clone();
                unify(
                    &mut self.ctx,
                    &Ty::Var(*v),
                    &Ty::Named {
                        name: name.clone(),
                        args: vec![],
                    },
                )?;
                name
            }
            _ => return Ok(None),
        };

        let Some(sig) = self.method_sig(&ty_name, &field.name).cloned() else {
            return Ok(None);
        };
        let has_self = sig
            .params
            .first()
            .map(|(n, _)| n == "self")
            .unwrap_or(false);
        if !has_self {
            // Associated method called on a value — not supported.
            return Ok(None);
        }
        let self_ty = Ty::Named {
            name: ty_name,
            args: vec![],
        };
        unify(&mut self.ctx, &base_ty, &self_ty)?;
        let param_tys: Vec<&Ty> = sig.params.iter().skip(1).map(|(_, t)| t).collect();
        if args.len() != param_tys.len() {
            return Err(TypeError::Unify(UnifyError::Mismatch {
                expected: format!("{} arguments", param_tys.len()),
                found: format!("{} arguments", args.len()),
            }));
        }
        for (arg, pty) in args.iter().zip(param_tys) {
            let aty = self.infer_expr(env, arg)?;
            unify(&mut self.ctx, &aty, pty)?;
        }
        Ok(Some(self.ctx.apply(&sig.ret)))
    }

    fn field_type(&mut self, base: &Ty, field: &str, span: Span) -> Result<Ty, TypeError> {
        let base = self.ctx.apply(base);
        if let Ty::Named { name, .. } = &base
            && let Some(fields) = self.structs.get(name)
        {
            return fields.get(field).cloned().ok_or(TypeError::UnknownType {
                name: field.to_string(),
                span,
            });
        }
        // Unannotated params stay as type vars. If exactly one known struct has
        // this field, constrain the var to that struct (issue #12).
        if let Ty::Var(v) = base {
            let mut candidates: Vec<(&String, &Ty)> = self
                .structs
                .iter()
                .filter_map(|(name, fields)| fields.get(field).map(|ty| (name, ty)))
                .collect();
            candidates.sort_by(|a, b| a.0.cmp(b.0));
            match candidates.as_slice() {
                [(name, field_ty)] => {
                    unify(
                        &mut self.ctx,
                        &Ty::Var(v),
                        &Ty::Named {
                            name: (*name).clone(),
                            args: vec![],
                        },
                    )?;
                    return Ok((*field_ty).clone());
                }
                [] => {}
                many => {
                    let names = many
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(TypeError::AmbiguousField {
                        field: field.to_string(),
                        candidates: names,
                        span,
                    });
                }
            }
        }
        Err(TypeError::UnknownType {
            name: field.to_string(),
            span,
        })
    }

    fn lookup(&mut self, env: &TypeEnv, name: &str, span: Span) -> Result<Ty, TypeError> {
        let scheme = env.get(name).ok_or_else(|| TypeError::UnknownName {
            name: name.to_string(),
            span,
        })?;
        Ok(instantiate(&mut self.ctx, scheme))
    }

    fn ast_type(&mut self, ty: &Type) -> Result<Ty, TypeError> {
        match &ty.kind {
            TypeKind::Named(id) => match id.name.as_str() {
                "Never" => Ok(Ty::Never),
                "()" => Ok(Ty::Unit),
                "int" => Ok(Ty::Int),
                "uint" => Ok(Ty::UInt),
                "float" => Ok(Ty::Float),
                "bool" => Ok(Ty::Bool),
                "char" => Ok(Ty::Char),
                "str" => Ok(Ty::Str),
                other => Ok(Ty::Named {
                    name: other.to_string(),
                    args: vec![],
                }),
            },
            TypeKind::Never => Ok(Ty::Never),
            TypeKind::Unit => Ok(Ty::Unit),
            TypeKind::Option(inner) => Ok(Ty::Option(Box::new(self.ast_type(inner)?))),
            TypeKind::Ref { mutable, inner } => Ok(Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.ast_type(inner)?),
            }),
            TypeKind::Tuple(ts) => Ok(Ty::Tuple(
                ts.iter()
                    .map(|t| self.ast_type(t))
                    .collect::<Result<_, _>>()?,
            )),
            TypeKind::Array { elem, len } => Ok(Ty::Array {
                elem: Box::new(self.ast_type(elem)?),
                len: *len,
            }),
            TypeKind::Slice(inner) => Ok(Ty::Slice(Box::new(self.ast_type(inner)?))),
            TypeKind::Fn { params, ret } => Ok(Ty::Fn {
                params: params
                    .iter()
                    .map(|p| self.ast_type(p))
                    .collect::<Result<_, _>>()?,
                ret: Box::new(self.ast_type(ret)?),
            }),
            TypeKind::Generic { base, args } => {
                let base_ty = self.ast_type(base)?;
                if let Ty::Named { name, .. } = base_ty {
                    Ok(Ty::Named {
                        name,
                        args: args
                            .iter()
                            .map(|a| self.ast_type(a))
                            .collect::<Result<_, _>>()?,
                    })
                } else {
                    Ok(base_ty)
                }
            }
            TypeKind::Constrained { inner, .. } => self.ast_type(inner),
        }
    }
}

fn stdlib_fn_types() -> Vec<(&'static str, Ty)> {
    vec![
        (
            "new",
            Ty::Fn {
                params: vec![],
                ret: Box::new(Ty::Named {
                    name: "vec".into(),
                    args: vec![],
                }),
            },
        ),
        (
            "push",
            Ty::Fn {
                params: vec![
                    Ty::Named {
                        name: "vec".into(),
                        args: vec![],
                    },
                    Ty::Int,
                ],
                ret: Box::new(Ty::Unit),
            },
        ),
        (
            "len",
            Ty::Fn {
                params: vec![Ty::Named {
                    name: "vec".into(),
                    args: vec![],
                }],
                ret: Box::new(Ty::Int),
            },
        ),
        (
            "read_to_string",
            Ty::Fn {
                params: vec![Ty::StrSlice],
                ret: Box::new(Ty::Str),
            },
        ),
        (
            "sleep_ms",
            Ty::Fn {
                params: vec![Ty::Int],
                ret: Box::new(Ty::Unit),
            },
        ),
        (
            "parse_ip",
            Ty::Fn {
                params: vec![Ty::Str],
                ret: Box::new(Ty::Str),
            },
        ),
    ]
}

/// Known Rust crate item stubs for typeck (Result Ok payload types).
fn rust_import_fn_type(crate_name: &str, item: &str) -> (Vec<Ty>, Ty) {
    let json_value = Ty::Named {
        name: "serde_json::Value".into(),
        args: vec![],
    };
    match (crate_name, item) {
        ("serde_json", "from_str") => (vec![Ty::Str], json_value),
        ("serde_json", "to_string" | "to_string_pretty" | "to_vec") => (vec![json_value], Ty::Str),
        ("serde_json", "from_value") => (vec![json_value.clone()], json_value),
        // Type-like imports (e.g. `Value as JsonValue`) — not callable; placeholder unit fn.
        ("serde_json", "Value") => (vec![], json_value),
        ("ureq", "get") => (vec![Ty::Str], Ty::Str),
        _ => (
            vec![Ty::Str],
            Ty::Named {
                name: "RustValue".into(),
                args: vec![],
            },
        ),
    }
}

/// Whether a known `rust = true` import returns Rust `Result` and should lower via Crisp `?`
/// / ambient errors (spec §14.2) instead of panic `.expect`.
pub fn rust_import_returns_result(crate_name: &str, item: &str) -> bool {
    matches!(
        (crate_name, item),
        (
            "serde_json",
            "from_str" | "to_string" | "to_string_pretty" | "to_vec" | "from_value"
        ) | ("ureq", "get")
    )
}
