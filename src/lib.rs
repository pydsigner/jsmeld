//! jsmeld - A Rust wrapper around SWC for JavaScript/TypeScript compilation and bundling
//!
//! This crate provides high-level APIs for compiling and bundling JavaScript/TypeScript code
//! using the SWC compiler.
//!
//! # Features
//!
//! - **Compilation**: Transform JavaScript/TypeScript files with configurable transforms
//! - **Bundling**: Bundle multiple modules into optimized output files

use pyo3::prelude::*;

pub mod compiler;
pub mod bundler;
pub mod config;
pub mod errors;
mod util;

pub use compiler::{compile, Compiler};
pub use bundler::{bundle, Bundler};
pub use config::{CompileOptions, BundleOptions};
pub use errors::{JSMeldError, JSMeldResult};

/// Python bindings for the jsmeld library.
#[pymodule]
mod jsmeld {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::bundle;
    #[pymodule_export]
    use super::compile;
}
