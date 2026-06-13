use clap::{Parser, Subcommand};
use crisp_reveal::reveal_types;
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
        file: String,
    },
    Lifetimes {
        file: String,
    },
    Errors {
        file: String,
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
        Commands::Ownership { .. }
        | Commands::Lifetimes { .. }
        | Commands::Errors { .. }
        | Commands::Traits { .. }
        | Commands::Rust { .. }
        | Commands::Seal { .. }
        | Commands::Expand { .. }
        | Commands::Diff { .. }
        | Commands::Map { .. } => {
            eprintln!("reveal: not yet implemented");
            std::process::exit(1);
        }
    }
}
