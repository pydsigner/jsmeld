//! Module bundler using SWC

use crate::config::BundleOptions;
use crate::errors::{JsmeldError, JsmeldResult};
use crate::util::parse_es_version;
use std::path::Path;
use std::sync::Arc;
use swc_bundler::{Bundler as SwcBundler, Config as BundlerConfig, Hook, Load, ModuleData};
use swc_common::{FileName, FilePathMapping, Globals, SourceMap, GLOBALS};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_loader::resolvers::{lru::CachingResolver, node::NodeModulesResolver};
use swc_ecma_loader::TargetEnv;
use swc_atoms::Atom;
use swc_common::errors::{ColorConfig, Handler};
use pyo3::prelude::*;

struct BundlerHook {
    //cm: Arc<SourceMap>,
}

/// Hook implementation for SWC bundler -- currently a no-op, but can be extended for custom behavior
impl Hook for BundlerHook {
    fn get_import_meta_props(
        &self,
        _span: swc_common::Span,
        _module: &swc_bundler::ModuleRecord,
    ) -> Result<Vec<swc_ecma_ast::KeyValueProp>, anyhow::Error> {
        Ok(vec![])
    }
}

pub struct Loader {
    pub cm: Arc<SourceMap>,
}

impl Load for Loader {
    fn load(&self, f: &FileName) -> Result<ModuleData, anyhow::Error> {
        let fm = match f {
            FileName::Real(path) => self.cm.load_file(path)?,
            _ => unreachable!(),
        };

        let module = swc_ecma_parser::parse_file_as_module(
            &fm,
            swc_ecma_parser::Syntax::Es(Default::default()),
            Default::default(),
            None,
            &mut Vec::new(),
        )
        .unwrap_or_else(|err| {
            let handler =
                Handler::with_tty_emitter(ColorConfig::Always, false, false, Some(self.cm.clone()));
            err.into_diagnostic(&handler).emit();
            panic!("failed to parse")
        });

        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }

}

#[pyfunction]
pub fn bundle(entry: String, target: String, minify: bool) -> JsmeldResult<String> {
    let options = BundleOptions {
        target: parse_es_version(target)?,
        minify,
        ..Default::default()
    };
    println!("Target: {:?}", options.target);

    let bundler = Bundler::new();
    bundler.bundle(entry.clone(), options)
}

/// Module bundler for JavaScript/TypeScript
pub struct Bundler {
    // Bundler state
    options: Option<BundleOptions>,
    cm: Arc<SourceMap>,
    globals: Globals,
}

impl Bundler {
    /// Create a new bundler instance
    pub fn new() -> Self {
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Bundler {
            options: None,
            cm,
            globals: Globals::new(),
        }
    }

    /// Initialize bundler with options
    pub fn with_options(options: BundleOptions) -> Self {
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Bundler {
            options: Some(options),
            cm,
            globals: Globals::new(),
        }
    }

    /// Bundle JavaScript/TypeScript files
    ///
    /// # Arguments
    ///
    /// * `entry_point` - Path to the entry file
    /// * `output_path` - Path where the bundle should be written
    /// * `options` - Bundling options
    ///
    /// # Returns
    ///
    /// The bundled code as a string
    pub fn bundle<P: AsRef<Path>>(
        &self,
        entry_point: P,
        options: BundleOptions,
    ) -> JsmeldResult<String> {
        let entry = entry_point.as_ref();

        GLOBALS.set(&self.globals, || {
            self.bundle_internal(entry, options)
        })
    }

    fn bundle_internal(&self, entry: &Path, options: BundleOptions) -> JsmeldResult<String> {
        // Setup resolver with node module resolution
        let resolver = CachingResolver::new(
            40,
            NodeModulesResolver::new(TargetEnv::Node, Default::default(), true),
        );
        let loader = Loader {
            cm: self.cm.clone(),
        };

        // Convert externals from String to Atom
        let externals: Vec<Atom> = options
            .externals
            .iter()
            .map(|s| Atom::from(s.as_str()))
            .collect();

        // Setup bundler configuration
        let config = BundlerConfig {
            require: true,
            disable_inliner: false,
            external_modules: externals,
            disable_fixer: false,
            disable_hygiene: false,
            disable_dce: false,
            module: Default::default(),
        };

        // Setup the hook
        let hook = BundlerHook {
            //cm: self.cm.clone(),
        };

        // Create the SWC bundler
        let mut bundler = SwcBundler::new(
            &self.globals,
            self.cm.clone(),
            &loader,
            &resolver,
            config,
            Box::new(hook),
        );

        // Bundle the entry point
        let entry_path = entry.canonicalize()
            .map_err(|e| JsmeldError::IoError(e))?;

        let entries = std::collections::HashMap::from([
            (
                "bundle".to_string(),
                FileName::Real(entry_path.clone()),
            ),
        ]);

        let mut modules = bundler
            .bundle(entries)
            .map_err(|e| JsmeldError::BundlingError(format!("Bundling failed: {}", e)))?;

        // Get the bundled module
        let bundled = modules
            .remove(0);

        // Generate code from the bundle
        let mut output_buf = vec![];
        let module = bundled.module; /* match bundled.kind {
            BundleKind::Named { name: _ } => bundled.module,
            BundleKind::Dynamic {} => bundled.module,
            BundleKind::Lib { name: _ } => bundled.module,
        }; */

        {
            let mut emitter = Emitter {
                cfg: swc_ecma_codegen::Config::default()
                    .with_minify(options.minify)
                    .with_target(options.target),
                cm: <Arc<swc_common::SourceMap> as Into<Arc<swc_common::SourceMap>>>::into(self.cm.clone()),
                comments: None,
                wr: JsWriter::new(self.cm.clone().into(), "\n", &mut output_buf, None),
            };

            emitter.emit_module(&module)
                .map_err(|e| JsmeldError::BundlingError(format!("Code generation failed: {}", e)))?;
        }

        String::from_utf8(output_buf)
            .map_err(|e| JsmeldError::BundlingError(format!("Invalid UTF-8 in output: {}", e)))
    }

    /// Add an external dependency (won't be bundled)
    pub fn add_external(&mut self, module_name: String) {
        if let Some(ref mut opts) = self.options {
            opts.externals.push(module_name);
        }
    }

    /// Get the current bundler options
    pub fn options(&self) -> Option<&BundleOptions> {
        self.options.as_ref()
    }

    /// Get the current bundler options (mutable)
    pub fn options_mut(&mut self) -> Option<&mut BundleOptions> {
        self.options.as_mut()
    }

    /// Set bundler options
    pub fn set_options(&mut self, options: BundleOptions) {
        self.options = Some(options);
    }
}

impl Default for Bundler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundler_creation() {
        let _bundler = Bundler::new();
    }

    #[test]
    fn test_bundler_with_options() {
        let options = BundleOptions {
            target: swc_ecma_ast::EsVersion::Es2020,
            minify: false,
            source_map: true,
            code_split: false,
            externals: vec![],
        };
        let bundler = Bundler::with_options(options);
        assert!(bundler.options().is_some());
    }

    #[test]
    fn test_add_external() {
        let mut bundler = Bundler::new();
        bundler.set_options(BundleOptions {
            target: swc_ecma_ast::EsVersion::Es2020,
            minify: false,
            source_map: true,
            code_split: false,
            externals: vec![],
        });
        bundler.add_external("react".to_string());
        assert_eq!(bundler.options().unwrap().externals.len(), 1);
    }
}
