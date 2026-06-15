//! Full-crate analysis cache for editor queries.

use crate::hints::InlayHint;
use crate::hints::inlay_hints_for_file;
use crate::hover::{HoverInfo, hover_at_offset};
use crate::lenses::CodeLens;
use crate::lenses::code_lenses_for_file;
use crate::overlays::CallOverlay;
use crate::overlays::call_overlays_for_file;
use crisp_errors::ErrorResult;
use crisp_ownership::OwnershipResult;
use crisp_regions::RegionResult;
use crisp_resolve::module::{ModuleGraph, load_module_graph};
use crisp_resolve::{Resolver, find_crate_root};
use crisp_rust_emit::resolve_rustc_fallbacks;
use crisp_typeck::{TypeChecker, TypedCrate};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("[E0074] resolve: {0}")]
    Resolve(#[from] crisp_resolve::ResolveError),
    #[error("[E0071] type: {0}")]
    Type(#[from] crisp_typeck::TypeError),
    #[error("[E0070] ownership: {0}")]
    Ownership(#[from] crisp_ownership::OwnershipError),
    #[error("[E0072] region: {0}")]
    Region(#[from] crisp_regions::RegionError),
    #[error("[E0073] errors: {0}")]
    Errors(#[from] crisp_errors::ErrorPassError),
    #[error("no crisp.toml found from {0}")]
    NoCrateRoot(PathBuf),
    #[error("file not in crate: {0}")]
    UnknownFile(PathBuf),
}

#[derive(Debug, Clone)]
pub struct CrispAnalysis {
    pub crate_root: PathBuf,
    pub graph: ModuleGraph,
    pub typed: TypedCrate,
    pub ownership: OwnershipResult,
    pub regions: RegionResult,
    pub errors: ErrorResult,
}

impl CrispAnalysis {
    pub fn analyze(path: &Path) -> Result<Self, AnalysisError> {
        let crate_root =
            find_crate_root(path).ok_or_else(|| AnalysisError::NoCrateRoot(path.to_path_buf()))?;
        Resolver::resolve_crate(&crate_root)?;
        let graph = load_module_graph(&crate_root)?;
        let typed = TypeChecker::check_crate(&crate_root)?;
        let ownership = match resolve_rustc_fallbacks(&crate_root) {
            Ok(o) => o,
            Err(crisp_rust_emit::FallbackResolveError::RustcUnavailable) => {
                crisp_ownership::OwnershipPass::analyze_crate(&crate_root)?
            }
            Err(crisp_rust_emit::FallbackResolveError::Ownership(e)) => return Err(e.into()),
            Err(crisp_rust_emit::FallbackResolveError::Type(e)) => return Err(e.into()),
            Err(crisp_rust_emit::FallbackResolveError::Resolve(e)) => return Err(e.into()),
            Err(crisp_rust_emit::FallbackResolveError::Exhausted { .. }) => {
                crisp_ownership::OwnershipPass::analyze_crate(&crate_root)?
            }
        };
        let regions = crisp_regions::RegionPass::assign_crate(&crate_root)?;
        let errors = crisp_errors::ErrorPass::analyze_crate(&crate_root)?;
        Ok(Self {
            crate_root,
            graph,
            typed,
            ownership,
            regions,
            errors,
        })
    }

    pub fn module_for_file(&self, file: &Path) -> Result<&str, AnalysisError> {
        let abs = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
        for node in self.graph.modules.values() {
            let node_abs = node
                .path
                .canonicalize()
                .unwrap_or_else(|_| node.path.clone());
            if node_abs == abs {
                return Ok(&node.module_path);
            }
        }
        Err(AnalysisError::UnknownFile(file.to_path_buf()))
    }

    pub fn source_file(&self, file: &Path) -> Result<&crisp_ast::item::SourceFile, AnalysisError> {
        let module = self.module_for_file(file)?;
        Ok(&self.graph.modules.get(module).expect("module exists").ast)
    }

    pub fn hover(&self, file: &Path, offset: u32) -> Result<Option<HoverInfo>, AnalysisError> {
        let module = self.module_for_file(file)?;
        let ast = self.source_file(file)?;
        Ok(hover_at_offset(
            ast,
            module,
            offset,
            &self.typed,
            &self.ownership,
            &self.regions,
            &self.errors,
        ))
    }

    pub fn inlay_hints(&self, file: &Path) -> Result<Vec<InlayHint>, AnalysisError> {
        let module = self.module_for_file(file)?;
        let ast = self.source_file(file)?;
        Ok(inlay_hints_for_file(
            ast,
            module,
            &self.typed,
            &self.ownership,
        ))
    }

    pub fn call_overlays(&self, file: &Path) -> Result<Vec<CallOverlay>, AnalysisError> {
        let module = self.module_for_file(file)?;
        let ast = self.source_file(file)?;
        Ok(call_overlays_for_file(
            ast,
            module,
            &self.errors,
            &self.typed,
        ))
    }

    pub fn code_lenses(&self, file: &Path) -> Result<Vec<CodeLens>, AnalysisError> {
        let ast = self.source_file(file)?;
        Ok(code_lenses_for_file(ast, &self.crate_root))
    }

    pub fn emitted_rust(&self) -> anyhow::Result<String> {
        crisp_reveal::reveal_rust(&self.crate_root)
    }
}
