use clap::{Parser, Subcommand};
use crisp_parser::Parser as CrispParser;
use crisp_resolve::{Resolver, find_crate_root};
use crisp_errors::ErrorPass;
use crisp_ownership::OwnershipPass;
use crisp_regions::RegionPass;
use crisp_rust_emit::{build_emitted, emit_to_target, resolve_rustc_fallbacks, run_emitted, run_tests, verify_sealed_api, PipelineError, TestHarnessError};
use crisp_typeck::TypeChecker;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "crpc",
    version,
    about = "Crisp transpiler — .crp to Rust to native"
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
            println!("{resolved:#?}");
            Ok(())
        }
        Commands::Check { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            Resolver::resolve_crate(&root)?;
            TypeChecker::check_crate(&root)?;
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
            eprintln!("crpc check: ok ({})", root.display());
            Ok(())
        }
        Commands::Build { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            match build_emitted(&root) {
                Ok(out_dir) => {
                    eprintln!("crpc build: ok ({})", out_dir.display());
                    Ok(())
                }
                Err(PipelineError::ToolchainUnavailable) => {
                    eprintln!("crpc build: emitted to target/rust/ (cargo not on PATH)");
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
                    eprintln!("crpc run: cargo not on PATH");
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
                        "crpc test: ok ({} runtime, {} compile-fail)",
                        report.runtime_passed, report.compile_fail_passed
                    );
                    Ok(())
                }
                Err(TestHarnessError::Other(e)) if e.to_string().contains("cargo not on PATH") => {
                    eprintln!("crpc test: cargo not on PATH");
                    std::process::exit(1);
                }
                Err(e) => Err(e.into()),
            }
        }
        Commands::Emit { path } => {
            let root = find_crate_root(PathBuf::from(&path).as_path())
                .unwrap_or_else(|| PathBuf::from(&path));
            let out = emit_to_target(&root)?;
            eprintln!("crpc emit: ok ({})", out.out_dir.display());
            Ok(())
        }
    }
}
