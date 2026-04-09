# sysml-parser

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
use sysml_parser::parse;

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

## Benchmarks

This repo includes Criterion benchmarks for parsing performance.

Run all benches:

```bash
cargo bench
```

Run the parser bench only:

```bash
cargo bench --bench parser_bench
```

### Benchmark fixtures

The primary benchmark fixture is read from:

- `C:\Git\sysml-examples\drone\sysml\SurveillanceDrone.sysml`

If the file is not present, that benchmark case is skipped.

Optional SysML v2 release fixtures are loaded from:

- `SYSML_V2_RELEASE_DIR` if set, otherwise `./sysml-v2-release`

Missing release fixtures are also skipped so `cargo bench` remains usable in minimal checkouts.

## CI and releases

CI runs on pushes and pull requests via [ci.yml](C:\Git\sysml-parser\.github\workflows\ci.yml) and covers:

- `cargo test`
- `cargo clippy -- -W clippy::all`
- the full ignored validation suite with SysML v2 release fixtures

GitHub releases are created by tagging a version like `v0.1.0`. The release workflow:

- reruns the test and validation gates
- runs `cargo package --allow-dirty --no-verify`
- uploads the packaged crate as a workflow artifact
- creates a GitHub Release for the tag
- publishes to crates.io when the `CARGO_REGISTRY_TOKEN` repository secret is configured

The release workflow can also be started manually with `workflow_dispatch`.
