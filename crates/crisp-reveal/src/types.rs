use crisp_resolve::find_crate_root;
use crisp_typeck::{TypeChecker, format_sig};
use std::path::Path;

pub fn reveal_types(crate_path: &Path) -> anyhow::Result<String> {
    let root = find_crate_root(crate_path).unwrap_or_else(|| crate_path.to_path_buf());
    let typed = TypeChecker::check_crate(&root)?;
    let mut lines: Vec<String> = typed
        .signatures
        .values()
        .map(|sig| {
            let mut line = format_sig(sig);
            if !sig.instantiations.is_empty() {
                line.push_str("  -- used as ");
                line.push_str(&sig.instantiations.join("; "));
            }
            line
        })
        .collect();
    lines.sort();
    for c in &typed.coercions {
        let slot = if c.to_float { "float" } else { "int" };
        let kind = if c.explicit {
            "explicit"
        } else if c.literal {
            "int literal"
        } else {
            "implicit"
        };
        lines.push(format!(
            "coercion @{start}..{end}: {kind} as {slot}",
            start = c.span.start,
            end = c.span.end
        ));
    }
    Ok(lines.join("\n"))
}
