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
pub mod util;

pub use compiler::{compile, Compiler, py_compile};
pub use bundler::{bundle, Bundler, py_bundle};
pub use config::JSMeldOptions;
pub use errors::{JSMeldError, JSMeldResult};

/// Python bindings for the jsmeld library.
#[pymodule]
mod jsmeld {
    #[pymodule_export(name = "bundle")]
    use super::py_bundle;
    #[pymodule_export(name = "compile")]
    use super::py_compile;
    #[pymodule_export(name = "JSMeldError")]
    use crate::errors::PyJSMeldError;
}
