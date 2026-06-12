use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "reveal", version, about = "Inspect inferred Crisp precision")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Types { file: String },
    Ownership { file: String },
    Lifetimes { file: String },
    Errors { file: String },
    Traits { file: String },
    Rust { file: String },
    Seal { crate_name: String },
    Expand { file: String },
    Diff { file: String },
    Map { file: String },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let _ = match cli.command {
        Commands::Types { file } => file,
        Commands::Ownership { file } => file,
        Commands::Lifetimes { file } => file,
        Commands::Errors { file } => file,
        Commands::Traits { file } => file,
        Commands::Rust { file } => file,
        Commands::Seal { crate_name } => crate_name,
        Commands::Expand { file } => file,
        Commands::Diff { file } => file,
        Commands::Map { file } => file,
    };
    eprintln!("reveal: not yet implemented");
    std::process::exit(1);
}
