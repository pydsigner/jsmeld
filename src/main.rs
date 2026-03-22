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

    /// Extract bundled styles into a separate CSS file, defaulting to <output>.css.
    #[arg(long)]
    extract_styles: bool,

    /// Path to write extracted bundled styles. Implies --extract-styles.
    #[arg(long)]
    style_output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let entry = cli.input.to_string_lossy().into_owned();
    if cli.bundle || !cli.compile {
        tracing::info!("Bundling {} to {}", entry, cli.output.display());

        let style_output = if cli.extract_styles || cli.style_output.is_some() {
            let css_path = cli
                .style_output
                .clone()
                .unwrap_or_else(|| cli.output.with_extension("css"));
            Some(css_path.to_string_lossy().into_owned())
        } else {
            None
        };

        let bundled = bundle(
            entry.clone(),
            JSMeldOptions {
                target: cli.target.clone(),
                minify: cli.minify,
                style_output,
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
        tracing::info!("Wrote bundle to {}", cli.output.display());
    }
    else {
        tracing::info!("Compiling {} to {}", entry, cli.output.display());
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
        tracing::info!("Wrote compiled output to {}", cli.output.display());
    }
    Ok(())
}
