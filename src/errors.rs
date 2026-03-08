//! Error types for jsmeld

use thiserror::Error;

/// Result type for jsmeld operations
pub type JsmeldResult<T> = Result<T, JsmeldError>;

/// Error type for jsmeld operations
#[derive(Error, Debug)]
pub enum JsmeldError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Bundling error: {0}")]
    BundlingError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Transform error: {0}")]
    TransformError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<&[swc_common::errors::Diagnostic]> for JsmeldError {
    fn from(diag: &[swc_common::errors::Diagnostic]) -> Self {
        // Convert the SWC error to our error type
        // Diagnostics contains error and diagnostics
        JsmeldError::CompilationError(format!("SWC error: {}", diag.iter().map(|d| d.message()).collect::<Vec<_>>().join("; ")))
    }
}
