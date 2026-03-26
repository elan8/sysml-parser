# AST Migration Notes

## 2026-03 library-coverage update

To support strict parsing of the full SysML/KerML library set without top-level recovery noise, package-body fallback declarations are now split into BNF-aligned modeled families:

- `PackageBodyElement::KermlSemanticDecl(Node<KermlSemanticDecl>)`
- `PackageBodyElement::KermlFeatureDecl(Node<KermlFeatureDecl>)`
- `PackageBodyElement::ExtendedLibraryDecl(Node<ExtendedLibraryDecl>)`

Where each node carries:

- `bnf_production`: primary BNF starter this declaration matched (e.g. `function`, `behavior`, `datatype`, `occurrence`, `expr`, `feature`, ...).
- `text`: original declaration text fragment that was consumed.

### Compatibility impact

- These are additive enum variants, so existing exhaustive matches over `PackageBodyElement` must add corresponding branches.
- Existing node shapes for all previously supported dedicated constructs remain unchanged.
- Diagnostics behavior changed for unsupported package-level declarations: many declarations that previously produced `recovered_package_body_element` diagnostics are now represented as modeled declaration nodes and no longer emit that recovery diagnostic.
- For several standard-library-heavy declaration families, parser coverage now prefers dedicated top-level node mapping (e.g. `RequirementDef`, `UseCaseDef`, `PartDef`, `PortDef`, `ViewpointDef`, `EnumDef`) while allowing permissive body consumption to avoid fallback at package level.

### Suggested downstream handling (Spec42)

- Treat modeled declaration nodes as non-fatal declaration nodes.
- For indexing/symbol extraction pipelines, either:
  - skip these nodes, or
  - parse each node `.text` with downstream heuristics if partial symbol extraction is desired.
- If downstream logic depends on fully detailed nested body members, treat empty/partial body element vectors as "parsed declaration shell" rather than parse failure; the declaration kind is now authoritative at package level.
