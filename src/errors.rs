//! Error types for jsmeld

use thiserror::Error;

/// Result type for jsmeld operations
pub type JSMeldResult<T> = Result<T, JSMeldError>;

/// Error type for jsmeld operations
#[derive(Error, Debug)]
pub enum JSMeldError {
    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Bundling error: {0}")]
    BundlingError(String),

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl From<&[swc_common::errors::Diagnostic]> for JSMeldError {
    fn from(diag: &[swc_common::errors::Diagnostic]) -> Self {
        // Convert the SWC error to our error type
        // Diagnostics contains error and diagnostics
        JSMeldError::CompilationError(format!("SWC error: {}", diag.iter().map(|d| d.message()).collect::<Vec<_>>().join("; ")))
    }
}

impl From<swc_ecma_parser::error::Error> for JSMeldError {
    fn from(err: swc_ecma_parser::error::Error) -> Self {
        JSMeldError::CompilationError(format!("SWC parsing error: {:#?}", err))
    }
}
