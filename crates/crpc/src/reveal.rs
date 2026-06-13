use clap::{Parser, Subcommand};
use crisp_reveal::{
    reveal_diff, reveal_errors, reveal_expand, reveal_lifetimes, reveal_map, reveal_ownership,
    reveal_rust, reveal_seal, reveal_traits, reveal_types,
};
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
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Rust {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Seal {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Expand {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Diff {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Map {
        #[arg(default_value = ".")]
        path: PathBuf,
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
        Commands::Traits { path } => {
            println!("{}", reveal_traits(&path)?);
            Ok(())
        }
        Commands::Rust { path } => {
            let root = crisp_resolve::find_crate_root(&path).unwrap_or(path);
            println!("{}", reveal_rust(&root)?);
            Ok(())
        }
        Commands::Seal { path } => {
            println!("{}", reveal_seal(&path)?);
            Ok(())
        }
        Commands::Expand { path } => {
            println!("{}", reveal_expand(&path)?);
            Ok(())
        }
        Commands::Diff { path } => {
            println!("{}", reveal_diff(&path)?);
            Ok(())
        }
        Commands::Map { path } => {
            println!("{}", reveal_map(&path)?);
            Ok(())
        }
    }
}
