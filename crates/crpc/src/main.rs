use clap::{Parser, Subcommand};
use crisp_parser::Parser as CrispParser;
use crisp_resolve::{Resolver, find_crate_root};
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
            eprintln!("crpc check: ok ({})", root.display());
            Ok(())
        }
        Commands::Build { path } => {
            eprintln!("crpc build: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Run { path } => {
            eprintln!("crpc run: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Test { path } => {
            eprintln!("crpc test: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Emit { path } => {
            eprintln!("crpc emit: not yet implemented (path: {path})");
            std::process::exit(1);
        }
    }
}
