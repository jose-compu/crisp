use crate::error::ResolveError;
use crisp_ast::generics::{apply_implicit_generics, defined_type_names, prelude_type_set};
use crisp_ast::item::SourceFile;
use crisp_parser::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub path: PathBuf,
    pub module_path: String,
    pub ast: SourceFile,
}

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    pub crate_root: PathBuf,
    pub src_root: PathBuf,
    pub modules: BTreeMap<String, ModuleNode>,
}

pub fn find_crate_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        if dir.join("crisp.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn load_module_graph(crate_root: &Path) -> Result<ModuleGraph, ResolveError> {
    let src_root = crate_root.join("src");
    if !src_root.is_dir() {
        return Err(ResolveError::NoSrcDir {
            root: crate_root.display().to_string(),
        });
    }

    let mut modules = BTreeMap::new();
    collect_crp_files(&src_root, &src_root, &mut modules)?;

    if modules.is_empty() {
        return Err(ResolveError::NoSrcDir {
            root: crate_root.display().to_string(),
        });
    }

    apply_free_type_binders(&mut modules)?;

    Ok(ModuleGraph {
        crate_root: crate_root.to_path_buf(),
        src_root,
        modules,
    })
}

/// Unbound type names become item generics (#75). Explicit `<T>` that shadows a
/// known type is E0049 (#78).
fn apply_free_type_binders(modules: &mut BTreeMap<String, ModuleNode>) -> Result<(), ResolveError> {
    let mut known = prelude_type_set();
    for node in modules.values() {
        known.extend(defined_type_names(&node.ast.items));
    }
    for node in modules.values_mut() {
        apply_implicit_generics(&mut node.ast.items, &known).map_err(|shadow| {
            ResolveError::GenericShadowsType {
                name: shadow.name,
                span: shadow.span,
            }
        })?;
    }
    Ok(())
}

fn collect_crp_files(
    src_root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, ModuleNode>,
) -> Result<(), ResolveError> {
    for entry in fs::read_dir(dir).map_err(|e| ResolveError::Io {
        path: dir.display().to_string(),
        source: e,
    })? {
        let entry = entry.map_err(|e| ResolveError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_crp_files(src_root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("crp") {
            continue;
        }
        let rel = path.strip_prefix(src_root).map_err(|_| ResolveError::Io {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad path"),
        })?;
        let module_path = rel
            .with_extension("")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, ".");
        let source = fs::read_to_string(&path).map_err(|e| ResolveError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let mut parser = Parser::new(&source).map_err(|e| ResolveError::Parse {
            path: path.display().to_string(),
            message: e.primary_message(),
            pos: e.byte_pos(),
        })?;
        let ast = parser.parse_file().map_err(|e| ResolveError::Parse {
            path: path.display().to_string(),
            message: e.primary_message(),
            pos: e.byte_pos(),
        })?;
        out.insert(
            module_path.clone(),
            ModuleNode {
                path,
                module_path,
                ast,
            },
        );
    }
    Ok(())
}
