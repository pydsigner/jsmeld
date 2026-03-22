//! Configuration structures for jsmeld

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

    /// Optional output path for extracted bundled styles. When set during
    /// bundling, style imports are emitted into this CSS file instead of being
    /// injected into the JavaScript bundle at runtime.
    #[serde(default)]
    pub style_output: Option<String>,
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
            style_output: None,
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
            .field("style_output", &self.style_output)
            .finish()
    }
}
