use crate::error::ResolveError;
use crate::module::{ModuleGraph, load_module_graph};
use crate::prelude::prelude_symbols;
use crate::symbols::{Symbol, SymbolKey, collect_module_symbols};
use crisp_ast::Span;
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::ident::Ident;
use crisp_ast::item::{Item, SourceFile, UseDecl};
use crisp_ast::pat::{Pat, PatKind};
use crisp_ast::ty::{Type, TypeBound, TypeKind};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    pub local_name: String,
    pub symbol: SymbolKey,
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
}

pub struct Resolver {
    graph: ModuleGraph,
    global: BTreeMap<SymbolKey, Symbol>,
}

impl Resolver {
    pub fn resolve_crate(crate_root: &Path) -> Result<ResolvedCrate, ResolveError> {
        let graph = load_module_graph(crate_root)?;
        let mut resolver = Self::new(graph)?;
        resolver.run()
    }

    fn new(graph: ModuleGraph) -> Result<Self, ResolveError> {
        let mut global = BTreeMap::new();
        for sym in prelude_symbols() {
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
        Ok(Self { graph, global })
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
        })
    }

    fn resolve_module_imports(
        &self,
        current: &str,
        file: &SourceFile,
    ) -> Result<Vec<ResolvedBinding>, ResolveError> {
        let mut scope: HashMap<String, SymbolKey> = HashMap::new();
        let mut bindings = Vec::new();

        for sym in prelude_symbols() {
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
        &self,
        current: &str,
        decl: &UseDecl,
        scope: &mut HashMap<String, SymbolKey>,
        bindings: &mut Vec<ResolvedBinding>,
    ) -> Result<(), ResolveError> {
        let target_module = self.resolve_use_path(current, &decl.path)?;
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
        let _ = (current, decl.is_pub);
        Ok(())
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
                Item::TestCompileFail(t) => self.check_block(&scope, &t.body)?,
                Item::Impl(i) => {
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
                Item::Use(_) | Item::TraitDef(_) | Item::ShapeDef(_) | Item::Extern(_) => {}
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
            TypeKind::Named(id) => self.check_name(scope, &id.name, id.span),
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
                        TypeBound::Shape(id) | TypeBound::Trait(id) => {
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
            | ExprKind::Break
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
        Err(ResolveError::UnresolvedName {
            name: name.to_string(),
            span,
        })
    }
}
