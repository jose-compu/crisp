//! Code lenses — "Show emitted Rust" on functions.

use crate::walk::all_functions;
use crisp_ast::Span;
use crisp_ast::item::SourceFile;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CodeLens {
    pub span: Span,
    pub title: String,
    pub command: String,
    pub arguments: Vec<String>,
}

pub fn code_lenses_for_file(file: &SourceFile, crate_root: &Path) -> Vec<CodeLens> {
    let root = crate_root.display().to_string();
    let mut lenses = Vec::new();
    let functions = all_functions(file);
    for f in &functions {
        lenses.push(CodeLens {
            span: f.name.span,
            title: "Show emitted Rust".into(),
            command: "crisp.showEmittedRust".into(),
            arguments: vec![root.clone(), f.name.name.clone()],
        });
    }
    if let Some(main) = functions.iter().find(|f| f.name.name == "main") {
        lenses.push(CodeLens {
            span: main.span,
            title: "Run crisp test".into(),
            command: "crisp.runTests".into(),
            arguments: vec![root],
        });
    }
    lenses
}
