//! Module bundler using SWC

use crate::compiler::Compiler;
use crate::config::{JSMeldOptions, StyleTransformHook, parse_options};
use crate::errors::{JSMeldError, JSMeldResult};
use crate::util::parse_es_version;
use anyhow::Context;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use swc_bundler::{Bundler as SwcBundler, Config as BundlerConfig, Hook, Load, ModuleData};
use swc_common::{FileName, FilePathMapping, Globals, SourceMap, GLOBALS};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_loader::resolvers::{lru::CachingResolver, node::NodeModulesResolver};
use swc_ecma_loader::TargetEnv;
use swc_atoms::Atom;
use pyo3::prelude::*;
use pyo3::types::PyDict;

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

pub struct Loader<'a> {
    pub cm: Arc<SourceMap>,
    pub compiler: &'a Compiler,
    pub preprocess_style_hooks: HashMap<String, Vec<StyleTransformHook>>,
    pub postprocess_style_hooks: HashMap<String, Vec<StyleTransformHook>>,
}

impl Loader<'_> {
    fn is_style_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("css") || ext.eq_ignore_ascii_case("less"))
            .unwrap_or(false)
    }

    fn build_style_module_source(style_source: &str) -> Result<String, anyhow::Error> {
        let style_literal = serde_json::to_string(style_source)
            .context("Failed to serialize CSS/LESS content")?;

        Ok(format!(
            "const __jsmeldStyle = {style_literal};\nif (typeof document !== \"undefined\") {{\n  const __jsmeldStyleTag = document.createElement(\"style\");\n  __jsmeldStyleTag.textContent = __jsmeldStyle;\n  document.head.appendChild(__jsmeldStyleTag);\n}}\nexport default __jsmeldStyle;\n"
        ))
    }

    fn extension_from_path(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
    }

    fn apply_style_hooks_for_extension(
        hooks_by_extension: &HashMap<String, Vec<StyleTransformHook>>,
        path: &Path,
        source: &str,
    ) -> Result<String, anyhow::Error> {
        let Some(extension) = Self::extension_from_path(path) else {
            return Ok(source.to_string());
        };

        let Some(hooks) = hooks_by_extension.get(&extension) else {
            return Ok(source.to_string());
        };

        let mut transformed = source.to_string();
        for hook in hooks {
            transformed = hook(path, &transformed).map_err(anyhow::Error::msg)?;
        }
        Ok(transformed)
    }
}

impl Load for Loader<'_> {
    fn load(&self, f: &FileName) -> Result<ModuleData, anyhow::Error> {
        let fm = match f {
            FileName::Real(path) if Self::is_style_file(path.as_path()) => {
                let style_source = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read style file: {}", path.display()))?;
                let preprocessed = Self::apply_style_hooks_for_extension(
                    &self.preprocess_style_hooks,
                    path.as_path(),
                    &style_source,
                )?;
                let postprocessed = Self::apply_style_hooks_for_extension(
                    &self.postprocess_style_hooks,
                    path.as_path(),
                    &preprocessed,
                )?;
                let module_source = Self::build_style_module_source(&postprocessed)?;
                self.cm.new_source_file(f.clone().into(), module_source)
            }
            FileName::Real(path) => {
                let source = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                let filename = path.to_str().unwrap_or("unknown.js");
                let compiled = self.compiler.compile(&source, filename)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                self.cm.new_source_file(f.clone().into(), compiled)
            }
            _ => unreachable!(),
        };

        let module = swc_ecma_parser::parse_file_as_module(
            &fm,
            swc_ecma_parser::Syntax::Es(Default::default()),
            Default::default(),
            None,
            &mut Vec::new(),
        ).map_err(|e| anyhow::anyhow!("Failed to parse module: {:#?}", e))?;

        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }

}

/// Bundle a JavaScript/TypeScript entry point and return the output as a string.
///
/// # Arguments
///
/// * `entry` – Path to the entry file.
/// * `options` – Bundling options.
pub fn bundle(entry: String, options: JSMeldOptions) -> JSMeldResult<String> {
    let bundler = Bundler::new(options);
    bundler.bundle(entry)
}

/// Python binding for [`bundle`].
///
/// # Arguments
///
/// * `entry` – Path to the entry file.
/// * `options` – Optional dict of bundle options. Supported keys:
///   - `target` (str): ES version, e.g. `"es2020"` (default: `"es5"`)
///   - `minify` (bool): Enable minification (default: `False`)
///   - `source_map` (bool): Emit source maps (default: `True`)
///   - `code_split` (bool): Enable code splitting (default: `False`)
///   - `externals` (list[str]): Modules to exclude from the bundle (default: `[]`)
///   - `preprocess_style_hooks` (dict[str, list[callable]]): Map of file extension to
///     a list of callables `(path: str, source: str) -> str` run before the style
///     module is emitted (default: `{}`)
///   - `postprocess_style_hooks` (dict[str, list[callable]]): Same shape as above,
///     run after the preprocess hooks (default: `{}`)
#[pyfunction(name = "bundle")]
#[pyo3(signature = (entry, options=None))]
pub fn py_bundle(entry: String, options: Option<Bound<'_, PyDict>>) -> JSMeldResult<String> {
    let bundle_options = match options {
        Some(ref dict) => parse_options(dict)?,
        None => JSMeldOptions::default(),
    };
    bundle(entry, bundle_options)
}

/// Module bundler for JavaScript/TypeScript
pub struct Bundler {
    options: JSMeldOptions,
    compiler: Compiler,
    cm: Arc<SourceMap>,
    globals: Globals,
}

impl Bundler {
    /// Create a new bundler instance with the given options.
    pub fn new(options: JSMeldOptions) -> Self {
        let compiler = Compiler::new(options.clone());
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Bundler {
            options,
            compiler,
            cm,
            globals: Globals::new(),
        }
    }

    /// Get a reference to the compiler used for preprocessing.
    pub fn compiler(&self) -> &Compiler {
        &self.compiler
    }

    /// Bundle JavaScript/TypeScript files
    ///
    /// # Arguments
    ///
    /// * `entry_point` - Path to the entry file
    ///
    /// # Returns
    ///
    /// The bundled code as a string
    pub fn bundle<P: AsRef<Path>>(
        &self,
        entry_point: P,
    ) -> JSMeldResult<String> {
        let entry = entry_point.as_ref();

        GLOBALS.set(&self.globals, || {
            self.bundle_internal(entry)
        })
    }

    fn bundle_internal(&self, entry: &Path) -> JSMeldResult<String> {
        // Setup resolver with node module resolution
        let resolver = CachingResolver::new(
            40,
            NodeModulesResolver::new(TargetEnv::Browser, Default::default(), true),
        );

        let loader = Loader {
            cm: self.cm.clone(),
            compiler: &self.compiler,
            preprocess_style_hooks: self.options.preprocess_style_hooks.clone(),
            postprocess_style_hooks: self.options.postprocess_style_hooks.clone(),
        };

        // Convert externals from String to Atom
        let externals: Vec<Atom> = self.options
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
            .map_err(|e| JSMeldError::IOError(e))?;

        let entries = std::collections::HashMap::from([
            (
                "bundle".to_string(),
                FileName::Real(entry_path.clone()),
            ),
        ]);

        let mut modules = bundler
            .bundle(entries)
            .map_err(|e| JSMeldError::BundlingError(format!("Bundling failed: {:#?}", e)))?;

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
                    .with_minify(self.options.minify)
                    .with_target(parse_es_version(self.options.target.clone())?),
                cm: <Arc<swc_common::SourceMap> as Into<Arc<swc_common::SourceMap>>>::into(self.cm.clone()),
                comments: None,
                wr: JsWriter::new(self.cm.clone().into(), "\n", &mut output_buf, None),
            };

            emitter.emit_module(&module)
                .map_err(|e| JSMeldError::BundlingError(format!("Code generation failed: {:#?}", e)))?;
        }

        String::from_utf8(output_buf)
            .map_err(|e| JSMeldError::BundlingError(format!("Invalid UTF-8 in output: {:#?}", e)))
    }

    /// Add an external dependency (won't be bundled)
    pub fn add_external(&mut self, module_name: String) {
        self.options.externals.push(module_name);
    }

    fn normalize_extension(extension: &str) -> String {
        extension.trim_start_matches('.').to_ascii_lowercase()
    }

    /// Add a style preprocess hook for a file extension.
    pub fn add_preprocess_style_hook(&mut self, extension: &str, hook: StyleTransformHook) {
        let key = Self::normalize_extension(extension);
        self.options.preprocess_style_hooks
            .entry(key)
            .or_default()
            .push(hook);
    }

    /// Add a style postprocess hook for a file extension.
    pub fn add_postprocess_style_hook(&mut self, extension: &str, hook: StyleTransformHook) {
        let key = Self::normalize_extension(extension);
        self.options.postprocess_style_hooks
            .entry(key)
            .or_default()
            .push(hook);
    }

    /// Get the current bundler options
    pub fn options(&self) -> &JSMeldOptions {
        &self.options
    }

    /// Get the current bundler options (mutable)
    pub fn options_mut(&mut self) -> &mut JSMeldOptions {
        &mut self.options
    }
}

impl Default for Bundler {
    fn default() -> Self {
        Self::new(JSMeldOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundler_creation() {
        let _bundler = Bundler::new(JSMeldOptions::default());
    }

    #[test]
    fn test_bundler_with_options() {
        let options = JSMeldOptions {
            target: "es2020".to_string(),
            minify: false,
            source_map: true,
            code_split: false,
            externals: vec![],
            preprocess_style_hooks: HashMap::new(),
            postprocess_style_hooks: HashMap::new(),
            ..Default::default()
        };
        let bundler = Bundler::new(options);
        assert_eq!(bundler.options().target, "es2020");
    }

    #[test]
    fn test_add_external() {
        let mut bundler = Bundler::new(JSMeldOptions::default());
        bundler.add_external("react".to_string());
        assert_eq!(bundler.options().externals.len(), 1);
    }

    #[test]
    fn test_style_module_generation() {
        let source = "body { color: red; }\n@var: 12px;";
        let module = Loader::build_style_module_source(source).expect("module generation should work");

        assert!(module.contains("document.createElement(\"style\")"));
        assert!(module.contains("export default __jsmeldStyle;"));
        assert!(module.contains("body { color: red; }\\n@var: 12px;"));
    }

    fn test_pre_hook(_path: &Path, source: &str) -> Result<String, String> {
        Ok(source.replace("red", "blue"))
    }

    fn test_post_hook(_path: &Path, source: &str) -> Result<String, String> {
        Ok(format!("{}\n/* post */", source))
    }

    #[test]
    fn test_style_hook_pipeline() {
        let path = Path::new("styles/main.css");
        let source = "body { color: red; }";

        let preprocess_hooks: HashMap<String, Vec<StyleTransformHook>> = HashMap::from([(
            "css".to_string(),
            vec![Arc::new(test_pre_hook) as StyleTransformHook],
        )]);
        let postprocess_hooks: HashMap<String, Vec<StyleTransformHook>> = HashMap::from([(
            "css".to_string(),
            vec![Arc::new(test_post_hook) as StyleTransformHook],
        )]);

        let preprocessed = Loader::apply_style_hooks_for_extension(&preprocess_hooks, path, source)
            .expect("preprocess hook should succeed");
        let postprocessed = Loader::apply_style_hooks_for_extension(&postprocess_hooks, path, &preprocessed)
            .expect("postprocess hook should succeed");

        assert_eq!(preprocessed, "body { color: blue; }");
        assert!(postprocessed.ends_with("/* post */"));
    }

    #[test]
    fn test_style_hook_pipeline_skips_other_extensions() {
        let preprocess_hooks: HashMap<String, Vec<StyleTransformHook>> = HashMap::from([(
            "css".to_string(),
            vec![Arc::new(test_pre_hook) as StyleTransformHook],
        )]);
        let source = "body { color: red; }";
        let less_path = Path::new("styles/theme.less");

        let out = Loader::apply_style_hooks_for_extension(&preprocess_hooks, less_path, source)
            .expect("hook application should succeed");

        assert_eq!(out, source);
    }
}
