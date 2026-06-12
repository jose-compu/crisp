use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crispc", version, about = "Crisp transpiler — .crp to Rust to native")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Analyze + emit; typecheck emitted Rust via cargo check (no codegen)
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
        Commands::Build { path } => {
            eprintln!("crispc build: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Run { path } => {
            eprintln!("crispc run: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Test { path } => {
            eprintln!("crispc test: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Check { path } => {
            eprintln!("crispc check: not yet implemented (path: {path})");
            std::process::exit(1);
        }
        Commands::Emit { path } => {
            eprintln!("crispc emit: not yet implemented (path: {path})");
            std::process::exit(1);
        }
    }
}
