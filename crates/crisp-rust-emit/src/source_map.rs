use crisp_ast::Span;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct EmitSourceMap {
    entries: BTreeMap<u32, Span>,
    /// Type/enum/alias/trait name → defining module path (`fail.a`). Used to emit `crate::` (#100).
    pub(crate) type_modules: BTreeMap<String, String>,
    /// Spans of `extern rust` declarations; rustc failure here is E0090, not a crisp bug (#116).
    pub extern_rust_spans: HashSet<Span>,
    rust_extern_by_key: BTreeMap<(String, String), Span>,
    /// Locals / params that are `IndexAssign` bases in the function being emitted (#141).
    pub(crate) index_mut_names: HashSet<String>,
}

impl EmitSourceMap {
    pub fn record(&mut self, rust_offset: u32, span: Span) {
        self.entries.insert(rust_offset, span);
    }

    pub fn lookup_line(&self, line: u32, source: &str) -> Option<Span> {
        let lines: Vec<&str> = source.lines().collect();
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

    pub fn is_extern_rust_span(&self, span: Span) -> bool {
        self.extern_rust_spans.contains(&span)
    }

    pub(crate) fn set_type_modules(&mut self, type_modules: BTreeMap<String, String>) {
        self.type_modules = type_modules;
    }

    pub(crate) fn set_rust_extern_spans(&mut self, spans: BTreeMap<(String, String), Span>) {
        self.extern_rust_spans = spans.values().copied().collect();
        self.rust_extern_by_key = spans;
    }

    pub(crate) fn rust_extern_span(&self, crate_name: &str, item: &str) -> Option<Span> {
        self.rust_extern_by_key
            .get(&(crate_name.to_string(), item.to_string()))
            .copied()
    }
}
