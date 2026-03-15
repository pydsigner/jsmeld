use std::path::PathBuf;
use clap::Parser;

use jsmeld::{bundle, compile, JSMeldOptions};

/// Simple CLI for jsmeld: bundle or compile an input file to an output path
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input file (entry point)
    input: PathBuf,
    /// Output path
    output: PathBuf,

    /// Bundle and compile the input file
    #[arg(group = "action", short, long)]
    bundle: bool,

    /// Only compile the input file
    #[arg(group = "action", short, long)]
    compile: bool,

    // Target JavaScript version (e.g., "es5", "es6", "es2020")
    #[arg(long, default_value = "es6")]
    target: String,

    /// Enable minification of output
    #[arg(short, long)]
    minify: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let entry = cli.input.to_string_lossy().into_owned();
    if cli.bundle || !cli.compile {
        println!("Bundling {} to {}", entry, cli.output.display());
        let bundled = bundle(
            entry.clone(),
            JSMeldOptions {
                target: cli.target.clone(),
                minify: cli.minify,
                ..Default::default()
            },
        )?;
        // Ensure parent directory exists before writing
        if let Some(parent) = cli.output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(&cli.output, bundled)?;
        println!("Wrote bundle to {}", cli.output.display());
    }
    else {
        println!("Compiling {} to {}", entry, cli.output.display());
        let compiled = compile(
            entry.clone(),
            JSMeldOptions {
                target: cli.target.clone(),
                minify: cli.minify,
                ..Default::default()
            },
        )?;
        // Ensure parent directory exists before writing
        if let Some(parent) = cli.output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(&cli.output, compiled)?;
        println!("Wrote compiled output to {}", cli.output.display());
    }
    Ok(())
}
