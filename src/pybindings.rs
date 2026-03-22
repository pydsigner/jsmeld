use crate::bundle;
use crate::compile;
use crate::config::{JSMeldOptions, StyleTransformHook};
use crate::errors::{JSMeldError, JSMeldResult};
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pyo3::create_exception!(jsmeld, PyJSMeldError, PyException);

impl From<JSMeldError> for PyErr {
    fn from(err: JSMeldError) -> PyErr {
        match err {
            JSMeldError::IOError(e) => PyJSMeldError::new_err(e.to_string()),
            JSMeldError::ConfigError(e) => PyValueError::new_err(e.to_string()),
            _ => PyJSMeldError::new_err(err.to_string()),
        }
    }
}

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
            .map_err(|e| {
                JSMeldError::ConfigError(format!(
                    "Hooks for '{ext}' must be a list of callables: {e}"
                ))
            })?;

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

fn parse_options(dict: &Bound<'_, PyDict>) -> JSMeldResult<JSMeldOptions> {
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
                    .map_err(|_| JSMeldError::ConfigError("'style_hooks' must be a dict".to_string()))?;
                parse_hooks_into(&hooks_dict, &mut opts.style_hooks)?;
            }
            "style_output" => opts.style_output = extract_opt(dict, "style_output")?,
            other => {
                return Err(JSMeldError::ConfigError(format!("Unknown option: '{other}'")));
            }
        }
    }

    Ok(opts)
}

#[pyfunction(name = "compile")]
#[pyo3(signature = (entry, options=None))]
pub fn py_compile(entry: String, options: Option<Bound<'_, PyDict>>) -> PyResult<String> {
    let opts = match options {
        Some(ref dict) => parse_options(dict)?,
        None => JSMeldOptions::default(),
    };
    Ok(compile(entry, opts)?)
}

#[pyfunction(name = "bundle")]
#[pyo3(signature = (entry, options=None))]
pub fn py_bundle(entry: String, options: Option<Bound<'_, PyDict>>) -> PyResult<String> {
    let bundle_options = match options {
        Some(ref dict) => parse_options(dict)?,
        None => JSMeldOptions::default(),
    };
    Ok(bundle(entry, bundle_options)?)
}

#[pymodule]
mod jsmeld {
    #[pymodule_export(name = "bundle")]
    use super::py_bundle;
    #[pymodule_export(name = "compile")]
    use super::py_compile;
    #[pymodule_export(name = "JSMeldError")]
    use super::PyJSMeldError;
}
