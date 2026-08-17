use crate::error::OwnershipError;
use crate::lattice::OwnershipMode;
use crate::result::{AutoClone, BindingUsages, OwnershipResult, OwnershipSignature};
use crate::usage::Usage;
use crisp_ast::expr::{BinaryOp, Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{FunctionDef, Item, SourceFile};
use crisp_ast::pat::PatKind;
use crisp_resolve::module::load_module_graph;
use crisp_typeck::{InferredSig, Ty, TypeChecker};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

const MUTATING_METHODS: &[&str] = &["push", "pop", "insert", "remove", "clear", "sort"];

pub struct OwnershipPass;

impl OwnershipPass {
    pub fn analyze_crate(crate_root: &Path) -> Result<OwnershipResult, OwnershipError> {
        let graph = load_module_graph(crate_root)?;
        let typed = TypeChecker::check_crate(crate_root)?;

        let mut fn_defs: BTreeMap<String, (String, FunctionDef)> = BTreeMap::new();
        for node in graph.modules.values() {
            collect_functions(&node.module_path, &node.ast, &mut fn_defs);
        }

        let mut param_modes: BTreeMap<String, Vec<OwnershipMode>> = BTreeMap::new();
        for (key, (_, def)) in &fn_defs {
            param_modes.insert(key.clone(), vec![OwnershipMode::Borrow; def.params.len()]);
        }

        let max_iters = fn_defs.len().max(1) * 4 + 8;
        for _ in 0..max_iters {
            let mut changed = false;
            for (key, (module, def)) in &fn_defs {
                let callee_modes = &param_modes;
                let (new_modes, auto_clones, errors) =
                    analyze_function(key, module, def, callee_modes, &fn_defs, &typed)?;
                if let Some(err) = errors {
                    return Err(err);
                }
                let prev = param_modes.get(key).cloned().unwrap_or_default();
                if prev != new_modes {
                    changed = true;
                    param_modes.insert(key.clone(), new_modes);
                }
                // store auto_clones temporarily - recompute on final pass
                let _ = auto_clones;
            }
            if !changed {
                break;
            }
        }

        let mut signatures = BTreeMap::new();
        for (key, (module, def)) in &fn_defs {
            let modes = param_modes.get(key).cloned().unwrap_or_default();
            let (_, auto_clones, errors) =
                analyze_function(key, module, def, &param_modes, &fn_defs, &typed)?;
            if let Some(err) = errors {
                return Err(err);
            }
            let params: Vec<_> = def
                .params
                .iter()
                .zip(modes.iter())
                .map(|(p, m)| (p.name.name.clone(), *m))
                .collect();
            signatures.insert(
                key.clone(),
                OwnershipSignature {
                    module: module.clone(),
                    name: def.name.name.clone(),
                    params,
                    ret_mode: return_mode(def, typed.signatures.get(key)),
                    auto_clones,
                    applied_fallbacks: vec![],
                    span: def.span,
                },
            );
        }

        Ok(OwnershipResult { signatures })
    }
}

fn collect_functions(
    module: &str,
    file: &SourceFile,
    out: &mut BTreeMap<String, (String, FunctionDef)>,
) {
    for item in &file.items {
        match item {
            Item::Function(f) => {
                let key = format!("{module}::{}", f.name.name);
                out.insert(key, (module.to_string(), f.clone()));
            }
            Item::Impl(ib) => {
                let ty_name = match &ib.ty.kind {
                    crisp_ast::ty::TypeKind::Named(id) => id.name.clone(),
                    _ => continue,
                };
                for f in &ib.items {
                    let key = format!("{module}::{ty_name}::{}", f.name.name);
                    out.insert(key, (module.to_string(), f.clone()));
                }
            }
            _ => {}
        }
    }
}

fn fn_key(module: &str, name: &str) -> String {
    format!("{module}::{name}")
}

fn resolve_callee_key(
    module: &str,
    func: &Expr,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
) -> Option<String> {
    match &func.kind {
        ExprKind::Ident(id) => {
            let local = fn_key(module, &id.name);
            if fn_defs.contains_key(&local) {
                return Some(local);
            }
            for (key, (m, def)) in fn_defs {
                if def.name.name == id.name {
                    return Some(key.clone());
                }
                if m != module && def.is_pub && def.name.name == id.name {
                    return Some(key.clone());
                }
            }
            None
        }
        // Associated / instance methods parse as Field under Call.
        ExprKind::Field { base, field } => {
            if let ExprKind::Ident(id) = &base.kind {
                let local = format!("{module}::{}::{}", id.name, field.name);
                if fn_defs.contains_key(&local) {
                    return Some(local);
                }
                let suffix = format!("::{}::{}", id.name, field.name);
                for key in fn_defs.keys() {
                    if key.ends_with(&suffix) {
                        return Some(key.clone());
                    }
                }
            }
            // Instance: `recv.method` — match any inherent method with this name.
            let suffix = format!("::{}", field.name);
            let mut hits: Vec<&String> = fn_defs
                .keys()
                .filter(|k| {
                    k.ends_with(&suffix)
                        && k.matches("::").count() >= 2
                        && fn_defs
                            .get(*k)
                            .map(|(_, d)| {
                                d.params
                                    .first()
                                    .map(|p| p.name.name == "self")
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false)
                })
                .collect();
            hits.sort();
            if hits.len() == 1 {
                return Some(hits[0].clone());
            }
            None
        }
        _ => None,
    }
}

fn is_copy_ty(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::Int | Ty::UInt | Ty::Float | Ty::Bool | Ty::Char | Ty::Unit | Ty::Never
    )
}

fn is_clone_ty(ty: &Ty, gens: &[String]) -> bool {
    is_copy_ty(ty)
        || ty.is_stringish()
        || matches!(ty, Ty::Var(_))
        || matches!(ty, Ty::Named { name, args } if args.is_empty() && gens.iter().any(|g| g == name))
        || matches!(ty, Ty::Named { name, args } if args.is_empty() && is_user_record(name))
}

fn is_user_record(name: &str) -> bool {
    !matches!(
        name,
        "vec" | "map" | "set" | "Future" | "JoinHandle" | "Option" | "Result"
    )
}

fn type_for_binding(
    name: &str,
    fn_key: &str,
    def: &FunctionDef,
    typed: &crisp_typeck::TypedCrate,
) -> Option<Ty> {
    if let Some(sig) = typed.signatures.get(fn_key) {
        for (pname, ty) in &sig.params {
            if pname == name {
                return Some(ty.clone());
            }
        }
    }
    for p in &def.params {
        if p.name.name == name
            && let Some(ref ty) = p.ty
        {
            return Some(ast_type_to_ty(ty));
        }
    }
    None
}

fn ast_type_to_ty(ty: &crisp_ast::ty::Type) -> Ty {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => match id.name.as_str() {
            "int" => Ty::Int,
            "uint" => Ty::UInt,
            "float" => Ty::Float,
            "bool" => Ty::Bool,
            "char" => Ty::Char,
            "str" => Ty::Str,
            _ => Ty::Named {
                name: id.name.clone(),
                args: vec![],
            },
        },
        TypeKind::Unit => Ty::Unit,
        TypeKind::Never => Ty::Never,
        _ => Ty::Error,
    }
}

fn return_mode(def: &FunctionDef, sig: Option<&InferredSig>) -> Option<OwnershipMode> {
    if let Some(s) = sig {
        if matches!(s.ret, Ty::Str) {
            return Some(OwnershipMode::Owned);
        }
        if matches!(s.ret, Ty::Ref { .. }) {
            return Some(OwnershipMode::Borrow);
        }
    }
    if let Some(ref ty) = def.ret_type {
        let t = ast_type_to_ty(ty);
        if matches!(t, Ty::Str) {
            return Some(OwnershipMode::Owned);
        }
    }
    None
}

struct Collector<'a> {
    module: &'a str,
    fn_key: &'a str,
    def: &'a FunctionDef,
    callee_modes: &'a BTreeMap<String, Vec<OwnershipMode>>,
    fn_defs: &'a BTreeMap<String, (String, FunctionDef)>,
    typed: &'a crisp_typeck::TypedCrate,
    usages: BindingUsages,
    use_order: BTreeMap<String, Vec<(Usage, crisp_ast::Span)>>,
    locals: HashSet<String>,
}

impl<'a> Collector<'a> {
    fn new(
        module: &'a str,
        fn_key: &'a str,
        def: &'a FunctionDef,
        callee_modes: &'a BTreeMap<String, Vec<OwnershipMode>>,
        fn_defs: &'a BTreeMap<String, (String, FunctionDef)>,
        typed: &'a crisp_typeck::TypedCrate,
    ) -> Self {
        let mut locals = HashSet::new();
        for p in &def.params {
            locals.insert(p.name.name.clone());
        }
        Self {
            module,
            fn_key,
            def,
            callee_modes,
            fn_defs,
            typed,
            usages: BindingUsages::default(),
            use_order: BTreeMap::new(),
            locals,
        }
    }

    fn record_use(&mut self, name: &str, usage: Usage, span: crisp_ast::Span) {
        let usage = if matches!(usage, Usage::Read) {
            if let Some(ty) = type_for_binding(name, self.fn_key, self.def, self.typed) {
                if is_copy_ty(&ty) { Usage::Copy } else { usage }
            } else {
                usage
            }
        } else {
            usage
        };
        self.usages.add(name, usage);
        self.use_order
            .entry(name.to_string())
            .or_default()
            .push((usage, span));
    }

    fn usage_for_mode(mode: OwnershipMode) -> Usage {
        match mode {
            OwnershipMode::Borrow => Usage::Read,
            OwnershipMode::MutBorrow => Usage::Mutate,
            OwnershipMode::Owned => Usage::MoveOut,
        }
    }

    fn apply_mode_to_expr(&mut self, expr: &Expr, mode: OwnershipMode) {
        match &expr.kind {
            ExprKind::Ident(id) if self.locals.contains(&id.name) => {
                self.record_use(&id.name, Self::usage_for_mode(mode), expr.span);
            }
            _ => self.walk_expr(expr),
        }
    }

    fn walk_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.walk_stmt(stmt);
        }
        if let Some(tail) = &block.tail {
            self.walk_expr(tail);
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(e) => self.walk_expr(e),
            Stmt::Bind { pat, value, .. } => {
                self.walk_expr(value);
                if let PatKind::Ident(name) = &pat.kind {
                    self.locals.insert(name.name.clone());
                    if let ExprKind::Ident(src) = &value.kind
                        && self.locals.contains(&src.name)
                    {
                        let usage =
                            if type_for_binding(&src.name, self.fn_key, self.def, self.typed)
                                .map(|t| is_copy_ty(&t))
                                .unwrap_or(false)
                            {
                                Usage::Copy
                            } else {
                                Usage::MoveOut
                            };
                        self.record_use(&src.name, usage, value.span);
                    }
                }
            }
            Stmt::Assign { target, value } => {
                self.record_use(&target.name, Usage::Mutate, target.span);
                self.walk_expr(value);
            }
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Ident(id) if self.locals.contains(&id.name) => {
                self.record_use(&id.name, Usage::Read, expr.span);
            }
            ExprKind::Block(b) => self.walk_block(b),
            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(cond);
                self.walk_expr(then_branch);
                if let Some(e) = else_branch {
                    self.walk_expr(e);
                }
            }
            ExprKind::Binary { op, left, right } => {
                self.walk_expr(left);
                self.walk_expr(right);
                if matches!(op, BinaryOp::Concat) {
                    // both sides read
                }
            }
            ExprKind::Unary { expr: inner, .. } => self.walk_expr(inner),
            ExprKind::Cast { expr: inner, .. } => self.walk_expr(inner),
            ExprKind::Call { func, args } => {
                if let Some(callee) = resolve_callee_key(self.module, func, self.fn_defs) {
                    let modes = self.callee_modes.get(&callee).cloned().unwrap_or_default();
                    let has_self = self
                        .fn_defs
                        .get(&callee)
                        .map(|(_, d)| {
                            d.params
                                .first()
                                .map(|p| p.name.name == "self")
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);
                    if has_self {
                        // `recv.method(args)` — modes[0] applies to the Field base.
                        if let ExprKind::Field { base, .. } = &func.kind {
                            let mode = modes.first().copied().unwrap_or(OwnershipMode::Borrow);
                            self.apply_mode_to_expr(base, mode);
                        }
                        for (i, arg) in args.iter().enumerate() {
                            let mode = modes.get(i + 1).copied().unwrap_or(OwnershipMode::Borrow);
                            self.apply_mode_to_expr(arg, mode);
                        }
                    } else {
                        for (i, arg) in args.iter().enumerate() {
                            let mode = modes.get(i).copied().unwrap_or(OwnershipMode::Borrow);
                            self.apply_mode_to_expr(arg, mode);
                        }
                    }
                } else {
                    self.walk_expr(func);
                    let callee_is_value = matches!(
                        &func.kind,
                        ExprKind::Ident(id) if self.locals.contains(&id.name)
                    ) || matches!(&func.kind, ExprKind::Lambda { .. });
                    for arg in args {
                        if callee_is_value {
                            self.apply_mode_to_expr(arg, OwnershipMode::Owned);
                        } else {
                            self.walk_expr(arg);
                        }
                    }
                }
            }
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                if MUTATING_METHODS.contains(&method.name.as_str()) {
                    self.apply_mode_to_expr(receiver, OwnershipMode::MutBorrow);
                } else {
                    self.walk_expr(receiver);
                }
                for arg in args {
                    self.walk_expr(arg);
                }
            }
            ExprKind::Field { base, .. } => self.walk_expr(base),
            ExprKind::Index { base, index } => {
                self.walk_expr(base);
                self.walk_expr(index);
            }
            ExprKind::Assign { target, value } => {
                self.record_use(&target.name, Usage::Mutate, target.span);
                self.walk_expr(value);
            }
            ExprKind::Bind { pat, value, .. } => {
                self.walk_expr(value);
                if let PatKind::Ident(name) = &pat.kind {
                    self.locals.insert(name.name.clone());
                    if let ExprKind::Ident(src) = &value.kind
                        && self.locals.contains(&src.name)
                    {
                        let usage =
                            if type_for_binding(&src.name, self.fn_key, self.def, self.typed)
                                .map(|t| is_copy_ty(&t))
                                .unwrap_or(false)
                            {
                                Usage::Copy
                            } else {
                                Usage::MoveOut
                            };
                        self.record_use(&src.name, usage, value.span);
                    }
                }
            }
            ExprKind::Return(Some(v)) => {
                if let ExprKind::Ident(id) = &v.kind
                    && self.locals.contains(&id.name)
                {
                    let usage = if type_for_binding(&id.name, self.fn_key, self.def, self.typed)
                        .map(|t| is_copy_ty(&t))
                        .unwrap_or(false)
                    {
                        Usage::Copy
                    } else {
                        Usage::MoveOut
                    };
                    self.record_use(&id.name, usage, v.span);
                }
                self.walk_expr(v);
            }
            ExprKind::Return(None) => {}
            ExprKind::StructLit { fields, .. } => {
                for f in fields {
                    self.walk_expr(&f.value);
                }
            }
            ExprKind::Pipe { left, right } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            ExprKind::Str(parts) => {
                for part in &parts.0 {
                    if let crisp_ast::expr::StringPart::Expr(e) = part {
                        self.walk_expr(e);
                    }
                }
            }
            ExprKind::While { cond, body } => {
                self.walk_expr(cond);
                self.walk_expr(body);
            }
            ExprKind::For { iter, body, .. } => {
                self.walk_expr(iter);
                self.walk_expr(body);
            }
            ExprKind::Loop(body) => self.walk_expr(body),
            ExprKind::Break(Some(v)) => self.walk_expr(v),
            ExprKind::Break(None) | ExprKind::Continue => {}
            ExprKind::Lambda { params, body } => {
                let saved = self.locals.clone();
                for p in params {
                    self.locals.insert(p.name.name.clone());
                }
                self.walk_expr(body);
                self.locals = saved;
            }
            _ => {}
        }
    }

    fn record_implicit_return(&mut self, body: &Expr) {
        let tail = match &body.kind {
            ExprKind::Block(b) => b.tail.as_deref(),
            other => {
                if matches!(other, ExprKind::Return(_)) {
                    return;
                }
                Some(body)
            }
        };
        let Some(expr) = tail else { return };
        if let ExprKind::Ident(id) = &expr.kind
            && self.locals.contains(&id.name)
        {
            let usage = if type_for_binding(&id.name, self.fn_key, self.def, self.typed)
                .map(|t| is_copy_ty(&t))
                .unwrap_or(false)
            {
                Usage::Copy
            } else {
                Usage::MoveOut
            };
            self.record_use(&id.name, usage, expr.span);
        }
    }

    fn detect_auto_clones(&self) -> Vec<AutoClone> {
        let mut out = Vec::new();
        for (name, events) in &self.use_order {
            let mut saw_move = false;
            for (usage, span) in events {
                if matches!(usage, Usage::MoveOut) {
                    saw_move = true;
                } else if saw_move
                    && matches!(usage, Usage::Read | Usage::Mutate)
                    && let Some(ty) = type_for_binding(name, self.fn_key, self.def, self.typed)
                    && is_clone_ty(
                        &ty,
                        self.typed
                            .signatures
                            .get(self.fn_key)
                            .map(|s| s.generics.as_slice())
                            .unwrap_or(&[]),
                    )
                    && !is_copy_ty(&ty)
                {
                    out.push(AutoClone {
                        binding: name.clone(),
                        span: *span,
                        note: format!("[auto-clone @ offset {}] {name}", span.start),
                    });
                }
            }
        }
        out
    }
}

#[allow(clippy::type_complexity)]
fn analyze_function(
    key: &str,
    module: &str,
    def: &FunctionDef,
    callee_modes: &BTreeMap<String, Vec<OwnershipMode>>,
    fn_defs: &BTreeMap<String, (String, FunctionDef)>,
    typed: &crisp_typeck::TypedCrate,
) -> Result<(Vec<OwnershipMode>, Vec<AutoClone>, Option<OwnershipError>), OwnershipError> {
    let mut collector = Collector::new(module, key, def, callee_modes, fn_defs, typed);
    collector.walk_expr(&def.body);
    collector.record_implicit_return(&def.body);
    let auto_clones = collector.detect_auto_clones();
    let typed_sig = typed.signatures.get(key);

    let mut modes = Vec::new();
    for p in &def.params {
        let mut inferred = collector.usages.mode_for(&p.name.name);
        // Copy scalars stay by-value (Owned); avoids `&f64` in struct fields / associated fns.
        if p.name.name != "self"
            && let Some(sig) = typed_sig
            && let Some((_, ty)) = sig.params.iter().find(|(n, _)| n == &p.name.name)
            && is_copy_ty(ty)
        {
            inferred = OwnershipMode::Owned;
        }
        if let Some(ann) = OwnershipMode::from_explicit(p.ownership) {
            if inferred > ann {
                return Ok((
                    vec![],
                    vec![],
                    Some(OwnershipError::ContradictsAnnotation {
                        name: p.name.name.clone(),
                        inferred: inferred.display().to_string(),
                        annotated: ann.display().to_string(),
                        span: p.span,
                    }),
                ));
            }
            modes.push(ann);
        } else {
            modes.push(inferred);
        }
    }

    Ok((modes, auto_clones, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn examples(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../examples/{name}"))
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"))
    }

    #[test]
    fn infer_hello_ownership() {
        let result = OwnershipPass::analyze_crate(&examples("hello")).expect("ownership hello");
        let greet = result.get("main", "greet").expect("greet sig");
        assert_eq!(greet.params[0].1, OwnershipMode::Borrow);
    }

    #[test]
    fn infer_move_out_return() {
        let result = OwnershipPass::analyze_crate(&fixture("consume")).expect("ownership consume");
        let id = result.get("main", "identity").expect("identity");
        assert_eq!(id.params[0].1, OwnershipMode::Owned);
    }

    #[test]
    fn infer_mutate_param() {
        let result = OwnershipPass::analyze_crate(&fixture("mutate")).expect("ownership mutate");
        let set = result.get("main", "set_value").expect("set_value");
        assert_eq!(set.params[0].1, OwnershipMode::MutBorrow);
    }

    #[test]
    fn explicit_borrow_ok() {
        let result = OwnershipPass::analyze_crate(&fixture("explicit_ok")).expect("explicit ok");
        let f = result.get("main", "read_only").expect("read_only");
        assert_eq!(f.params[0].1, OwnershipMode::Borrow);
    }

    #[test]
    fn explicit_contradiction_errors() {
        let err = OwnershipPass::analyze_crate(&fixture("explicit_bad"))
            .expect_err("should fail annotation");
        assert!(matches!(err, OwnershipError::ContradictsAnnotation { .. }));
    }

    #[test]
    fn detect_auto_clone_after_move() {
        let result = OwnershipPass::analyze_crate(&fixture("auto_clone")).expect("auto clone");
        let f = result.get("main", "forward").expect("forward");
        assert!(!f.auto_clones.is_empty());
    }
}
