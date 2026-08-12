//! `reveal` CLI — inspect inferred Crisp precision (spec §16).
//!
//! Built as a second binary of the `crpc` package:
//! `cargo build -p crpc` → `target/debug/reveal`.

use clap::{Parser, Subcommand};
use crisp_reveal::{
    reveal_diff, reveal_errors, reveal_expand, reveal_lifetimes, reveal_map, reveal_ownership,
    reveal_rust, reveal_seal, reveal_traits, reveal_types,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "reveal",
    version,
    about = "Inspect inferred Crisp precision (spec §16)",
    long_about = "Inspect what the Crisp compiler inferred (spec §16).\n\n\
                  `crpc` builds and runs your project. `reveal` explains the hidden precision:\n\
                  types, ownership (`&` / `&mut`), lifetimes, error sets, traits, and emitted Rust.\n\n\
                  Install (ships both `crpc` and `reveal`):\n\
                  \x20 cargo install --path crates/crpc --locked\n\n\
                  Start here:\n\
                  \x20 reveal types examples/hello\n\
                  \x20 reveal rust examples/hello\n\n\
                  Deep overlays: `types`, `ownership`, `lifetimes`, `errors`, `rust`, `seal`, `traits`.\n\
                  Shallower today: `expand`, `diff`, `map` — see QUICKSTART §10 and\
                  \ndocs/KNOWN_LIMITATIONS.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inferred type signatures for functions in the crate
    #[command(long_about = "Print inferred signatures (params, return, error sets). Spec §16.1.")]
    Types {
        /// Crate root or path under a crisp.toml (default: .)
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Borrow / move / copy modes and §7.6 rustc fallbacks
    #[command(long_about = "Print ownership modes per parameter and local. Spec §16.1.")]
    Ownership {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Emitted lifetime parameters
    #[command(long_about = "Print lifetime assignments from the region pass. Spec §16.1.")]
    Lifetimes {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Per-function reachable CrispError variant sets
    #[command(long_about = "Print ambient `!` error sets as CrispError variants. Spec §16.1.")]
    Errors {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// User traits / impl Trait for + shape trait summary
    #[command(
        long_about = "List user `trait` / `impl Trait for` from CIR, plus any shape traits.\n\
                      Try: `reveal traits examples/show_trait`.\n\
                      Shapes still fail resolve with E0039 (#21)."
    )]
    Traits {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Emitted Rust for the crate (main / lib entry)
    #[command(
        long_about = "Emit via the normal pipeline and print the primary Rust entry file.\n\
                      Spec §16.1 `reveal rust`."
    )]
    Rust {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Frozen sealed-crate API (crisp.lock or computed)
    #[command(long_about = "Show the sealed pub API (§12.5). Spec §16.1 `reveal seal`.")]
    Seal {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Annotated Crisp outline (shallow stubs for bodies)
    #[command(
        long_about = "Print signatures plus a shallow body outline (`<inferred>` / `<body>`).\n\
                      Not a full annotated source rewrite yet — see KNOWN_LIMITATIONS."
    )]
    Expand {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Summary of Crisp fn names vs emitted Rust (not a full side-by-side)
    #[command(
        long_about = "Compare function names present in Crisp vs emitted Rust.\n\
                      Spec §16.1 asks for a true side-by-side; this is a name-level summary today."
    )]
    Diff {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    /// Alloc / drop notes against CIR (generic, not span-accurate)
    #[command(
        long_about = "Annotate alloc/drop-related CIR notes. Spec §16.1 wants span-accurate\
                      \nmap against emitted Rust; current output is a coarser summary."
    )]
    Map {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Types { path } => run("types", &path, reveal_types),
        Commands::Ownership { path } => run("ownership", &path, reveal_ownership),
        Commands::Lifetimes { path } => run("lifetimes", &path, reveal_lifetimes),
        Commands::Errors { path } => run("errors", &path, reveal_errors),
        Commands::Traits { path } => run("traits", &path, reveal_traits),
        Commands::Rust { path } => {
            let root = crisp_resolve::find_crate_root(&path).unwrap_or(path);
            run("rust", &root, reveal_rust)
        }
        Commands::Seal { path } => run("seal", &path, reveal_seal),
        Commands::Expand { path } => run("expand", &path, reveal_expand),
        Commands::Diff { path } => run("diff", &path, reveal_diff),
        Commands::Map { path } => run("map", &path, reveal_map),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("reveal: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    cmd: &str,
    path: &std::path::Path,
    f: impl FnOnce(&std::path::Path) -> anyhow::Result<String>,
) -> anyhow::Result<()> {
    let display = path.display();
    let out = f(path).map_err(|e| {
        anyhow::anyhow!(
            "{cmd} failed for `{display}`: {e}\n\
             hint: pass a crate root (directory with crisp.toml) or a path inside one; \
             try `reveal {cmd} examples/hello`"
        )
    })?;
    println!("{out}");
    Ok(())
}
