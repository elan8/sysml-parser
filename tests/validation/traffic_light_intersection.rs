//! Parser tests for `tests/fixtures/TrafficLightIntersection.sysml`.

use std::path::Path;
use sysml_parser::ast::{PackageBodyElement, RootElement};
use sysml_parser::parse;

/// Path to the TrafficLightIntersection fixture.
fn traffic_light_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("TrafficLightIntersection.sysml")
}

#[test]
fn test_parse_traffic_light_intersection() {
    super::init_log();
    let path = traffic_light_fixture_path();
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");
    log::debug!("input len: {} bytes", input.len());

    let result = parse(&input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!(
            "parse should succeed for TrafficLightIntersection.sysml: {:?}",
            e
        ),
    };

    assert_eq!(
        root.elements.len(),
        1,
        "expected exactly one root element (package TrafficLightIntersection)"
    );
    let first = &root.elements[0];
    let package = match &first.value {
        RootElement::Package(p) => &p.value,
        other => panic!("expected root to be a Package, got {:?}", other),
    };
    assert_eq!(
        package.identification.name.as_deref(),
        Some("TrafficLightIntersection"),
        "root package should be named TrafficLightIntersection"
    );

    let body = match &package.body {
        sysml_parser::ast::PackageBody::Brace { elements } => elements,
        _ => panic!("expected package body to be brace form"),
    };

    // Count key top-level constructs present in the fixture
    let has_part_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::PartDef(_)));
    let has_requirement_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::RequirementDef(_)));
    let has_use_case_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::UseCaseDef(_)));
    let has_state_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::StateDef(_)));
    let has_constraint_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::ConstraintDef(_)));
    let has_satisfy = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::Satisfy(_)));
    let has_port_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::PortDef(_)));
    let has_action_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::ActionDef(_)));

    assert!(has_part_def, "fixture should contain part defs");
    assert!(
        has_requirement_def,
        "fixture should contain requirement defs"
    );
    assert!(has_use_case_def, "fixture should contain use case defs");
    assert!(has_state_def, "fixture should contain state defs");
    assert!(has_constraint_def, "fixture should contain constraint defs");
    assert!(has_satisfy, "fixture should contain satisfy statements");
    assert!(has_port_def, "fixture should contain port defs");
    assert!(has_action_def, "fixture should contain action defs");
}
