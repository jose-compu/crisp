use crisp_ast::Span;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorSet {
    names: BTreeSet<String>,
}

impl ErrorSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>) {
        self.names.insert(name.into());
    }

    pub fn extend(&mut self, other: &ErrorSet) {
        self.names.extend(other.names.iter().cloned());
    }

    pub fn remove(&mut self, name: &str) {
        self.names.remove(name);
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.names.iter()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn union(a: &ErrorSet, b: &ErrorSet) -> ErrorSet {
        let mut out = a.clone();
        out.extend(b);
        out
    }

    pub fn subtract(base: &ErrorSet, handled: &ErrorSet) -> ErrorSet {
        let mut out = base.clone();
        for name in handled.iter() {
            out.names.remove(name);
        }
        out
    }
}

impl FromIterator<String> for ErrorSet {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut s = ErrorSet::new();
        for x in iter {
            s.insert(x);
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct ErrorSig {
    pub module: String,
    pub name: String,
    pub fallible: bool,
    pub errors: ErrorSet,
    pub declared: Option<ErrorSet>,
    pub asserts_never: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CrispErrorVariant {
    pub name: String,
    pub payload_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct CrispErrorEnum {
    pub variants: Vec<CrispErrorVariant>,
}

#[derive(Debug, Clone, Default)]
pub struct ErrorResult {
    pub signatures: std::collections::BTreeMap<String, ErrorSig>,
    pub crisp_error: CrispErrorEnum,
}

impl ErrorResult {
    pub fn get(&self, module: &str, name: &str) -> Option<&ErrorSig> {
        self.signatures.get(&format!("{module}::{name}"))
    }
}
