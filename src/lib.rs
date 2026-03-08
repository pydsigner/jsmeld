//! jsmeld - A Rust wrapper around SWC for JavaScript/TypeScript compilation and bundling
//!
//! This crate provides high-level APIs for compiling and bundling JavaScript/TypeScript code
//! using the SWC compiler.
//!
//! # Features
//!
//! - **Compilation**: Transform JavaScript/TypeScript files with configurable transforms
//! - **Bundling**: Bundle multiple modules into optimized output files
//! - **Code Generation**: Generate optimized JavaScript/TypeScript code
//! - **Error Handling**: Comprehensive error reporting

pub mod compiler;
pub mod bundler;
pub mod config;
pub mod errors;

pub use compiler::Compiler;
pub use bundler::Bundler;
pub use config::{CompileOptions, BundleOptions};
pub use errors::{JsmeldError, JsmeldResult};

/// Version of jsmeld
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
