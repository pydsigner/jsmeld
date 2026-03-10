//! JavaScript/TypeScript compiler using SWC

use crate::config::CompileOptions;
use crate::errors::{JSMeldError, JSMeldResult};
use crate::util::parse_es_version;
use pyo3::prelude::*;
use std::path::Path;
use std::sync::Arc;
use swc::config::SourceMapsConfig;
use swc::{HandlerOpts, try_with_handler};
use swc::{Compiler as SwcCompiler, config::{Config, Options}};
use swc_common::{SourceMap, FilePathMapping, FileName, GLOBALS, Globals};

#[pyfunction]
pub fn compile(entry: String, target: String, minify: bool) -> JSMeldResult<String> {
    let options = CompileOptions {
        target: parse_es_version(target)?,
        minify,
        ..Default::default()
    };

    let compiler = Compiler::new();
    compiler.compile_file(entry, options)
}

/// JavaScript/TypeScript compiler
pub struct Compiler {
    swc: Arc<SwcCompiler>,
    globals: Globals,
}

impl Compiler {
    /// Create a new compiler instance
    pub fn new() -> Self {
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Compiler {
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
    /// * `options` - Compilation options
    ///
    /// # Returns
    ///
    /// The compiled code as a string
    pub fn compile(
        &self,
        source: &str,
        filename: &str,
        options: CompileOptions,
    ) -> JSMeldResult<String> {
        GLOBALS.set(&self.globals, || {
            self.compile_internal(source, filename, options)
        })
    }

    fn compile_internal(
        &self,
        source: &str,
        filename: &str,
        _options: CompileOptions,
    ) -> JSMeldResult<String> {
        let fm = self.swc.cm.new_source_file(
            FileName::Real(filename.into()).into(),
            source.to_string(),
        );

        let mut config = Config {
            minify: _options.minify.into(),
            source_maps: Some(SourceMapsConfig::Bool(_options.source_map)),
            ..Default::default()
        };
        config.jsc.target = Some(_options.target);
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
    /// * `options` - Compilation options
    ///
    /// # Returns
    ///
    /// The compiled code as a string
    pub fn compile_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        options: CompileOptions,
    ) -> JSMeldResult<String> {
        let path = file_path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|e| JSMeldError::IoError(e))?;

        let filename = path
            .to_str()
            .unwrap_or("unknown.js");

        self.compile(&source, filename, options)
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
            self.compile_internal(source, filename, CompileOptions::default())
        })
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let _compiler = Compiler::new();
    }

    #[test]
    fn test_compile_simple_code() {
        let compiler = Compiler::new();
        let result = compiler.compile(
            "const x = 42;",
            "test.js",
            CompileOptions::default(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_typescript() {
        let compiler = Compiler::new();
        let result = compiler.compile(
            "const x: number = 42;",
            "test.ts",
            CompileOptions::default(),
        );
        assert!(result.is_ok());
    }
}
