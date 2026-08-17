use crate::display::format_ty;
use crate::env::{TypeEnv, collect_free_vars, generalize, instantiate, scheme, substitute_var};
use crate::types::{InferContext, InferredSig, Scheme, Ty, is_arith_bound};
use crate::unify::{UnifyError, unify};
use crisp_ast::Span;
use crisp_ast::count_holes;
use crisp_ast::expr::{BinaryOp, Block, Expr, ExprKind, FieldInit, Stmt, UnaryOp};
use crisp_ast::ident::Ident;
use crisp_ast::is_hole_ident;
use crisp_ast::item::{ExternBlock, FunctionDef, ImplBlock, Item, SourceFile, TypeBody};
use crisp_ast::lift_holes;
use crisp_ast::pat::{Pat, PatKind};
use crisp_ast::ty::{Type, TypeKind};
use crisp_resolve::module::load_module_graph;
use crisp_resolve::{ResolvedRustImport, Resolver};
use std::collections::{BTreeMap, BTreeSet};
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
    #[error("[E0084] cannot instantiate `{func}` with `{ty}`: `{ty}` does not implement `{bound}`")]
    UnsatisfiedBound {
        func: String,
        ty: String,
        bound: String,
        span: Span,
    },
    #[error(
        "[E0085] implicit closure has {found} hole(s) `_` but a function of {expected} parameter(s) is expected; write `|x, y| …`"
    )]
    HoleArity {
        expected: usize,
        found: usize,
        span: Span,
    },
    #[error(
        "[E0086] hole `_` is only valid where a function value is expected; write `|x| …` or use `_` as a call argument"
    )]
    HoleMisplaced { span: Span },
    #[error("unification error: {message}")]
    UnifyAt { message: String, span: Span },
}

#[derive(Debug, Clone)]
pub struct TypedCrate {
    pub signatures: BTreeMap<String, InferredSig>,
    /// `TypeName` → `method` → signature key (`module::Type::method`).
    pub inherent_methods: BTreeMap<String, BTreeMap<String, String>>,
    /// Resolved `use <crate> { … }` / `use rust.<crate> { … }` imports (spec §14.2).
    pub rust_imports: Vec<ResolvedRustImport>,
    /// `module::Trait for Type` → inferred trait args when the impl omitted `<>` (#77).
    pub impl_trait_args: BTreeMap<String, Vec<Ty>>,
}

#[derive(Debug, Clone)]
struct TraitMethodStub {
    params: Vec<(String, Option<Ty>)>,
    ret: Option<Ty>,
}

#[derive(Debug, Clone)]
struct CallInst {
    args: Vec<Ty>,
    span: Span,
}

pub struct TypeChecker {
    ctx: InferContext,
    env: TypeEnv,
    structs: BTreeMap<String, BTreeMap<String, Ty>>,
    /// Named `shape` definitions (structural; also present in `structs` for field access).
    shapes: BTreeSet<String>,
    /// Type/shape name → declared type-parameter names (`Pair` → `["A", "B"]`).
    type_params: BTreeMap<String, Vec<String>>,
    /// Trait name → declared type-parameter names.
    trait_generics: BTreeMap<String, Vec<String>>,
    /// Rigid generic bindings in the current definition (`T` → `Named("T")`).
    generic_params: BTreeMap<String, Ty>,
    /// Enum name → variant name → payload field types.
    enums: BTreeMap<String, BTreeMap<String, Vec<Ty>>>,
    /// Trait name → method stubs (`self` filled at impl site).
    traits: BTreeMap<String, BTreeMap<String, TraitMethodStub>>,
    signatures: BTreeMap<String, InferredSig>,
    /// `TypeName` → `method` → signature key.
    inherent_methods: BTreeMap<String, BTreeMap<String, String>>,
    /// Stack of expected `break` value types for nested `loop` expressions (§6.3).
    loop_break_tys: Vec<Ty>,
    /// Fresh vars for omitted impl trait args (`module::Trait for Type`).
    impl_trait_fresh: BTreeMap<String, Vec<Ty>>,
    /// Finalized inferred impl trait args.
    impl_trait_args: BTreeMap<String, Vec<Ty>>,
    /// Call-site argument types for crate-internal specialization (#76).
    fn_instantiations: BTreeMap<String, Vec<CallInst>>,
    /// Named generics used in operators / unique trait methods (`T` → `Add` / `Show`).
    arith_named: BTreeMap<String, BTreeSet<String>>,
    /// Unannotated type vars used as bound subjects (mapped to names after generalization).
    arith_vars: BTreeMap<u32, BTreeSet<String>>,
    /// `TypeName` → traits implemented in this crate (`Point` → `Show`).
    trait_impls: BTreeMap<String, BTreeSet<String>>,
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
        // Apply substitutions from later modules, then name leftover holes.
        // Otherwise `math.double` (before `math.scale`) publishes `twice<T>(x: float) -> T`.
        checker.seal_open_signatures();
        checker.specialize_internal_functions()?;
        Ok(TypedCrate {
            signatures: checker.signatures,
            inherent_methods: checker.inherent_methods,
            rust_imports: resolved.rust_imports,
            impl_trait_args: checker.impl_trait_args,
        })
    }

    fn new() -> Self {
        Self {
            ctx: InferContext::new(),
            env: TypeEnv::new(),
            structs: BTreeMap::new(),
            shapes: BTreeSet::new(),
            type_params: BTreeMap::new(),
            trait_generics: BTreeMap::new(),
            generic_params: BTreeMap::new(),
            enums: BTreeMap::new(),
            traits: BTreeMap::new(),
            signatures: BTreeMap::new(),
            inherent_methods: BTreeMap::new(),
            loop_break_tys: Vec::new(),
            impl_trait_fresh: BTreeMap::new(),
            impl_trait_args: BTreeMap::new(),
            fn_instantiations: BTreeMap::new(),
            arith_named: BTreeMap::new(),
            arith_vars: BTreeMap::new(),
            trait_impls: BTreeMap::new(),
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
                    generics: Vec::new(),
                    is_pub: false,
                    inferred_from_use: false,
                    instantiations: Vec::new(),
                    mono_args: None,
                    op_bounds: BTreeMap::new(),
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
        self.register_prelude_traits();
    }

    fn register_prelude_traits(&mut self) {
        self.traits.entry("Show".into()).or_insert_with(|| {
            BTreeMap::from([(
                "show".into(),
                TraitMethodStub {
                    params: vec![("self".into(), None)],
                    ret: Some(Ty::Str),
                },
            )])
        });
        self.traits.entry("Eq".into()).or_insert_with(|| {
            BTreeMap::from([(
                "equal".into(),
                TraitMethodStub {
                    params: vec![("self".into(), None), ("other".into(), None)],
                    ret: Some(Ty::Bool),
                },
            )])
        });
        self.traits.entry("Ord".into()).or_insert_with(|| {
            BTreeMap::from([(
                "compare".into(),
                TraitMethodStub {
                    params: vec![("self".into(), None), ("other".into(), None)],
                    ret: Some(Ty::Int),
                },
            )])
        });
    }

    fn collect_types(&mut self, module: &str, file: &SourceFile) {
        for item in &file.items {
            if let Item::TypeDef(td) = item {
                let gens: Vec<String> = td.generics.iter().map(|g| g.name.clone()).collect();
                if !gens.is_empty() {
                    self.type_params.insert(td.name.name.clone(), gens.clone());
                }
                let saved = self.bind_rigid_generics(&gens);
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
                self.generic_params = saved;
            } else if let Item::ShapeDef(shape) = item {
                // Data shapes participate in field access like structs (#61).
                let gens: Vec<String> = shape.generics.iter().map(|g| g.name.clone()).collect();
                if !gens.is_empty() {
                    self.type_params
                        .insert(shape.name.name.clone(), gens.clone());
                }
                let saved = self.bind_rigid_generics(&gens);
                let mut field_map = BTreeMap::new();
                for f in &shape.fields {
                    if let crisp_ast::item::ShapeField::Data { name, ty, .. } = f
                        && let Ok(field_ty) = self.ast_type(ty)
                    {
                        field_map.insert(name.name.clone(), self.ctx.apply(&field_ty));
                    }
                }
                self.shapes.insert(shape.name.name.clone());
                self.structs.insert(shape.name.name.clone(), field_map);
                self.env.insert(
                    shape.name.name.clone(),
                    scheme(Ty::Named {
                        name: shape.name.name.clone(),
                        args: vec![],
                    }),
                );
                self.generic_params = saved;
            } else if let Item::TraitDef(td) = item {
                let gens: Vec<String> = td.generics.iter().map(|g| g.name.clone()).collect();
                if !gens.is_empty() {
                    self.trait_generics
                        .insert(td.name.name.clone(), gens.clone());
                }
                let saved = self.bind_rigid_generics(&gens);
                let mut methods = BTreeMap::new();
                for m in &td.items {
                    let mut params = Vec::new();
                    let mut ok = true;
                    for p in &m.params {
                        let ty = if p.name.name == "self" && p.ty.is_none() {
                            None
                        } else if let Some(ast_ty) = &p.ty {
                            match self.ast_type(ast_ty) {
                                Ok(t) => Some(t),
                                Err(_) => {
                                    ok = false;
                                    break;
                                }
                            }
                        } else {
                            Some(self.ctx.fresh())
                        };
                        params.push((p.name.name.clone(), ty));
                    }
                    if !ok {
                        continue;
                    }
                    let ret = match &m.ret_type {
                        Some(t) => match self.ast_type(t) {
                            Ok(t) => Some(t),
                            Err(_) => continue,
                        },
                        None => None,
                    };
                    methods.insert(m.name.name.clone(), TraitMethodStub { params, ret });
                }
                self.traits.insert(td.name.name.clone(), methods);
                self.generic_params = saved;
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
                    let gens: Vec<String> = f.generics.iter().map(|g| g.name.clone()).collect();
                    let saved = self.bind_rigid_generics(&gens);
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
                    let fn_ty = Ty::Fn {
                        params,
                        ret: Box::new(ret),
                    };
                    self.env.insert(
                        f.name.name.clone(),
                        generalize_named_params(&fn_ty, &gens, &mut self.ctx),
                    );
                    self.generic_params = saved;
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
        if let Some(tn) = &ib.trait_name {
            self.trait_impls
                .entry(ty_name.clone())
                .or_default()
                .insert(tn.name.clone());
        }
        let trait_subst = self.impl_trait_subst(module, ib, &ty_name)?;
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
            let mut ret = if let Some(t) = &f.ret_type {
                self.ast_type(t)?
            } else {
                self.ctx.fresh()
            };
            if !trait_subst.is_empty() {
                params = params
                    .into_iter()
                    .map(|t| subst_named_params(&t, &trait_subst))
                    .collect();
                ret = subst_named_params(&ret, &trait_subst);
            }
            if let Some(tn) = &ib.trait_name
                && let Some(methods) = self.traits.get(&tn.name)
                && let Some(stub) = methods.get(&f.name.name)
                && let Some(trait_ret) = &stub.ret
            {
                ret = subst_named_params(trait_ret, &trait_subst);
            }
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
                    generics: Vec::new(),
                    is_pub: f.is_pub,
                    inferred_from_use: false,
                    instantiations: Vec::new(),
                    mono_args: None,
                    op_bounds: BTreeMap::new(),
                },
            );
        }
        // Trait impl: register remaining trait methods (including defaults) on the type (#59).
        if let Some(tn) = &ib.trait_name
            && let Some(trait_methods) = self.traits.get(&tn.name).cloned()
        {
            for (mname, stub) in trait_methods {
                let methods = self.inherent_methods.entry(ty_name.clone()).or_default();
                if methods.contains_key(&mname) {
                    continue;
                }
                let key = format!("{module}::{ty_name}::{mname}");
                methods.insert(mname.clone(), key.clone());
                let sig_params: Vec<(String, Ty)> = stub
                    .params
                    .into_iter()
                    .map(|(pname, pty)| {
                        let ty = if pname == "self" {
                            self_ty.clone()
                        } else {
                            let t = pty.unwrap_or_else(|| self.ctx.fresh());
                            subst_named_params(&t, &trait_subst)
                        };
                        (pname, ty)
                    })
                    .collect();
                let sig_ret =
                    subst_named_params(&stub.ret.unwrap_or_else(|| self.ctx.fresh()), &trait_subst);
                self.signatures.insert(
                    key,
                    InferredSig {
                        module: module.to_string(),
                        name: mname,
                        impl_ty: Some(ty_name.clone()),
                        params: sig_params,
                        ret: sig_ret,
                        span: ib.span,
                        generics: Vec::new(),
                        is_pub: false,
                        inferred_from_use: false,
                        instantiations: Vec::new(),
                        mono_args: None,
                        op_bounds: BTreeMap::new(),
                    },
                );
            }
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

    /// Env used to decide which holes may be named now. Skip this item (its own
    /// stub still holds the body vars) and skip already-checked items so a later
    /// `id(x) = x` can still generalize after an earlier `wrap(x) = id(x)`.
    fn env_for_generalize(&self, self_name: &str) -> TypeEnv {
        let mut env = self.env.clone();
        env.remove(self_name);
        for sig in self.signatures.values() {
            if sig.impl_ty.is_some() || sig.name.starts_with("test::") {
                continue;
            }
            if sig.name != self_name {
                env.remove(&sig.name);
            }
        }
        env
    }

    /// After every body has been checked, substitute and name leftover holes.
    fn seal_open_signatures(&mut self) {
        let keys: Vec<String> = self.signatures.keys().cloned().collect();
        for key in keys {
            let Some(sig) = self.signatures.get(&key).cloned() else {
                continue;
            };
            let param_types: Vec<(String, Ty)> = sig
                .params
                .iter()
                .map(|(n, t)| (n.clone(), self.ctx.apply(t)))
                .collect();
            let ret = self.ctx.apply(&sig.ret);
            if !sig.generics.is_empty() {
                if let Some(s) = self.signatures.get_mut(&key) {
                    s.params = param_types;
                    s.ret = ret;
                }
                continue;
            }
            let fn_ty = Ty::Fn {
                params: param_types.iter().map(|(_, t)| t.clone()).collect(),
                ret: Box::new(ret.clone()),
            };
            let (named, inferred) = name_free_vars(&fn_ty);
            if inferred.is_empty() {
                if let Some(s) = self.signatures.get_mut(&key) {
                    s.params = param_types;
                    s.ret = ret;
                }
                continue;
            }
            let mut params = param_types;
            let mut named_ret = ret;
            if let Ty::Fn { params: ps, ret: r } = &named {
                for (i, t) in ps.iter().enumerate() {
                    if let Some(slot) = params.get_mut(i) {
                        slot.1 = t.clone();
                    }
                }
                named_ret = r.as_ref().clone();
            }
            if let Some(s) = self.signatures.get_mut(&key) {
                s.params = params;
                s.ret = named_ret;
                s.generics = inferred;
                s.inferred_from_use = true;
            }
        }
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
        self.finalize_impl_trait_args(module, ib, &ty_name)?;
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
                generics: Vec::new(),
                is_pub: f.is_pub,
                inferred_from_use: false,
                instantiations: Vec::new(),
                mono_args: None,
                op_bounds: BTreeMap::new(),
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
                    generics: Vec::new(),
                    is_pub: false,
                    inferred_from_use: false,
                    instantiations: Vec::new(),
                    mono_args: None,
                    op_bounds: BTreeMap::new(),
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
                generics: Vec::new(),
                is_pub: false,
                inferred_from_use: false,
                instantiations: Vec::new(),
                mono_args: None,
                op_bounds: BTreeMap::new(),
            },
        );
        Ok(())
    }

    fn check_function(&mut self, module: &str, f: &FunctionDef) -> Result<(), TypeError> {
        self.arith_named.clear();
        self.arith_vars.clear();
        let gens: Vec<String> = f.generics.iter().map(|g| g.name.clone()).collect();
        let saved = self.bind_rigid_generics(&gens);
        // Reuse stub param/ret vars so call-site unifications from earlier modules stick.
        // Explicit generics are instantiated per call; the body is checked with rigid names.
        let stub = if gens.is_empty() {
            self.env
                .get(&f.name.name)
                .map(|s| instantiate(&mut self.ctx, s))
                .map(|t| self.ctx.apply(&t))
        } else {
            None
        };
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
        let param_types: Vec<(String, Ty)> = param_vars
            .iter()
            .map(|(n, t)| (n.clone(), self.ctx.apply(t)))
            .collect();
        let ret = self.ctx.apply(&ret);
        let fn_params: Vec<Ty> = param_types.iter().map(|(_, t)| t.clone()).collect();
        let mut fn_ty = Ty::Fn {
            params: fn_params,
            ret: Box::new(ret.clone()),
        };
        let mut gens = gens;
        let mut param_types = param_types;
        let mut ret = ret;
        let mut inferred_from_use = false;
        // Unannotated items with leftover free vars become a scheme (#76).
        // Forward-ref calls that already pinned the stub stay monomorphic.
        // Explicit `<>` / free type names (`x: T`) are pins and are not specialized.
        // Do not name holes that still belong to unchecked stubs (callee later in
        // filename order): `twice(x) = scale(x, 2.0)` must wait for `scale`.
        let pre_ty = fn_ty.clone();
        if gens.is_empty() {
            let env_wo = self.env_for_generalize(&f.name.name);
            let gen_scheme = generalize(&env_wo, &mut self.ctx, &fn_ty);
            if !gen_scheme.vars.is_empty() {
                let (named, inferred) = name_vars(&fn_ty, &gen_scheme.vars);
                if !inferred.is_empty() {
                    fn_ty = named;
                    gens = inferred;
                    inferred_from_use = true;
                    if let Ty::Fn { params, ret: r } = &fn_ty {
                        for (i, t) in params.iter().enumerate() {
                            if let Some(slot) = param_types.get_mut(i) {
                                slot.1 = t.clone();
                            }
                        }
                        ret = r.as_ref().clone();
                    }
                }
            }
        }
        let op_bounds = self.take_op_bounds(&pre_ty, &gens);
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
                generics: gens.clone(),
                is_pub: f.is_pub,
                inferred_from_use,
                instantiations: Vec::new(),
                mono_args: None,
                op_bounds,
            },
        );
        self.env.insert(
            f.name.name.clone(),
            if gens.is_empty() {
                scheme(fn_ty)
            } else {
                generalize_named_params(&fn_ty, &gens, &mut self.ctx)
            },
        );
        self.generic_params = saved;
        Ok(())
    }

    /// Record call-site instantiations. Internal single-use schemes set `mono_args`
    /// for emit; the typeck scheme stays generic so ownership still sees `T` (#76).
    /// Concrete instantiations must satisfy inferred bounds (#84).
    fn specialize_internal_functions(&mut self) -> Result<(), TypeError> {
        let insts = std::mem::take(&mut self.fn_instantiations);
        self.check_instantiation_bounds(&insts)?;
        let keys: Vec<String> = self.signatures.keys().cloned().collect();
        for key in keys {
            let Some(sig) = self.signatures.get(&key) else {
                continue;
            };
            if let Some(uses) = insts.get(&sig.name) {
                let mut labels: Vec<String> = uses
                    .iter()
                    .map(|u| u.args.iter().map(format_ty).collect::<Vec<_>>().join(", "))
                    .collect();
                labels.sort();
                labels.dedup();
                if let Some(sig) = self.signatures.get_mut(&key) {
                    sig.instantiations = labels;
                }
            }
            let Some(sig) = self.signatures.get(&key) else {
                continue;
            };
            if sig.is_pub
                || !sig.inferred_from_use
                || sig.generics.is_empty()
                || sig.impl_ty.is_some()
            {
                continue;
            }
            let Some(uses) = insts.get(&sig.name) else {
                continue;
            };
            if uses.is_empty() {
                continue;
            }
            let first = &uses[0].args;
            if !first.iter().all(ty_is_ground) {
                continue;
            }
            if !uses.iter().all(|u| &u.args == first) {
                continue;
            }
            if let Some(sig) = self.signatures.get_mut(&key) {
                sig.mono_args = Some(first.clone());
            }
        }
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
            ExprKind::Ident(id) if is_hole_ident(&id.name) => {
                Err(TypeError::HoleMisplaced { span: id.span })
            }
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
                        return Err(TypeError::UnifyAt {
                            message: format!("type mismatch: expected function, found {other:?}"),
                            span: expr.span,
                        });
                    }
                };
                if args.len() != params.len() {
                    return Err(TypeError::Unify(UnifyError::Mismatch {
                        expected: format!("{} arguments", params.len()),
                        found: format!("{} arguments", args.len()),
                    }));
                }
                for (arg, pty) in args.iter().zip(params.iter()) {
                    let aty = self.infer_call_arg(env, arg, pty)?;
                    self.unify_or_shape(&aty, pty)?;
                }
                if let ExprKind::Ident(id) = &func.kind {
                    let applied: Vec<Ty> = params.iter().map(|p| self.ctx.apply(p)).collect();
                    if applied.iter().all(ty_is_ground) {
                        self.propagate_callee_bounds(&id.name, &applied);
                        self.fn_instantiations
                            .entry(id.name.clone())
                            .or_default()
                            .push(CallInst {
                                args: applied,
                                span: expr.span,
                            });
                    }
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
            ExprKind::StructLit { name, fields } => self.check_struct_lit(env, name, fields),
            ExprKind::Bind { pat, value, .. } => {
                let ty = self.infer_value(env, value)?;
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
            ExprKind::While { cond, body } => {
                let cty = self.infer_expr(env, cond)?;
                unify(&mut self.ctx, &cty, &Ty::Bool)?;
                self.infer_expr(env, body)?;
                Ok(Ty::Unit)
            }
            ExprKind::For { pat, iter, body } => {
                let iter_ty = self.infer_expr(env, iter)?;
                let vec_ty = Ty::Named {
                    name: "vec".into(),
                    args: vec![],
                };
                // MVP: `for` iterates `vec` (emits `Vec<i64>`). Unify unconstrained
                // iterators with `vec`; element type is `int`.
                let item_ty = match self.ctx.apply(&iter_ty) {
                    Ty::Named { name, .. } if name == "vec" => Ty::Int,
                    other => {
                        unify(&mut self.ctx, &other, &vec_ty)?;
                        Ty::Int
                    }
                };
                let mut local = env.clone();
                self.infer_pat(&mut local, pat, &item_ty)?;
                self.infer_expr(&mut local, body)?;
                Ok(Ty::Unit)
            }
            ExprKind::Loop(body) => {
                let break_ty = self.ctx.fresh();
                self.loop_break_tys.push(break_ty.clone());
                let _ = self.infer_expr(env, body)?;
                self.loop_break_tys.pop();
                Ok(self.ctx.apply(&break_ty))
            }
            ExprKind::Break(value) => {
                let vt = if let Some(v) = value {
                    self.infer_expr(env, v)?
                } else {
                    Ty::Unit
                };
                if let Some(expected) = self.loop_break_tys.last().cloned() {
                    unify(&mut self.ctx, &vt, &expected)?;
                }
                Ok(Ty::Never)
            }
            ExprKind::Continue => Ok(Ty::Never),
            ExprKind::Assign { target, value } => {
                let expected = self.lookup(env, &target.name, target.span)?;
                let got = self.infer_value(env, value)?;
                unify(&mut self.ctx, &got, &expected).map_err(|e| TypeError::UnifyAt {
                    message: e.to_string(),
                    span: expr.span,
                })?;
                Ok(Ty::Unit)
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
                let t = self.ctx.apply(&lt);
                let r = self.ctx.apply(&rt);
                if matches!(t, Ty::Float) || matches!(r, Ty::Float) {
                    unify(&mut self.ctx, &t, &Ty::Float)?;
                    Ok(Ty::Float)
                } else if matches!(t, Ty::Int | Ty::UInt) || matches!(r, Ty::Int | Ty::UInt) {
                    unify(&mut self.ctx, &t, &Ty::Int)?;
                    Ok(Ty::Int)
                } else if let Some(op) = arith_trait_name(op) {
                    self.record_arith(&t, op);
                    Ok(t)
                } else {
                    unify(&mut self.ctx, &t, &Ty::Int)?;
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

    fn infer_value(&mut self, env: &mut TypeEnv, expr: &Expr) -> Result<Ty, TypeError> {
        if count_holes(expr) > 0 {
            self.infer_hole_lambda(env, expr, None)
        } else {
            self.infer_expr(env, expr)
        }
    }

    fn infer_call_arg(
        &mut self,
        env: &mut TypeEnv,
        arg: &Expr,
        expected: &Ty,
    ) -> Result<Ty, TypeError> {
        let expected = self.ctx.apply(expected);
        if count_holes(arg) > 0 {
            return self.infer_hole_lambda(env, arg, Some(&expected));
        }
        self.infer_expr(env, arg)
    }

    fn infer_hole_lambda(
        &mut self,
        env: &mut TypeEnv,
        expr: &Expr,
        expected: Option<&Ty>,
    ) -> Result<Ty, TypeError> {
        let found = count_holes(expr);
        if found == 0 {
            return self.infer_expr(env, expr);
        }
        if let Some(exp) = expected {
            match self.ctx.apply(exp) {
                Ty::Fn { params, .. } if params.len() != found => {
                    return Err(TypeError::HoleArity {
                        expected: params.len(),
                        found,
                        span: expr.span,
                    });
                }
                Ty::Fn { .. } => {}
                _ => return Err(TypeError::HoleMisplaced { span: expr.span }),
            }
        }
        let lifted = lift_holes(expr).expect("count_holes > 0");
        self.infer_expr(env, &lifted)
    }

    fn infer_block(&mut self, env: &mut TypeEnv, block: &Block) -> Result<Ty, TypeError> {
        let mut local = env.clone();
        for stmt in &block.stmts {
            match stmt {
                Stmt::Bind { pat, value, .. } => {
                    let ty = self.infer_value(&mut local, value)?;
                    self.infer_pat(&mut local, pat, &ty)?;
                }
                Stmt::Assign { target, value } => {
                    let expected = self.lookup(&local, &target.name, target.span)?;
                    let got = self.infer_value(&mut local, value)?;
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
                // Value restriction (#78): locals and `mut` bindings stay monomorphic.
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
    ) -> Result<Ty, TypeError> {
        let schema =
            self.structs
                .get(&name.name)
                .cloned()
                .ok_or_else(|| TypeError::UnknownType {
                    name: name.name.clone(),
                    span: name.span,
                })?;
        let gens = self
            .type_params
            .get(&name.name)
            .cloned()
            .unwrap_or_default();
        let subst: BTreeMap<String, Ty> =
            gens.iter().map(|g| (g.clone(), self.ctx.fresh())).collect();
        let schema: BTreeMap<String, Ty> = schema
            .iter()
            .map(|(k, v)| (k.clone(), subst_named_params(v, &subst)))
            .collect();
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
        let args: Vec<Ty> = gens
            .iter()
            .map(|g| self.ctx.apply(subst.get(g).expect("generic subst")))
            .collect();
        Ok(Ty::Named {
            name: name.name.clone(),
            args,
        })
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

        // Instance: `v.magnitude()` / `v.scale(2.0)` / generic `x.show()` → `T: Show` (#84).
        let base_ty = self.infer_expr(env, base)?;
        let base_ty = self.ctx.apply(&base_ty);
        if self.is_bound_subject(&base_ty)
            && let Some(ret) = self.try_infer_bound_method(env, &base_ty, field, args)?
        {
            return Ok(Some(ret));
        }

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
        if let Ty::Named { name, args } = &base
            && let Some(fields) = self.instantiate_schema(name, args)
        {
            return fields.get(field).cloned().ok_or(TypeError::UnknownType {
                name: field.to_string(),
                span,
            });
        }
        // Unannotated params stay as type vars. If exactly one known struct has
        // this field, constrain the var to that struct (issue #12).
        // Exclude shapes — they are constraints, not concrete constructors.
        if let Ty::Var(v) = base {
            let mut candidates: Vec<(&String, &Ty)> = self
                .structs
                .iter()
                .filter(|(name, _)| !self.shapes.contains(*name))
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

    /// Unify normally, or accept structural match when the expected type is a shape (§3.5).
    fn unify_or_shape(&mut self, actual: &Ty, expected: &Ty) -> Result<(), TypeError> {
        let expected = self.ctx.apply(expected);
        if let Ty::Named { name, args } = &expected
            && self.shapes.contains(name)
        {
            return self.check_shape_arg(actual, name, args);
        }
        unify(&mut self.ctx, actual, &expected)?;
        Ok(())
    }

    fn check_shape_arg(
        &mut self,
        actual: &Ty,
        shape_name: &str,
        shape_args: &[Ty],
    ) -> Result<(), TypeError> {
        let actual = self.ctx.apply(actual);
        let Some(shape_fields) = self.instantiate_schema(shape_name, shape_args) else {
            return Err(TypeError::UnknownType {
                name: shape_name.to_string(),
                span: Span::new(0, 0),
            });
        };
        match &actual {
            Ty::Named { name, args } if name == shape_name => {
                if args.len() == shape_args.len() {
                    for (a, b) in args.iter().zip(shape_args) {
                        unify(&mut self.ctx, a, b)?;
                    }
                }
                Ok(())
            }
            Ty::Named { name, args } => {
                let Some(fields) = self.instantiate_schema(name, args) else {
                    return Err(TypeError::Unify(UnifyError::Mismatch {
                        expected: format!("type satisfying shape `{shape_name}`"),
                        found: name.clone(),
                    }));
                };
                for (fname, fty) in &shape_fields {
                    let Some(aty) = fields.get(fname) else {
                        return Err(TypeError::Unify(UnifyError::Mismatch {
                            expected: format!("shape `{shape_name}` (field `{fname}: {fty:?}`)"),
                            found: name.clone(),
                        }));
                    };
                    unify(&mut self.ctx, aty, fty).map_err(|err| {
                        TypeError::Unify(UnifyError::Mismatch {
                            expected: format!("shape `{shape_name}` (field `{fname}: {fty:?}`)"),
                            found: format!("{name} ({err})"),
                        })
                    })?;
                }
                Ok(())
            }
            other => Err(TypeError::Unify(UnifyError::Mismatch {
                expected: format!("type satisfying shape `{shape_name}`"),
                found: format!("{other:?}"),
            })),
        }
    }

    fn lookup(&mut self, env: &TypeEnv, name: &str, span: Span) -> Result<Ty, TypeError> {
        let scheme = env.get(name).ok_or_else(|| TypeError::UnknownName {
            name: name.to_string(),
            span,
        })?;
        Ok(instantiate(&mut self.ctx, scheme))
    }

    fn record_arith(&mut self, ty: &Ty, op: &str) {
        self.record_bound(ty, op);
    }

    fn record_bound(&mut self, ty: &Ty, bound: &str) {
        match self.ctx.apply(ty) {
            Ty::Named { name, args } if args.is_empty() => {
                self.arith_named
                    .entry(name)
                    .or_default()
                    .insert(bound.into());
            }
            Ty::Var(v) => {
                self.arith_vars.entry(v).or_default().insert(bound.into());
            }
            _ => {}
        }
    }

    fn take_op_bounds(&mut self, pre_ty: &Ty, gens: &[String]) -> BTreeMap<String, Vec<String>> {
        let mut named = std::mem::take(&mut self.arith_named);
        let vars = std::mem::take(&mut self.arith_vars);
        let mut applied_vars: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        // Body recording may key a var that later unified with the stub ret (#84).
        for (v, ops) in vars {
            match self.ctx.apply(&Ty::Var(v)) {
                Ty::Named { name, args } if args.is_empty() => {
                    named.entry(name).or_default().extend(ops);
                }
                Ty::Var(w) => {
                    applied_vars.entry(w).or_default().extend(ops);
                }
                _ => {}
            }
        }
        let mut free = Vec::new();
        collect_free_vars(pre_ty, &mut free);
        free.sort_unstable();
        free.dedup();
        for (i, v) in free.iter().enumerate() {
            if let Some(ops) = applied_vars.get(v) {
                named
                    .entry(generic_name(i))
                    .or_default()
                    .extend(ops.iter().cloned());
            }
        }
        let mut out = BTreeMap::new();
        for g in gens {
            if let Some(ops) = named.remove(g) {
                let mut list: Vec<String> = ops.into_iter().collect();
                list.sort();
                out.insert(g.clone(), list);
            }
        }
        out
    }

    fn is_bound_subject(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Var(_) => true,
            Ty::Named { name, args } if args.is_empty() => self.generic_params.contains_key(name),
            _ => false,
        }
    }

    fn try_infer_bound_method(
        &mut self,
        env: &mut TypeEnv,
        base_ty: &Ty,
        field: &Ident,
        args: &[Expr],
    ) -> Result<Option<Ty>, TypeError> {
        let mut candidates: Vec<String> = self
            .traits
            .iter()
            .filter(|(name, methods)| {
                methods.contains_key(&field.name)
                    && self
                        .trait_generics
                        .get(*name)
                        .map(|g| g.is_empty())
                        .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();
        candidates.sort();
        match candidates.as_slice() {
            [] => Ok(None),
            [trait_name] => {
                let stub = self
                    .traits
                    .get(trait_name)
                    .and_then(|m| m.get(&field.name))
                    .cloned()
                    .ok_or_else(|| TypeError::UnknownName {
                        name: field.name.clone(),
                        span: field.span,
                    })?;
                self.record_bound(base_ty, trait_name);
                let param_tys: Vec<Ty> = stub
                    .params
                    .iter()
                    .skip(1)
                    .map(|(_, t)| t.clone().unwrap_or_else(|| base_ty.clone()))
                    .collect();
                if args.len() != param_tys.len() {
                    return Err(TypeError::Unify(UnifyError::Mismatch {
                        expected: format!("{} arguments", param_tys.len()),
                        found: format!("{} arguments", args.len()),
                    }));
                }
                for (arg, pty) in args.iter().zip(param_tys.iter()) {
                    let aty = self.infer_expr(env, arg)?;
                    unify(&mut self.ctx, &aty, pty)?;
                }
                Ok(Some(stub.ret.unwrap_or_else(|| base_ty.clone())))
            }
            many => Err(TypeError::Unify(UnifyError::Mismatch {
                expected: format!("unique trait providing `{}`", field.name),
                found: many.join(", "),
            })),
        }
    }

    fn free_fn_sig(&self, name: &str) -> Option<&InferredSig> {
        self.signatures
            .values()
            .find(|s| s.name == name && s.impl_ty.is_none())
    }

    fn propagate_callee_bounds(&mut self, fname: &str, applied: &[Ty]) {
        let Some(sig) = self.free_fn_sig(fname) else {
            return;
        };
        if sig.op_bounds.is_empty() {
            return;
        }
        let bounds = sig.op_bounds.clone();
        let gens = sig.generics.clone();
        let params = sig.params.clone();
        let mut subst = BTreeMap::new();
        for ((_, scheme_ty), inst_ty) in params.iter().zip(applied.iter()) {
            collect_generic_subst(scheme_ty, inst_ty, &gens, &mut subst);
        }
        for (g, bs) in &bounds {
            if let Some(ty) = subst.get(g) {
                for b in bs {
                    self.record_bound(ty, b);
                }
            }
        }
    }

    fn check_instantiation_bounds(
        &self,
        insts: &BTreeMap<String, Vec<CallInst>>,
    ) -> Result<(), TypeError> {
        for (fname, uses) in insts {
            let Some(sig) = self.free_fn_sig(fname) else {
                continue;
            };
            if sig.op_bounds.is_empty() || sig.generics.is_empty() {
                continue;
            }
            for use_site in uses {
                let mut subst = BTreeMap::new();
                for ((_, scheme_ty), inst_ty) in sig.params.iter().zip(use_site.args.iter()) {
                    collect_generic_subst(scheme_ty, inst_ty, &sig.generics, &mut subst);
                }
                for g in &sig.generics {
                    let Some(bounds) = sig.op_bounds.get(g) else {
                        continue;
                    };
                    let Some(ty) = subst.get(g) else {
                        continue;
                    };
                    if !self.ty_is_checkable(ty) {
                        continue;
                    }
                    for bound in bounds {
                        if !self.ty_implements(ty, bound) {
                            return Err(TypeError::UnsatisfiedBound {
                                func: fname.clone(),
                                ty: format_ty(ty),
                                bound: bound.clone(),
                                span: use_site.span,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn ty_is_checkable(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Int
            | Ty::UInt
            | Ty::Float
            | Ty::Bool
            | Ty::Char
            | Ty::Str
            | Ty::StrSlice
            | Ty::Unit
            | Ty::Never => true,
            Ty::Named { name, args } => {
                (self.structs.contains_key(name) || self.enums.contains_key(name))
                    && !self.shapes.contains(name)
                    && args.iter().all(|a| self.ty_is_checkable(a))
            }
            Ty::Option(inner) | Ty::Slice(inner) | Ty::Ref { inner, .. } => {
                self.ty_is_checkable(inner)
            }
            Ty::Tuple(ts) => ts.iter().all(|t| self.ty_is_checkable(t)),
            _ => false,
        }
    }

    fn ty_implements(&self, ty: &Ty, bound: &str) -> bool {
        if is_arith_bound(bound) {
            return matches!(ty, Ty::Int | Ty::UInt | Ty::Float);
        }
        match ty {
            Ty::Named { name, args } if args.is_empty() => self
                .trait_impls
                .get(name)
                .is_some_and(|s| s.contains(bound)),
            _ => false,
        }
    }

    fn bind_rigid_generics(&mut self, gens: &[String]) -> BTreeMap<String, Ty> {
        let saved = self.generic_params.clone();
        for g in gens {
            self.generic_params.insert(
                g.clone(),
                Ty::Named {
                    name: g.clone(),
                    args: vec![],
                },
            );
        }
        saved
    }

    fn instantiate_schema(&self, name: &str, args: &[Ty]) -> Option<BTreeMap<String, Ty>> {
        let fields = self.structs.get(name)?.clone();
        let Some(gens) = self.type_params.get(name) else {
            return Some(fields);
        };
        if args.len() != gens.len() {
            return Some(fields);
        }
        let subst: BTreeMap<String, Ty> = gens.iter().cloned().zip(args.iter().cloned()).collect();
        Some(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), subst_named_params(v, &subst)))
                .collect(),
        )
    }

    fn impl_trait_key(module: &str, trait_name: &str, ty_name: &str) -> String {
        format!("{module}::{trait_name} for {ty_name}")
    }

    fn impl_trait_subst(
        &mut self,
        module: &str,
        ib: &ImplBlock,
        ty_name: &str,
    ) -> Result<BTreeMap<String, Ty>, TypeError> {
        let Some(tn) = &ib.trait_name else {
            return Ok(BTreeMap::new());
        };
        let gens = self
            .trait_generics
            .get(&tn.name)
            .cloned()
            .unwrap_or_default();
        if gens.is_empty() {
            return Ok(BTreeMap::new());
        }
        if ib.trait_args.is_empty() {
            let mut subst = BTreeMap::new();
            let mut fresh = Vec::new();
            for g in &gens {
                let v = self.ctx.fresh();
                subst.insert(g.clone(), v.clone());
                fresh.push(v);
            }
            self.impl_trait_fresh
                .insert(Self::impl_trait_key(module, &tn.name, ty_name), fresh);
            return Ok(subst);
        }
        self.trait_arg_subst(&tn.name, &ib.trait_args)
    }

    fn finalize_impl_trait_args(
        &mut self,
        module: &str,
        ib: &ImplBlock,
        ty_name: &str,
    ) -> Result<(), TypeError> {
        let Some(tn) = &ib.trait_name else {
            return Ok(());
        };
        let key = Self::impl_trait_key(module, &tn.name, ty_name);
        let Some(fresh) = self.impl_trait_fresh.remove(&key) else {
            return Ok(());
        };
        let mut args = Vec::new();
        for t in fresh {
            let applied = self.ctx.apply(&t);
            if matches!(applied, Ty::Var(_)) {
                return Err(TypeError::UnknownType {
                    name: format!(
                        "cannot infer `{}` type arguments for `{ty_name}`; write `impl {}<...> for {ty_name}`",
                        tn.name, tn.name
                    ),
                    span: ib.span,
                });
            }
            args.push(applied);
        }
        self.impl_trait_args.insert(key, args);
        Ok(())
    }

    fn trait_arg_subst(
        &mut self,
        trait_name: &str,
        args: &[Type],
    ) -> Result<BTreeMap<String, Ty>, TypeError> {
        let Some(gens) = self.trait_generics.get(trait_name).cloned() else {
            return Ok(BTreeMap::new());
        };
        if gens.is_empty() || args.is_empty() {
            return Ok(BTreeMap::new());
        }
        let mut subst = BTreeMap::new();
        for (g, ast_ty) in gens.iter().zip(args.iter()) {
            subst.insert(g.clone(), self.ast_type(ast_ty)?);
        }
        Ok(subst)
    }

    fn ast_type(&mut self, ty: &Type) -> Result<Ty, TypeError> {
        match &ty.kind {
            TypeKind::Named(id) => {
                if let Some(bound) = self.generic_params.get(&id.name) {
                    return Ok(bound.clone());
                }
                match id.name.as_str() {
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
                }
            }
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

fn arith_trait_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("Add"),
        BinaryOp::Sub => Some("Sub"),
        BinaryOp::Mul => Some("Mul"),
        BinaryOp::Div => Some("Div"),
        _ => None,
    }
}

fn subst_named_params(ty: &Ty, subst: &BTreeMap<String, Ty>) -> Ty {
    match ty {
        Ty::Named { name, args } if args.is_empty() => {
            subst.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        Ty::Named { name, args } => Ty::Named {
            name: name.clone(),
            args: args.iter().map(|a| subst_named_params(a, subst)).collect(),
        },
        Ty::Fn { params, ret } => Ty::Fn {
            params: params
                .iter()
                .map(|p| subst_named_params(p, subst))
                .collect(),
            ret: Box::new(subst_named_params(ret, subst)),
        },
        Ty::Option(inner) => Ty::Option(Box::new(subst_named_params(inner, subst))),
        Ty::Slice(inner) => Ty::Slice(Box::new(subst_named_params(inner, subst))),
        Ty::Array { elem, len } => Ty::Array {
            elem: Box::new(subst_named_params(elem, subst)),
            len: *len,
        },
        Ty::Ref { mutable, inner } => Ty::Ref {
            mutable: *mutable,
            inner: Box::new(subst_named_params(inner, subst)),
        },
        Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| subst_named_params(t, subst)).collect()),
        other => other.clone(),
    }
}

fn ty_is_ground(ty: &Ty) -> bool {
    let mut vars = Vec::new();
    collect_free_vars(ty, &mut vars);
    vars.is_empty()
}

fn collect_generic_subst(scheme: &Ty, inst: &Ty, gens: &[String], out: &mut BTreeMap<String, Ty>) {
    match (scheme, inst) {
        (Ty::Named { name, args }, inst) if args.is_empty() && gens.iter().any(|g| g == name) => {
            out.entry(name.clone()).or_insert_with(|| inst.clone());
        }
        (Ty::Named { name: n1, args: a1 }, Ty::Named { name: n2, args: a2 })
            if n1 == n2 && a1.len() == a2.len() =>
        {
            for (s, i) in a1.iter().zip(a2.iter()) {
                collect_generic_subst(s, i, gens, out);
            }
        }
        (
            Ty::Fn {
                params: p1,
                ret: r1,
            },
            Ty::Fn {
                params: p2,
                ret: r2,
            },
        ) if p1.len() == p2.len() => {
            for (s, i) in p1.iter().zip(p2.iter()) {
                collect_generic_subst(s, i, gens, out);
            }
            collect_generic_subst(r1, r2, gens, out);
        }
        (Ty::Option(a), Ty::Option(b)) | (Ty::Slice(a), Ty::Slice(b)) => {
            collect_generic_subst(a, b, gens, out);
        }
        (Ty::Array { elem: a, .. }, Ty::Array { elem: b, .. }) => {
            collect_generic_subst(a, b, gens, out);
        }
        (Ty::Ref { inner: a, .. }, Ty::Ref { inner: b, .. }) => {
            collect_generic_subst(a, b, gens, out);
        }
        (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => {
            for (s, i) in a.iter().zip(b.iter()) {
                collect_generic_subst(s, i, gens, out);
            }
        }
        _ => {}
    }
}

fn generic_name(i: usize) -> String {
    match i {
        0 => "T".into(),
        1 => "U".into(),
        2 => "V".into(),
        3 => "W".into(),
        n => format!("T{n}"),
    }
}

fn name_free_vars(ty: &Ty) -> (Ty, Vec<String>) {
    let mut vars = Vec::new();
    collect_free_vars(ty, &mut vars);
    vars.sort_unstable();
    vars.dedup();
    name_vars(ty, &vars)
}

fn name_vars(ty: &Ty, vars: &[u32]) -> (Ty, Vec<String>) {
    let mut vars = vars.to_vec();
    vars.sort_unstable();
    vars.dedup();
    if vars.is_empty() {
        return (ty.clone(), Vec::new());
    }
    let names: Vec<String> = vars
        .iter()
        .enumerate()
        .map(|(i, _)| generic_name(i))
        .collect();
    let mut named = ty.clone();
    for (v, name) in vars.iter().zip(&names) {
        named = substitute_var(
            &named,
            *v,
            &Ty::Named {
                name: name.clone(),
                args: vec![],
            },
        );
    }
    (named, names)
}

fn generalize_named_params(ty: &Ty, gens: &[String], ctx: &mut InferContext) -> Scheme {
    if gens.is_empty() {
        return scheme(ty.clone());
    }
    let mut subst = BTreeMap::new();
    let mut vars = Vec::new();
    for g in gens {
        let fresh = ctx.fresh();
        if let Ty::Var(v) = &fresh {
            vars.push(*v);
        }
        subst.insert(g.clone(), fresh);
    }
    Scheme {
        vars,
        ty: subst_named_params(ty, &subst),
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
