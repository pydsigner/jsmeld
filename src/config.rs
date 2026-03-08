//! Configuration structures for jsmeld

use serde::{Deserialize, Serialize};


/// Options for JavaScript/TypeScript compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOptions {
    /// JavaScript target version
    #[serde(default)]
    pub target: swc_ecma_ast::EsVersion,

    /// Enable minification
    #[serde(default)]
    pub minify: bool,

    /// Enable source maps
    #[serde(default)]
    pub source_map: bool,

    /// Enable TypeScript support
    #[serde(default = "default_true")]
    pub typescript: bool,

    /// Module system (e.g., "commonjs", "esm")
    #[serde(default = "default_module")]
    pub module: String,

    /// Enable strict mode
    #[serde(default = "default_true")]
    pub strict: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions {
            target: swc_ecma_ast::EsVersion::default(),
            minify: false,
            source_map: true,
            typescript: true,
            module: "esm".to_string(),
            strict: true,
        }
    }
}


/// Options for bundling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleOptions {
    /// JavaScript target version
    #[serde(default)]
    pub target: swc_ecma_ast::EsVersion,

    /// Enable minification
    #[serde(default)]
    pub minify: bool,

    /// Enable source maps
    #[serde(default)]
    pub source_map: bool,

    /// Entry point file
    pub entry: String,

    /// Output file path
    pub output: String,

    /// Enable code splitting
    #[serde(default)]
    pub code_split: bool,

    /// External dependencies (won't be bundled)
    #[serde(default)]
    pub externals: Vec<String>,
}

impl Default for BundleOptions {
    fn default() -> Self {
        BundleOptions {
            target: swc_ecma_ast::EsVersion::default(),
            minify: false,
            source_map: true,
            entry: "src/index.js".to_string(),
            output: "dist/bundle.js".to_string(),
            code_split: false,
            externals: vec![],
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_module() -> String {
    "esm".to_string()
}
