# Language Server Readiness Backlog

This backlog captures the work needed to make the parser's error handling robust enough for language-server use.

## Goal

The parser should support editor workflows such as:

- partial AST construction in the presence of syntax errors
- accurate, stable diagnostics with useful ranges
- resilient parsing that localizes damage to the smallest possible region
- stable downstream features such as outline, symbols, hover, semantic tokens, and navigation

## Current status

The parser already has a meaningful resilient-parsing base:

- local recovery exists in major body parsers
- `ParseErrorNode` variants exist in the highest-value body ASTs
- `parse_with_diagnostics()` returns partial AST + diagnostics together
- panic-safety and recovery behavior are covered by dedicated tests plus the validation suites

The remaining backlog below is the still-actionable work after those completed steps.

## Priority P0

### 1. Tighten diagnostics emitted from current recovery paths

The current recovery architecture is in place, but many diagnostics are still generic or derived from `nom` defaults in [`src/parser/mod.rs`](C:\Git\sysml-parser\src\parser\mod.rs).

Expected outcome:

- more precise `expected` messages
- more targeted `suggestion` text
- less reliance on generic messages such as `expected keyword or token`

### 2. Expand AST error-node coverage to additional grammar scopes

The most important body scopes already have error nodes, but coverage is not yet uniform across all nested grammar families.

Priority candidates:

- view/rendering-related bodies
- constraint/calculation internals
- additional nested requirement and state substructures
- parser areas that still recover by skipping without preserving a stable AST placeholder

Expected outcome:

- invalid regions remain visible to editor features more consistently
- recovery behavior becomes easier to reason about and debug

### 3. Remove recovery paths that still silently reshape invalid input

Some recovery is still tolerant in ways that help parsing continue but may hide the exact structural problem.

Expected outcome:

- better diagnostic fidelity
- less surprising editor behavior around malformed blocks and unmatched delimiters

### 4. Add recovery-focused tests per construct

Expand tests beyond end-to-end fixtures in [`tests/parser_tests.rs`](C:\Git\sysml-parser\tests\parser_tests.rs) and validation tests.

Add dedicated malformed-input cases for:

- package bodies
- part bodies
- attributes
- requirements
- use cases
- state machines
- views
- constraints

Each test should check:

- error location
- error code or expected message
- partial AST remains usable
- later siblings still parse
- no infinite loop

## Priority P1

### 5. Normalize recovery patterns across parser modules

Several modules already contain local recovery loops:

- [`src/parser/package.rs`](C:\Git\sysml-parser\src\parser\package.rs)
- [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)
- [`src/parser/action.rs`](C:\Git\sysml-parser\src\parser\action.rs)
- [`src/parser/state.rs`](C:\Git\sysml-parser\src\parser\state.rs)
- [`src/parser/requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)
- [`src/parser/usecase.rs`](C:\Git\sysml-parser\src\parser\usecase.rs)

Unify them around shared patterns or combinators so they all:

- guarantee forward progress
- report errors consistently
- sync at the right structural boundary

Expected outcome:

- more predictable parser behavior
- lower maintenance cost

### 6. Add or tighten grammar-aware sync helpers where recovery is still coarse

[`src/parser/lex.rs`](C:\Git\sysml-parser\src\parser\lex.rs) already provides shared recovery helpers. Extend them only where current scopes still over-skip or recover at the wrong structural boundary.

Examples:

- finer-grained sync for view bodies
- finer-grained sync for constraint/calculation internals
- narrower helpers for parser areas that still rely on generic brace or statement skipping

Expected outcome:

- more accurate recovery than generic line-based skipping
- less over-skipping into later constructs

### 7. Make spans robust under recovery

Review span generation in [`src/ast.rs`](C:\Git\sysml-parser\src\ast.rs) and parser modules.

Ensure that:

- recovered nodes have meaningful ranges
- error nodes carry useful spans
- LSP ranges remain stable even for malformed input

Expected outcome:

- reliable editor highlighting and navigation around syntax errors

## Priority P2

### 8. Separate strict parsing and resilient parsing more clearly

The API in [`src/lib.rs`](C:\Git\sysml-parser\src\lib.rs) already distinguishes `parse()` from `parse_with_diagnostics()`.

Make the internal architecture reflect that more explicitly:

- strict parse path for CI and validation
- resilient parse path for language-server/editor scenarios

Expected outcome:

- less coupling between test-suite parsing behavior and editor recovery behavior

### 9. Evaluate richer error infrastructure

Investigate whether to adopt:

- `nom-supreme` `ErrorTree`
- custom `expect()`-style combinators
- explicit parser state for accumulated diagnostics

Expected outcome:

- richer diagnostics
- less ad hoc error plumbing
- better foundation for long-term parser evolution

## Suggested Delivery Plan

### Phase 1

- tighten diagnostics
- expand error-node coverage
- remove silent reshaping paths

### Phase 2

- expand recovery tests
- normalize recovery patterns across modules
- add narrower sync helpers where current recovery is still coarse

### Phase 3

- harden spans
- separate strict and resilient parse paths

### Phase 4

- evaluate richer error infrastructure

## Definition of Done for Language-Server Use

The parser should be considered language-server-ready when:

- malformed documents still produce a useful partial AST
- diagnostics are local, stable, and specific
- later siblings remain parseable after common syntax mistakes
- no known infinite-loop or zero-progress recovery paths remain
- recovery behavior is covered by targeted tests
- error handling architecture is documented and consistent across modules
