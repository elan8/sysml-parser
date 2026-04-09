# SysML v2 Parser Status

This document is the narrative status view of the parser as of April 9, 2026. It complements the compact snapshot in [`docs/BNF_COMPLIANCE_MATRIX.md`](C:\Git\sysml-parser\docs\BNF_COMPLIANCE_MATRIX.md).

Reference grammar:

- [`sysml-v2-release/bnf/SysML-textual-bnf.kebnf`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf)

## Scope

This is a parser-status document, not a semantic conformance claim.

It describes:

- which major textual grammar families currently have dedicated parser coverage
- where the parser still relies on permissive or shell-style parsing
- what remains unfinished even though the validation and library suites are green

It does not claim semantic conformance for:

- name resolution
- type checking
- cross-reference validation
- KerML/SysML well-formedness constraints
- derived or implicit abstract-syntax behavior

## Current baseline

The parser currently has a stronger baseline than the earlier gap analysis implied:

- `cargo test` is green
- `cargo test --test validation -- --include-ignored` is green
- the full validation suite is green
- the strict Systems Library and full-library syntax gates are green
- the Systems Library and full-library node-shape gates both pass with `ExtendedLibraryDecl = 0`

That means the project is no longer in a state where large parser families are simply absent from the implementation baseline. The remaining work is now mostly about depth, fidelity, and editor-grade resilience.

## What is implemented and validated

The parser has dedicated modules for the major SysML-facing construct families in [`src/parser`](C:\Git\sysml-parser\src\parser):

- packages and imports
- aliases
- attributes
- enumerations
- items
- parts
- ports
- connections
- interfaces
- allocations
- flows
- actions
- states
- calculations and constraints
- requirements and concerns
- cases, analysis cases, and verification cases
- use cases
- views, viewpoints, and renderings
- metadata
- occurrences and individuals

This does not mean every production in those families is fully grammar-faithful, but it does mean these families are represented by real parser entry points and are covered by the current validation baseline.

## What is still partial or permissive

### Generic definition and usage grammar remains the largest architectural gap

The spec's shared definition/usage/specialization layer is still only partially modeled. Much of that logic remains distributed across construct-specific parsers such as parts, ports, attributes, actions, and states instead of being represented by a unified grammar layer.

Impact:

- many legal combinations from the spec are still unavailable
- support is broader for common fixture forms than for the full grammar surface
- extending support consistently across families is harder than it should be

### Several families are parsed as useful subsets rather than full productions

The parser now covers flows, allocations, case families, and other formerly missing areas, but many of those families still operate as subset parsers rather than complete grammar implementations.

Common remaining limitations:

- subset-only body item coverage
- incomplete specialization/prefix combinations
- simplified handling of nested forms
- dedicated nodes for top-level declarations without equally deep body modeling

The strongest areas today are still packages/imports, parts, requirements, and the general library-validation path.

### Some bodies are still accepted permissively

Several modules still use `skip_until_brace_end()` or related helpers from [`src/parser/lex.rs`](C:\Git\sysml-parser\src\parser\lex.rs) to consume bodies without fully parsing their internal grammar.

This still appears in or influences:

- [`src/parser/metadata.rs`](C:\Git\sysml-parser\src\parser\metadata.rs)
- [`src/parser/occurrence.rs`](C:\Git\sysml-parser\src\parser\occurrence.rs)
- [`src/parser/alias.rs`](C:\Git\sysml-parser\src\parser\alias.rs)
- [`src/parser/import.rs`](C:\Git\sysml-parser\src\parser\import.rs)
- parts of connection, view, action, state, requirement, interface, port, and enumeration parsing

Impact:

- broad fixture compatibility is preserved
- top-level declaration coverage is stronger than deep body fidelity
- body-level AST precision is still the main technical debt

### Expression support is useful but still incomplete

[`src/parser/expr.rs`](C:\Git\sysml-parser\src\parser\expr.rs) now has precedence-aware parsing for its supported operators, so the earlier flat-chain description is outdated. Even so, expression coverage is still only a subset of full SysML/KerML `OwnedExpression`.

Impact:

- common arithmetic, logical, member-access, and simple literal forms work
- broader expression-family coverage is still needed for full spec fidelity across constraints, calculations, guards, filters, and value parts

## Modeled fallback nodes

Package-level fallback handling is no longer just "recover and emit diagnostics." The parser now models several declaration families directly in the AST:

- `KermlSemanticDecl`
- `KermlFeatureDecl`
- `ExtendedLibraryDecl`

These nodes are defined in [`src/ast.rs`](C:\Git\sysml-parser\src\ast.rs) and produced by package-level parsing in [`src/parser/package.rs`](C:\Git\sysml-parser\src\parser\package.rs).

Current status:

- `KermlSemanticDecl` and `KermlFeatureDecl` are intentional modeled families for broader library coverage
- `ExtendedLibraryDecl` has been driven down to `0` in the current Systems Library and full std-library node-shape gates

This means package-level fallback elimination is no longer the main quality problem. The remaining issue is how deeply the parser models bodies after a declaration has already been recognized.

## What "not finished" means now

The parser is not blocked on missing top-level support for major SysML families anymore. "Not finished" now means:

1. generic definition/usage/specialization grammar is not yet unified
2. body-level modeling is still permissive in several modules
3. action/state/expression coverage is still subset-oriented compared with the full grammar
4. language-server recovery quality still needs hardening beyond the current solid baseline
5. semantic conformance remains largely out of scope

## Recommended next work

If the goal is deeper parser fidelity rather than just keeping the current suites green, the highest-value work is:

1. introduce a more explicit shared definition/usage/specialization layer
2. replace permissive body skipping with real grammar parsing in the remaining high-value modules
3. continue expanding expression coverage
4. deepen behavioral grammar for actions and states
5. continue language-server hardening: more specific diagnostics, broader error-node coverage, and more recovery-focused tests

## Summary

The parser now covers a broad, validated SysML v2 subset and passes the repo's strict library and validation gates. The main story is no longer "large language families are missing." The main story is that the parser has broad declaration coverage, but still needs deeper body parsing, a better shared grammar foundation, and stronger editor-oriented recovery to approach full grammar fidelity.
