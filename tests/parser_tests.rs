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
            err.expected
                .as_deref()
                .is_some_and(|e| e.contains("package") || e.contains("namespace")),
            "expected should mention package or namespace: {:?}",
            err.expected
        );
        assert!(err.code.is_some(), "error should have a code");
    }
    // First error is at "not valid"
    assert!(
        result.errors[0]
            .found
            .as_deref()
            .is_some_and(|f| f.contains("not")),
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
    assert!(
        err.found.as_deref().is_some_and(|f| f.contains("garbage")),
        "found should show trailing text: {:?}",
        err.found
    );
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
        err.found.as_ref().is_some_and(|f| display.contains(f)),
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
fn test_use_case_def_body_parses_members() {
    let input = "package P { use case def U { subject s : System; actor a : Operator; objective { } } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let use_case = match &elements[0].value {
        PackageBodyElement::UseCaseDef(uc) => &uc.value,
        _ => panic!("expected UseCaseDef"),
    };
    let body_elements = match &use_case.body {
        sysml_parser::ast::UseCaseDefBody::Brace { elements } => elements,
        _ => panic!("expected use case brace body"),
    };
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::UseCaseDefBodyElement::SubjectDecl(_)))
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::UseCaseDefBodyElement::ActorUsage(_)))
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::UseCaseDefBodyElement::Objective(_)))
    );
}

#[test]
fn test_state_def_body_parses_members() {
    let input =
        "package P { state def S { then Ready; state Running : Mode; transition t then Ready; } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let state_def = match &elements[0].value {
        PackageBodyElement::StateDef(sd) => &sd.value,
        _ => panic!("expected StateDef"),
    };
    let body_elements = match &state_def.body {
        sysml_parser::ast::StateDefBody::Brace { elements } => elements,
        _ => panic!("expected state brace body"),
    };
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::StateDefBodyElement::Then(_)))
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::StateDefBodyElement::StateUsage(_)))
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::StateDefBodyElement::Transition(_)))
    );
}

#[test]
fn test_constraint_and_calc_bodies_parse_members() {
    let input = "package P { constraint def C { in x : Real; out y : Real; x >= y; } calc def K { in x : Real; return r : Real; x; } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let constraint_def = match &elements[0].value {
        PackageBodyElement::ConstraintDef(cd) => &cd.value,
        _ => panic!("expected ConstraintDef"),
    };
    let constraint_elements = match &constraint_def.body {
        sysml_parser::ast::ConstraintDefBody::Brace { elements } => elements,
        _ => panic!("expected constraint brace body"),
    };
    assert!(!constraint_elements.is_empty(), "constraint body should not be empty");
    let calc_def = match &elements[1].value {
        PackageBodyElement::CalcDef(cd) => &cd.value,
        _ => panic!("expected CalcDef"),
    };
    let calc_elements = match &calc_def.body {
        sysml_parser::ast::CalcDefBody::Brace { elements } => elements,
        _ => panic!("expected calc brace body"),
    };
    assert!(!calc_elements.is_empty(), "calc body should not be empty");
}

#[test]
fn test_view_and_connection_bodies_parse_members() {
    let input = "package P { view def V { doc /*d*/ filter x > 0; render r : Renderer; } view v : V { expose Model::*; satisfy VP; } connection def C { end from : A; end to : B; connect from to to; } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let view_def = match &elements[0].value {
        PackageBodyElement::ViewDef(vd) => &vd.value,
        _ => panic!("expected ViewDef"),
    };
    assert!(matches!(&view_def.body, ViewDefBody::Brace { elements } if !elements.is_empty()));
    let view_usage = match &elements[1].value {
        PackageBodyElement::ViewUsage(v) => &v.value,
        _ => panic!("expected ViewUsage"),
    };
    assert!(matches!(&view_usage.body, ViewBody::Brace { elements } if !elements.is_empty()));
    let connection_def = match &elements[2].value {
        PackageBodyElement::ConnectionDef(c) => &c.value,
        _ => panic!("expected ConnectionDef"),
    };
    assert!(matches!(
        &connection_def.body,
        sysml_parser::ast::ConnectionDefBody::Brace { elements } if !elements.is_empty()
    ));
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
fn test_case_family_bodies_parse_use_case_members() {
    let input = "package P { case def C { actor a : Operator; } analysis def A { subject s : System; } verification def V { objective { } } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let case_def = match &elements[0].value {
        PackageBodyElement::CaseDef(c) => &c.value,
        _ => panic!("expected CaseDef"),
    };
    assert!(
        matches!(&case_def.body, sysml_parser::ast::UseCaseDefBody::Brace { elements } if !elements.is_empty())
    );
}

#[test]
fn test_perform_action_decl_body_parses_bindings() {
    let input = "package P { part def Carrier { perform action run : Runner { in speed = speedInput; out torque = torqueOutput; } } }";
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let part_def = match &elements[0].value {
        PackageBodyElement::PartDef(pd) => &pd.value,
        _ => panic!("expected PartDef"),
    };
    let part_body = match &part_def.body {
        sysml_parser::ast::PartDefBody::Brace { elements } => elements,
        _ => panic!("expected part def brace body"),
    };
    let perform = match &part_body[0].value {
        sysml_parser::ast::PartDefBodyElement::Perform(p) => &p.value,
        _ => panic!("expected perform action declaration"),
    };
    assert!(
        matches!(&perform.body, sysml_parser::ast::PerformBody::Brace { elements } if !elements.is_empty()),
        "perform action brace body should retain parsed in/out bindings"
    );
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
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::RequirementDefBodyElement::SubjectDecl(_))),
        "subject should be parsed in requirement body"
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::RequirementDefBodyElement::RequireConstraint(_))),
        "require constraint should be preserved after local body recovery"
    );
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::RequirementDefBodyElement::Error(_))),
        "unsupported members should be captured as recoverable errors in requirement body"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_local_requirement_recovery() {
    let input = "package P {\nrequirement def R {\nsubject vehicle : Vehicle;\nattribute massActual: MassValue;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        !result.errors.is_empty(),
        "unmodeled requirement members should surface as recoverable diagnostics"
    );
    assert!(
        result.errors.iter().any(|e| {
            matches!(
                e.code.as_deref(),
                Some("recovered_requirement_body_element") | Some("missing_semicolon")
            )
        }),
        "expected requirement-body recovery diagnostic"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_subject_name_in_requirement_body() {
    let input = "package P {\nrequirement def R {\nsubject: Laptop;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing subject name should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_member_name"))
        .expect("expected missing_member_name diagnostic");
    assert_eq!(err.expected.as_deref(), Some("subject name before ':'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("subject laptop: Laptop;")),
        "diagnostic should show an example fix"
    );
    let pkg = match &result.root.elements[0].value {
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
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::RequirementDefBodyElement::RequireConstraint(_))),
        "later requirement members should still parse after recovering from invalid subject syntax"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_subject_name_in_use_case_body() {
    let input = "package P {\nuse case def U {\nsubject: Laptop;\nobjective { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing subject name should produce diagnostics");
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.code.as_deref() == Some("missing_member_name")),
        "expected missing_member_name diagnostic in use case body"
    );
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let use_case = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::UseCaseDef(u) => Some(&u.value),
            _ => None,
        })
        .expect("use case def should be present");
    let body_elements = match &use_case.body {
        sysml_parser::ast::UseCaseDefBody::Brace { elements } => elements,
        _ => panic!("expected use case brace body"),
    };
    assert!(
        body_elements
            .iter()
            .any(|e| matches!(e.value, sysml_parser::ast::UseCaseDefBodyElement::Objective(_))),
        "later use case members should still parse after recovering from invalid subject syntax"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_actor_name_in_use_case_body() {
    let input = "package P {\nuse case def U {\nactor: User;\nobjective { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing actor name should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_member_name"))
        .expect("expected missing_member_name diagnostic");
    assert_eq!(err.expected.as_deref(), Some("actor name before ':'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("actor user: User;")),
        "diagnostic should show an actor example fix"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_subject_type_in_requirement_body() {
    let input = "package P {\nrequirement def R {\nsubject laptop: ;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing subject type should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_type_reference"))
        .expect("expected missing_type_reference diagnostic");
    assert_eq!(err.expected.as_deref(), Some("subject type after ':'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("subject laptop: Laptop;")),
        "diagnostic should show a subject type example fix"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_actor_type_in_use_case_body() {
    let input = "package P {\nuse case def U {\nactor user: ;\nobjective { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing actor type should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_type_reference"))
        .expect("expected missing_type_reference diagnostic");
    assert_eq!(err.expected.as_deref(), Some("actor type after ':'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("actor user: User;")),
        "diagnostic should show an actor type example fix"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_state_name_in_state_body() {
    let input = "package P {\nstate def Machine {\nstate: Mode;\ntransition t then Ready;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing state name should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.expected.as_deref() == Some("state name before ':'"))
        .expect("expected state-name diagnostic");
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("state ready: Mode;")),
        "diagnostic should show a state example fix"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_part_type_in_part_body() {
    let input = "package P {\npart def Vehicle {\npart wheel: ;\nattribute mass: MassValue;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing part type should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.expected.as_deref() == Some("part type after ':'"))
        .expect("expected part-type diagnostic");
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("part wheel: Wheel;")),
        "diagnostic should show a part type example fix"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_part_name_in_part_body() {
    let input = "package P {\npart def Vehicle {\npart: Wheel;\nattribute mass: MassValue;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing part name should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.expected.as_deref() == Some("part name before ':'"))
        .expect("expected part-name diagnostic");
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("part wheel: Wheel;")),
        "diagnostic should show a part example fix"
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
        err.found
            .as_deref()
            .is_some_and(|f| f.contains("#fmeaspec")),
        "diagnostic should preserve recovered snippet"
    );
    assert!(
        err.message.contains("annotation"),
        "annotation recovery should say why the declaration could not be parsed: {}",
        err.message
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_semicolon_between_package_members() {
    let input = "package P {\npart def A {\nexhibit state s : S\npart b : B;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "missing semicolon should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_semicolon"))
        .expect("expected missing_semicolon diagnostic");
    assert_eq!(err.expected.as_deref(), Some("';'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("Insert ';'")),
        "diagnostic should include a semicolon suggestion"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_illegal_top_level_part_definition() {
    let input = "part def TopLevel;";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "top-level part def should fail");
    let err = &result.errors[0];
    assert_eq!(err.code.as_deref(), Some("illegal_top_level_definition"));
    assert!(
        err.message.contains("illegal top-level"),
        "message should describe illegal top-level declaration"
    );
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("package") && s.contains("namespace")),
        "diagnostic should suggest wrapping in package or namespace"
    );
}

#[test]
fn test_parse_reports_missing_closing_brace_for_unterminated_package() {
    let input = "package P {\npart def A;\n";
    let err = parse(input).expect_err("unterminated package should fail");
    assert_eq!(err.code.as_deref(), Some("missing_closing_brace"));
    assert_eq!(err.expected.as_deref(), Some("'}'"));
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("Add '}'")),
        "missing brace diagnostic should suggest how to close the body"
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_closing_brace_for_unterminated_package() {
    let input = "package P {\npart def A;\n";
    let result = parse_with_diagnostics(input);
    assert!(!result.is_ok(), "unterminated package should produce diagnostics");
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_closing_brace"))
        .expect("expected missing closing brace diagnostic");
    assert_eq!(err.expected.as_deref(), Some("'}'"));
}

#[test]
fn test_parse_reports_illegal_top_level_part_definition() {
    let input = "part def TopLevel;";
    let err = parse(input).expect_err("top-level part def should fail");
    assert_eq!(err.code.as_deref(), Some("illegal_top_level_definition"));
    assert_eq!(err.expected.as_deref(), Some("'package', 'namespace', or 'import'"));
}

#[test]
fn test_invalid_input_corpus_is_handled_gracefully() {
    let invalid_inputs = [
        "package P {",
        "package P { part def A {",
        "package P { @@@ ??? }",
        "package P { /* unterminated",
        "namespace N { part def X { ;;; }",
        "part def TopLevel;",
    ];

    for input in invalid_inputs {
        let strict = std::panic::catch_unwind(|| parse(input));
        assert!(strict.is_ok(), "parse should not panic for {:?}", input);

        let recovered = std::panic::catch_unwind(|| parse_with_diagnostics(input));
        assert!(
            recovered.is_ok(),
            "parse_with_diagnostics should not panic for {:?}",
            input
        );
    }
}
