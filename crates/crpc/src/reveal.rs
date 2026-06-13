use clap::{Parser, Subcommand};
use crisp_reveal::{reveal_errors, reveal_lifetimes, reveal_ownership, reveal_rust, reveal_types};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "reveal", version, about = "Inspect inferred Crisp precision")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Types {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Ownership {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Lifetimes {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Errors {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Traits {
        file: String,
    },
    Rust {
        file: String,
    },
    Seal {
        crate_name: String,
    },
    Expand {
        file: String,
    },
    Diff {
        file: String,
    },
    Map {
        file: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Types { path } => {
            println!("{}", reveal_types(&path)?);
            Ok(())
        }
        Commands::Ownership { path } => {
            println!("{}", reveal_ownership(&path)?);
            Ok(())
        }
        Commands::Lifetimes { path } => {
            println!("{}", reveal_lifetimes(&path)?);
            Ok(())
        }
        Commands::Errors { path } => {
            println!("{}", reveal_errors(&path)?);
            Ok(())
        }
        Commands::Rust { file } => {
            let path = PathBuf::from(&file);
            let root = crisp_resolve::find_crate_root(&path).unwrap_or(path);
            println!("{}", reveal_rust(&root)?);
            Ok(())
        }
        Commands::Traits { .. }
        | Commands::Seal { .. }
        | Commands::Expand { .. }
        | Commands::Diff { .. }
        | Commands::Map { .. } => {
            eprintln!("reveal: not yet implemented");
            std::process::exit(1);
        }
    }
}
