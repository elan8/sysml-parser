# BNF Compliance Matrix

Reference grammar:

- `sysml-v2-release/bnf/SysML-textual-bnf.kebnf`

Status labels:

- `implemented`: dedicated AST + dedicated parser path
- `partial`: dedicated parser exists for common forms, but not full production coverage
- `modeled`: parsed into BNF-aligned modeled declaration nodes (`KermlSemanticDecl` / `KermlFeatureDecl` / `ExtendedLibraryDecl`)

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

## Current quality baseline (2026-03-26)

- Systems Library node-shape gate now passes with `ExtendedLibraryDecl = 0`.
- Full std library node-shape gate now also passes with `ExtendedLibraryDecl = 0`.
- Remaining improvement track is no longer package-level fallback elimination, but deeper body-level modeling precision for currently permissive declaration bodies.
