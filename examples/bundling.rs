//! Bundling example

use jsmeld::{Bundler, BundleOptions};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("jsmeld - Bundling Example\n");

    let mut bundler = Bundler::new();

    // Create bundling options
    let options = BundleOptions {
        entry: "src/index.js".to_string(),
        output: "dist/bundle.js".to_string(),
        target: swc_ecma_ast::EsVersion::Es2020,
        minify: true,
        source_map: true,
        code_split: false,
        externals: vec![
            "react".to_string(),
            "react-dom".to_string(),
        ],
    };

    println!("Bundler configuration:");
    println!("  Entry: {}", options.entry);
    println!("  Output: {}", options.output);
    println!("  Target: {:?}", options.target);
    println!("  Minify: {}", options.minify);
    println!("  Source Maps: {}", options.source_map);
    println!("  Code Split: {}", options.code_split);
    println!("  Externals: {:?}\n", options.externals);

    // Configure the bundler
    bundler.set_options(options.clone());

    // Add additional external
    bundler.add_external("lodash".to_string());

    if let Some(opts) = bundler.options() {
        println!("Updated external dependencies: {:?}\n", opts.externals);
    }

    println!("Ready to bundle!");
    println!("Note: This is a demonstration. Actual bundling requires real entry files.\n");

    Ok(())
}
