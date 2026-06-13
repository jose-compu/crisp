//! Fallible probe emission — Result<T, CrispError> lowering (spec §9).

use crate::probe::{format_rust_ret, format_rust_ty};
use crisp_ast::expr::{Block, Expr, ExprKind, Stmt};
use crisp_ast::item::{FunctionDef, Item};
use crisp_errors::{ErrorResult, format_crisp_error_enum};
use crisp_ownership::{OwnershipResult, OwnershipSignature};
use crisp_resolve::module::ModuleGraph;
use crisp_typeck::TypedCrate;
use std::collections::BTreeMap;
use std::fmt::Write;

pub fn emit_fallible_probe_crate(
    graph: &ModuleGraph,
    typed: &TypedCrate,
    ownership: &OwnershipResult,
    errors: &ErrorResult,
) -> String {
    let mut out = String::from("#![allow(dead_code, unused_variables)]\n");
    emit_type_defs(graph, &mut out);
    if !errors.crisp_error.variants.is_empty() {
        let _ = writeln!(out, "\n{}", format_crisp_error_enum(&errors.crisp_error));
    }
    let fallible: BTreeMap<String, bool> = errors
        .signatures
        .iter()
        .map(|(k, s)| (k.clone(), s.fallible))
        .collect();
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::Function(f) = item {
                let key = format!("{}::{}", node.module_path, f.name.name);
                if f.name.name == "main" {
                    continue;
                }
                let osig = ownership.signatures.get(&key);
                let tsig = typed.signatures.get(&key);
                let esig = errors.signatures.get(&key);
                if let (Some(o), Some(t), Some(e)) = (osig, tsig, esig) {
                    emit_fallible_function(
                        &mut out,
                        f,
                        o,
                        t,
                        e,
                        &fallible,
                        &ownership.signatures,
                        &node.module_path,
                    );
                }
            }
        }
    }
    out
}

fn emit_type_defs(graph: &ModuleGraph, out: &mut String) {
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let Item::TypeDef(td) = item {
                if let crisp_ast::item::TypeBody::Struct(fields) = &td.body {
                    let _ = writeln!(out, "#[derive(Debug, Clone, Default)]");
                    let _ = write!(out, "struct {} {{", td.name.name);
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        let _ = write!(
                            out,
                            "pub {}: {}",
                            f.name.name,
                            field_ty(&f.ty.kind)
                        );
                    }
                    let _ = writeln!(out, "}}");
                }
            }
        }
    }
}

fn field_ty(kind: &crisp_ast::ty::TypeKind) -> &'static str {
    use crisp_ast::ty::TypeKind;
    match kind {
        TypeKind::Named(id) => match id.name.as_str() {
            "int" => "i64",
            "str" => "String",
            _ => "String",
        },
        _ => "String",
    }
}

fn fallible_rust_param(
    name: &str,
    ty: Option<&crisp_typeck::Ty>,
    mode: crisp_ownership::OwnershipMode,
    osig: &crisp_ownership::OwnershipSignature,
) -> String {
    use crisp_ownership::OwnershipMode;
    use crisp_typeck::Ty;

    let clone_applied = osig
        .applied_fallbacks
        .iter()
        .any(|f| f.kind == crisp_ownership::FallbackKind::CloneAtMove);
    let force_owned = !clone_applied
        && osig.auto_clones.iter().any(|ac| ac.binding == name)
        && ty.is_some_and(|t| t.is_stringish());

    if force_owned {
        return format!("{name}: String");
    }

    let unresolved_str = ty.is_none_or(|t| matches!(t, Ty::Str | Ty::StrSlice | Ty::Var(_)));
    if unresolved_str && matches!(mode, OwnershipMode::Borrow) {
        return format!("{name}: &str");
    }

    let inner = match ty {
        Some(Ty::Str) | Some(Ty::StrSlice) | Some(Ty::Var(_)) => "String".into(),
        Some(t) => format_rust_ty(t),
        None => "_".into(),
    };
    match mode {
        OwnershipMode::Borrow => format!("{name}: &{inner}"),
        OwnershipMode::MutBorrow => format!("{name}: &mut {inner}"),
        OwnershipMode::Owned => format!("{name}: {inner}"),
    }
}

fn emit_fallible_function(
    out: &mut String,
    def: &FunctionDef,
    osig: &OwnershipSignature,
    tsig: &crisp_typeck::InferredSig,
    esig: &crisp_errors::ErrorSig,
    fallible: &BTreeMap<String, bool>,
    ownership_sigs: &BTreeMap<String, OwnershipSignature>,
    module: &str,
) {
    let _ = writeln!(out);
    let params = osig
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, mode))| {
            let ty = tsig.params.get(i).map(|(_, t)| t);
            fallible_rust_param(name, ty, *mode, osig)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let inner_ret = if matches!(tsig.ret, crisp_typeck::Ty::Unit) {
        "()".into()
    } else if matches!(tsig.ret, crisp_typeck::Ty::Var(_)) {
        "Config".into()
    } else {
        format_rust_ty(&tsig.ret)
    };
    let ret = if esig.fallible {
        format!(" -> Result<{inner_ret}, CrispError>")
    } else {
        format_rust_ret(&tsig.ret)
    };

    let _ = writeln!(out, "pub fn {}({params}){ret} {{", def.name.name);
    emit_fallible_expr(
        out,
        &def.body,
        osig,
        fallible,
        ownership_sigs,
        module,
        esig.fallible,
        1,
        true,
    );
    let _ = writeln!(out, "}}");
}

fn emit_fallible_expr(
    out: &mut String,
    expr: &Expr,
    osig: &OwnershipSignature,
    fallible: &BTreeMap<String, bool>,
    ownership_sigs: &BTreeMap<String, OwnershipSignature>,
    module: &str,
    fn_fallible: bool,
    indent: usize,
    is_tail: bool,
) {
    if let ExprKind::Throw(inner) = &expr.kind {
        let pad = "    ".repeat(indent);
        let _ = write!(out, "{pad}return Err(");
        emit_throw_payload(out, inner);
        let _ = writeln!(out, ");");
        return;
    }
    match &expr.kind {
        ExprKind::Block(b) => emit_fallible_block(
            out,
            b,
            osig,
            fallible,
            ownership_sigs,
            module,
            fn_fallible,
            indent,
        ),
        ExprKind::Call { func, args } => {
            let pad = "    ".repeat(indent);
            if is_tail && fn_fallible {
                let _ = write!(out, "{pad}Ok(");
            } else if !is_tail {
                let _ = write!(out, "{pad}let _v = ");
            }
            emit_call(out, func, args, osig, fallible, ownership_sigs, module);
            if callee_is_fallible(func, fallible, module) {
                let _ = write!(out, "?");
            }
            let _ = writeln!(out, ")");
            if is_tail && fn_fallible {
                let _ = writeln!(out);
            }
        }
        _ if is_tail && fn_fallible => {
            let pad = "    ".repeat(indent);
            let _ = write!(out, "{pad}Ok(");
            emit_simple_expr(out, expr, osig);
            let _ = writeln!(out, ")");
        }
        _ => {
            crate::probe::emit_body(out, expr, osig, indent);
            if is_tail && fn_fallible {
                let pad = "    ".repeat(indent);
                let _ = writeln!(out, "{pad}Ok(())");
            }
        }
    }
}

fn emit_fallible_block(
    out: &mut String,
    block: &Block,
    osig: &OwnershipSignature,
    fallible: &BTreeMap<String, bool>,
    ownership_sigs: &BTreeMap<String, OwnershipSignature>,
    module: &str,
    fn_fallible: bool,
    indent: usize,
) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Bind { pat, value, .. } => {
                if let crisp_ast::pat::PatKind::Ident(name) = &pat.kind {
                    let pad = "    ".repeat(indent);
                    let _ = write!(out, "{pad}let {} = ", name.name);
                    if let ExprKind::Call { func, args } = &value.kind {
                        emit_call(out, func, args, osig, fallible, ownership_sigs, module);
                        if callee_is_fallible(func, fallible, module) {
                            let _ = write!(out, "?");
                        }
                        let _ = writeln!(out, ";");
                    } else {
                        emit_simple_expr(out, value, osig);
                        let _ = writeln!(out, ";");
                    }
                }
            }
            Stmt::Expr(e) => emit_fallible_expr(
                out,
                e,
                osig,
                fallible,
                ownership_sigs,
                module,
                fn_fallible,
                indent,
                false,
            ),
            Stmt::Assign { target, value } => {
                let pad = "    ".repeat(indent);
                let _ = write!(out, "{pad}{} = ", target.name);
                emit_simple_expr(out, value, osig);
                let _ = writeln!(out, ";");
            }
        }
    }
    if let Some(tail) = &block.tail {
        emit_fallible_expr(
            out,
            tail,
            osig,
            fallible,
            ownership_sigs,
            module,
            fn_fallible,
            indent,
            true,
        );
    } else if fn_fallible {
        let pad = "    ".repeat(indent);
        let _ = writeln!(out, "{pad}Ok(())");
    }
}

fn callee_is_fallible(
    func: &Expr,
    fallible: &BTreeMap<String, bool>,
    module: &str,
) -> bool {
    if let ExprKind::Ident(id) = &func.kind {
        let key = format!("{module}::{}", id.name);
        return fallible.get(&key).copied().unwrap_or(false);
    }
    false
}

fn emit_call(
    out: &mut String,
    func: &Expr,
    args: &[Expr],
    osig: &OwnershipSignature,
    fallible: &BTreeMap<String, bool>,
    ownership_sigs: &BTreeMap<String, OwnershipSignature>,
    module: &str,
) {
    let _ = fallible;
    let callee_osig = if let ExprKind::Ident(id) = &func.kind {
        let key = format!("{module}::{}", id.name);
        ownership_sigs.get(&key)
    } else {
        None
    };
    if let ExprKind::Ident(id) = &func.kind {
        let _ = write!(out, "{}", id.name);
    }
    let _ = write!(out, "(");
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            let _ = write!(out, ", ");
        }
        if let (Some(callee), ExprKind::Ident(id)) = (callee_osig, &arg.kind) {
            if callee
                .params
                .get(i)
                .is_some_and(|(_, mode)| matches!(mode, crisp_ownership::OwnershipMode::Borrow))
            {
                let _ = write!(out, "&{}", id.name);
                continue;
            }
        }
        emit_simple_expr(out, arg, osig);
    }
    let _ = write!(out, ")");
}

fn emit_simple_expr(out: &mut String, expr: &Expr, osig: &crisp_ownership::OwnershipSignature) {
    use crisp_ast::expr::StringPart;
    match &expr.kind {
        ExprKind::Ident(id) => {
            let _ = write!(out, "{}", id.name);
        }
        ExprKind::Str(parts) => {
            let _ = write!(out, "\"");
            for part in &parts.0 {
                if let StringPart::Lit(s) = part {
                    let _ = write!(out, "{s}");
                }
            }
            let _ = write!(out, "\".to_string()");
        }
        ExprKind::StructLit { name, fields } => {
            let _ = write!(out, "{} {{", name.name);
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                let _ = write!(out, "{}: ", f.name.name);
                emit_simple_expr(out, &f.value, osig);
            }
            let _ = write!(out, "}}");
        }
        ExprKind::Call { func, args } => {
            emit_call(out, func, args, osig, &BTreeMap::new(), &BTreeMap::new(), "");
        }
        _ => {
            let _ = write!(out, "()");
        }
    }
}

fn emit_throw_payload(out: &mut String, expr: &Expr) {
    match &expr.kind {
        ExprKind::StructLit { name, fields } => {
            let _ = write!(out, "CrispError::{}({} {{", name.name, name.name);
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                let _ = write!(out, "{}: ", f.name.name);
                emit_literal_or_expr(out, &f.value);
            }
            let _ = write!(out, " }})");
        }
        _ => {
            let _ = write!(out, "CrispError::Thrown(\"error\".into())");
        }
    }
}

fn emit_literal_or_expr(out: &mut String, expr: &Expr) {
    match &expr.kind {
        ExprKind::Str(parts) => {
            let _ = write!(out, "\"");
            for part in &parts.0 {
                if let crisp_ast::expr::StringPart::Lit(s) = part {
                    let _ = write!(out, "{s}");
                }
            }
            let _ = write!(out, "\".into()");
        }
        ExprKind::Int(n) => {
            let _ = write!(out, "{n}");
        }
        _ => {
            let _ = write!(out, "Default::default()");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustc::check_rust_source;
    use crisp_errors::ErrorPass;
    use crisp_ownership::OwnershipPass;
    use crisp_resolve::module::load_module_graph;
    use crisp_typeck::TypeChecker;
    use std::path::PathBuf;

    #[test]
    fn fallible_probe_emits_compilable_rust() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/fallible");
        let graph = load_module_graph(&root).expect("graph");
        let typed = TypeChecker::check_crate(&root).expect("typeck");
        let ownership = OwnershipPass::analyze_crate(&root).expect("ownership");
        let errors = ErrorPass::analyze_crate(&root).expect("errors");
        let src = emit_fallible_probe_crate(&graph, &typed, &ownership, &errors);
        if let Err(crate::rustc::RustcError::NotFound) = check_rust_source(&src) {
            return;
        }
        check_rust_source(&src).expect("fallible probe rustc");
    }
}
