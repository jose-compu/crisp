use crate::symbols::{Symbol, SymbolKey, SymbolKind, Visibility};
use crisp_ast::Span;

/// Built-in prelude symbols (spec §15). `std/prelude.crp` is documentation until populated.
pub fn prelude_symbols() -> Vec<Symbol> {
    let module = "std.prelude".to_string();
    let span = Span::default();
    let types = [
        "int", "uint", "float", "bool", "char", "str", "Never", "vec", "map", "set",
    ];
    let fns = ["log", "print", "some", "none", "assert_eq"];

    let mut out = Vec::new();
    for name in types {
        out.push(Symbol {
            key: SymbolKey {
                module: module.clone(),
                name: name.to_string(),
            },
            kind: SymbolKind::PreludeType,
            visibility: Visibility::Public,
            span,
            from_prelude: true,
        });
    }
    for name in fns {
        out.push(Symbol {
            key: SymbolKey {
                module: module.clone(),
                name: name.to_string(),
            },
            kind: SymbolKind::PreludeFn,
            visibility: Visibility::Public,
            span,
            from_prelude: true,
        });
    }
    out
}
