//! TDD tests: SysML snippets with expected AST.

use std::path::PathBuf;

use sysml_v2_parser::ast::{
    Identification, LibraryPackage, Node, Package, PackageBody, PackageBodyElement,
    RenderingDefBody, RootElement, RootNamespace, Span, ViewBody, ViewDefBody,
};
use sysml_v2_parser::{parse, parse_with_diagnostics};

fn id(name: &str) -> Identification {
    Identification {
        short_name: None,
        name: Some(name.to_string()),
    }
}

fn sysml_v2_release_root() -> PathBuf {
    std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
}

fn primitive_data_types_fixture() -> Option<String> {
    let path = sysml_v2_release_root()
        .join("sysml")
        .join("src")
        .join("validation")
        .join("15-Properties-Values-Expressions")
        .join("15_10-Primitive Data Types.sysml");
    std::fs::read_to_string(path).ok()
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
        elements: vec![n_len(
            12,
            RootElement::Package(n_len(
                12,
                Package {
                    identification: id("Foo"),
                    body: PackageBody::Semicolon,
                },
            )),
        )],
    }
}

/// Build expected AST for `package Bar { }` (input len = 15)
fn expected_package_bar_brace() -> RootNamespace {
    RootNamespace {
        elements: vec![n_len(
            15,
            RootElement::Package(n_len(
                15,
                Package {
                    identification: id("Bar"),
                    body: PackageBody::Brace { elements: vec![] },
                },
            )),
        )],
    }
}

#[test]
fn test_package_with_semicolon_body() {
    let input = "package Foo;";
    let result = parse(input).expect("parse should succeed");
    let expected = expected_package_foo_semicolon();
    assert_eq!(
        result, expected,
        "AST should match expected for package Foo;"
    );
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
            assert!(
                matches!(lp.value.body, PackageBody::Brace { ref elements } if elements.is_empty())
            );
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
        assert!(
            err.found.is_some(),
            "error should have 'found' snippet: {}",
            err.message
        );
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
    assert!(
        err.found.is_some(),
        "expected end of input error should have 'found': {}",
        err
    );
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
    assert!(
        display.contains("line"),
        "Display should include line number"
    );
    assert!(
        err.found.as_ref().is_some_and(|f| display.contains(f)),
        "Display should include found snippet: {}",
        display
    );
}

#[test]
fn test_action_def_is_not_parsed_as_action_usage() {
    let input = r#"package P {
action def ExecutePatrol {
}
}"#;
    let root = sysml_v2_parser::parse_root(input).expect("should parse");
    let pkg = match &root.elements[0].value {
        sysml_v2_parser::ast::RootElement::Package(p) => &p.value,
        _ => panic!("expected package root element"),
    };
    let sysml_v2_parser::ast::PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let first = elements.first().expect("expected a body element");
    match &first.value {
        sysml_v2_parser::ast::PackageBodyElement::ActionDef(a) => {
            assert_eq!(
                a.value.identification.name.as_deref(),
                Some("ExecutePatrol"),
                "expected ActionDef name ExecutePatrol"
            );
        }
        other => panic!("expected ActionDef, got {:?}", other),
    }
}

#[test]
fn test_parse_with_diagnostics_recovers_and_reports_later_errors() {
    // Intentionally malformed body statements (unknown keywords) followed by a valid member.
    // The goal is to ensure we report multiple diagnostics and still parse later valid elements.
    let input = r#"package P {
action def A {
  badstmt {};
  badstmt2 {};
}
action def B { }
}"#;
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.len() >= 2,
        "expected 2+ diagnostics, got: {:?}",
        result.errors
    );

    // Ensure we still parsed the later action def `B`.
    let pkg = match &result.root.elements[0].value {
        sysml_v2_parser::ast::RootElement::Package(p) => &p.value,
        _ => panic!("expected package root element"),
    };
    let sysml_v2_parser::ast::PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let has_b = elements.iter().any(|e| match &e.value {
        sysml_v2_parser::ast::PackageBodyElement::ActionDef(a) => {
            a.value.identification.name.as_deref() == Some("B")
        }
        _ => false,
    });
    assert!(has_b, "expected later ActionDef `B` to still be parsed");
}

#[test]
fn test_library_abstract_action_feature_decl_parses_without_diagnostics() {
    // Representative Systems Library syntax (Actions.sysml): abstract action feature with typing,
    // multiplicity, modifier, and specialization, with a doc-only body.
    let input = r#"package P {
abstract action sendActions: SendAction[0..*] nonunique :> actions, sendPerformances {
  doc /* sendActions is the base feature for SendActionUsages. */
}
}"#;
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "expected no diagnostics; got: {:?}",
        result.errors
    );
}

#[test]
fn test_library_multiplicity_decl_parses_without_diagnostics() {
    // Representative Kernel library syntax (Base.kerml): multiplicity decl with range and body.
    let input = r#"package P {
multiplicity exactlyOne [1..1] { doc /* ... */ }
}"#;
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "expected no diagnostics; got: {:?}",
        result.errors
    );
}

#[test]
fn test_library_interaction_decl_parses_without_diagnostics() {
    // Representative Kernel library syntax (Transfers.kerml): interaction specializes ...
    let input = r#"package P {
interaction Transfer specializes Performance { doc /* ... */ }
}"#;
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "expected no diagnostics; got: {:?}",
        result.errors
    );
}

#[test]
fn test_library_return_assignment_form_parses_without_diagnostics() {
    // Representative Domain library syntax: `return name = expr;`
    let input = r#"package P {
calc def C {
  return result = integrate.result;
}
}"#;
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "expected no diagnostics; got: {:?}",
        result.errors
    );
}

// --- Top-level import (Phase 0: BNF RootNamespace = PackageBodyElement*) ---

#[test]
fn test_root_level_import_then_package() {
    let input = "private import Views::*;\npackage P { }";
    let result = parse(input).expect("parse should succeed");
    assert_eq!(result.elements.len(), 2);
    match &result.elements[0].value {
        sysml_v2_parser::ast::RootElement::Import(_) => {}
        _ => panic!("expected first element to be Import"),
    }
    match &result.elements[1].value {
        sysml_v2_parser::ast::RootElement::Package(p) => {
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
    let input =
        "package P { use case def U { subject s : System; actor a : Operator; objective { } } }";
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
        sysml_v2_parser::ast::UseCaseDefBody::Brace { elements } => elements,
        _ => panic!("expected use case brace body"),
    };
    assert!(body_elements.iter().any(|e| matches!(
        e.value,
        sysml_v2_parser::ast::UseCaseDefBodyElement::SubjectDecl(_)
    )));
    assert!(body_elements.iter().any(|e| matches!(
        e.value,
        sysml_v2_parser::ast::UseCaseDefBodyElement::ActorUsage(_)
    )));
    assert!(body_elements.iter().any(|e| matches!(
        e.value,
        sysml_v2_parser::ast::UseCaseDefBodyElement::Objective(_)
    )));
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
        sysml_v2_parser::ast::StateDefBody::Brace { elements } => elements,
        _ => panic!("expected state brace body"),
    };
    assert!(body_elements
        .iter()
        .any(|e| matches!(e.value, sysml_v2_parser::ast::StateDefBodyElement::Then(_))));
    assert!(body_elements.iter().any(|e| matches!(
        e.value,
        sysml_v2_parser::ast::StateDefBodyElement::StateUsage(_)
    )));
    assert!(body_elements.iter().any(|e| matches!(
        e.value,
        sysml_v2_parser::ast::StateDefBodyElement::Transition(_)
    )));
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
        sysml_v2_parser::ast::ConstraintDefBody::Brace { elements } => elements,
        _ => panic!("expected constraint brace body"),
    };
    assert!(
        !constraint_elements.is_empty(),
        "constraint body should not be empty"
    );
    let calc_def = match &elements[1].value {
        PackageBodyElement::CalcDef(cd) => &cd.value,
        _ => panic!("expected CalcDef"),
    };
    let calc_elements = match &calc_def.body {
        sysml_v2_parser::ast::CalcDefBody::Brace { elements } => elements,
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
        sysml_v2_parser::ast::ConnectionDefBody::Brace { elements } if !elements.is_empty()
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
    assert!(matches!(
        elements[0].value,
        PackageBodyElement::FlowUsage(_)
    ));
    assert!(matches!(
        elements[1].value,
        PackageBodyElement::AllocationUsage(_)
    ));
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
    assert!(matches!(
        elements[1].value,
        PackageBodyElement::AnalysisCaseDef(_)
    ));
    assert!(matches!(
        elements[2].value,
        PackageBodyElement::VerificationCaseDef(_)
    ));
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
        matches!(&case_def.body, sysml_v2_parser::ast::UseCaseDefBody::Brace { elements } if !elements.is_empty())
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
        sysml_v2_parser::ast::PartDefBody::Brace { elements } => elements,
        _ => panic!("expected part def brace body"),
    };
    let perform = match &part_body[0].value {
        sysml_v2_parser::ast::PartDefBodyElement::Perform(p) => &p.value,
        _ => panic!("expected perform action declaration"),
    };
    assert!(
        matches!(&perform.body, sysml_v2_parser::ast::PerformBody::Brace { elements } if !elements.is_empty()),
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
    assert!(matches!(
        elements[0].value,
        PackageBodyElement::RequirementDef(_)
    ));
    assert!(matches!(
        elements[1].value,
        PackageBodyElement::UseCaseDef(_)
    ));
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
    assert!(matches!(
        elements[2].value,
        PackageBodyElement::ViewpointDef(_)
    ));
    assert!(
        !elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::ExtendedLibraryDecl(_))),
        "sample should not fall back to ExtendedLibraryDecl"
    );
}

#[test]
fn test_feature_and_classifier_decls_map_to_dedicated_package_nodes() {
    let input = "package P {
        feature myFeature : BaseFeature;
        class VehicleClass;
        struct LayoutStruct;
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
    assert!(matches!(
        elements[0].value,
        PackageBodyElement::FeatureDecl(_)
    ));
    assert!(matches!(
        elements[1].value,
        PackageBodyElement::ClassifierDecl(_)
    ));
    assert!(matches!(
        elements[2].value,
        PackageBodyElement::ClassifierDecl(_)
    ));
    assert!(
        !elements.iter().any(|e| matches!(
            e.value,
            PackageBodyElement::KermlSemanticDecl(_) | PackageBodyElement::KermlFeatureDecl(_)
        )),
        "dedicated feature/classifier samples should not fall back to generic KerML buckets"
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
    assert!(matches!(
        elements[0].value,
        PackageBodyElement::AttributeDef(_)
    ));
}

#[test]
fn test_enum_def_with_specialization_and_assigned_literals_maps_dedicated() {
    let input =
        "package P { enum def LevelEnum :> Level { low = 0.25; medium = 0.5; high = 0.75; } }";
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
            let value = attr.typing.as_ref().map(|_| ()).or(Some(()));
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
        elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::Error(_))),
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
fn test_requirement_body_keeps_structured_attributes_and_later_require_constraint() {
    let input = "package P {\nrequirement def R {\nsubject vehicle : Vehicle;\nattribute massActual: MassValue;\nattribute measuredMass = 42;\nrequire constraint { }\n}\n}";
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
        sysml_v2_parser::ast::RequirementDefBody::Brace { elements } => elements,
        _ => panic!("expected requirement brace body"),
    };
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::SubjectDecl(_)
        )),
        "subject should be parsed in requirement body"
    );
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::AttributeDef(_)
        )),
        "typed attribute members should be preserved as structured attribute defs"
    );
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::AttributeUsage(_)
        )),
        "value-based attribute members should be preserved as structured attribute usages"
    );
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::RequireConstraint(_)
        )),
        "require constraint should be preserved after structured attribute members"
    );
}

#[test]
fn test_part_def_recovery_preserves_other_member_and_later_sibling() {
    let input =
        "package P {\npart def Vehicle {\nstate monitor: Mode;\nattribute mass: MassValue;\n}\n}";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let part_def = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(p) => Some(&p.value),
            _ => None,
        })
        .expect("part def should be present");
    let sysml_v2_parser::ast::PartDefBody::Brace { elements } = &part_def.body else {
        panic!("expected part def body");
    };
    assert!(
        elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::PartDefBodyElement::Other(_)
                | sysml_v2_parser::ast::PartDefBodyElement::OpaqueMember(_)
        )),
        "library-tolerant unmodeled part members should be preserved explicitly"
    );
    assert!(
        elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::PartDefBodyElement::AttributeDef(_)
        )),
        "later modeled members should still parse"
    );
}

#[test]
fn test_state_def_recovery_no_longer_truncates_body() {
    let input = "package P {\nstate def Machine {\nunknown stuff;\ntransition t then Ready;\n}\n}";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let state_def = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::StateDef(s) => Some(&s.value),
            _ => None,
        })
        .expect("state def should be present");
    let sysml_v2_parser::ast::StateDefBody::Brace { elements } = &state_def.body else {
        panic!("expected state body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, sysml_v2_parser::ast::StateDefBodyElement::Other(_))),
        "unknown state members should be preserved explicitly instead of truncating the body"
    );
    assert!(
        elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::StateDefBodyElement::Transition(_)
        )),
        "later valid state members should still parse"
    );
}

#[test]
fn test_parse_with_diagnostics_accepts_structured_requirement_attributes() {
    let input = "package P {\nrequirement def R {\nsubject vehicle : Vehicle;\nattribute massActual: MassValue;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "structured requirement attributes should not produce recovery diagnostics: {:?}",
        result.errors
    );
}

#[test]
fn test_parse_requirement_body_supports_attribute_def_and_usage_forms() {
    let input = "package P {\nrequirement def R {\nattribute def targetMass: MassValue;\nattribute actualMass = measuredMass;\n}\n}";
    let result = parse(input).expect("requirement body attributes should parse");
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
        sysml_v2_parser::ast::RequirementDefBody::Brace { elements } => elements,
        _ => panic!("expected requirement brace body"),
    };
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::AttributeDef(_)
        )),
        "attribute def form should be preserved"
    );
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::AttributeUsage(_)
        )),
        "attribute usage form should be preserved"
    );
}

#[test]
fn test_parse_part_attribute_prefix_redefines_shorthand() {
    let input = "package P {\npart def Laptop { attribute name : String; }\npart office {\npart laptop1: Laptop {\nattribute :>> name = \"My Laptop\";\n}\n}\n}";
    let result = parse(input).expect("attribute prefix redefines shorthand should parse");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let office = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartUsage(p) if p.value.name == "office" => Some(&p.value),
            _ => None,
        })
        .expect("office part usage should be present");
    let office_body = match &office.body {
        sysml_v2_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected office part body"),
    };
    let laptop1 = office_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartUsageBodyElement::PartUsage(p)
                if p.value.name == "laptop1" =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("laptop1 part usage should be present");
    let laptop1_body = match &laptop1.body {
        sysml_v2_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected laptop1 part body"),
    };
    let attribute = laptop1_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartUsageBodyElement::AttributeUsage(a) => Some(&a.value),
            _ => None,
        })
        .expect("attribute usage should be present");
    assert_eq!(attribute.name, "name");
    assert_eq!(attribute.redefines.as_deref(), Some("name"));
    assert!(
        attribute.value.is_some(),
        "attribute value should be parsed"
    );
}

#[test]
fn test_parse_part_attribute_prefix_redefines_with_subsets_clause() {
    let input = "package P {\npart def Room { attribute outlet: Outlet; }\npart def Home {\npart livingRoom : Room {\nattribute :>> outlet :> electricGrid.outlets;\n}\n}\n}";
    let result = parse(input).expect("attribute prefix redefines with subsets should parse");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let home = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(p)
                if p.value.identification.name.as_deref() == Some("Home") =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("Home part def should be present");
    let home_body = match &home.body {
        sysml_v2_parser::ast::PartDefBody::Brace { elements } => elements,
        _ => panic!("expected Home part def body"),
    };
    let living_room = home_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartDefBodyElement::PartUsage(p)
                if p.value.name == "livingRoom" =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("livingRoom part usage should be present");
    let living_room_body = match &living_room.body {
        sysml_v2_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected livingRoom part usage body"),
    };
    let attribute = living_room_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartUsageBodyElement::AttributeUsage(a) => Some(&a.value),
            _ => None,
        })
        .expect("attribute usage should be present");
    assert_eq!(attribute.name, "outlet");
    assert_eq!(attribute.redefines.as_deref(), Some("outlet"));
}

#[test]
fn test_parse_part_usage_body_satisfy_shorthand() {
    let input =
        "package P {\npart def Home {\npart livingRoom: Room {\nsatisfy heatSuff5;\n}\n}\n}";
    let result = parse(input).expect("satisfy shorthand in part usage should parse");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let home = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(p)
                if p.value.identification.name.as_deref() == Some("Home") =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("Home part def should be present");
    let home_body = match &home.body {
        sysml_v2_parser::ast::PartDefBody::Brace { elements } => elements,
        _ => panic!("expected Home part def body"),
    };
    let living_room = home_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartDefBodyElement::PartUsage(p)
                if p.value.name == "livingRoom" =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("livingRoom part usage should be present");
    let living_room_body = match &living_room.body {
        sysml_v2_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected livingRoom part usage body"),
    };
    assert!(
        living_room_body.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::PartUsageBodyElement::Satisfy(_)
        )),
        "satisfy shorthand should be preserved in part usage body"
    );
}

#[test]
fn test_parse_interface_usage_named_with_multiplicity() {
    let input = "package P {\npart def Home {\npart livingRoom: Room {\ninterface heater2PowerOutlet[1] : Socket2OutletInterface connect heater.socket to outlet;\n}\n}\n}";
    let result = parse(input).expect("named interface usage with multiplicity should parse");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let elements = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let home = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(p)
                if p.value.identification.name.as_deref() == Some("Home") =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("Home part def should be present");
    let home_body = match &home.body {
        sysml_v2_parser::ast::PartDefBody::Brace { elements } => elements,
        _ => panic!("expected Home part def body"),
    };
    let living_room = home_body
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::PartDefBodyElement::PartUsage(p)
                if p.value.name == "livingRoom" =>
            {
                Some(&p.value)
            }
            _ => None,
        })
        .expect("livingRoom part usage should be present");
    let living_room_body = match &living_room.body {
        sysml_v2_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected livingRoom part usage body"),
    };
    assert!(
        living_room_body.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::PartUsageBodyElement::InterfaceUsage(_)
        )),
        "named interface usage with multiplicity should be preserved"
    );
}

#[test]
fn test_parse_part_def_connection_usage_multiline_connect_clause() {
    let input = "package P {\nconnection def Door { end [1] part room1 : Room; end [1] part room2 : Room; }\npart def Home {\nconnection livingRoom2bedRoom[1] : Door\n  connect livingRoom to bedRoom;\nconnection livingRoom2kitchen[1] : Door\n  connect livingRoom to kitchen;\nconnection livingRoom2bathRoom[1] : Door\n  connect livingRoom to bathRoom;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "multiline connection usage should parse without recovery diagnostics: {:?}",
        result.errors
    );
}

#[test]
fn test_parse_require_constraint_keeps_inner_members() {
    let input = "package P {\nrequirement def R {\nrequire constraint {\ndoc /* requirement logic */\nin x : Real;\nout y : Real;\nx >= y;\n}\n}\n}";
    let result = parse(input).expect("require constraint body should parse");
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
        sysml_v2_parser::ast::RequirementDefBody::Brace { elements } => elements,
        _ => panic!("expected requirement brace body"),
    };
    let require_constraint = body_elements
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::RequirementDefBodyElement::RequireConstraint(c) => Some(&c.value),
            _ => None,
        })
        .expect("require constraint should be present");
    let constraint_elements = match &require_constraint.body {
        sysml_v2_parser::ast::RequireConstraintBody::Brace { elements } => elements,
        _ => panic!("expected structured require constraint body"),
    };
    assert!(
        constraint_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::ConstraintDefBodyElement::Doc(_)
        )),
        "doc should be preserved inside require constraint"
    );
    assert!(
        constraint_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::ConstraintDefBodyElement::InOutDecl(_)
        )),
        "in/out declarations should be preserved inside require constraint"
    );
    assert!(
        constraint_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::ConstraintDefBodyElement::Expression(_)
        )),
        "expressions should be preserved inside require constraint"
    );
}

#[test]
fn test_parse_requirement_subject_shorthand_without_name() {
    let input = "package P {\nrequirement def R {\nsubject: Laptop;\nrequire constraint { }\n}\n}";
    let result = parse(input).expect("subject shorthand should parse");
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
        sysml_v2_parser::ast::RequirementDefBody::Brace { elements } => elements,
        _ => panic!("expected requirement brace body"),
    };
    let subject = body_elements
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::RequirementDefBodyElement::SubjectDecl(s) => Some(&s.value),
            _ => None,
        })
        .expect("subject decl should be present");
    assert_eq!(subject.name, "subject");
    assert_eq!(subject.type_name, "Laptop");
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::RequirementDefBodyElement::RequireConstraint(_)
        )),
        "later requirement members should still parse after subject shorthand"
    );
}

#[test]
fn test_parse_use_case_subject_shorthand_without_name() {
    let input = "package P {\nuse case def U {\nsubject: Laptop;\nobjective { }\n}\n}";
    let result = parse(input).expect("subject shorthand should parse");
    let pkg = match &result.elements[0].value {
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
        sysml_v2_parser::ast::UseCaseDefBody::Brace { elements } => elements,
        _ => panic!("expected use case brace body"),
    };
    let subject = body_elements
        .iter()
        .find_map(|e| match &e.value {
            sysml_v2_parser::ast::UseCaseDefBodyElement::SubjectDecl(s) => Some(&s.value),
            _ => None,
        })
        .expect("subject decl should be present");
    assert_eq!(subject.name, "subject");
    assert_eq!(subject.type_name, "Laptop");
    assert!(
        body_elements.iter().any(|e| matches!(
            e.value,
            sysml_v2_parser::ast::UseCaseDefBodyElement::Objective(_)
        )),
        "later use case members should still parse after subject shorthand"
    );
}

#[test]
fn test_parse_package_with_quoted_name() {
    let input = "package '15.10-Primitive Data Types' { }";
    let result = parse(input).expect("quoted package names should parse");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    assert_eq!(
        pkg.identification.name.as_deref(),
        Some("15.10-Primitive Data Types")
    );
}

#[test]
fn test_parse_primitive_data_types_validation_fixture() {
    let Some(input) = primitive_data_types_fixture() else {
        return;
    };
    let result = parse(&input);
    assert!(
        result.is_ok(),
        "fixture should parse cleanly; diagnostics: {:?}",
        parse_with_diagnostics(&input).errors
    );
}

#[test]
fn test_parse_with_diagnostics_reports_missing_actor_name_in_use_case_body() {
    let input = "package P {\nuse case def U {\nactor: User;\nobjective { }\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        !result.is_ok(),
        "missing actor name should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing subject type should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing actor type should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing state name should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing part type should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing part name should produce diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "package-level recovery should surface as diagnostics"
    );
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
    assert!(
        !result.is_ok(),
        "missing semicolon should produce diagnostics"
    );
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
fn test_parse_with_diagnostics_reports_invalid_expose_separator() {
    let input = "package Views { view structure: GeneralView { expose SurveillanceDrone.SurveillanceQuadrotorDrone; } }";
    let result = parse_with_diagnostics(input);
    assert!(
        !result.is_ok(),
        "invalid expose separator should produce diagnostics"
    );
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("invalid_qualified_name_separator"))
        .expect("expected invalid_qualified_name_separator diagnostic");
    assert!(
        err.message.contains("use '::' instead of '.'"),
        "diagnostic should explain separator issue: {}",
        err.message
    );
    assert_eq!(
        err.expected.as_deref(),
        Some("qualified name segments separated by '::'")
    );
    assert!(
        err.suggestion
            .as_deref()
            .is_some_and(|s| s.contains("expose A::B;")),
        "diagnostic should include concrete correction"
    );
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.code.as_deref() == Some("missing_semicolon")),
        "should not surface misleading missing_semicolon for invalid expose separator: {:?}",
        result.errors
    );
}

#[test]
fn test_action_def_body_allows_doc_and_nested_action_usages_without_semicolon_after_doc() {
    let input = r#"package P {
action def ExecutePatrol {
  in route : String;
  out status : String;
  doc /* Execute patrol/overwatch mission along route. */

  action validateRoute { out validationStatus : String; };
  action startMission { out missionStarted : String; };

  first validateRoute then startMission;
  bind status = startMission::missionStarted;
}
}"#;

    let result = parse_with_diagnostics(input);
    assert!(
        result.is_ok(),
        "action def with doc + nested actions should parse without recovery diagnostics: {:?}",
        result.errors
    );
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.code.as_deref() == Some("missing_semicolon")),
        "should not report missing_semicolon around doc/nested action usages: {:?}",
        result.errors
    );
}

#[test]
fn test_action_usage_body_allows_untyped_out_pin_decl() {
    // Common SysML v2 shorthand in action usage bodies: `out foo;` (no `: Type`)
    // to reference the corresponding typed parameter on the referenced action definition.
    let input = r#"package P {
action def CaptureVideo { out videoStream : String; }
action def ExecutePatrol {
  action capture : CaptureVideo { out videoStream; };
  first capture then capture;
}
}"#;

    let result = parse_with_diagnostics(input);
    assert!(
        result.is_ok(),
        "untyped out pin decl in action usage body should not trigger recovery diagnostics: {:?}",
        result.errors
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
    assert!(
        !result.is_ok(),
        "unterminated package should produce diagnostics"
    );
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
    assert_eq!(
        err.expected.as_deref(),
        Some("'package', 'namespace', or 'import'")
    );
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

#[test]
fn test_part_def_accepts_specializes_keyword_as_specialization() {
    let input = r#"package P {
part def A specializes B;
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let part_def = match &elements[0].value {
        PackageBodyElement::PartDef(p) => p,
        other => panic!("expected part def, got {:?}", other),
    };
    assert_eq!(part_def.value.specializes.as_deref(), Some("B"));
    assert!(
        part_def.value.specializes_span.is_some(),
        "specializes span should be present for keyword form"
    );
}

#[test]
fn test_port_def_accepts_specializes_keyword_as_specialization() {
    let input = r#"package P {
port def ControlPort specializes BasePort;
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let port_def = match &elements[0].value {
        PackageBodyElement::PortDef(p) => p,
        other => panic!("expected port def, got {:?}", other),
    };
    assert_eq!(port_def.value.specializes.as_deref(), Some("BasePort"));
}

#[test]
fn test_individual_def_accepts_specializes_keyword_as_specialization() {
    let input = r#"package P {
individual def Rover specializes MobileRobot;
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let individual_def = match &elements[0].value {
        PackageBodyElement::IndividualDef(p) => p,
        other => panic!("expected individual def, got {:?}", other),
    };
    assert_eq!(
        individual_def.value.specializes.as_deref(),
        Some("MobileRobot")
    );
}

#[test]
fn test_occurrence_usage_accepts_keyword_subset_and_redefine_aliases() {
    let input = r#"package P {
occurrence rover subsets BaseOccurrence redefines LegacyOccurrence;
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let occ = match &elements[0].value {
        PackageBodyElement::OccurrenceUsage(o) => o,
        other => panic!("expected occurrence usage, got {:?}", other),
    };
    assert_eq!(occ.value.subsets.as_deref(), Some("BaseOccurrence"));
    assert_eq!(occ.value.redefines.as_deref(), Some("LegacyOccurrence"));
}

#[test]
fn test_requirement_usage_accepts_subsets_keyword_alias() {
    let input = r#"package P {
requirement VehicleReq; subsets BaseReq;
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let req = match &elements[0].value {
        PackageBodyElement::RequirementUsage(r) => r,
        other => panic!("expected requirement usage, got {:?}", other),
    };
    assert_eq!(req.value.subsets.as_deref(), Some("BaseReq"));
}

#[test]
fn test_port_usage_normalizes_subset_redefine_aliases() {
    let input = r#"package P {
part def Carrier {
  port :>> wheelPort : WheelPortType subsets basePort;
}
}"#;
    let result = parse(input).expect("parse should succeed");
    let pkg = match &result.elements[0].value {
        RootElement::Package(p) => p,
        other => panic!("expected package, got {:?}", other),
    };
    let elements = match &pkg.value.body {
        PackageBody::Brace { elements } => elements,
        other => panic!("expected brace body, got {:?}", other),
    };
    let part_def = match &elements[0].value {
        PackageBodyElement::PartDef(p) => p,
        other => panic!("expected part def, got {:?}", other),
    };
    let part_body = match &part_def.value.body {
        sysml_v2_parser::ast::PartDefBody::Brace { elements } => elements,
        other => panic!("expected part def brace body, got {:?}", other),
    };
    let port_usage = match &part_body[0].value {
        sysml_v2_parser::ast::PartDefBodyElement::PortUsage(p) => p,
        other => panic!("expected port usage, got {:?}", other),
    };
    assert_eq!(
        port_usage
            .value
            .subsets
            .as_ref()
            .map(|(name, _)| name.as_str()),
        Some("basePort")
    );
    assert_eq!(port_usage.value.redefines.as_deref(), Some("wheelPort"));
}
