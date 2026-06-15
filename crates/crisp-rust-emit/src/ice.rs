//! Map rustc failures to Crisp spans (spec §17.3).

use crate::source_map::EmitSourceMap;
use crisp_ast::Span;

pub fn map_rustc_failure(stderr: &str, source: &str, map: &EmitSourceMap) -> Option<Span> {
    let line = parse_rustc_line(stderr)?;
    map.lookup_line(line, source)
}

fn parse_rustc_line(stderr: &str) -> Option<u32> {
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix(" --> ") {
            if let Some(loc) = rest.split(':').nth(1) {
                return loc.parse().ok();
            }
        }
        if let Some(idx) = line.find(".rs:") {
            let after = &line[idx + 3..];
            if let Some(colon) = after.find(':') {
                return after[..colon].parse().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rustc_line_number() {
        let stderr = "error[E0308]: mismatched types\n --> src/main.rs:14:26\n";
        assert_eq!(parse_rustc_line(stderr), Some(14));
    }
}
