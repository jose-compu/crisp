use crate::lattice::OwnershipMode;
use crate::usage::Usage;
use crisp_ast::Span;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct AutoClone {
    pub binding: String,
    pub span: Span,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct OwnershipSignature {
    pub module: String,
    pub name: String,
    pub params: Vec<(String, OwnershipMode)>,
    pub ret_mode: Option<OwnershipMode>,
    pub auto_clones: Vec<AutoClone>,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct OwnershipResult {
    pub signatures: BTreeMap<String, OwnershipSignature>,
}

impl OwnershipResult {
    pub fn get(&self, module: &str, name: &str) -> Option<&OwnershipSignature> {
        self.signatures.get(&format!("{module}::{name}"))
    }
}

#[derive(Debug, Clone, Default)]
pub struct BindingUsages {
    usages: BTreeMap<String, Usage>,
}

impl BindingUsages {
    pub fn add(&mut self, name: &str, usage: Usage) {
        self.usages
            .entry(name.to_string())
            .and_modify(|u| *u = crate::usage::join_usage(*u, usage))
            .or_insert(usage);
    }

    pub fn mode_for(&self, name: &str) -> OwnershipMode {
        self.usages
            .get(name)
            .map(|u| OwnershipMode::from_usage(*u))
            .unwrap_or(OwnershipMode::Borrow)
    }

    pub fn usages(&self) -> &BTreeMap<String, Usage> {
        &self.usages
    }
}
