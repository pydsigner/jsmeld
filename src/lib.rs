//! jsmeld - A Rust wrapper around SWC for JavaScript/TypeScript compilation and bundling
//!
//! This crate provides high-level APIs for compiling and bundling JavaScript/TypeScript code
//! using the SWC compiler.
//!
//! # Features
//!
//! - **Compilation**: Transform JavaScript/TypeScript files with configurable transforms
//! - **Bundling**: Bundle multiple modules into optimized output files

pub mod compiler;
pub mod bundler;
pub mod config;
pub mod errors;
pub mod util;
#[cfg(feature = "extension-module")]
pub mod pybindings;

pub use compiler::{compile, Compiler};
pub use bundler::{bundle, Bundler};
pub use config::JSMeldOptions;
pub use errors::{JSMeldError, JSMeldResult};
