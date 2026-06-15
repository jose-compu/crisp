use crisp_ast::Span;
use std::collections::BTreeMap;

/// Maps emitted Rust byte offsets to originating Crisp spans (spec §17.3).
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    entries: BTreeMap<u32, Span>,
}

impl SourceMap {
    pub fn insert(&mut self, rust_offset: u32, span: Span) {
        self.entries.insert(rust_offset, span);
    }

    pub fn lookup_rust_line(&self, line: u32, lines: &[&str]) -> Option<Span> {
        let mut offset = 0u32;
        for (i, l) in lines.iter().enumerate() {
            if (i as u32 + 1) == line {
                return self.entries.range(..=offset).next_back().map(|(_, s)| *s);
            }
            offset += l.len() as u32 + 1;
        }
        self.entries.values().next().copied()
    }

    pub fn entries(&self) -> &BTreeMap<u32, Span> {
        &self.entries
    }
}
