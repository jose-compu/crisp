//! Minimal Rust probe emission for borrow-check validation (spec §7.6).

use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{FunctionDef, Item, TypeBody};
use crisp_ast::pat::PatKind;
use crisp_ownership::{FallbackKind, OwnershipMode, OwnershipResult};
use crisp_resolve::module::ModuleGraph;
use crisp_typeck::Ty;
use crisp_typeck::TypedCrate;
use std::fmt::Write;

pub fn emit_probe_crate(
    graph: &ModuleGraph,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
) -> String {
    let mut out = String::from("#![allow(dead_code, unused_variables, unused_imports)]\n");
    let _ = writeln!(out, "fn log<T: std::fmt::Display>(_: T) {{}}");
    let _ = writeln!(out, "fn print<T: std::fmt::Debug>(v: T) {{ let _ = v; }}");

    // Emit type definitions first so probe functions can name them.
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::TypeDef(td) = item {
                emit_type_def(&mut out, td);
            }
        }
    }

    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::Function(f) = item {
                let key = format!("{}::{}", node.module_path, f.name.name);
                let osig = ownership.signatures.get(&key);
                let tsig = typed.signatures.get(&key);
                if let (Some(o), Some(t)) = (osig, tsig) {
                    emit_function(&mut out, f, o, t);
                }
            }
        }
    }
    out
}

fn emit_type_def(out: &mut String, td: &crisp_ast::item::TypeDef) {
    match &td.body {
        TypeBody::Struct(fields) => {
            let _ = writeln!(out, "#[derive(Clone)]");
            let _ = writeln!(out, "struct {} {{", td.name.name);
            for f in fields {
                let ty = ast_type_rust(&f.ty);
                let _ = writeln!(out, "    {}: {ty},", f.name.name);
            }
            let _ = writeln!(out, "}}");
        }
        TypeBody::Enum(variants) => {
            let _ = writeln!(out, "#[derive(Clone)]");
            let _ = writeln!(out, "enum {} {{", td.name.name);
            for v in variants {
                if v.fields.is_empty() {
                    let _ = writeln!(out, "    {},", v.name.name);
                } else {
                    let tys: Vec<_> = v.fields.iter().map(ast_type_rust).collect();
                    let _ = writeln!(out, "    {}({}),", v.name.name, tys.join(", "));
                }
            }
            let _ = writeln!(out, "}}");
        }
        TypeBody::Alias(ty) => {
            let _ = writeln!(out, "type {} = {};", td.name.name, ast_type_rust(ty));
        }
    }
}

fn ast_type_rust(ty: &crisp_ast::ty::Type) -> String {
    use crisp_ast::ty::TypeKind;
    match &ty.kind {
        TypeKind::Named(id) => match id.name.as_str() {
            "int" => "i64".into(),
            "uint" => "u64".into(),
            "float" => "f64".into(),
            "bool" => "bool".into(),
            "char" => "char".into(),
            "str" => "String".into(),
            "Never" => "!".into(),
            other => other.to_string(),
        },
        TypeKind::Unit | TypeKind::Never => "()".into(),
        TypeKind::Option(inner) => format!("Option<{}>", ast_type_rust(inner)),
        TypeKind::Tuple(ts) => format!(
            "({})",
            ts.iter().map(ast_type_rust).collect::<Vec<_>>().join(", ")
        ),
        TypeKind::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", ast_type_rust(inner))
            } else {
                format!("&{}", ast_type_rust(inner))
            }
        }
        _ => "_".into(),
    }
}

fn emit_function(
    out: &mut String,
    def: &FunctionDef,
    osig: &crisp_ownership::OwnershipSignature,
    tsig: &crisp_typeck::InferredSig,
) {
    let _ = writeln!(out);
    let params = osig
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, mode))| {
            let ty = tsig.params.get(i).map(|(_, t)| t);
            format_rust_param(name, ty, *mode, osig)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = format_rust_ret(&tsig.ret);
    let _ = writeln!(out, "pub fn {}({params}){ret} {{", def.name.name);
    emit_body(out, &def.body, osig, 1);
    let _ = writeln!(out, "}}");
}

pub(crate) fn format_rust_param(
    name: &str,
    ty: Option<&Ty>,
    mode: OwnershipMode,
    osig: &crisp_ownership::OwnershipSignature,
) -> String {
    let clone_applied = osig
        .applied_fallbacks
        .iter()
        .any(|f| f.kind == FallbackKind::CloneAtMove);
    let force_owned = !clone_applied
        && osig.auto_clones.iter().any(|ac| ac.binding == name)
        && ty.is_some_and(|t| t.is_stringish());

    if force_owned {
        return format!("{name}: String");
    }

    let inner = ty.map(format_rust_ty).unwrap_or_else(|| "_".into());
    match mode {
        OwnershipMode::Borrow if ty.is_some_and(|t| t.is_stringish()) => {
            format!("{name}: &str")
        }
        OwnershipMode::Borrow => format!("{name}: &{inner}"),
        OwnershipMode::MutBorrow => format!("{name}: &mut {inner}"),
        OwnershipMode::Owned => format!("{name}: {inner}"),
    }
}

pub(crate) fn format_rust_ret(ty: &Ty) -> String {
    if matches!(ty, Ty::Unit) {
        String::new()
    } else {
        format!(" -> {}", format_rust_ty(ty))
    }
}

pub(crate) fn format_rust_ty(ty: &Ty) -> String {
    match ty {
        Ty::Str => "String".into(),
        Ty::Int => "i64".into(),
        Ty::UInt => "u64".into(),
        Ty::Float => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Char => "char".into(),
        Ty::Unit | Ty::Never => "()".into(),
        Ty::Var(_) => "_".into(),
        Ty::StrSlice => "str".into(),
        Ty::Named { name, args } if args.is_empty() => match name.as_str() {
            "vec" => "Vec<_>".into(),
            "map" => "std::collections::HashMap<_, _>".into(),
            "set" => "std::collections::HashSet<_>".into(),
            other => other.to_string(),
        },
        Ty::Named { name, args } => format!(
            "{name}<{}>",
            args.iter()
                .map(format_rust_ty)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Ty::Option(inner) => format!("Option<{}>", format_rust_ty(inner)),
        Ty::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", format_rust_ty(inner))
            } else {
                format!("&{}", format_rust_ty(inner))
            }
        }
        Ty::Tuple(ts) => format!(
            "({})",
            ts.iter().map(format_rust_ty).collect::<Vec<_>>().join(", ")
        ),
        Ty::Fn { .. } | Ty::Array { .. } | Ty::Slice(_) | Ty::Error => "_".into(),
    }
}

pub(crate) fn emit_body(
    out: &mut String,
    expr: &Expr,
    osig: &crisp_ownership::OwnershipSignature,
    indent: usize,
) {
    match &expr.kind {
        ExprKind::Block(b) => emit_block(out, b, osig, indent),
        other => emit_expr_stmt(out, expr, osig, indent, other),
    }
}

fn emit_block(
    out: &mut String,
    block: &Block,
    osig: &crisp_ownership::OwnershipSignature,
    indent: usize,
) {
    for stmt in &block.stmts {
        emit_stmt(out, stmt, osig, indent);
    }
    if let Some(tail) = &block.tail {
        emit_tail(out, tail, osig, indent);
    }
}

fn emit_stmt(
    out: &mut String,
    stmt: &Stmt,
    osig: &crisp_ownership::OwnershipSignature,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    match stmt {
        Stmt::Expr(e) => {
            let _ = write!(out, "{pad}");
            emit_expr(out, e, osig);
            let _ = writeln!(out, ";");
        }
        Stmt::Bind { pat, value, .. } => {
            if let PatKind::Ident(name) = &pat.kind {
                let clone = should_clone_at_bind(osig, &name.name, value);
                let _ = write!(out, "{pad}let {} = ", name.name);
                if clone {
                    emit_expr(out, value, osig);
                    let _ = write!(out, ".clone()");
                } else {
                    emit_expr(out, value, osig);
                }
                let _ = writeln!(out, ";");
            }
        }
        Stmt::Assign { target, value } => {
            let _ = write!(out, "{pad}{} = ", target.name);
            emit_expr(out, value, osig);
            let _ = writeln!(out, ";");
        }
    }
}

fn should_clone_at_bind(
    osig: &crisp_ownership::OwnershipSignature,
    binding: &str,
    value: &Expr,
) -> bool {
    if !osig
        .applied_fallbacks
        .iter()
        .any(|f| f.kind == FallbackKind::CloneAtMove)
    {
        return false;
    }
    if let ExprKind::Ident(src) = &value.kind {
        return osig.auto_clones.iter().any(|ac| ac.binding == src.name);
    }
    let _ = binding;
    false
}

fn emit_tail(
    out: &mut String,
    expr: &Expr,
    osig: &crisp_ownership::OwnershipSignature,
    indent: usize,
) {
    let pad = "    ".repeat(indent);
    let _ = write!(out, "{pad}");
    emit_expr(out, expr, osig);
    let _ = writeln!(out);
}

fn emit_expr_stmt(
    out: &mut String,
    expr: &Expr,
    osig: &crisp_ownership::OwnershipSignature,
    indent: usize,
    kind: &ExprKind,
) {
    let pad = "    ".repeat(indent);
    let _ = write!(out, "{pad}");
    emit_expr_inner(out, expr, osig, kind);
    let _ = writeln!(out);
}

fn emit_expr(out: &mut String, expr: &Expr, osig: &crisp_ownership::OwnershipSignature) {
    emit_expr_inner(out, expr, osig, &expr.kind);
}

fn emit_expr_inner(
    out: &mut String,
    _expr: &Expr,
    osig: &crisp_ownership::OwnershipSignature,
    kind: &ExprKind,
) {
    match kind {
        ExprKind::Ident(id) => {
            let _ = write!(out, "{}", id.name);
        }
        ExprKind::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        ExprKind::Str(parts) => {
            let has_expr = parts
                .0
                .iter()
                .any(|p| matches!(p, crisp_ast::expr::StringPart::Expr(_)));
            if has_expr {
                let _ = write!(out, "format!(\"");
                let mut args = Vec::new();
                for part in &parts.0 {
                    match part {
                        crisp_ast::expr::StringPart::Lit(s) => {
                            let _ = write!(out, "{}", escape_format_lit(s));
                        }
                        crisp_ast::expr::StringPart::Expr(e) => {
                            let _ = write!(out, "{{}}");
                            args.push(e);
                        }
                    }
                }
                let _ = write!(out, "\"");
                for e in args {
                    let _ = write!(out, ", ");
                    emit_expr(out, e, osig);
                }
                let _ = write!(out, ")");
            } else {
                let _ = write!(out, "\"");
                for part in &parts.0 {
                    if let crisp_ast::expr::StringPart::Lit(s) = part {
                        let _ = write!(out, "{}", escape_str(s));
                    }
                }
                let _ = write!(out, "\".to_string()");
            }
        }
        ExprKind::Int(n) => {
            let _ = write!(out, "{n}");
        }
        ExprKind::Float(f) => {
            if f.fract() == 0.0 {
                let _ = write!(out, "{f}.0");
            } else {
                let _ = write!(out, "{f}");
            }
        }
        ExprKind::Binary { op, left, right } => {
            if matches!(op, crisp_ast::expr::BinaryOp::Concat) {
                let _ = write!(out, "format!(\"{{}}{{}}\", ");
                emit_expr(out, left, osig);
                let _ = write!(out, ", ");
                emit_expr(out, right, osig);
                let _ = write!(out, ")");
                return;
            }
            if matches!(op, crisp_ast::expr::BinaryOp::Pow) {
                let _ = write!(out, "(");
                emit_expr(out, left, osig);
                let _ = write!(out, ").powf(");
                emit_expr(out, right, osig);
                let _ = write!(out, ")");
                return;
            }
            emit_expr(out, left, osig);
            let op_s = match op {
                crisp_ast::expr::BinaryOp::Add => "+",
                crisp_ast::expr::BinaryOp::Sub => "-",
                crisp_ast::expr::BinaryOp::Mul => "*",
                crisp_ast::expr::BinaryOp::Div => "/",
                crisp_ast::expr::BinaryOp::Eq => "==",
                crisp_ast::expr::BinaryOp::Lt => "<",
                crisp_ast::expr::BinaryOp::Gt => ">",
                _ => "+",
            };
            let _ = write!(out, " {op_s} ");
            emit_expr(out, right, osig);
        }
        ExprKind::Call { func, args } => {
            // Enum ctor Color.Custom(...)
            if let ExprKind::Field { base, field } = &func.kind
                && let ExprKind::Ident(ty) = &base.kind
                && ty
                    .name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
            {
                let _ = write!(out, "{}::{}(", ty.name, field.name);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(out, ", ");
                    }
                    emit_expr(out, arg, osig);
                }
                let _ = write!(out, ")");
                return;
            }
            if let ExprKind::Ident(id) = &func.kind {
                if (id.name == "print" || id.name == "log") && args.len() == 1 {
                    let _ = write!(out, "{}(", id.name);
                    emit_expr(out, &args[0], osig);
                    let _ = write!(out, ")");
                    return;
                }
            }
            emit_expr(out, func, osig);
            let _ = write!(out, "(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                emit_expr(out, arg, osig);
            }
            let _ = write!(out, ")");
        }
        ExprKind::Field { base, field } => {
            if let ExprKind::Ident(ty) = &base.kind
                && ty
                    .name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
            {
                let _ = write!(out, "{}::{}", ty.name, field.name);
                return;
            }
            emit_expr(out, base, osig);
            let _ = write!(out, ".{}", field.name);
        }
        ExprKind::StructLit { name, fields } => {
            let _ = write!(out, "{} {{ ", name.name);
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                let _ = write!(out, "{}: ", f.name.name);
                emit_expr(out, &f.value, osig);
            }
            let _ = write!(out, " }}");
        }
        ExprKind::Block(b) => {
            let _ = writeln!(out, "{{");
            emit_block(out, b, osig, 1);
            let _ = write!(out, "}}");
        }
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let _ = write!(out, "if ");
            emit_expr(out, cond, osig);
            let _ = write!(out, " {{ ");
            emit_expr(out, then_branch, osig);
            let _ = write!(out, " }}");
            if let Some(e) = else_branch {
                let _ = write!(out, " else {{ ");
                emit_expr(out, e, osig);
                let _ = write!(out, " }}");
            }
        }
        _ => {
            let _ = write!(out, "()");
        }
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_format_lit(s: &str) -> String {
    escape_str(s).replace('{', "{{").replace('}', "}}")
}
