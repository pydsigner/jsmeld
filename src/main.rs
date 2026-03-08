use jsmeld::{Compiler, CompileOptions};

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a compiler instance
    let compiler = Compiler::new();

    // Example: Compile a simple JavaScript snippet
    let source = r#"
        const greeting = "Hello, World!";
        console.log(greeting);
    "#;

    let options = CompileOptions::default();
    let result = compiler.compile(source, "example.js", options)?;

    println!("Compiled output:\n{}", result);

    Ok(())
}
