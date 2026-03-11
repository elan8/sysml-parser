# Validation fixture tests

Each SysML file under `sysml-v2-release/sysml/src/validation/` can have a corresponding Rust test module in this directory for easier maintenance.

## Layout

- **`tests/validation.rs`** – Integration test crate that pulls in all validation test modules via `#[path = "validation/..."] mod ...`.
- **`tests/validation/<name>.rs`** – One module per validation fixture (e.g. `parts_tree_1a.rs` for `01-Parts Tree/1a-Parts Tree.sysml`).

## Adding a new validation test

1. Add a new `.rs` file in `tests/validation/` (e.g. `use_case_18.rs` for `18-Use Case/18-Use Case.sysml`). Use a valid Rust module name (no leading digits; replace spaces and hyphens with underscores).

2. In that file:
   - Use `validation_fixture_path(relative)` to build the path: e.g. `validation_fixture_path("18-Use Case").join("18-Use Case.sysml")`.
   - Define the expected AST and a `#[test]` that reads the fixture, calls `parse()`, and asserts `result == expected`.

3. In `tests/validation.rs`, add:
   ```rust
   #[path = "validation/use_case_18.rs"]
   mod use_case_18;
   ```

## Shared helper

`validation_fixture_path(relative)` in each module builds the path to `sysml-v2-release/sysml/src/validation/<relative>`. Copy the helper into new modules or refactor into a shared test helper in the crate later.
