# BNF Compliance Matrix

This is the primary compact coverage snapshot for the parser.

Reference grammar:

- `sysml-v2-release/bnf/SysML-textual-bnf.kebnf`

Status labels:

- `implemented`: dedicated AST + dedicated parser path, exercised by the current validation baseline
- `partial`: dedicated parser exists and is validated for common forms, but coverage may still rely on permissive body parsing or subset-only grammar support
- `modeled`: parsed into BNF-aligned modeled declaration nodes (`KermlSemanticDecl` / `KermlFeatureDecl` / `ExtendedLibraryDecl`) instead of a fully dedicated construct-specific AST

## Package-level declaration families

- `package`, `library package`, `namespace`, `import`: `implemented`
- `part`, `port`, `attribute`, `action`, `state`, `requirement`, `case`, `analysis`, `verification`, `flow`, `allocation`, `interface`, `view`, `viewpoint`, `rendering`, `metadata`, `enum`: `partial`
- KerML semantic families (`behavior`, `function`, `datatype`, `assoc`, `struct`, `metaclass`, `class`, `classifier`, `feature`, `step`): `modeled`
- KerML feature logic families (`occurrence`, `expr`, `predicate`, `succession`): `modeled`
- Extended declaration starters (`message`, `concern` and remaining library declarations): `modeled`

## Validation gates

- `test_systems_library_strict_no_diagnostics`: required green
- `test_full_library_strict_no_diagnostics`: required green
- `test_full_library_suite`: broad integration visibility
- `test_systems_library_node_types_no_extended`: required green (**hard 0 `ExtendedLibraryDecl` for Systems Library**)
- `test_full_library_node_types_no_extended`: required green (**hard 0 `ExtendedLibraryDecl` for full std library**)
  - supports staged migration threshold via env var `FULL_LIBRARY_EXTENDED_MAX`
  - default threshold is `0` (strict)

## Current quality baseline (2026-04-09)

- `cargo test` is green.
- `cargo test --test validation -- --include-ignored` is green, including the full validation suite and full SysML library gates.
- Systems Library node-shape validation passes with `ExtendedLibraryDecl = 0`.
- Full std-library node-shape validation also passes with `ExtendedLibraryDecl = 0`.
- The main remaining work is deeper body-level modeling precision and language-server hardening, not package-level fallback elimination.
