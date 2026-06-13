use anyhow::Result;
use crisp_resolve::{find_crate_root, module::load_module_graph};
use crisp_typeck::{TypeChecker, format_sig};
use std::fmt::Write;
use std::path::Path;

pub fn reveal_expand(crate_path: &Path) -> Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let graph = load_module_graph(&root)?;
    let mut out = String::new();
    for node in graph.modules.values() {
        for item in &node.ast.items {
            if let crisp_ast::item::Item::Function(f) = item {
                let key = format!("{}::{}", node.module_path, f.name.name);
                if let Some(sig) = typed.signatures.get(&key) {
                    let _ = writeln!(out, "{}", format_sig(sig));
                    let _ = writeln!(out, "{}", emit_function_body_stub(f));
                    let _ = writeln!(out);
                }
            }
        }
    }
    Ok(out)
}

fn emit_function_body_stub(f: &crisp_ast::item::FunctionDef) -> String {
    use crisp_ast::expr::ExprKind;
    match &f.body.kind {
        ExprKind::Block(b) => {
            let mut lines = vec![format!("{}(...) = {{", f.name.name)];
            for stmt in &b.stmts {
                if let crisp_ast::expr::Stmt::Bind { pat, .. } = stmt {
                    if let crisp_ast::pat::PatKind::Ident(id) = &pat.kind {
                        lines.push(format!("    {} := <inferred>", id.name));
                    }
                }
            }
            lines.push("}".into());
            lines.join("\n")
        }
        _ => format!("{}(...) = <body>", f.name.name),
    }
}
