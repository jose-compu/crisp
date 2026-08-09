//! rustc-style diagnostic formatting (spec §17.4).

use crate::{Diagnostic, Severity};
use crisp_ast::Span;

#[derive(Debug, Clone)]
pub struct FormattedDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: Severity,
    pub span: Span,
    pub notes: Vec<String>,
    pub rendered: String,
}

pub fn format_diagnostic(
    source: &str,
    code: &str,
    message: &str,
    span: Span,
    severity: Severity,
    notes: &[String],
) -> FormattedDiagnostic {
    format_diagnostic_at("source", source, code, message, span, severity, notes)
}

pub fn format_diagnostic_at(
    file: &str,
    source: &str,
    code: &str,
    message: &str,
    span: Span,
    severity: Severity,
    notes: &[String],
) -> FormattedDiagnostic {
    let rendered = render(file, source, code, message, span, severity, notes);
    FormattedDiagnostic {
        code: code.to_string(),
        message: message.to_string(),
        severity,
        span,
        notes: notes.to_vec(),
        rendered,
    }
}

pub fn from_diagnostic(source: &str, diag: &Diagnostic) -> FormattedDiagnostic {
    format_diagnostic(
        source,
        &diag.code,
        &diag.message,
        diag.span,
        diag.severity,
        &diag.notes,
    )
}

/// Format an unresolved-name diagnostic with optional import / module hint.
pub fn format_unresolved_name(
    file: &str,
    source: &str,
    name: &str,
    span: Span,
    hint: Option<&str>,
) -> FormattedDiagnostic {
    let mut notes = Vec::new();
    if let Some(h) = hint {
        notes.push(format!("help: {h}"));
    } else {
        notes.push(
            "help: check spelling, `use` imports, and that the defining module is imported".into(),
        );
    }
    format_diagnostic_at(
        file,
        source,
        "E0035",
        &format!("unresolved name `{name}`"),
        span,
        Severity::Error,
        &notes,
    )
}

fn line_col(source: &str, offset: u32) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i as u32 >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn render(
    file: &str,
    source: &str,
    code: &str,
    message: &str,
    span: Span,
    severity: Severity,
    notes: &[String],
) -> String {
    let (line, col) = line_col(source, span.start);
    let sev = match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARNING",
        Severity::Note => "NOTE",
    };
    let mut out = format!("{sev} [{code}]: {message}\n  --> {file}:{line}:{col}\n");
    let lines: Vec<&str> = source.lines().collect();
    if line > 0 && line <= lines.len() {
        let src_line = lines[line - 1];
        out.push_str(&format!("   |\n{line:>3} | {src_line}\n   | "));
        let pad = col.saturating_sub(1);
        for _ in 0..pad {
            out.push(' ');
        }
        let highlight_len = span.len().max(1) as usize;
        for _ in 0..highlight_len.min(src_line.len().saturating_sub(pad)) {
            out.push('^');
        }
        out.push('\n');
    }
    for note in notes {
        if let Some(rest) = note.strip_prefix("help:") {
            out.push_str(&format!("   = help:{rest}\n"));
        } else if let Some(rest) = note.strip_prefix("note:") {
            out.push_str(&format!("   = note:{rest}\n"));
        } else {
            out.push_str(&format!("   = note: {note}\n"));
        }
    }
    out
}

pub fn format_ownership_contradiction(
    source: &str,
    name: &str,
    inferred: &str,
    annotated: &str,
    span: Span,
) -> FormattedDiagnostic {
    format_diagnostic(
        source,
        "E0050",
        &format!(
            "ownership contradicts annotation on `{name}`: inferred `{inferred}`, annotated `{annotated}`"
        ),
        span,
        Severity::Error,
        &[format!(
            "either drop the `{annotated}` annotation or clone before the move"
        )],
    )
}

pub fn format_type_mismatch(
    source: &str,
    expected: &str,
    found: &str,
    span: Span,
) -> FormattedDiagnostic {
    format_diagnostic(
        source,
        "E0041",
        &format!("type mismatch: expected `{expected}`, found `{found}`"),
        span,
        Severity::Error,
        &[],
    )
}
