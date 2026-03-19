# jsmeld

A wrapper around SWC for compiling and bundling JavaScript in Rust and Python.

- [Rust usage](#rust-usage)
- [Python usage](#python-usage)

## Rust usage

Both `compile` and `bundle` take a file path and a `JSMeldOptions` struct:

```rust
use jsmeld::{compile, bundle, JSMeldOptions};

// Compile a single file
let output = compile(
    "./src/app.ts".to_string(),
    JSMeldOptions {
        target: "es2020".to_string(),
        minify: true,
        ..Default::default()
    },
)?;

// Bundle an entry point
let output = bundle(
    "./src/index.js".to_string(),
    JSMeldOptions {
        target: "es2020".to_string(),
        externals: vec!["react".to_string()],
        ..Default::default()
    },
)?;
```

### `JSMeldOptions`

A single unified options struct used by both compilation and bundling.

| Field | Type | Default |
|-------|------|---------|
| `target` | `String` | `"es6"` |
| `minify` | `bool` | `false` |
| `source_map` | `bool` | `true` |
| `typescript` | `bool` | `true` |
| `module` | `String` | `"esm"` |
| `strict` | `bool` | `true` |
| `code_split` | `bool` | `false` |
| `externals` | `Vec<String>` | `[]` |
| `style_hooks` | `HashMap<String, Vec<StyleTransformHook>>` | `{}` |

### Style hooks

`JSMeldOptions` supports style transformation hooks keyed by file extension.
Hooks are executed in order when a matching style file is loaded during bundling.

Each key is a file extension (e.g. `"css"`, `"less"`) and each value is an ordered list of hook closures.

Hook type:

```rust
type StyleTransformHook = Arc<dyn Fn(&Path, &str) -> Result<String, String> + Send + Sync>;
```

#### Registering hooks in `JSMeldOptions`

```rust
use std::sync::Arc;
use std::collections::HashMap;
use jsmeld::{Bundler, JSMeldOptions};

let mut options = JSMeldOptions::default();

options.style_hooks.insert(
    "less".to_string(),
    vec![Arc::new(|_path, source| {
        // transform LESS -> CSS here
        Ok(source.to_string())
    })],
);

options.style_hooks.entry("css".to_string()).or_default().push(
    Arc::new(|_path, source| {
        // post-process CSS here
        Ok(source.to_string())
    }),
);

let bundler = Bundler::new(options);
let output = bundler.bundle("./src/index.js")?;
```

#### Registering hooks via `Bundler`

If you initialize a `Bundler` with options, you can append hooks per extension:

```rust
use std::sync::Arc;
use jsmeld::{Bundler, JSMeldOptions};

let mut bundler = Bundler::new(JSMeldOptions::default());
bundler.add_style_hook("css", Arc::new(|_path, source| {
    Ok(source.to_string())
}));
```

Notes:

- Extensions are normalized to lowercase.
- Leading dots are allowed (e.g. `".css"`).
- Hooks run only for matching style-file extensions.

---

## Python usage

The Python package exposes two top-level functions. Both accept an optional
`options` dictionary with the same keys as [`JSMeldOptions` above](#jsmeldoptions).

```python
import jsmeld

# Compile a single file (all options are optional)
output: str = jsmeld.compile("src/app.ts", {
    "target": "es2020",
    "minify": True,
})

# Bundle an entry point
output: str = jsmeld.bundle("src/index.js", {
    "target": "es2020",
    "externals": ["react"],
})
```

### Style hooks from Python

`style_hooks` can be passed as a dictionary mapping file extensions to lists
of callables `(path: str, source: str) -> str`:

```python
def add_banner(path: str, source: str) -> str:
    return f"/* bundled by jsmeld */\n{source}"

output = jsmeld.bundle("src/index.js", {
    "style_hooks": {
        "css": [add_banner],
    },
})
```
