//! JavaScript/TypeScript compiler using SWC

use crate::config::{JSMeldOptions, parse_options};
use crate::errors::{JSMeldError, JSMeldResult};
use crate::util::parse_es_version;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::Path;
use std::sync::Arc;
use swc::config::SourceMapsConfig;
use swc::{HandlerOpts, try_with_handler};
use swc::{Compiler as SwcCompiler, config::{Config, Options}};
use swc_common::{SourceMap, FilePathMapping, FileName, GLOBALS, Globals};

/// Compile a JavaScript/TypeScript file and return the output as a string.
///
/// # Arguments
///
/// * `entry` – Path to the source file.
/// * `options` – Compilation options.
pub fn compile(entry: String, options: JSMeldOptions) -> JSMeldResult<String> {
    let compiler = Compiler::new(options);
    compiler.compile_file(entry)
}

/// Python binding for [`compile`].
///
/// # Arguments
///
/// * `entry` – Path to the source file.
/// * `options` – Optional dict of options. See [`crate::bundler::parse_options`] for keys.
#[pyfunction(name = "compile")]
#[pyo3(signature = (entry, options=None))]
pub fn py_compile(entry: String, options: Option<Bound<'_, PyDict>>) -> JSMeldResult<String> {
    let opts = match options {
        Some(ref dict) => parse_options(dict)?,
        None => JSMeldOptions::default(),
    };
    compile(entry, opts)
}

/// JavaScript/TypeScript compiler
pub struct Compiler {
    options: JSMeldOptions,
    swc: Arc<SwcCompiler>,
    globals: Globals,
}

impl Compiler {
    /// Create a new compiler instance with the given options.
    pub fn new(options: JSMeldOptions) -> Self {
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Compiler {
            options,
            swc: Arc::new(SwcCompiler::new(cm)),
            globals: Globals::new(),
        }
    }

    /// Compile JavaScript/TypeScript code from a string
    ///
    /// # Arguments
    ///
    /// * `source` - The source code to compile
    /// * `filename` - The filename for error reporting
    ///
    /// # Returns
    ///
    /// The compiled code as a string
    pub fn compile(
        &self,
        source: &str,
        filename: &str,
    ) -> JSMeldResult<String> {
        GLOBALS.set(&self.globals, || {
            self.compile_internal(source, filename)
        })
    }

    fn compile_internal(
        &self,
        source: &str,
        filename: &str,
    ) -> JSMeldResult<String> {
        let fm = self.swc.cm.new_source_file(
            FileName::Real(filename.into()).into(),
            source.to_string(),
        );

        let mut config = Config {
            minify: self.options.minify.into(),
            source_maps: Some(SourceMapsConfig::Bool(self.options.source_map)),
            ..Default::default()
        };
        config.jsc.target = Some(parse_es_version(self.options.target.clone())?);
        let opts = Options {
            config,
            filename: filename.to_string(),
            ..Default::default()
        };

        let output = try_with_handler(self.swc.cm.clone(), HandlerOpts::default(), |handler| {
            self.swc.process_js_file(fm, handler, &opts)
        }).map_err(|err| JSMeldError::from(err.diagnostics()))?;

        Ok(output.code)
    }


    /// Compile a file from disk
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to compile
    ///
    /// # Returns
    ///
    /// The compiled code as a string
    pub fn compile_file<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> JSMeldResult<String> {
        let path = file_path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|e| JSMeldError::IOError(e))?;

        let filename = path
            .to_str()
            .unwrap_or("unknown.js");

        self.compile(&source, filename)
    }

    /// Transform code with specific transforms
    ///
    /// # Arguments
    ///
    /// * `source` - The source code to transform
    /// * `filename` - The filename for error reporting
    /// * `_transforms` - List of transforms to apply
    ///
    /// # Returns
    ///
    /// The transformed code as a string
    pub fn transform(
        &self,
        source: &str,
        filename: &str,
        _transforms: Vec<String>,
    ) -> JSMeldResult<String> {
        GLOBALS.set(&self.globals, || {
            // For now, just compile with default options
            // TODO: Apply specific transforms based on the transforms list
            self.compile_internal(source, filename)
        })
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new(JSMeldOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let _compiler = Compiler::new(JSMeldOptions::default());
    }

    #[test]
    fn test_compile_simple_code() {
        let compiler = Compiler::new(JSMeldOptions::default());
        let result = compiler.compile(
            "const x = 42;",
            "test.js",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_typescript() {
        let compiler = Compiler::new(JSMeldOptions::default());
        let result = compiler.compile(
            "const x: number = 42;",
            "test.ts",
        );
        assert!(result.is_ok());
    }
}
