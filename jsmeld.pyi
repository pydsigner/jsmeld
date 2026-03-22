from typing import Optional, Dict, List, Callable, TypedDict


class JSMeldError(Exception):
    """
    Raised for errors coming from jsmeld (compilation, bundling, config, IO).
    """


StyleHook = Callable[[str, str], str]


class JSMeldOptions(TypedDict, total=False):
    target: str
    minify: bool
    source_map: bool
    typescript: bool
    module: str
    strict: bool
    code_split: bool
    externals: List[str]
    # Optional CSS output path used by bundling to extract styles from JS output
    style_output: str
    # Map extension (e.g. "css") -> list of callables (path, source) -> transformed source
    style_hooks: Dict[str, List[StyleHook]]


def compile(entry: str, options: Optional[JSMeldOptions] = None) -> str: ...


def bundle(entry: str, options: Optional[JSMeldOptions] = None) -> str: ...
