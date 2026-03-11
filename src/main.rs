use std::path::PathBuf;
use clap::Parser;

use jsmeld::{bundle, compile, JSMeldOptions};

/// Simple CLI for jsmeld: bundle and/or compile an input file to an output path
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input file (entry point)
    #[arg(short, long)]
    input: PathBuf,

    /// Bundle output path
    #[arg(short, long, default_value = None)]
    bundle: Option<PathBuf>,

    /// Compile output path
    #[arg(short, long, default_value = None)]
    compile: Option<PathBuf>,

    // Target JavaScript version (e.g., "es5", "es6", "es2020")
    #[arg(long, default_value = "es6")]
    target: String,

    /// Enable minification for bundling/compilation
    #[arg(short, long)]
    minify: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let mut entry = cli.input.to_string_lossy().into_owned();

    if let Some(bundle_path) = &cli.bundle {
        println!("Bundling {} to {}", cli.input.display(), bundle_path.display());
        let bundled = bundle(
            entry.clone(),
            JSMeldOptions {
                target: cli.target.clone(),
                minify: cli.minify,
                ..Default::default()
            },
        )?;
        // Ensure parent directory exists before writing
        if let Some(parent) = bundle_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(bundle_path, bundled)?;
        println!("Wrote bundle to {}", bundle_path.display());
        // Use bundled output as entry for compilation
        entry = bundle_path.to_string_lossy().into_owned();
    }

    if let Some(compile_path) = &cli.compile {
        println!("Compiling {} to {}", entry, compile_path.display());
        let compiled = compile(
            entry.clone(),
            JSMeldOptions {
                target: cli.target.clone(),
                minify: cli.minify,
                ..Default::default()
            },
        )?;
        // Ensure parent directory exists before writing
        if let Some(parent) = compile_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(compile_path, compiled)?;
        println!("Wrote compiled output to {}", compile_path.display());
    }

    Ok(())
}
