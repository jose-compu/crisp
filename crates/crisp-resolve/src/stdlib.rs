//! Virtual `std.*` module symbols (spec §15).

use crate::symbols::{Symbol, SymbolKey, SymbolKind, Visibility};
use crisp_ast::Span;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct StdFn {
    pub module: &'static str,
    pub name: &'static str,
    pub rust_path: &'static str,
}

/// Std functions that lower to Rust paths in emit.
pub fn std_functions() -> &'static [StdFn] {
    &[
        StdFn {
            module: "std.vec",
            name: "new",
            rust_path: "Vec::new",
        },
        StdFn {
            module: "std.vec",
            name: "push",
            rust_path: "Vec::push",
        },
        StdFn {
            module: "std.vec",
            name: "len",
            rust_path: "Vec::len",
        },
        StdFn {
            module: "std.fs",
            name: "read_to_string",
            rust_path: "std::fs::read_to_string",
        },
        StdFn {
            module: "std.io",
            name: "stdin_line",
            rust_path: "std::io::stdin",
        },
        StdFn {
            module: "std.sync",
            name: "sleep_ms",
            rust_path: "tokio::time::sleep",
        },
        StdFn {
            module: "std.atomic",
            name: "new_int",
            rust_path: "std::sync::atomic::AtomicI64::new",
        },
    ]
}

pub fn stdlib_symbols() -> Vec<Symbol> {
    let span = Span::default();
    let mut out = Vec::new();
    for f in std_functions() {
        out.push(Symbol {
            key: SymbolKey {
                module: f.module.to_string(),
                name: f.name.to_string(),
            },
            kind: SymbolKind::PreludeFn,
            visibility: Visibility::Public,
            span,
            from_prelude: true,
        });
    }
    for (module, types) in [
        ("std.option", ["Option"]),
        ("std.result", ["Result"]),
        ("std.string", ["String"]),
        ("std.vec", ["Vec"]),
        ("std.map", ["HashMap"]),
        ("std.set", ["HashSet"]),
    ] {
        for name in types {
            out.push(Symbol {
                key: SymbolKey {
                    module: module.to_string(),
                    name: name.to_string(),
                },
                kind: SymbolKind::PreludeType,
                visibility: Visibility::Public,
                span,
                from_prelude: true,
            });
        }
    }
    out
}

pub fn stdlib_fn_modules() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for f in std_functions() {
        map.insert(f.name.to_string(), f.module.to_string());
    }
    map
}

pub fn std_rust_path(module: &str, name: &str) -> Option<&'static str> {
    std_functions()
        .iter()
        .find(|f| f.module == module && f.name == name)
        .map(|f| f.rust_path)
}
