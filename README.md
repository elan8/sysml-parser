# sysml-v2-parser

SysML v2 textual notation parser for Rust.

This crate parses SysML v2 and related KerML textual syntax into an AST and also exposes a resilient editor-oriented parsing mode that returns partial AST + diagnostics.

## Current status

- library parser for a broad SysML v2 subset
- strict and resilient parsing entry points
- green unit/integration test suite
- green full validation and std-library gates when run with the SysML v2 release fixtures

## API

The main public entry points are:

- `parse(input)` for strict parsing
- `parse_for_editor(input)` for partial AST + diagnostics

Example:

```rust
use sysml_v2_parser::parse;

fn main() {
    let model = parse("package Demo;").expect("valid SysML");
    assert_eq!(model.elements.len(), 1);
}
```

## Development

Run the default test suite:

```bash
cargo test
```

Run formatting/lint checks used in CI:

```bash
cargo clippy -- -W clippy::all
```

Run the full validation suite against the SysML v2 release tree:

```bash
cargo test --test validation -- --include-ignored
```

If the release fixtures are not in `./sysml-v2-release`, set:

```bash
SYSML_V2_RELEASE_DIR=/path/to/SysML-v2-Release
```
