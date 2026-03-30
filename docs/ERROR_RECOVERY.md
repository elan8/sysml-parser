# Error Recovery in the SysML Parser

This document describes the current recovery architecture after the language-server-oriented recovery improvements.

## Current Model

The parser now uses a **hybrid recovery strategy**:

1. **Grammar-level recovery inside body parsers**
2. **Top-level recovery in `parse_with_diagnostics()`**
3. **Explicit AST error nodes for recovered regions**

This is a meaningful step toward language-server use because malformed regions are no longer only "skipped"; they can now remain visible in the AST.

## Core Behavior

### 1. Grammar-level recovery

Several body parsers recover locally instead of immediately aborting their enclosing construct:

- package bodies
- part definition bodies
- part usage bodies
- requirement bodies
- action definition bodies
- action usage bodies
- state bodies
- use case bodies

These parsers now:

1. try to parse the next known body element
2. if parsing fails at a plausible body-element starter, recover locally
3. skip the malformed statement or block
4. resynchronize to the next likely body element or closing `}`
5. insert an AST error node covering the skipped region

Relevant files:

- [`src/parser/package.rs`](C:\Git\sysml-parser\src\parser\package.rs)
- [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)
- [`src/parser/requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)
- [`src/parser/action.rs`](C:\Git\sysml-parser\src\parser\action.rs)
- [`src/parser/state.rs`](C:\Git\sysml-parser\src\parser\state.rs)
- [`src/parser/usecase.rs`](C:\Git\sysml-parser\src\parser\usecase.rs)

### 2. Shared sync-point helpers

Recovery now relies on shared helpers in [`src/parser/lex.rs`](C:\Git\sysml-parser\src\parser\lex.rs):

- `skip_statement_or_block()`
- `skip_to_next_body_element_or_end()`
- `recover_body_element()`
- shared starter tables such as:
  - `PACKAGE_BODY_STARTERS`
  - `PART_BODY_STARTERS`
  - `REQUIREMENT_BODY_STARTERS`
  - `ACTION_BODY_STARTERS`
  - `STATE_BODY_STARTERS`
  - `USE_CASE_BODY_STARTERS`

This makes recovery behavior more consistent across grammar scopes.

### 3. AST error nodes

Recovered regions are represented with `ParseErrorNode` in [`src/ast.rs`](C:\Git\sysml-parser\src\ast.rs).

Error-node variants currently exist in:

- `PackageBodyElement`
- `PartDefBodyElement`
- `PartUsageBodyElement`
- `RequirementDefBodyElement`
- `ActionDefBodyElement`
- `ActionUsageBodyElement`
- `StateDefBodyElement`
- `UseCaseDefBodyElement`

Each `ParseErrorNode` currently stores:

- `message`
- `code`
- `expected`
- `found`
- `suggestion`

### 4. Diagnostics from the AST

`parse_with_diagnostics()` in [`src/parser/mod.rs`](C:\Git\sysml-parser\src\parser\mod.rs) now does two things:

1. collects top-level parse failures using the outer recovery loop
2. traverses the resulting AST and turns recovery nodes into `ParseError` diagnostics

This is important because many syntax problems are now captured locally inside the grammar rather than surfacing only as top-level failures.

## Top-level Recovery

Top-level recovery still exists for cases where parsing cannot even build the surrounding root/package structure.

The outer loop in `parse_with_diagnostics()`:

1. parses one root element at a time
2. on error, records a diagnostic conditionally
3. skips to the next sync point
4. continues parsing

This is still useful, but it is no longer the only or primary recovery mechanism for nested content.

## Current Strengths

- recovery is closer to the grammar than before
- malformed nested content can remain visible in the AST
- diagnostics can now originate from recovered body regions
- shared sync helpers reduce module-specific recovery drift
- recovery loops explicitly guard against zero progress

## Current Limitations

### 1. No central diagnostic state threaded through parsing

Diagnostics are still collected after parsing or at top level, not directly emitted from every parser via shared state.

That means the parser is more resilient, but not yet fully built around a "parsing never fails" architecture.

### 2. Error nodes are not yet everywhere

The most important body scopes are covered, but not every grammar scope or nested construct has explicit error nodes yet.

### 3. Some diagnostics remain generic

Recovery-node diagnostics now have more specific codes than before, but many parser errors still use generic `nom`-derived messages such as:

- `expected keyword or token`
- `parse error`

### 4. Recovery still depends on starter tables

Starter tables are much better than snippet heuristics, but they are still manually curated. They must evolve with grammar coverage.

## Invariants

The parser should preserve these recovery invariants:

- every recovery step must either consume input or stop
- malformed nested content should not poison later siblings unnecessarily
- recovered regions should be visible in the AST where practical
- `parse_with_diagnostics()` should produce partial AST + diagnostics together
- invalid surveillance fixtures such as `test {}` must still be reported as real errors
- parser entrypoints (`parse`, `parse_with_diagnostics`) must never panic on user input; malformed input must be surfaced as `Err` and/or diagnostics

## Tests

Recovery behavior is currently exercised by:

- surveillance invalid-fixture tests in [`tests/validation/surveillance_drone.rs`](C:\Git\sysml-parser\tests\validation\surveillance_drone.rs)
- parser recovery tests in [`tests/parser_tests.rs`](C:\Git\sysml-parser\tests\parser_tests.rs)
- the full validation suite

The newer parser tests explicitly verify:

- sibling recovery after malformed members
- AST error node insertion
- local diagnostics generated from recovery nodes
- panic-safety over malformed corpora and generated arbitrary text

Enforcement points:

- compile-time lint policy in [`src/lib.rs`](C:\Git\sysml-parser\src\lib.rs) denies `unwrap`, `expect`, and `panic!` in non-test code paths
- panic-safety integration tests in [`tests/parser_panic_safety.rs`](C:\Git\sysml-parser\tests\parser_panic_safety.rs)

## Recommended Next Steps

### Short term

- add AST error nodes to more grammar scopes as needed
- make construct-specific diagnostics more precise
- remove or narrow remaining generic top-level heuristics where local recovery already covers the case

### Medium term

- move toward parser-state-based diagnostic accumulation instead of post-pass extraction
- make error codes and suggestions more systematic
- consider dedicated error-node variants for especially important grammar categories

### Long term

- evaluate a more explicit resilient-parser architecture, such as `expect()`-style combinators or richer error trees

## Summary

The parser is now substantially closer to being language-server-usable than the earlier top-level-only recovery model:

- local recovery exists in important grammar scopes
- recovered regions can survive as AST nodes
- diagnostics can be derived from those recovered regions

It is not yet a fully resilient parser architecture, but it now has the right structural pieces to keep improving in that direction.
