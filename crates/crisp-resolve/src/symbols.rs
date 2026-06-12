use crisp_ast::Span;
use crisp_ast::item::Item;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Type,
    Trait,
    Shape,
    Const,
    ExternFn,
    PreludeType,
    PreludeFn,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolKey {
    pub module: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub key: SymbolKey,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub span: Span,
    pub from_prelude: bool,
}

impl Symbol {
    pub fn is_exported(&self) -> bool {
        self.visibility == Visibility::Public || self.from_prelude
    }
}

pub fn collect_module_symbols(module_path: &str, items: &[Item]) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for item in items {
        match item {
            Item::Function(f) => symbols.push(Symbol {
                key: SymbolKey {
                    module: module_path.to_string(),
                    name: f.name.name.clone(),
                },
                kind: SymbolKind::Function,
                visibility: if f.is_pub {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                span: f.name.span,
                from_prelude: false,
            }),
            Item::TypeDef(t) => symbols.push(Symbol {
                key: SymbolKey {
                    module: module_path.to_string(),
                    name: t.name.name.clone(),
                },
                kind: SymbolKind::Type,
                visibility: if t.is_pub {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                span: t.name.span,
                from_prelude: false,
            }),
            Item::TraitDef(t) => symbols.push(Symbol {
                key: SymbolKey {
                    module: module_path.to_string(),
                    name: t.name.name.clone(),
                },
                kind: SymbolKind::Trait,
                visibility: Visibility::Public,
                span: t.name.span,
                from_prelude: false,
            }),
            Item::ShapeDef(s) => symbols.push(Symbol {
                key: SymbolKey {
                    module: module_path.to_string(),
                    name: s.name.name.clone(),
                },
                kind: SymbolKind::Shape,
                visibility: Visibility::Public,
                span: s.name.span,
                from_prelude: false,
            }),
            Item::Const(c) => symbols.push(Symbol {
                key: SymbolKey {
                    module: module_path.to_string(),
                    name: c.name.name.clone(),
                },
                kind: SymbolKind::Const,
                visibility: Visibility::Private,
                span: c.name.span,
                from_prelude: false,
            }),
            Item::Extern(e) => {
                for f in &e.functions {
                    symbols.push(Symbol {
                        key: SymbolKey {
                            module: module_path.to_string(),
                            name: f.name.name.clone(),
                        },
                        kind: SymbolKind::ExternFn,
                        visibility: Visibility::Public,
                        span: f.name.span,
                        from_prelude: false,
                    });
                }
            }
            Item::Use(_) | Item::Impl(_) | Item::Test(_) | Item::TestCompileFail(_) => {}
        }
    }
    symbols
}
