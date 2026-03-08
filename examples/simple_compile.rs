//! Simple compilation example

use jsmeld::{Compiler, CompileOptions};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("jsmeld - Simple Compilation Example\n");

    // Create a compiler instance
    let compiler = Compiler::new();

    // Example JavaScript code
    let source = r#"
// Simple greeting function
function greet(name) {
    console.log(`Hello, ${name}!`);
}

// Call the function
greet("World");
    "#;

    println!("Source code:");
    println!("---");
    println!("{}", source);
    println!("---\n");

    // Compile with default options
    println!("Compiling with default options...");
    let options = CompileOptions::default();
    let result = compiler.compile(source, "example.js", options)?;
    println!("Result:");
    println!("{}\n", result);

    // Compile with ES5 target
    println!("Compiling with ES5 target...");
    let options = CompileOptions {
        target: swc_ecma_ast::EsVersion::Es5,
        minify: false,
        source_map: false,
        ..Default::default()
    };
    let result = compiler.compile(source, "example.js", options)?;
    println!("Result:");
    println!("{}\n", result);

    // Compile with minification
    println!("Compiling with minification...");
    let options = CompileOptions {
        target: swc_ecma_ast::EsVersion::Es2020,
        minify: true,
        ..Default::default()
    };
    let result = compiler.compile(source, "example.js", options)?;
    println!("Result:");
    println!("{}\n", result);

    Ok(())
}
