use clap::{Parser, Subcommand};
use crisp_diagnostics::{Severity, format_diagnostic_at, format_unresolved_name};
use crisp_errors::ErrorPass;
use crisp_ownership::OwnershipPass;
use crisp_parser::Parser as CrispParser;
use crisp_regions::RegionPass;
use crisp_resolve::module::load_module_graph;
use crisp_resolve::{ResolveError, Resolver, find_crate_root};
use crisp_rust_emit::{
    PipelineError, TestHarnessError, build_emitted, emit_to_target, resolve_rustc_fallbacks,
    run_emitted, run_tests, verify_sealed_api,
};
use crisp_typeck::{TypeChecker, TypeError};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "crisp",
    version,
    about = "Crisp language toolchain — .crp to Rust to native"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse .crp source and print AST (debug)
    Parse { file: PathBuf },
    /// Resolve modules, imports, and names for a crate
    Resolve {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Analyze, emit Rust, invoke rustc
    Build {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Build and run
    Run {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Run tests
    Test {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Resolve + typecheck (fast)
    Check {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Emit Rust to target/rust/ and stop
    Emit {
        #[arg(default_value = ".")]
        path: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Parse { file } => {
            let src = fs::read_to_string(&file)?;
            let mut parser = CrispParser::new(&src)?;
            let module = parser.parse_file()?;
            println!("{module:#?}");
            Ok(())
        }
        Commands::Resolve { path } => {
            let root = find_crate_root(&path).unwrap_or(path);
            let resolved = Resolver::resolve_crate(&root)?;
            print_resolve_warnings(&resolved.warnings);
            println!("{resolved:#?}");
            Ok(())
        }
        Commands::Check { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            match Resolver::resolve_crate(&root) {
                Err(e) => {
                    print_resolve_diagnostic(&root, &e);
                    return Err(e.into());
                }
                Ok(resolved) => print_resolve_warnings(&resolved.warnings),
            }
            if let Err(e) = TypeChecker::check_crate(&root) {
                print_type_diagnostic(&root, &e);
                return Err(e.into());
            }
            match resolve_rustc_fallbacks(&root) {
                Ok(_) => {}
                Err(crisp_rust_emit::FallbackResolveError::RustcUnavailable) => {
                    OwnershipPass::analyze_crate(&root)?;
                }
                Err(e) => return Err(e.into()),
            }
            RegionPass::assign_crate(&root)?;
            ErrorPass::analyze_crate(&root)?;
            verify_sealed_api(&root)?;
            eprintln!("crisp check: ok ({})", root.display());
            Ok(())
        }
        Commands::Build { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            match build_emitted(&root) {
                Ok(out_dir) => {
                    eprintln!("crisp build: ok ({})", out_dir.display());
                    Ok(())
                }
                Err(PipelineError::ToolchainUnavailable) => {
                    eprintln!("crisp build: emitted to target/rust/ (cargo not on PATH)");
                    emit_to_target(&root)?;
                    std::process::exit(1);
                }
                Err(e) => Err(e.into()),
            }
        }
        Commands::Run { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            match run_emitted(&root) {
                Ok(stdout) => {
                    print!("{stdout}");
                    Ok(())
                }
                Err(PipelineError::ToolchainUnavailable) => {
                    eprintln!("crisp run: cargo not on PATH");
                    std::process::exit(1);
                }
                Err(e) => Err(e.into()),
            }
        }
        Commands::Test { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            match run_tests(&root) {
                Ok(report) => {
                    eprintln!(
                        "crisp test: ok ({} runtime, {} compile-fail)",
                        report.runtime_passed, report.compile_fail_passed
                    );
                    Ok(())
                }
                Err(TestHarnessError::Other(e)) if e.to_string().contains("cargo not on PATH") => {
                    eprintln!("crisp test: cargo not on PATH");
                    std::process::exit(1);
                }
                Err(e) => Err(e.into()),
            }
        }
        Commands::Emit { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            let out = emit_to_target(&root)?;
            eprintln!("crisp emit: ok ({})", out.out_dir.display());
            Ok(())
        }
    }
}

fn print_resolve_warnings(warnings: &[crisp_resolve::ResolveWarning]) {
    for w in warnings {
        eprintln!("warning: {w}");
    }
}

fn print_resolve_diagnostic(root: &Path, err: &ResolveError) {
    match err {
        ResolveError::UnresolvedName {
            name, span, hint, ..
        } => {
            if let Some((file, source)) = source_for_span(root, *span) {
                let rendered =
                    format_unresolved_name(&file, &source, name, *span, hint.as_deref()).rendered;
                eprintln!("{rendered}");
                return;
            }
        }
        ResolveError::ShapesUnsupported { name, span } => {
            if let Some((file, source)) = source_for_span(root, *span) {
                let rendered = format_diagnostic_at(
                    &file,
                    &source,
                    "E0039",
                    &format!("shapes are not yet supported (`{name}`)"),
                    *span,
                    Severity::Error,
                    &["help: remove the `shape` definition or bound".into()],
                )
                .rendered;
                eprintln!("{rendered}");
                return;
            }
        }
        _ => {}
    }
    eprintln!("{err}");
}

fn print_type_diagnostic(root: &Path, err: &TypeError) {
    match err {
        TypeError::UnknownName { name, span } | TypeError::UnknownType { name, span } => {
            if let Some((file, source)) = source_for_span(root, *span) {
                let code = if matches!(err, TypeError::UnknownType { .. }) {
                    "E0040"
                } else {
                    "E0041"
                };
                let rendered = format_diagnostic_at(
                    &file,
                    &source,
                    code,
                    &err.to_string().replacen(&format!("[{code}] "), "", 1),
                    *span,
                    Severity::Error,
                    &[],
                )
                .rendered;
                eprintln!("{rendered}");
                let _ = name;
                return;
            }
        }
        TypeError::AmbiguousField {
            field,
            candidates,
            span,
        } => {
            if let Some((file, source)) = source_for_span(root, *span) {
                let rendered = format_diagnostic_at(
                    &file,
                    &source,
                    "E0043",
                    &format!(
                        "ambiguous field `{field}` on unresolved type; annotate the parameter (candidates: {candidates})"
                    ),
                    *span,
                    Severity::Error,
                    &["help: write `param: StructName` on the function parameter".into()],
                )
                .rendered;
                eprintln!("{rendered}");
                return;
            }
        }
        TypeError::Resolve(inner) => {
            print_resolve_diagnostic(root, inner);
            return;
        }
        _ => {}
    }
    eprintln!("{err}");
}

/// Best-effort: find a module source whose length covers `span.end`.
fn source_for_span(root: &Path, span: crisp_ast::Span) -> Option<(String, String)> {
    let graph = load_module_graph(root).ok()?;
    let mut best: Option<(String, String)> = None;
    for node in graph.modules.values() {
        let Ok(source) = fs::read_to_string(&node.path) else {
            continue;
        };
        if (source.len() as u32) < span.end {
            continue;
        }
        let rel = node
            .path
            .strip_prefix(root)
            .unwrap_or(node.path.as_path())
            .display()
            .to_string();
        best = Some((rel, source));
        // Prefer main when multiple match.
        if node.module_path == "main" {
            break;
        }
    }
    best
}
