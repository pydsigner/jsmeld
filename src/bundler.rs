//! Module bundler using SWC

use crate::compiler::Compiler;
use crate::config::{JSMeldOptions, StyleTransformHook};
use crate::errors::{JSMeldError, JSMeldResult};
use crate::util::parse_es_version;
use anyhow::Context;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use swc_bundler::{Bundler as SwcBundler, Config as BundlerConfig, Hook, Load, ModuleData};
use swc_common::{FileName, FilePathMapping, Globals, SourceMap, GLOBALS};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_loader::resolvers::{lru::CachingResolver, node::NodeModulesResolver};
use swc_ecma_loader::TargetEnv;
use swc_atoms::Atom;

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
    pub compiler: Compiler,
    pub style_hooks: HashMap<String, Vec<StyleTransformHook>>,
    pub extracted_styles: Option<Arc<Mutex<Vec<String>>>>,
}

impl Loader {
    fn is_style_file(path: &Path) -> bool {
        static STYLE_EXTS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
            HashSet::from(["css", "less", "sass", "scss", "styl"])
        });
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => {
                let ext_lc = ext.to_ascii_lowercase();
                STYLE_EXTS.contains(ext_lc.as_str())
            }
            None => false,
        }
    }

    fn build_style_module_source(style_source: &str, inject_runtime_style: bool) -> Result<String, anyhow::Error> {
        let style_literal = serde_json::to_string(style_source)
            .context("Failed to serialize CSS/LESS content")?;

        if inject_runtime_style {
            Ok(format!(
                "const __jsmeldStyle = {style_literal};\nif (typeof document !== \"undefined\") {{\n  const __jsmeldStyleTag = document.createElement(\"style\");\n  __jsmeldStyleTag.textContent = __jsmeldStyle;\n  document.head.appendChild(__jsmeldStyleTag);\n}}\nexport default __jsmeldStyle;\n"
            ))
        } else {
            Ok(format!("const __jsmeldStyle = {style_literal};\n"))
        }
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

impl Load for Loader {
    fn load(&self, f: &FileName) -> Result<ModuleData, anyhow::Error> {
        let fm = match f {
            FileName::Real(path) if Self::is_style_file(path.as_path()) => {
                let style_source = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read style file: {}", path.display()))?;
                let transformed = Self::apply_style_hooks_for_extension(
                    &self.style_hooks,
                    path.as_path(),
                    &style_source,
                )?;

                let should_extract_styles = self.extracted_styles.is_some();
                if let Some(extracted_styles) = &self.extracted_styles {
                    extracted_styles
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Failed to lock style output buffer"))?
                        .push(transformed.clone());
                }

                let module_source = Self::build_style_module_source(
                    &transformed,
                    !should_extract_styles,
                )?;
                self.compiler.cm().new_source_file(f.clone().into(), module_source)
            }
            FileName::Real(path) => {
                let source = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                let filename = path.to_str().unwrap_or("unknown.js");
                let compiled = self.compiler.compile(&source, filename)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                self.compiler.cm().new_source_file(f.clone().into(), compiled)
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

/// Module bundler for JavaScript/TypeScript
pub struct Bundler {
    options: JSMeldOptions,
    compiler: Compiler,
    globals: Arc<Globals>,
}

impl Bundler {
    /// Create a new bundler instance with the given options.
    pub fn new(options: JSMeldOptions) -> Self {
        let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
        let globals = Arc::new(Globals::new());
        let compiler = Compiler::with_source_map(options.clone(), cm, globals.clone());
        Bundler {
            options,
            compiler,
            globals,
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

        let extracted_styles = match &self.options.style_output {
            Some(_) => Some(Arc::new(Mutex::new(Vec::<String>::new()))),
            None => None,
        };

        let loader = Loader {
            compiler: self.compiler.clone(),
            style_hooks: self.options.style_hooks.clone(),
            extracted_styles: extracted_styles.clone(),
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
        let cm = self.compiler.cm();
        let mut bundler = SwcBundler::new(
            &self.globals,
            cm.clone(),
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
        let module = bundled.module;
        {
            let mut emitter = Emitter {
                cfg: swc_ecma_codegen::Config::default()
                    .with_minify(self.options.minify)
                    .with_target(parse_es_version(self.options.target.clone())?),
                cm: <Arc<SourceMap> as Into<Arc<SourceMap>>>::into(cm.clone()),
                comments: None,
                wr: JsWriter::new(cm.clone().into(), "\n", &mut output_buf, None),
            };

            emitter.emit_module(&module)
                .map_err(|e| JSMeldError::BundlingError(format!("Code generation failed: {:#?}", e)))?;
        }

        if let Some(style_output) = &self.options.style_output {
            let css = extracted_styles
                .expect("Style output is enabled but no extracted styles found")
                .lock()
                .ok()
                .map(|guard| guard.join("\n"))
                .unwrap_or_default();

            let css_path = Path::new(style_output);
            if let Some(parent) = css_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(JSMeldError::IOError)?;
                }
            }

            std::fs::write(css_path, css).map_err(JSMeldError::IOError)?;
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

    /// Add a style hook for a file extension.
    pub fn add_style_hook(&mut self, extension: &str, hook: StyleTransformHook) {
        let key = Self::normalize_extension(extension);
        self.options.style_hooks
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
            style_hooks: HashMap::new(),
            style_output: None,
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
        let module = Loader::build_style_module_source(source, true).expect("module generation should work");

        assert!(module.contains("document.createElement(\"style\")"));
        assert!(module.contains("export default __jsmeldStyle;"));
        assert!(module.contains("body { color: red; }\\n@var: 12px;"));
    }

    #[test]
    fn test_style_module_generation_without_runtime_injection() {
        let source = "body { color: red; }";
        let module = Loader::build_style_module_source(source, false)
            .expect("module generation should work");

        assert!(!module.contains("document.createElement(\"style\")"));
        assert!(module.contains("export default __jsmeldStyle;"));
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

        let hooks: HashMap<String, Vec<StyleTransformHook>> = HashMap::from([(
            "css".to_string(),
            vec![
                Arc::new(test_pre_hook) as StyleTransformHook,
                Arc::new(test_post_hook) as StyleTransformHook,
            ],
        )]);

        let result = Loader::apply_style_hooks_for_extension(&hooks, path, source)
            .expect("hooks should succeed");

        assert!(result.contains("blue"));
        assert!(result.ends_with("/* post */"));
    }

    #[test]
    fn test_style_hook_pipeline_skips_other_extensions() {
        let hooks: HashMap<String, Vec<StyleTransformHook>> = HashMap::from([(
            "css".to_string(),
            vec![Arc::new(test_pre_hook) as StyleTransformHook],
        )]);
        let source = "body { color: red; }";
        let less_path = Path::new("styles/theme.less");

        let out = Loader::apply_style_hooks_for_extension(&hooks, less_path, source)
            .expect("hook application should succeed");

        assert_eq!(out, source);
    }
}
