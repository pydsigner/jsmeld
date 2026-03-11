//! Configuration structures for jsmeld

use crate::errors::{JSMeldError, JSMeldResult};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// A style transform hook: receives a file path and style source, returns modified source.
/// Stored as a reference-counted closure so Python callables can be wrapped inside.
pub type StyleTransformHook = Arc<dyn Fn(&Path, &str) -> Result<String, String> + Send + Sync>;

/// Unified options for JavaScript/TypeScript compilation and bundling.
#[derive(Clone, Serialize, Deserialize)]
pub struct JSMeldOptions {
    // ── Shared ───────────────────────────────────────────────────────

    /// JavaScript target version (e.g. "es5", "es2020", "esnext")
    #[serde(default)]
    pub target: String,

    /// Enable minification
    #[serde(default)]
    pub minify: bool,

    /// Enable source maps
    #[serde(default)]
    pub source_map: bool,

    // ── Compilation ──────────────────────────────────────────────────

    /// Enable TypeScript support
    #[serde(default)]
    pub typescript: bool,

    /// Module system (e.g., "commonjs", "esm")
    #[serde(default)]
    pub module: String,

    /// Enable strict mode
    #[serde(default)]
    pub strict: bool,

    // ── Bundling ─────────────────────────────────────────────────────

    /// Enable code splitting
    #[serde(default)]
    pub code_split: bool,

    /// External dependencies (won't be bundled)
    #[serde(default)]
    pub externals: Vec<String>,

    /// Style transform hooks keyed by file extension, executed in order when a
    /// style file is loaded during bundling.
    #[serde(skip, default)]
    pub style_hooks: HashMap<String, Vec<StyleTransformHook>>,
}

impl Default for JSMeldOptions {
    fn default() -> Self {
        JSMeldOptions {
            target: "es6".to_string(),
            minify: false,
            source_map: true,
            typescript: true,
            module: "esm".to_string(),
            strict: true,
            code_split: false,
            externals: vec![],
            style_hooks: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for JSMeldOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSMeldOptions")
            .field("target", &self.target)
            .field("minify", &self.minify)
            .field("source_map", &self.source_map)
            .field("typescript", &self.typescript)
            .field("module", &self.module)
            .field("strict", &self.strict)
            .field("code_split", &self.code_split)
            .field("externals", &self.externals)
            .field("style_hooks", &format!("<{} extension(s)>", self.style_hooks.len()))
            .finish()
    }
}

// ── Python dict → JSMeldOptions ──────────────────────────────────────────

/// Extract a typed value from a Python dict, returning a [`JSMeldError::ConfigError`]
/// on type mismatch.
fn extract_opt<'py, T>(
    dict: &Bound<'py, PyDict>,
    key: &str,
) -> JSMeldResult<Option<T>>
where
    for<'a> T: FromPyObject<'a, 'py, Error = PyErr>,
{
    match dict.get_item(key) {
        Ok(Some(val)) => val
            .extract::<T>()
            .map(Some)
            .map_err(|e| JSMeldError::ConfigError(format!("Invalid '{key}': {e}"))),
        Ok(None) => Ok(None),
        Err(e) => Err(JSMeldError::ConfigError(format!("Error reading '{key}': {e}"))),
    }
}

/// Populate a `HashMap<String, Vec<StyleTransformHook>>` from a Python dict that maps
/// extension strings to lists of Python callables.
fn parse_hooks_into(
    hooks_dict: &Bound<'_, PyDict>,
    target: &mut HashMap<String, Vec<StyleTransformHook>>,
) -> JSMeldResult<()> {
    for (ext_val, list_val) in hooks_dict.iter() {
        let ext: String = ext_val
            .extract()
            .map_err(|e| JSMeldError::ConfigError(format!("Hook key must be a string: {e}")))?;
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();

        let callables: Vec<Py<PyAny>> = list_val
            .extract::<Vec<Py<PyAny>>>()
            .map_err(|e| JSMeldError::ConfigError(format!("Hooks for '{ext}' must be a list of callables: {e}")))?;

        let hooks: Vec<StyleTransformHook> = callables
            .into_iter()
            .map(|py_callable| {
                Arc::new(move |path: &Path, src: &str| {
                    Python::try_attach(|py| {
                        let path_str = path.to_str().unwrap_or("");
                        py_callable
                            .bind(py)
                            .call1((path_str, src))
                            .and_then(|r| r.extract::<String>())
                            .map_err(|e| e.to_string())
                    })
                    .unwrap_or_else(|| Err("Python interpreter not available".to_string()))
                }) as StyleTransformHook
            })
            .collect();

        target.insert(ext, hooks);
    }
    Ok(())
}

/// Parse a Python dict into [`JSMeldOptions`].
///
/// Recognised keys: `target`, `minify`, `source_map`, `typescript`, `module`, `strict`,
/// `code_split`, `externals`, `style_hooks`.
pub fn parse_options(dict: &Bound<'_, PyDict>) -> JSMeldResult<JSMeldOptions> {
    let mut opts = JSMeldOptions::default();

    for (key, _) in dict.iter() {
        let key: String = key
            .extract()
            .map_err(|e| JSMeldError::ConfigError(format!("Option key must be a string: {e}")))?;

        match key.as_str() {
            "target" => opts.target = extract_opt(dict, "target")?.unwrap(),
            "minify" => opts.minify = extract_opt(dict, "minify")?.unwrap(),
            "source_map" => opts.source_map = extract_opt(dict, "source_map")?.unwrap(),
            "typescript" => opts.typescript = extract_opt(dict, "typescript")?.unwrap(),
            "module" => opts.module = extract_opt(dict, "module")?.unwrap(),
            "strict" => opts.strict = extract_opt(dict, "strict")?.unwrap(),
            "code_split" => opts.code_split = extract_opt(dict, "code_split")?.unwrap(),
            "externals" => opts.externals = extract_opt(dict, "externals")?.unwrap(),
            "style_hooks" => {
                let hooks_dict = dict
                    .get_item("style_hooks")
                    .ok()
                    .flatten()
                    .unwrap()
                    .cast_into::<PyDict>()
                    .map_err(|_| JSMeldError::ConfigError(
                        "'style_hooks' must be a dict".to_string(),
                    ))?;
                parse_hooks_into(&hooks_dict, &mut opts.style_hooks)?;
            }
            other => {
                return Err(JSMeldError::ConfigError(format!("Unknown option: '{other}'")));
            }
        }
    }

    Ok(opts)
}
