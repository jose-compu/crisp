use crisp_resolve::Resolver;
use std::path::PathBuf;

fn server_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/server")
}

fn normalize_paths(mut resolved: crisp_resolve::ResolvedCrate) -> crisp_resolve::ResolvedCrate {
    resolved.crate_root = ".".to_string();
    for module in &mut resolved.modules {
        if let Some(name) = module.file.rsplit("/src/").next() {
            module.file = format!("./src/{name}");
        }
    }
    resolved
}

#[test]
fn resolve_server_multifile() {
    let root = server_root();
    let resolved = normalize_paths(Resolver::resolve_crate(&root).expect("resolve server example"));
    assert_eq!(resolved.modules.len(), 3);
    insta::assert_debug_snapshot!(resolved);
}

#[test]
fn resolve_server_main_imports_config_and_greet() {
    let root = server_root();
    let resolved = Resolver::resolve_crate(&root).expect("resolve server");
    let main = resolved
        .modules
        .iter()
        .find(|m| m.module_path == "main")
        .expect("main module");
    assert!(main.scope.iter().any(|s| s == "Config"));
    assert!(main.scope.iter().any(|s| s == "greet"));
    assert!(main.scope.iter().any(|s| s == "log"));
}
