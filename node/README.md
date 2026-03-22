# jsmeld

Run `jsmeld` directly with `npx`:

```bash
npx jsmeld src/index.js dist/bundle.js --bundle
```

See available options:

```bash
npx jsmeld --help
```

```text
A Rust wrapper around SWC for JavaScript/TypeScript compilation and bundling

Usage: jsmeld [OPTIONS] <INPUT> <OUTPUT>

Arguments:
    <INPUT>   Input file (entry point)
    <OUTPUT>  Output path

Options:
	-b, --bundle                        Bundle and compile the input file
	-c, --compile                       Only compile the input file
	--target <TARGET>                   [default: es6]
	-m, --minify                        Enable minification of output
    --extract-styles                    Extract bundled styles into a separate CSS file, defaulting to <output>.css
    --style-output <STYLE_OUTPUT>       Path to write extracted bundled styles. Implies --extract-styles
    -h, --help                          Print help
    -V, --version                       Print version
```

This package uses optional dependencies to install the matching prebuilt binary.
