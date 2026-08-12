//! Resolve-time warnings (non-fatal).

use crisp_ast::Span;
use std::fmt;

/// Non-fatal resolve diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveWarning {
    /// Bare `use {name}` binds a Crisp module that shares a name with a `rust = true` dep.
    ModuleShadowsRustDep { name: String, span: Span },
}

impl ResolveWarning {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ModuleShadowsRustDep { .. } => "W0048",
        }
    }
}

impl fmt::Display for ResolveWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModuleShadowsRustDep { name, .. } => write!(
                f,
                "[W0048] `{name}` is both a Crisp module and a Rust dependency; \
                 bare `use {name}` binds the Crisp module; \
                 use `use rust.{name} {{ … }}` for the crate (spec §14.2, #41)"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crisp_ast::Span;

    #[test]
    fn w0048_display_mentions_code_and_disambiguation() {
        let w = ResolveWarning::ModuleShadowsRustDep {
            name: "config".into(),
            span: Span::new(0, 1),
        };
        assert_eq!(w.code(), "W0048");
        let msg = w.to_string();
        assert!(msg.contains("W0048"));
        assert!(msg.contains("config"));
        assert!(msg.contains("use rust.config"));
    }
}
