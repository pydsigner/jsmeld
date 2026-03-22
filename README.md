# jsmeld

A wrapper around SWC for compiling and bundling JavaScript in Rust and Python.

- [Rust usage](#rust-usage)
- [CLI usage](#cli-usage)
- [NPM usage](#npm-usage)
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
        style_output: Some("./dist/index.css".to_string()),
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
| `style_output` | `Option<String>` | `None` |
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

## CLI usage

The CLI accepts an input file and output path:

```bash
cargo run -- <input> <output> [options]
```

If installed as a binary, replace `cargo run --` with `jsmeld`.

### Common commands

Compile a single file:

```bash
cargo run -- src/app.ts dist/app.js --compile
```

Bundle an entry point:

```bash
cargo run -- src/index.js dist/bundle.js --bundle
```

Bundle with minification and a target:

```bash
cargo run -- src/index.js dist/bundle.js --bundle --minify --target es2020
```

Extract styles to `dist/bundle.css` (default sidecar path):

```bash
cargo run -- src/index.js dist/bundle.js --bundle --extract-styles
```

Extract styles to a custom path:

```bash
cargo run -- src/index.js dist/bundle.js --bundle --style-output dist/styles/app.css
```

Notes:

- `--style-output` implies style extraction automatically.
- If neither `--bundle` nor `--compile` is provided, bundling is used.

### Help output

```bash
$ jsmeld --help
A Rust wrapper around SWC for JavaScript/TypeScript compilation and bundling

Usage: jsmeld [OPTIONS] <INPUT> <OUTPUT>

Arguments:
    <INPUT>   Input file (entry point)
    <OUTPUT>  Output path

Options:
    -b, --bundle                       Bundle and compile the input file
    -c, --compile                      Only compile the input file
            --target <TARGET>              [default: es6]
    -m, --minify                       Enable minification of output
            --extract-styles               Extract bundled styles into a separate CSS file, defaulting to <output>.css
            --style-output <STYLE_OUTPUT>  Path to write extracted bundled styles. Implies --extract-styles
    -h, --help                         Print help
    -V, --version                      Print version
```

---

## NPM usage

Install the cross-platform CLI package:

```bash
npm install --save-dev jsmeld
```

Run it via `npx`:

```bash
npx jsmeld src/index.js dist/bundle.js --bundle --minify
```

Or use it from npm scripts:

```json
{
    "scripts": {
        "build:js": "jsmeld src/index.js dist/bundle.js --bundle"
    }
}
```

The top-level `jsmeld` package uses `optionalDependencies` to install the
matching prebuilt binary package for your platform.

Published binary package names:

- `jsmeld-linux-x64-gnu`
- `jsmeld-linux-arm64-gnu`
- `jsmeld-linux-x64-musl`
- `jsmeld-linux-arm64-musl`
- `jsmeld-darwin-x64`
- `jsmeld-darwin-arm64`
- `jsmeld-win32-x64-msvc`
- `jsmeld-win32-arm64-msvc`

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
    "style_output": "dist/index.css",
})
```
