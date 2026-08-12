//! Stdio Language Server Protocol host for Crisp (#56).
//!
//! Run: `crisp-lsp` (stdio). Cursor/VS Code can spawn this binary for `.crp` files.

use crisp_lsp::{CrispAnalysis, InlayHintKind as CrispInlayKind};
use crisp_resolve::find_crate_root;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    /// Open document text (UTF-8).
    documents: Arc<Mutex<HashMap<Url, String>>>,
    /// Cached analysis keyed by crate root.
    analysis: Arc<Mutex<HashMap<PathBuf, CrispAnalysis>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "crisp-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "crisp-lsp ready (stdio)")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        {
            let mut docs = self.documents.lock().await;
            docs.insert(uri.clone(), text);
        }
        self.refresh_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let mut docs = self.documents.lock().await;
            docs.insert(params.text_document.uri.clone(), change.text);
        }
        // Full re-analyze on each change is acceptable for small Crisp crates.
        self.refresh_diagnostics(&params.text_document.uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.refresh_diagnostics(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.documents.lock().await;
        docs.remove(&params.text_document.uri);
        let _ = self
            .client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(path) = uri_to_path(&uri) else {
            return Ok(None);
        };
        let Some(analysis) = self.ensure_analysis(&path).await else {
            return Ok(None);
        };
        let src = {
            let docs = self.documents.lock().await;
            docs.get(&uri)
                .cloned()
                .unwrap_or_else(|| std::fs::read_to_string(&path).unwrap_or_default())
        };
        let offset = position_to_offset(&src, pos);
        match analysis.hover(&path, offset) {
            Ok(Some(info)) => Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("### {}\n\n{}", info.title, info.markdown),
                }),
                range: None,
            })),
            _ => Ok(None),
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let Some(path) = uri_to_path(&uri) else {
            return Ok(None);
        };
        let Some(analysis) = self.ensure_analysis(&path).await else {
            return Ok(None);
        };
        let src = {
            let docs = self.documents.lock().await;
            docs.get(&uri)
                .cloned()
                .unwrap_or_else(|| std::fs::read_to_string(&path).unwrap_or_default())
        };
        let Ok(hints) = analysis.inlay_hints(&path) else {
            return Ok(None);
        };
        let out: Vec<InlayHint> = hints
            .into_iter()
            .map(|h| {
                let kind = match h.kind {
                    CrispInlayKind::Type => Some(InlayHintKind::TYPE),
                    CrispInlayKind::Ownership => Some(InlayHintKind::PARAMETER),
                };
                InlayHint {
                    position: offset_to_position(&src, h.position),
                    label: InlayHintLabel::String(h.label),
                    kind,
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                }
            })
            .collect();
        Ok(Some(out))
    }
}

impl Backend {
    async fn ensure_analysis(&self, file: &Path) -> Option<CrispAnalysis> {
        let root = find_crate_root(file)?;
        {
            let cache = self.analysis.lock().await;
            if let Some(a) = cache.get(&root) {
                return Some(a.clone());
            }
        }
        match CrispAnalysis::analyze(&root) {
            Ok(a) => {
                let mut cache = self.analysis.lock().await;
                cache.insert(root, a.clone());
                Some(a)
            }
            Err(_) => None,
        }
    }

    async fn refresh_diagnostics(&self, uri: &Url) {
        let Some(path) = uri_to_path(uri) else {
            return;
        };
        // Drop cached analysis for this crate so disk/open buffer stay coherent on save.
        if let Some(root) = find_crate_root(&path) {
            let mut cache = self.analysis.lock().await;
            cache.remove(&root);
        }

        let diagnostics = match CrispAnalysis::analyze(&path) {
            Ok(a) => {
                let mut cache = self.analysis.lock().await;
                if let Some(root) = find_crate_root(&path) {
                    cache.insert(root, a);
                }
                Vec::new()
            }
            Err(e) => {
                let src = {
                    let docs = self.documents.lock().await;
                    docs.get(uri)
                        .cloned()
                        .unwrap_or_else(|| std::fs::read_to_string(&path).unwrap_or_default())
                };
                vec![Diagnostic {
                    range: Range {
                        start: Position::new(0, 0),
                        end: offset_to_position(&src, src.len().min(1) as u32),
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("crisp-lsp".into()),
                    message: e.to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                }]
            }
        };
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

fn position_to_offset(src: &str, pos: Position) -> u32 {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in src.char_indices() {
        if line == pos.line && col == pos.character {
            return i as u32;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    src.len() as u32
}

fn offset_to_position(src: &str, offset: u32) -> Position {
    let target = offset as usize;
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in src.char_indices() {
        if i >= target {
            return Position::new(line, col);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(Mutex::new(HashMap::new())),
        analysis: Arc::new(Mutex::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
