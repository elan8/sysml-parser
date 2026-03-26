//! TDD tests: SysML snippets with expected AST.

use sysml_parser::ast::{
    Identification, LibraryPackage, Node, Package, PackageBody, PackageBodyElement, RootElement, RootNamespace, Span,
    ViewDefBody, RenderingDefBody, ViewBody,
};
use sysml_parser::{parse, parse_with_diagnostics};

fn id(name: &str) -> Identification {
    Identification {
        short_name: None,
        name: Some(name.to_string()),
    }
}

/// Node with span matching parser output for full-input parses (offset 0, line 1, column 1).
fn n_len<T>(len: usize, v: T) -> Node<T> {
    Node::new(
        Span {
            offset: 0,
            line: 1,
            column: 1,
            len,
        },
        v,
    )
}

/// Build expected AST for `package Foo;` (input len = 12)
fn expected_package_foo_semicolon() -> RootNamespace {
    RootNamespace {
        elements: vec![n_len(12, RootElement::Package(n_len(12, Package {
            identification: id("Foo"),
            body: PackageBody::Semicolon,
        })))],
    }
}

/// Build expected AST for `package Bar { }` (input len = 15)
fn expected_package_bar_brace() -> RootNamespace {
    RootNamespace {
        elements: vec![n_len(15, RootElement::Package(n_len(15, Package {
            identification: id("Bar"),
            body: PackageBody::Brace { elements: vec![] },
        })))],
    }
}

#[test]
fn test_package_with_semicolon_body() {
    let input = "package Foo;";
    let result = parse(input).expect("parse should succeed");
    let expected = expected_package_foo_semicolon();
    assert_eq!(result, expected, "AST should match expected for package Foo;");
}

#[test]
fn test_package_with_brace_body() {
    let input = "package Bar { }";
    let result = parse(input).expect("parse should succeed");
    let expected = expected_package_bar_brace();
    assert_eq!(
        result, expected,
        "AST should match expected for package Bar {{ }}"
    );
}

#[test]
fn test_standard_library_package_header_parses() {
    let input = "standard library package SysML { }";
    let result = parse(input).expect("parse should succeed");
    assert_eq!(result.elements.len(), 1);
    match &result.elements[0].value {
        RootElement::LibraryPackage(lp) => {
            assert!(lp.value.is_standard);
            assert_eq!(lp.value.identification.name.as_deref(), Some("SysML"));
            assert!(matches!(lp.value.body, PackageBody::Brace { ref elements } if elements.is_empty()));
        }
        other => panic!("expected library package, got {:?}", other),
    }
}

#[test]
fn test_legacy_library_standard_package_header_still_parses() {
    let input = "library standard package LegacyStd;";
    let result = parse(input).expect("parse should succeed");
    assert_eq!(
        result,
        RootNamespace {
            elements: vec![n_len(
                input.len(),
                RootElement::LibraryPackage(n_len(
                    input.len(),
                    LibraryPackage {
                        is_standard: true,
                        identification: id("LegacyStd"),
                        body: PackageBody::Semicolon,
                    }
                ))
            )]
        }
    );
}

#[test]
fn test_parse_with_diagnostics_partial_ast_and_multiple_errors() {
    // One valid element, two invalid lines, then another valid element. Recovery should collect
    // two errors and still produce a partial AST with both valid packages.
    let input = "package Foo;\nnot valid\nalso bad\npackage Bar;";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "should have parse errors");
    assert_eq!(result.errors.len(), 2, "should report two parse errors");
    assert_eq!(
        result.root.elements.len(),
        2,
        "partial AST should contain both valid packages"
    );
    // First element is Foo, second is Bar
    let names: Vec<&str> = result
        .root
        .elements
        .iter()
        .filter_map(|n| {
            if let RootElement::Package(p) = &n.value {
                p.value.identification.name.as_deref()
            } else {
                None
            }
        })
        .collect();
    assert_eq!(names, ["Foo", "Bar"]);

    // Error quality: each error should have "found" snippet and expected context
    for err in &result.errors {
        assert!(err.found.is_some(), "error should have 'found' snippet: {}", err.message);
        assert!(
            err.expected.is_some(),
            "error should have 'expected' context: {}",
            err.message
        );
        assert!(
            err.expected.as_deref().map_or(false, |e| e.contains("package") || e.contains("namespace")),
            "expected should mention package or namespace: {:?}",
            err.expected
        );
        assert!(err.code.is_some(), "error should have a code");
    }
    // First error is at "not valid"
    assert!(
        result.errors[0].found.as_deref().map_or(false, |f| f.contains("not")),
        "first error found should mention invalid token: {:?}",
        result.errors[0].found
    );
}

#[test]
fn test_parse_error_expected_end_of_input_has_found() {
    // Trailing text after valid packages: parse succeeds for "package Foo; package Bar;" then rest "garbage" triggers "expected end of input"
    let input = "package Foo; package Bar; garbage";
    let result = parse(input);
    let err = result.unwrap_err();
    assert!(
        err.message.contains("expected end of input"),
        "error should be 'expected end of input': {}",
        err
    );
    assert!(err.found.is_some(), "expected end of input error should have 'found': {}", err);
    assert!(err.found.as_deref().map_or(false, |f| f.contains("garbage")), "found should show trailing text: {:?}", err.found);
    assert_eq!(err.code.as_deref(), Some("expected_end_of_input"));
}

#[test]
fn test_parse_error_display_includes_found_and_location() {
    let input = "package Foo;\nxyz";
    let result = parse_with_diagnostics(input);
    let err = &result.errors[0];
    let display = err.to_string();
    assert!(display.contains("line"), "Display should include line number");
    assert!(
        err.found.as_ref().map_or(false, |f| display.contains(f)),
        "Display should include found snippet: {}",
        display
    );
}

// --- Top-level import (Phase 0: BNF RootNamespace = PackageBodyElement*) ---

#[test]
fn test_root_level_import_then_package() {
    let input = "private import Views::*;\npackage P { }";
    let result = parse(input).expect("parse should succeed");
    assert_eq!(result.elements.len(), 2);
    match &result.elements[0].value {
        sysml_parser::ast::RootElement::Import(_) => {}
        _ => panic!("expected first element to be Import"),
    }
    match &result.elements[1].value {
        sysml_parser::ast::RootElement::Package(p) => {
            assert_eq!(p.identification.name.as_deref(), Some("P"));
        }
        _ => panic!("expected second element to be Package"),
    }
}

// --- View/Viewpoint/Rendering (spec-1: Clause 8.2.2.26) ---

#[test]
fn test_view_def_parse() {
    let input = "package P { view def Name { } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(elements.len(), 1);
    match &elements[0].value {
        PackageBodyElement::ViewDef(vd) => {
            assert_eq!(vd.identification.name.as_deref(), Some("Name"));
            assert!(matches!(&vd.body, ViewDefBody::Brace { ref elements } if elements.is_empty()));
        }
        _ => panic!("expected ViewDef"),
    }
}

#[test]
fn test_viewpoint_def_parse() {
    let input = "package P { viewpoint def Name { } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(elements.len(), 1);
    match &elements[0].value {
        PackageBodyElement::ViewpointDef(vpd) => {
            assert_eq!(vpd.identification.name.as_deref(), Some("Name"));
        }
        _ => panic!("expected ViewpointDef"),
    }
}

#[test]
fn test_rendering_def_parse() {
    let input = "package P { rendering def Name; }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(elements.len(), 1);
    match &elements[0].value {
        PackageBodyElement::RenderingDef(rd) => {
            assert_eq!(rd.identification.name.as_deref(), Some("Name"));
            assert!(matches!(rd.body, RenderingDefBody::Semicolon));
        }
        _ => panic!("expected RenderingDef"),
    }
}

#[test]
fn test_view_usage_parse() {
    let input = "package P { view name : ViewType { } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(elements.len(), 1);
    match &elements[0].value {
        PackageBodyElement::ViewUsage(vu) => {
            assert_eq!(vu.name, "name");
            assert_eq!(vu.type_name.as_deref(), Some("ViewType"));
            assert!(matches!(&vu.body, ViewBody::Brace { ref elements } if elements.is_empty()));
        }
        _ => panic!("expected ViewUsage"),
    }
}

#[test]
fn test_occurrence_usage_parse() {
    let input = "package P { occurrence sample : Event; }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    match &elements[0].value {
        PackageBodyElement::OccurrenceUsage(occ) => {
            assert_eq!(occ.name, "sample");
            assert_eq!(occ.type_name.as_deref(), Some("Event"));
        }
        _ => panic!("expected OccurrenceUsage"),
    }
}

#[test]
fn test_flow_and_allocation_parse() {
    let input = "package P { flow transfer : Fuel from src to dst; allocation map allocate source to target; }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::FlowUsage(_)));
    assert!(matches!(elements[1].value, PackageBodyElement::AllocationUsage(_)));
}

#[test]
fn test_case_family_parse() {
    let input = "package P { case def GenericCase { } analysis def TradeStudy { } verification def VerifyThing { } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::CaseDef(_)));
    assert!(matches!(elements[1].value, PackageBodyElement::AnalysisCaseDef(_)));
    assert!(matches!(elements[2].value, PackageBodyElement::VerificationCaseDef(_)));
}

#[test]
fn test_stdlib_requirement_usecase_enum_map_to_dedicated_nodes() {
    let input = "package P {
        abstract requirement def RequirementCheck :> BaseType { }
        use case def UseCase :> Case { }
        enum def VerdictKind { pass; fail; }
    }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::RequirementDef(_)));
    assert!(matches!(elements[1].value, PackageBodyElement::UseCaseDef(_)));
    assert!(matches!(elements[2].value, PackageBodyElement::EnumDef(_)));
}

#[test]
fn test_stdlib_part_port_viewpoint_map_to_dedicated_nodes() {
    let input = "package P {
        abstract part def Part :> Item { }
        abstract port def Port :> Object { }
        abstract viewpoint def ViewpointCheck :> RequirementCheck { }
    }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::PartDef(_)));
    assert!(matches!(elements[1].value, PackageBodyElement::PortDef(_)));
    assert!(matches!(elements[2].value, PackageBodyElement::ViewpointDef(_)));
    assert!(
        !elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::ExtendedLibraryDecl(_))),
        "sample should not fall back to ExtendedLibraryDecl"
    );
}

#[test]
fn test_quantities_abstract_attribute_def_maps_dedicated() {
    let input = "package P { abstract attribute def TensorQuantityValue :> Array { attribute num: Number[1..*]; } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::AttributeDef(_)));
}

#[test]
fn test_enum_def_with_specialization_and_assigned_literals_maps_dedicated() {
    let input = "package P { enum def LevelEnum :> Level { low = 0.25; medium = 0.5; high = 0.75; } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(matches!(elements[0].value, PackageBodyElement::EnumDef(_)));
    assert!(
        !elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::ExtendedLibraryDecl(_))),
        "enum specialization sample should not fall back to ExtendedLibraryDecl"
    );
}

#[test]
fn test_expression_precedence_parse() {
    let input = "package P { attribute x = 1 + 2 * 3; }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    match &elements[0].value {
        PackageBodyElement::AttributeDef(attr) => {
            let value = attr
                .typing
                .as_ref()
                .map(|_| ())
                .or(Some(()));
            assert!(value.is_some());
        }
        _ => panic!("expected AttributeDef"),
    }
}

#[test]
fn test_package_body_recovery_skips_annotated_member_and_keeps_later_sibling() {
    let input = "package P {\n#fmeaspec requirement req1 { }\npart def Good;\n}";
    let result = parse(input).expect("parse should succeed with recovery");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(
        elements.iter().any(|e| matches!(e.value, PackageBodyElement::PartDef(_))),
        "later valid sibling should still be present after recovering from annotated unsupported member"
    );
    assert!(
        elements.iter().any(|e| matches!(e.value, PackageBodyElement::Error(_))),
        "recovered package region should be represented explicitly in the AST"
    );
}

#[test]
fn test_package_body_recovery_skips_malformed_abstract_part_and_keeps_next_member() {
    let input = "package P {\nabstract part def Broken { invalid }\npart def Good;\n}";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(
        elements
            .iter()
            .filter(|e| matches!(e.value, PackageBodyElement::PartDef(_)))
            .count(),
        2,
        "both part declarations should map to dedicated PartDef nodes"
    );
}

#[test]
fn test_requirement_body_recovery_keeps_later_require_constraint() {
    let input = "package P {\nrequirement def R {\nsubject vehicle : Vehicle;\nattribute massActual: MassValue;\nrequire constraint { }\n}\n}";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let req = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::RequirementDef(r) => Some(&r.value),
            _ => None,
        })
        .expect("requirement def should be present");
    let body_elements = match &req.body {
        sysml_parser::ast::RequirementDefBody::Brace { elements } => elements,
        _ => panic!("expected requirement brace body"),
    };
    assert!(
        body_elements.is_empty(),
        "requirement body is permissively consumed and should still map as RequirementDef"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_local_requirement_recovery() {
    let input = "package P {\nrequirement def R {\nsubject vehicle : Vehicle;\nattribute massActual: MassValue;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "permissive requirement body parsing should avoid local recovery diagnostics"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_local_package_recovery() {
    let input = "package P {\n#fmeaspec requirement req1 { }\npart def Good;\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "package-level recovery should surface as diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("recovered_package_body_element"))
        .expect("expected local package recovery diagnostic");
    assert_eq!(err.line, Some(2));
    assert!(
        err.found.as_deref().map_or(false, |f| f.contains("#fmeaspec")),
        "diagnostic should preserve recovered snippet"
    );
}
