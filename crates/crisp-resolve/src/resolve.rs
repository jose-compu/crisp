use crate::error::ResolveError;
use crate::module::{ModuleGraph, load_module_graph};
use crate::prelude::prelude_symbols;
use crate::stdlib::stdlib_symbols;
use crate::symbols::{Symbol, SymbolKey, SymbolKind, Visibility, collect_module_symbols};
use crate::warning::ResolveWarning;
use crisp_ast::Span;
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::ident::Ident;
use crisp_ast::item::{Item, SourceFile, UseDecl};
use crisp_ast::pat::{Pat, PatKind};
use crisp_ast::ty::{Type, TypeBound, TypeKind};
use crisp_manifest::{read_manifest, resolve_dependencies};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    pub local_name: String,
    pub symbol: SymbolKey,
}

/// A Rust-crate import binding (spec §14.2), for later typeck/emit.
///
/// Primary surface: `use serde_json { from_str }`. Compat: `use rust.serde_json { … }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRustImport {
    pub crisp_module: String,
    pub crate_name: String,
    pub item: String,
    pub local_name: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub module_path: String,
    pub file: String,
    pub imports: Vec<ResolvedBinding>,
    pub scope: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedCrate {
    pub crate_root: String,
    pub modules: Vec<ResolvedModule>,
    pub symbol_count: usize,
    pub rust_imports: Vec<ResolvedRustImport>,
    pub warnings: Vec<ResolveWarning>,
}

pub struct Resolver {
    graph: ModuleGraph,
    global: BTreeMap<SymbolKey, Symbol>,
    /// Crate names from `crisp.toml` with `rust = true` (plus auto tokio when applicable).
    rust_deps: HashSet<String>,
    /// Deps present but not marked `rust = true` (for E0045).
    unmarked_deps: HashSet<String>,
    rust_imports: Vec<ResolvedRustImport>,
    warnings: Vec<ResolveWarning>,
}

impl Resolver {
    pub fn resolve_crate(crate_root: &Path) -> Result<ResolvedCrate, ResolveError> {
        let graph = load_module_graph(crate_root)?;
        let (rust_deps, unmarked_deps) = load_dep_sets(crate_root)?;
        let mut resolver = Self::new(graph, rust_deps, unmarked_deps)?;
        resolver.run()
    }

    fn new(
        graph: ModuleGraph,
        rust_deps: HashSet<String>,
        unmarked_deps: HashSet<String>,
    ) -> Result<Self, ResolveError> {
        let mut global = BTreeMap::new();
        for sym in prelude_symbols().into_iter().chain(stdlib_symbols()) {
            global.insert(sym.key.clone(), sym);
        }
        for (module_path, node) in &graph.modules {
            for sym in collect_module_symbols(module_path, &node.ast.items) {
                if let Some(prev) = global.get(&sym.key) {
                    return Err(ResolveError::DuplicateDef {
                        name: sym.key.name.clone(),
                        module: sym.key.module.clone(),
                        span: sym.span.merge(prev.span),
                    });
                }
                global.insert(sym.key.clone(), sym);
            }
        }
        Ok(Self {
            graph,
            global,
            rust_deps,
            unmarked_deps,
            rust_imports: Vec::new(),
            warnings: Vec::new(),
        })
    }

    fn run(&mut self) -> Result<ResolvedCrate, ResolveError> {
        let mut resolved_modules = Vec::new();
        for (module_path, node) in &self.graph.modules.clone() {
            let imports = self.resolve_module_imports(module_path, &node.ast)?;
            let scope: Vec<String> = imports.iter().map(|b| b.local_name.clone()).collect();
            self.check_module_references(module_path, &node.ast, &imports)?;
            resolved_modules.push(ResolvedModule {
                module_path: module_path.clone(),
                file: node.path.display().to_string(),
                imports,
                scope,
            });
        }
        Ok(ResolvedCrate {
            crate_root: self.graph.crate_root.display().to_string(),
            modules: resolved_modules,
            symbol_count: self.global.len(),
            rust_imports: self.rust_imports.clone(),
            warnings: self.warnings.clone(),
        })
    }

    fn resolve_module_imports(
        &mut self,
        current: &str,
        file: &SourceFile,
    ) -> Result<Vec<ResolvedBinding>, ResolveError> {
        let mut scope: HashMap<String, SymbolKey> = HashMap::new();
        let mut bindings = Vec::new();

        for sym in prelude_symbols().into_iter().chain(stdlib_symbols()) {
            scope.insert(sym.key.name.clone(), sym.key.clone());
            bindings.push(ResolvedBinding {
                local_name: sym.key.name.clone(),
                symbol: sym.key.clone(),
            });
        }

        for sym in collect_module_symbols(current, &file.items) {
            scope.insert(sym.key.name.clone(), sym.key.clone());
            if !bindings.iter().any(|b| b.local_name == sym.key.name) {
                bindings.push(ResolvedBinding {
                    local_name: sym.key.name.clone(),
                    symbol: sym.key.clone(),
                });
            }
        }

        for item in &file.items {
            let Item::Use(use_decl) = item else {
                continue;
            };
            self.apply_use(current, use_decl, &mut scope, &mut bindings)?;
        }

        Ok(bindings)
    }

    fn apply_use(
        &mut self,
        current: &str,
        decl: &UseDecl,
        scope: &mut HashMap<String, SymbolKey>,
        bindings: &mut Vec<ResolvedBinding>,
    ) -> Result<(), ResolveError> {
        // Compat alias: `use rust.<crate> { … }` / `use rust::<crate> { … }`.
        if decl.path.first().is_some_and(|p| p.name == "rust") {
            let path_str = decl
                .path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            if decl.path.len() != 2 {
                return Err(ResolveError::RustUsePathInvalid {
                    path: path_str,
                    span: decl.span,
                });
            }
            let crate_name = decl.path[1].name.clone();
            return self.bind_rust_crate(current, &crate_name, decl, scope, bindings);
        }

        // Crisp module wins when present (TS-like bare crate path otherwise).
        if let Some(target_module) = self.lookup_crisp_module(current, &decl.path) {
            let joined = decl
                .path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            if self.rust_deps.contains(&joined) {
                self.warnings.push(ResolveWarning::ModuleShadowsRustDep {
                    name: joined.clone(),
                    span: decl.span,
                });
            }
            return self.bind_crisp_use(target_module, decl, scope, bindings);
        }

        // Bare `use serde_json { … }` when it is (or claims to be) a Cargo dependency.
        if decl.path.len() == 1 {
            let crate_name = decl.path[0].name.clone();
            if self.rust_deps.contains(&crate_name) || self.unmarked_deps.contains(&crate_name) {
                return self.bind_rust_crate(current, &crate_name, decl, scope, bindings);
            }
        }

        Err(ResolveError::ModuleNotFound {
            path: decl
                .path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
        })
    }

    fn bind_crisp_use(
        &self,
        target_module: String,
        decl: &UseDecl,
        scope: &mut HashMap<String, SymbolKey>,
        bindings: &mut Vec<ResolvedBinding>,
    ) -> Result<(), ResolveError> {
        let span = decl.span;
        if let Some(imports) = &decl.imports {
            for imp in imports {
                let sym = self.lookup_export(&target_module, &imp.name.name)?;
                let local = imp
                    .alias
                    .as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| imp.name.name.clone());
                self.insert_binding(&local, sym.key.clone(), scope, bindings, span)?;
            }
        } else {
            let exported: Vec<_> = self
                .global
                .values()
                .filter(|s| s.key.module == target_module && s.is_exported() && !s.from_prelude)
                .cloned()
                .collect();
            for sym in exported {
                self.insert_binding(&sym.key.name, sym.key.clone(), scope, bindings, span)?;
            }
        }
        let _ = decl.is_pub;
        Ok(())
    }

    fn bind_rust_crate(
        &mut self,
        current: &str,
        crate_name: &str,
        decl: &UseDecl,
        scope: &mut HashMap<String, SymbolKey>,
        bindings: &mut Vec<ResolvedBinding>,
    ) -> Result<(), ResolveError> {
        if self.unmarked_deps.contains(crate_name) && !self.rust_deps.contains(crate_name) {
            return Err(ResolveError::RustCrateNotMarked {
                name: crate_name.to_string(),
                span: decl.span,
            });
        }
        if !self.rust_deps.contains(crate_name) {
            return Err(ResolveError::RustCrateNotFound {
                name: crate_name.to_string(),
                span: decl.span,
            });
        }
        let Some(imports) = &decl.imports else {
            return Err(ResolveError::RustImportNeedsList {
                name: crate_name.to_string(),
                span: decl.span,
            });
        };

        let module = format!("rust.{crate_name}");
        for imp in imports {
            let key = SymbolKey {
                module: module.clone(),
                name: imp.name.name.clone(),
            };
            self.global.entry(key.clone()).or_insert_with(|| Symbol {
                key: key.clone(),
                kind: SymbolKind::RustFn,
                visibility: Visibility::Public,
                span: imp.span,
                from_prelude: false,
            });
            let local = imp
                .alias
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| imp.name.name.clone());
            self.insert_binding(&local, key, scope, bindings, decl.span)?;
            self.rust_imports.push(ResolvedRustImport {
                crisp_module: current.to_string(),
                crate_name: crate_name.to_string(),
                item: imp.name.name.clone(),
                local_name: local,
            });
        }
        Ok(())
    }

    /// Crisp module path for a `use`, if one exists (does not consider Rust deps).
    fn lookup_crisp_module(&self, current: &str, path: &[Ident]) -> Option<String> {
        self.resolve_use_path(current, path).ok()
    }

    fn insert_binding(
        &self,
        local: &str,
        key: SymbolKey,
        scope: &mut HashMap<String, SymbolKey>,
        bindings: &mut Vec<ResolvedBinding>,
        span: Span,
    ) -> Result<(), ResolveError> {
        if let Some(prev) = scope.get(local) {
            if prev != &key {
                return Err(ResolveError::AmbiguousImport {
                    name: local.to_string(),
                    span,
                });
            }
            return Ok(());
        }
        scope.insert(local.to_string(), key.clone());
        bindings.push(ResolvedBinding {
            local_name: local.to_string(),
            symbol: key,
        });
        Ok(())
    }

    fn resolve_use_path(&self, current: &str, path: &[Ident]) -> Result<String, ResolveError> {
        if path.is_empty() {
            return Err(ResolveError::ModuleNotFound {
                path: "(empty)".to_string(),
            });
        }
        let joined = path
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        if path.first().map(|p| p.name.as_str()) == Some("std") {
            return Ok(joined);
        }
        if self.graph.modules.contains_key(&joined) {
            return Ok(joined);
        }
        // sibling import: `use config` from `main` -> `config`
        if self.graph.modules.contains_key(&joined) {
            return Ok(joined);
        }
        // relative to current module's directory prefix
        let current_dir = current.rsplit_once('.').map(|(d, _)| d).unwrap_or("");
        let candidate = if current_dir.is_empty() {
            joined.clone()
        } else {
            format!("{current_dir}.{joined}")
        };
        if self.graph.modules.contains_key(&candidate) {
            return Ok(candidate);
        }
        Err(ResolveError::ModuleNotFound { path: joined })
    }

    fn lookup_export(&self, module: &str, name: &str) -> Result<&Symbol, ResolveError> {
        let key = SymbolKey {
            module: module.to_string(),
            name: name.to_string(),
        };
        let sym = self
            .global
            .get(&key)
            .ok_or_else(|| ResolveError::NotExported {
                name: name.to_string(),
                module: module.to_string(),
                span: Span::default(),
            })?;
        if !sym.is_exported() {
            return Err(ResolveError::PrivateImport {
                name: name.to_string(),
                module: module.to_string(),
                span: sym.span,
            });
        }
        Ok(sym)
    }

    fn check_module_references(
        &self,
        current: &str,
        file: &SourceFile,
        imports: &[ResolvedBinding],
    ) -> Result<(), ResolveError> {
        let scope: HashMap<String, SymbolKey> = imports
            .iter()
            .map(|b| (b.local_name.clone(), b.symbol.clone()))
            .collect();
        for item in &file.items {
            match item {
                Item::Function(f) => {
                    let mut local = scope.clone();
                    for p in &f.params {
                        local.insert(
                            p.name.name.clone(),
                            SymbolKey {
                                module: "_param".to_string(),
                                name: p.name.name.clone(),
                            },
                        );
                        if let Some(ty) = &p.ty {
                            self.check_type(&local, ty)?;
                        }
                    }
                    if let Some(ty) = &f.ret_type {
                        self.check_type(&local, ty)?;
                    }
                    self.check_expr(&local, &f.body)?;
                }
                Item::TypeDef(t) => {
                    self.check_type_def(&scope, t)?;
                }
                Item::Const(c) => self.check_expr(&scope, &c.value)?,
                Item::Test(t) => self.check_block(&scope, &t.body)?,
                Item::TestCompileFail(_) => {}
                Item::Impl(i) => {
                    if let Some(tn) = &i.trait_name {
                        self.check_name(&scope, &tn.name, tn.span)?;
                    }
                    self.check_type(&scope, &i.ty)?;
                    for f in &i.items {
                        let mut local = scope.clone();
                        for p in &f.params {
                            local.insert(
                                p.name.name.clone(),
                                SymbolKey {
                                    module: "_param".to_string(),
                                    name: p.name.name.clone(),
                                },
                            );
                            if let Some(ty) = &p.ty {
                                self.check_type(&local, ty)?;
                            }
                        }
                        if let Some(ty) = &f.ret_type {
                            self.check_type(&local, ty)?;
                        }
                        self.check_expr(&local, &f.body)?;
                    }
                }
                Item::TraitDef(t) => {
                    for item in &t.items {
                        let mut local = scope.clone();
                        for p in &item.params {
                            local.insert(
                                p.name.name.clone(),
                                SymbolKey {
                                    module: "_param".to_string(),
                                    name: p.name.name.clone(),
                                },
                            );
                            if let Some(ty) = &p.ty {
                                self.check_type(&local, ty)?;
                            }
                        }
                        if let Some(ty) = &item.ret_type {
                            self.check_type(&local, ty)?;
                        }
                        if let Some(body) = &item.default_body {
                            self.check_expr(&local, body)?;
                        }
                    }
                }
                Item::ShapeDef(s) => {
                    for f in &s.fields {
                        match f {
                            crisp_ast::item::ShapeField::Data { ty, .. } => {
                                self.check_type(&scope, ty)?;
                            }
                            crisp_ast::item::ShapeField::Method {
                                params, ret_type, ..
                            } => {
                                for p in params {
                                    if let Some(ty) = &p.ty {
                                        self.check_type(&scope, ty)?;
                                    }
                                }
                                self.check_type(&scope, ret_type)?;
                            }
                        }
                    }
                }
                Item::Use(_) | Item::Extern(_) => {}
            }
        }
        let _ = current;
        Ok(())
    }

    fn check_type_def(
        &self,
        scope: &HashMap<String, SymbolKey>,
        t: &crisp_ast::item::TypeDef,
    ) -> Result<(), ResolveError> {
        use crisp_ast::item::TypeBody;
        match &t.body {
            TypeBody::Struct(fields) => {
                for f in fields {
                    self.check_type(scope, &f.ty)?;
                    if let Some(def) = &f.default {
                        self.check_expr(scope, def)?;
                    }
                }
            }
            TypeBody::Enum(variants) => {
                for v in variants {
                    for ty in &v.fields {
                        self.check_type(scope, ty)?;
                    }
                }
            }
            TypeBody::Alias(ty) => self.check_type(scope, ty)?,
        }
        Ok(())
    }

    fn check_type(
        &self,
        scope: &HashMap<String, SymbolKey>,
        ty: &Type,
    ) -> Result<(), ResolveError> {
        match &ty.kind {
            TypeKind::Named(id) => {
                self.check_name(scope, &id.name, id.span)?;
                self.reject_shape_type(scope, &id.name, id.span)
            }
            TypeKind::Option(inner) | TypeKind::Slice(inner) | TypeKind::Ref { inner, .. } => {
                self.check_type(scope, inner)
            }
            TypeKind::Tuple(types) => {
                for t in types {
                    self.check_type(scope, t)?;
                }
                Ok(())
            }
            TypeKind::Array { elem, .. } => self.check_type(scope, elem),
            TypeKind::Fn { params, ret } => {
                for p in params {
                    self.check_type(scope, p)?;
                }
                self.check_type(scope, ret)
            }
            TypeKind::Constrained { inner, bounds } => {
                for b in bounds {
                    match b {
                        TypeBound::Shape(id) => {
                            self.check_name(scope, &id.name, id.span)?;
                            // Ensure the name is a shape (not a random type).
                            if let Some(key) = scope.get(&id.name)
                                && let Some(sym) = self.global.get(key)
                                && sym.kind != SymbolKind::Shape
                            {
                                return Err(ResolveError::UnresolvedName {
                                    name: id.name.clone(),
                                    span: id.span,
                                    message: format!(
                                        "[E0035] `{name}` is not a shape",
                                        name = id.name
                                    ),
                                    hint: Some("shape bounds require a `shape` definition".into()),
                                });
                            }
                        }
                        TypeBound::Trait(id) => {
                            self.check_name(scope, &id.name, id.span)?;
                        }
                    }
                }
                self.check_type(scope, inner)
            }
            TypeKind::Never | TypeKind::Unit => Ok(()),
            TypeKind::Generic { base, args } => {
                self.check_type(scope, base)?;
                for a in args {
                    self.check_type(scope, a)?;
                }
                Ok(())
            }
        }
    }

    fn check_block(
        &self,
        scope: &HashMap<String, SymbolKey>,
        block: &Block,
    ) -> Result<(), ResolveError> {
        let mut local = scope.clone();
        for stmt in &block.stmts {
            match stmt {
                Stmt::Bind { pat, value, .. } => {
                    self.check_expr(&local, value)?;
                    self.bind_pat(&mut local, pat)?;
                }
                Stmt::Assign { target, value } => {
                    self.check_name(&local, &target.name, target.span)?;
                    self.check_expr(&local, value)?;
                }
                Stmt::Expr(e) => self.check_expr(&local, e)?,
            }
        }
        if let Some(tail) = &block.tail {
            self.check_expr(&local, tail)?;
        }
        Ok(())
    }

    fn bind_pat(
        &self,
        scope: &mut HashMap<String, SymbolKey>,
        pat: &Pat,
    ) -> Result<(), ResolveError> {
        match &pat.kind {
            PatKind::Ident(id) => {
                scope.insert(
                    id.name.clone(),
                    SymbolKey {
                        module: "_local".to_string(),
                        name: id.name.clone(),
                    },
                );
            }
            PatKind::Wildcard => {}
            PatKind::Tuple(pats) => {
                for p in pats {
                    self.bind_pat(scope, p)?;
                }
            }
            PatKind::Slice { prefix, rest } => {
                for p in prefix {
                    self.bind_pat(scope, p)?;
                }
                if let Some(id) = rest {
                    scope.insert(
                        id.name.clone(),
                        SymbolKey {
                            module: "_local".to_string(),
                            name: id.name.clone(),
                        },
                    );
                }
            }
            PatKind::Struct { fields, .. } => {
                for f in fields {
                    if let Some(p) = &f.pat {
                        self.bind_pat(scope, p)?;
                    }
                }
            }
            PatKind::Enum { args, .. } => {
                for p in args {
                    self.bind_pat(scope, p)?;
                }
            }
            PatKind::Literal(_) => {}
            PatKind::Type { inner, .. } => self.bind_pat(scope, inner)?,
        }
        Ok(())
    }

    fn check_expr(
        &self,
        scope: &HashMap<String, SymbolKey>,
        expr: &Expr,
    ) -> Result<(), ResolveError> {
        match &expr.kind {
            ExprKind::Ident(id) => self.check_name(scope, &id.name, id.span),
            ExprKind::Block(b) => self.check_block(scope, b),
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.check_expr(scope, cond)?;
                self.check_expr(scope, then_branch)?;
                if let Some(e) = else_branch {
                    self.check_expr(scope, e)?;
                }
                Ok(())
            }
            ExprKind::Match { scrutinee, arms } => {
                self.check_expr(scope, scrutinee)?;
                let mut local = scope.clone();
                for arm in arms {
                    self.bind_pat(&mut local, &arm.pat)?;
                    if let Some(g) = &arm.guard {
                        self.check_expr(&local, g)?;
                    }
                    self.check_expr(&local, &arm.body)?;
                }
                Ok(())
            }
            ExprKind::For { pat, iter, body } => {
                self.check_expr(scope, iter)?;
                let mut local = scope.clone();
                self.bind_pat(&mut local, pat)?;
                self.check_expr(&local, body)
            }
            ExprKind::While { cond, body } => {
                self.check_expr(scope, cond)?;
                self.check_expr(scope, body)
            }
            ExprKind::Loop(body)
            | ExprKind::Async(body)
            | ExprKind::Await(body)
            | ExprKind::Spawn(body)
            | ExprKind::Unsafe(body)
            | ExprKind::Try(body) => self.check_expr(scope, body),
            ExprKind::Break(Some(v)) => self.check_expr(scope, v),
            ExprKind::Lambda { params, body } => {
                let mut local = scope.clone();
                for p in params {
                    local.insert(
                        p.name.name.clone(),
                        SymbolKey {
                            module: "_local".to_string(),
                            name: p.name.name.clone(),
                        },
                    );
                    if let Some(ty) = &p.ty {
                        self.check_type(&local, ty)?;
                    }
                }
                self.check_expr(&local, body)
            }
            ExprKind::Call { func, args } => {
                self.check_expr(scope, func)?;
                for a in args {
                    self.check_expr(scope, a)?;
                }
                Ok(())
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.check_expr(scope, receiver)?;
                for a in args {
                    self.check_expr(scope, a)?;
                }
                Ok(())
            }
            ExprKind::Field { base, .. } => self.check_expr(scope, base),
            ExprKind::Index { base, index } => {
                self.check_expr(scope, base)?;
                self.check_expr(scope, index)
            }
            ExprKind::Unary { expr, .. } | ExprKind::Throw(expr) | ExprKind::Return(Some(expr)) => {
                self.check_expr(scope, expr)
            }
            ExprKind::Binary { left, right, .. } | ExprKind::Pipe { left, right, .. } => {
                self.check_expr(scope, left)?;
                self.check_expr(scope, right)
            }
            ExprKind::Assign { target, value } => {
                self.check_name(scope, &target.name, target.span)?;
                self.check_expr(scope, value)
            }
            ExprKind::Bind { pat, value, .. } => {
                self.check_expr(scope, value)?;
                let mut local = scope.clone();
                self.bind_pat(&mut local, pat)
            }
            ExprKind::StructLit { name, fields } => {
                self.check_name(scope, &name.name, name.span)?;
                for f in fields {
                    self.check_expr(scope, &f.value)?;
                }
                Ok(())
            }
            ExprKind::Str(parts) => {
                for part in &parts.0 {
                    if let crisp_ast::expr::StringPart::Expr(e) = part {
                        self.check_expr(scope, e)?;
                    }
                }
                Ok(())
            }
            ExprKind::Catch { body, arms } => {
                self.check_expr(scope, body)?;
                let mut local = scope.clone();
                for arm in arms {
                    self.bind_pat(&mut local, &arm.pat)?;
                    self.check_expr(&local, &arm.body)?;
                }
                Ok(())
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Unit
            | ExprKind::Break(None)
            | ExprKind::Continue
            | ExprKind::Return(None) => Ok(()),
        }
    }

    fn check_name(
        &self,
        scope: &HashMap<String, SymbolKey>,
        name: &str,
        span: Span,
    ) -> Result<(), ResolveError> {
        if scope.contains_key(name) {
            return Ok(());
        }
        if name == "Self" || name.starts_with('_') {
            return Ok(());
        }
        let hint = self.unresolved_hint(scope, name);
        let message = match &hint {
            Some(h) => format!("[E0035] unresolved name `{name}`\nhelp: {h}"),
            None => format!("[E0035] unresolved name `{name}`"),
        };
        Err(ResolveError::UnresolvedName {
            name: name.to_string(),
            span,
            message,
            hint,
        })
    }

    fn unresolved_hint(&self, scope: &HashMap<String, SymbolKey>, name: &str) -> Option<String> {
        let _ = scope;
        let mut modules: Vec<&str> = self
            .global
            .values()
            .filter(|s| s.key.name == name && !s.from_prelude)
            .map(|s| s.key.module.as_str())
            .collect();
        modules.sort_unstable();
        modules.dedup();
        if modules.is_empty() {
            return None;
        }
        let module = modules[0];
        Some(format!(
            "`{name}` is defined in module `{module}`; add `use {module} {{ {name} }}` \
(sibling modules are not visible by filename order alone)"
        ))
    }

    fn reject_shape_type(
        &self,
        _scope: &HashMap<String, SymbolKey>,
        _name: &str,
        _span: Span,
    ) -> Result<(), ResolveError> {
        // Named shapes are supported as types (v1.5 / #61).
        Ok(())
    }
}

fn load_dep_sets(crate_root: &Path) -> Result<(HashSet<String>, HashSet<String>), ResolveError> {
    let manifest = read_manifest(crate_root).map_err(|e| ResolveError::Manifest {
        root: crate_root.display().to_string(),
        message: e.to_string(),
    })?;
    let deps = resolve_dependencies(&manifest);
    let mut rust_deps = HashSet::new();
    let mut unmarked_deps = HashSet::new();
    for dep in deps {
        if dep.rust {
            rust_deps.insert(dep.name);
        } else {
            unmarked_deps.insert(dep.name);
        }
    }
    for (name, spec) in &manifest.dependencies {
        use crisp_manifest::DependencySpec;
        match spec {
            DependencySpec::Version(_) => {
                unmarked_deps.insert(name.clone());
            }
            DependencySpec::Detailed { rust, .. } if !*rust => {
                unmarked_deps.insert(name.clone());
            }
            _ => {}
        }
    }
    Ok((rust_deps, unmarked_deps))
}
