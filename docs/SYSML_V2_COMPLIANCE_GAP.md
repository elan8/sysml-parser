# SysML v2 Compliance Gap Analysis

This document compares the current parser implementation against the SysML v2 textual grammar in [`sysml-v2-release/bnf/SysML-textual-bnf.kebnf`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf).

## Scope

This is a **parser compliance** analysis, not a full semantic compliance analysis.

It answers:

- which SysML v2 textual grammar areas are already implemented
- which are only partially supported
- which are still missing
- which parser architecture gaps block broader grammar coverage

It does **not** claim semantic conformance for:

- name resolution
- type checking
- cross-reference validation
- KerML/SysML well-formedness constraints
- derived or implicit abstract-syntax behavior

## High-Level Conclusion

The parser is currently:

- strong enough for the existing validation suite
- usable for a meaningful SysML v2 subset
- not yet fully compliant with the full textual grammar in the official `.kebnf`

The biggest remaining gaps are:

1. the generic Definition/Usage/Specialization grammar layer is only partially modeled
2. several large language families are still missing entirely
3. action/state behavioral syntax is implemented as a subset
4. the expression parser only covers a simplified subset of `OwnedExpression`
5. several modules still skip bodies instead of parsing them according to the grammar

## Current Parser Surface

The current parser modules are in [`src/parser`](C:\Git\sysml-parser\src\parser):

- [`action.rs`](C:\Git\sysml-parser\src\parser\action.rs)
- [`alias.rs`](C:\Git\sysml-parser\src\parser\alias.rs)
- [`attribute.rs`](C:\Git\sysml-parser\src\parser\attribute.rs)
- [`connection.rs`](C:\Git\sysml-parser\src\parser\connection.rs)
- [`constraint.rs`](C:\Git\sysml-parser\src\parser\constraint.rs)
- [`dependency.rs`](C:\Git\sysml-parser\src\parser\dependency.rs)
- [`enumeration.rs`](C:\Git\sysml-parser\src\parser\enumeration.rs)
- [`expr.rs`](C:\Git\sysml-parser\src\parser\expr.rs)
- [`import.rs`](C:\Git\sysml-parser\src\parser\import.rs)
- [`individual.rs`](C:\Git\sysml-parser\src\parser\individual.rs)
- [`interface.rs`](C:\Git\sysml-parser\src\parser\interface.rs)
- [`item.rs`](C:\Git\sysml-parser\src\parser\item.rs)
- [`metadata.rs`](C:\Git\sysml-parser\src\parser\metadata.rs)
- [`metadata_annotation.rs`](C:\Git\sysml-parser\src\parser\metadata_annotation.rs)
- [`occurrence.rs`](C:\Git\sysml-parser\src\parser\occurrence.rs)
- [`package.rs`](C:\Git\sysml-parser\src\parser\package.rs)
- [`part.rs`](C:\Git\sysml-parser\src\parser\part.rs)
- [`port.rs`](C:\Git\sysml-parser\src\parser\port.rs)
- [`requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)
- [`state.rs`](C:\Git\sysml-parser\src\parser\state.rs)
- [`usecase.rs`](C:\Git\sysml-parser\src\parser\usecase.rs)
- [`view.rs`](C:\Git\sysml-parser\src\parser\view.rs)

The AST lives in [`src/ast.rs`](C:\Git\sysml-parser\src\ast.rs).

## Compliance Matrix

### 1. Packages, imports, annotations

Status: `Mostly implemented`

Spec areas:

- `Package`, `LibraryPackage`, `PackageBody`, `PackageBodyElement`
- `AliasMember`
- `Import`
- `Comment`, `Documentation`, `TextualRepresentation`

Spec references:

- [`SysML-textual-bnf.kebnf#L109`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L109)
- [`SysML-textual-bnf.kebnf#L142`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L142)
- [`SysML-textual-bnf.kebnf#L149`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L149)
- [`SysML-textual-bnf.kebnf#L82`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L82)

Current implementation:

- [`src/parser/package.rs`](C:\Git\sysml-parser\src\parser\package.rs)
- [`src/parser/import.rs`](C:\Git\sysml-parser\src\parser\import.rs)
- [`src/parser/alias.rs`](C:\Git\sysml-parser\src\parser\alias.rs)
- [`src/parser/requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)

Notes:

- package structure and top-level dispatch are in good shape
- imports support the common forms, including filter-package style
- documentation/comments/textual representations exist
- some relationship bodies are still simplified and may skip content

### 2. Generic definition/usage framework

Status: `Partially implemented`

Spec areas:

- `DefinitionPrefix`
- `DefinitionDeclaration`
- `DefinitionBodyItem`
- `UsagePrefix`
- `UsageDeclaration`
- `UsageCompletion`
- `FeatureSpecializationPart`
- `SubclassificationPart`

Spec references:

- [`SysML-textual-bnf.kebnf#L225`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L225)
- [`SysML-textual-bnf.kebnf#L237`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L237)
- [`SysML-textual-bnf.kebnf#L308`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L308)
- [`SysML-textual-bnf.kebnf#L417`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L417)

Current implementation:

- spread across [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs), [`src/parser/port.rs`](C:\Git\sysml-parser\src\parser\port.rs), [`src/parser/attribute.rs`](C:\Git\sysml-parser\src\parser\attribute.rs), [`src/parser/action.rs`](C:\Git\sysml-parser\src\parser\action.rs), [`src/parser/state.rs`](C:\Git\sysml-parser\src\parser\state.rs)

Gap:

- there is no single grammar layer for the generic definition/usage model
- prefixes and specialization syntax are implemented per construct
- many legal combinations from the spec are therefore unavailable

Impact:

- this is the single biggest structural blocker for full SysML v2 coverage

### 3. Attributes

Status: `Partially implemented`

Spec areas:

- `AttributeDefinition`
- `AttributeUsage`

Spec reference:

- [`SysML-textual-bnf.kebnf#L510`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L510)

Current implementation:

- [`src/parser/attribute.rs`](C:\Git\sysml-parser\src\parser\attribute.rs)

Notes:

- basic defs and usages work
- shorthand forms are supported
- value parsing is still simplified in several places
- full feature specialization coverage is not present

### 4. Enumerations

Status: `Mostly implemented`

Spec area:

- `EnumerationDefinition`

Spec reference:

- [`SysML-textual-bnf.kebnf#L518`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L518)

Current implementation:

- [`src/parser/enumeration.rs`](C:\Git\sysml-parser\src\parser\enumeration.rs)

Notes:

- basic enum definitions are covered
- advanced interactions with generic specialization infrastructure still depend on the larger generic-definition gap

### 5. Occurrences, individuals, portions, events

Status: `Partially implemented to missing`

Spec areas:

- `OccurrenceDefinition`
- `OccurrenceUsage`
- `IndividualDefinition`
- `IndividualUsage`
- `PortionUsage`
- `snapshot`
- `timeslice`
- `EventOccurrenceUsage`
- `SourceSuccessionMember`

Spec references:

- [`SysML-textual-bnf.kebnf#L548`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L548)

Current implementation:

- defs: [`src/parser/occurrence.rs`](C:\Git\sysml-parser\src\parser\occurrence.rs)
- individuals: [`src/parser/individual.rs`](C:\Git\sysml-parser\src\parser\individual.rs)

Gap:

- occurrence definitions exist, but occurrence usages are not covered as a general family
- `snapshot`, `timeslice`, `portion`, and event occurrence syntax are missing
- source succession is only partially represented in specific constructs

### 6. Items

Status: `Mostly implemented`

Spec areas:

- `ItemDefinition`
- `ItemUsage`

Spec reference:

- [`SysML-textual-bnf.kebnf#L611`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L611)

Current implementation:

- `item def` exists in [`src/parser/item.rs`](C:\Git\sysml-parser\src\parser\item.rs)

Gap:

- general `item` usage support is not represented as a first-class parser family

### 7. Parts

Status: `Partially implemented`

Spec areas:

- `PartDefinition`
- `PartUsage`

Spec reference:

- [`SysML-textual-bnf.kebnf#L620`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L620)

Current implementation:

- [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)

Notes:

- this is one of the strongest parts of the parser today
- common `part def` and `part` usage patterns work
- body recovery is relatively mature

Gap:

- still not aligned with the full generic usage/specialization grammar
- not all prefix combinations and feature-specialization combinations are supported

### 8. Ports

Status: `Partially implemented`

Spec areas:

- `PortDefinition`
- `PortUsage`
- conjugated typing

Spec references:

- [`SysML-textual-bnf.kebnf#L628`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L628)

Current implementation:

- [`src/parser/port.rs`](C:\Git\sysml-parser\src\parser\port.rs)

Notes:

- core `port def` and `port` usage support exists
- `~Type` style typing is partially supported
- attributes inside port defs now parse

Gap:

- conjugated-port semantics from the spec are not modeled fully
- the implicit conjugated definition behavior from the abstract syntax is not represented
- generic specialization combinations remain incomplete

### 9. Connections and bindings

Status: `Partially implemented`

Spec areas:

- `ConnectionDefinition`
- `ConnectionUsage`
- `BindingConnectorAsUsage`
- `SuccessionAsUsage`
- binary and n-ary connector parts

Spec references:

- [`SysML-textual-bnf.kebnf#L664`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L664)

Current implementation:

- defs: [`src/parser/connection.rs`](C:\Git\sysml-parser\src\parser\connection.rs)
- part-level `bind` and `connect`: [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)

Gap:

- no dedicated general `connection` usage parser
- n-ary connectors are not implemented
- succession syntax is only partially modeled
- connector end semantics are simplified

### 10. Interfaces

Status: `Partially implemented`

Spec areas:

- `InterfaceDefinition`
- `InterfaceUsage`
- interface ends and interface parts

Spec references:

- [`SysML-textual-bnf.kebnf#L720`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L720)
- [`SysML-textual-bnf.kebnf#L757`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L757)

Current implementation:

- [`src/parser/interface.rs`](C:\Git\sysml-parser\src\parser\interface.rs)
- part-level interface usage support in [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)

Notes:

- common `interface def` and typed `interface ... connect ... to ...` patterns work
- `end port` and `~Type` support exist in a useful subset

Gap:

- interface grammar is still subset-only
- n-ary interface parts are missing
- full interface body-item coverage is not implemented

### 11. Allocations

Status: `Missing`

Spec areas:

- `AllocationDefinition`
- `AllocationUsage`

Spec reference:

- [`SysML-textual-bnf.kebnf#L788`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L788)

Current implementation:

- there is no dedicated allocation parser module
- `allocate` exists only as a part/action-level simplified construct in [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)

Gap:

- no grammar-faithful allocation definition/usage support

### 12. Flows and messages

Status: `Mostly missing`

Spec areas:

- `FlowDefinition`
- `FlowUsage`
- `SuccessionFlowUsage`
- `Message`
- flow payloads
- flow ends

Spec references:

- [`SysML-textual-bnf.kebnf#L802`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L802)
- [`SysML-textual-bnf.kebnf#L825`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L825)

Current implementation:

- AST contains a `Flow` type in [`src/ast.rs`](C:\Git\sysml-parser\src\ast.rs)

Gap:

- there is no `flow.rs`
- no actual grammar implementation for flow defs/usages/messages
- payload features and flow-end syntax are absent

### 13. Actions

Status: `Partially implemented`

Spec areas:

- `ActionDefinition`
- `ActionUsage`
- `PerformActionUsage`
- action nodes
- initial nodes
- guarded successions
- `send`, `accept`, `assign`, `terminate`, `if`, `while`, `for`

Spec references:

- [`SysML-textual-bnf.kebnf#L894`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L894)
- [`SysML-textual-bnf.kebnf#L944`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L944)

Current implementation:

- [`src/parser/action.rs`](C:\Git\sysml-parser\src\parser\action.rs)
- some perform syntax also in [`src/parser/part.rs`](C:\Git\sysml-parser\src\parser\part.rs)

Notes:

- basic `action def`
- basic `action` usage
- simple perform forms
- local recovery support

Gap:

- broad behavioral grammar from the spec is still absent
- no full action-node family
- loop/branch/assignment/send/accept coverage is incomplete
- guarded and target succession coverage is only partial

### 14. States and transitions

Status: `Partially implemented`

Spec areas:

- `StateDefinition`
- `StateUsage`
- `ExhibitStateUsage`
- `TransitionUsage`
- entry/do/exit subactions
- target transitions
- triggers, guards, effects
- parallel states

Spec references:

- [`SysML-textual-bnf.kebnf#L1191`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1191)

Current implementation:

- [`src/parser/state.rs`](C:\Git\sysml-parser\src\parser\state.rs)

Notes:

- `state def`, `state`, `transition`, `entry`, `then`, nested state usage all exist in useful form

Gap:

- `do` and `exit` members are not fully covered
- `parallel` is only handled as a loose modifier in some paths
- triggers/effects are simplified
- transition grammar is much richer in the spec than in the implementation

### 15. Calculations and constraints

Status: `Partially implemented`

Spec areas:

- `CalculationDefinition`
- `CalculationUsage`
- `ConstraintDefinition`
- `ConstraintUsage`
- `AssertConstraintUsage`

Spec references:

- [`SysML-textual-bnf.kebnf#L1351`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1351)
- [`SysML-textual-bnf.kebnf#L1378`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1378)

Current implementation:

- [`src/parser/constraint.rs`](C:\Git\sysml-parser\src\parser\constraint.rs)

Notes:

- defs exist
- some body items are supported

Gap:

- usage forms are incomplete
- much of the calculation/constraint internals still depend on simplified expression coverage

### 16. Requirements and concerns

Status: `Partially implemented`

Spec areas:

- `RequirementDefinition`
- `RequirementUsage`
- `SatisfyRequirementUsage`
- `ConcernDefinition`
- `ConcernUsage`
- subjects, frames, verify, actors, stakeholders

Spec references:

- [`SysML-textual-bnf.kebnf#L1400`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1400)
- [`SysML-textual-bnf.kebnf#L1462`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1462)
- [`SysML-textual-bnf.kebnf#L1489`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1489)

Current implementation:

- [`src/parser/requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)

Notes:

- this area is relatively mature
- requirement defs/usages, satisfy, subject, frame, concern usage, and some verification-related forms are present

Gap:

- concern definitions are not a dedicated parser family
- stakeholder support is not complete
- some nested requirement constraint forms still skip internal bodies

### 17. Cases, analysis cases, verification cases

Status: `Mostly missing`

Spec areas:

- `CaseDefinition`
- `CaseUsage`
- `AnalysisCaseDefinition`
- `AnalysisCaseUsage`
- `VerificationCaseDefinition`
- `VerificationCaseUsage`

Spec references:

- [`SysML-textual-bnf.kebnf#L1499`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1499)
- [`SysML-textual-bnf.kebnf#L1529`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1529)
- [`SysML-textual-bnf.kebnf#L1539`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1539)

Current implementation:

- no dedicated parser modules for these families

Gap:

- these language families are currently absent as grammar-faithful parsers

### 18. Use cases

Status: `Partially implemented`

Spec areas:

- `UseCaseDefinition`
- `UseCaseUsage`
- `IncludeUseCaseUsage`

Spec references:

- [`SysML-textual-bnf.kebnf#L1560`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1560)

Current implementation:

- [`src/parser/usecase.rs`](C:\Git\sysml-parser\src\parser\usecase.rs)

Notes:

- basic `use case def`
- basic `use case` usage
- actor/objective/subject subset

Gap:

- `include` use case syntax is missing
- the broader case-family grammar is not unified with use cases

### 19. Views, viewpoints, renderings

Status: `Partially implemented`

Spec areas:

- `ViewDefinition`
- `ViewUsage`
- `ViewpointDefinition`
- `ViewpointUsage`
- `RenderingDefinition`
- `RenderingUsage`
- `Expose`

Spec references:

- [`SysML-textual-bnf.kebnf#L1580`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1580)
- [`SysML-textual-bnf.kebnf#L1632`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1632)
- [`SysML-textual-bnf.kebnf#L1642`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1642)

Current implementation:

- [`src/parser/view.rs`](C:\Git\sysml-parser\src\parser\view.rs)

Notes:

- a useful subset exists
- expose and rendering constructs are present

Gap:

- several forms are simplified
- they still depend on incomplete generic usage support

### 20. Metadata

Status: `Partially implemented`

Spec areas:

- `MetadataDefinition`
- `MetadataUsage`
- `PrefixMetadataAnnotation`
- `PrefixMetadataMember`
- metadata bodies
- `ExtendedDefinition`
- `ExtendedUsage`

Spec references:

- [`SysML-textual-bnf.kebnf#L1652`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1652)
- [`SysML-textual-bnf.kebnf#L1695`](C:\Git\sysml-parser\sysml-v2-release\bnf\SysML-textual-bnf.kebnf#L1695)

Current implementation:

- metadata defs: [`src/parser/metadata.rs`](C:\Git\sysml-parser\src\parser\metadata.rs)
- metadata annotations/usages: [`src/parser/metadata_annotation.rs`](C:\Git\sysml-parser\src\parser\metadata_annotation.rs)

Gap:

- both modules are intentionally simplified
- metadata bodies are largely skipped
- prefix metadata and extended definition/usage support are incomplete

## Cross-Cutting Gaps

### Expressions are still a major blocker

Status: `Partially implemented`

Current implementation:

- [`src/parser/expr.rs`](C:\Git\sysml-parser\src\parser\expr.rs)

Gap:

- no real precedence-aware expression grammar
- `binary_chain()` currently parses all operators in a flat left-associative chain
- many `OwnedExpression` forms from SysML/KerML are not supported

Impact:

- this limits correctness across constraints, calculations, guards, value parts, filters, and more

### Body skipping hides compliance gaps

Several parsers use `skip_until_brace_end()` from [`src/parser/lex.rs`](C:\Git\sysml-parser\src\parser\lex.rs) to accept bodies without actually parsing the grammar inside them.

This is currently used in or influences:

- [`src/parser/metadata.rs`](C:\Git\sysml-parser\src\parser\metadata.rs)
- [`src/parser/occurrence.rs`](C:\Git\sysml-parser\src\parser\occurrence.rs)
- [`src/parser/alias.rs`](C:\Git\sysml-parser\src\parser\alias.rs)
- [`src/parser/import.rs`](C:\Git\sysml-parser\src\parser\import.rs)
- parts of [`src/parser/connection.rs`](C:\Git\sysml-parser\src\parser\connection.rs)
- parts of [`src/parser/requirement.rs`](C:\Git\sysml-parser\src\parser\requirement.rs)

Impact:

- good for recovery and fixture compatibility
- not sufficient for full textual-grammar compliance

### Semantic compliance is mostly out of scope today

Even after the missing syntax is implemented, the parser will still need additional layers for:

- qualified-name resolution
- conjugated port semantics
- feature direction conformance
- implicit/derived abstract-syntax behavior
- KerML constraint enforcement

## Recommended Roadmap Toward Full Compliance

### Phase 1: Build the missing grammar foundation

1. Introduce explicit generic parsers for:
   - `DefinitionPrefix`
   - `UsagePrefix`
   - `SubclassificationPart`
   - `FeatureSpecializationPart`
   - `ValuePart`
2. Refactor construct parsers to reuse that common layer.
3. Expand AST types so the generic grammar has a stable representation.

### Phase 2: Fill the large missing language families

1. Add dedicated parsers for:
   - flows/messages
   - allocations
   - cases/analysis/verification cases
   - occurrence usages and event occurrences
2. Hook them into [`src/parser/package.rs`](C:\Git\sysml-parser\src\parser\package.rs) and relevant body parsers.

### Phase 3: Expand behavioral grammar

1. Extend actions with:
   - send
   - accept
   - assignment
   - terminate
   - if/while/for
   - guarded successions
2. Extend states with:
   - do
   - exit
   - trigger/effect forms
   - full target-transition syntax
   - proper parallel-state support

### Phase 4: Upgrade expression parsing

1. Replace flat binary parsing with precedence-based expression parsing.
2. Add the missing `OwnedExpression` families used by:
   - constraints
   - calculations
   - filters
   - guards
   - value parts

### Phase 5: Replace skipped bodies with real grammar parsing

1. Remove `skip_until_brace_end()`-style acceptance where a real grammar body exists.
2. Add targeted conformance tests per grammar family.

## Suggested Priority Order

If the goal is true parser compliance, I would prioritize work in this order:

1. generic definition/usage grammar layer
2. expressions
3. flows and allocations
4. case/analysis/verification families
5. occurrence usages
6. action/state behavioral expansion
7. metadata/extended definitions
8. cleanup of remaining skipped bodies

## Summary

The parser currently covers an important and useful SysML v2 subset, but it is still some distance from full compliance with the official textual grammar.

The most important fact is that the remaining work is not just "more individual constructs". The largest compliance gap is architectural: the parser does not yet model the generic definition/usage/specialization layer that the SysML v2 grammar uses across almost every major construct family.

Once that foundation exists, the remaining missing language families become much more straightforward to add consistently.
