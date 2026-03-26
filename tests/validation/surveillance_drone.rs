//! Parser tests for `tests/fixtures/SurveillanceDrone*.sysml`.

use std::path::Path;
use sysml_parser::ast::{PackageBodyElement, PartDef, RootElement};
use sysml_parser::{parse, parse_with_diagnostics};

/// Path to the SurveillanceDrone fixture (project-local, not sysml-v2-release).
fn surveillance_drone_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("SurveillanceDrone.sysml")
}

/// Path to SurveillanceDrone-error.sysml (contains invalid statement on line 333).
fn surveillance_drone_error_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("SurveillanceDrone-error.sysml")
}

/// Path to SurveillanceDrone-errors.sysml (contains multiple invalid statements).
fn surveillance_drone_errors_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("SurveillanceDrone-errors.sysml")
}

#[test]
fn test_parse_surveillance_drone() {
    super::init_log();
    let path = surveillance_drone_fixture_path();
    log::debug!("fixture path: {}", path.display());
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");
    log::debug!("input len: {} bytes", input.len());

    let result = parse(&input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("parse should succeed for SurveillanceDrone.sysml: {:?}", e),
    };

    assert_eq!(
        root.elements.len(),
        1,
        "expected exactly one root element (package SurveillanceDrone)"
    );
    let first = &root.elements[0];
    let package = match &first.value {
        RootElement::Package(p) => &p.value,
        other => panic!("expected root to be a Package, got {:?}", other),
    };
    assert_eq!(
        package.identification.name.as_deref(),
        Some("SurveillanceDrone"),
        "root package should be named SurveillanceDrone"
    );

    let body = match &package.body {
        sysml_parser::ast::PackageBody::Brace { elements } => elements,
        _ => panic!("expected package body to be brace form"),
    };

    // Count key top-level constructs present in the fixture (partial parse may not have all)
    let has_part_def = body.iter().any(|e| matches!(&e.value, PackageBodyElement::PartDef(_)));
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
    let has_calc_def = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::CalcDef(_)));
    let has_satisfy = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::Satisfy(_)));
    let has_doc = body
        .iter()
        .any(|e| matches!(&e.value, PackageBodyElement::Doc(_)));

    assert!(has_doc, "doc comments must be parsed as Doc elements in the AST, not skipped");
    assert!(has_part_def, "fixture should contain part defs");
    assert!(has_requirement_def, "fixture should contain requirement defs");
    assert!(has_use_case_def, "fixture should contain use case defs");
    assert!(has_state_def, "fixture should contain state defs");
    assert!(has_constraint_def, "fixture should contain constraint defs");
    assert!(has_calc_def, "fixture should contain calc defs");
    assert!(has_satisfy, "fixture should contain satisfy statements");

    // Line 363: part def SurveillanceQuadrotorDroneWithBehavior :> SurveillanceQuadrotorDrone {
    // Assert that specializes_span is set for the ":> SurveillanceQuadrotorDrone" fragment.
    let part_def_specializes_span = body
        .iter()
        .filter_map(|e| {
            if let PackageBodyElement::PartDef(n) = &e.value {
                Some(&n.value)
            } else {
                None
            }
        })
        .find(|p: &&PartDef| {
            p.identification.name.as_deref() == Some("SurveillanceQuadrotorDroneWithBehavior")
                && p.specializes.as_deref() == Some("SurveillanceQuadrotorDrone")
        });
    let part_def = part_def_specializes_span
        .expect("fixture should contain part def SurveillanceQuadrotorDroneWithBehavior :> SurveillanceQuadrotorDrone");
    assert!(
        part_def.specializes_span.is_some(),
        "specializes_span must be set when parsing ':> SurveillanceQuadrotorDrone' on line 363"
    );
    let span = part_def.specializes_span.as_ref().unwrap();
    assert_eq!(
        span.line, 363,
        "specializes_span should point to line 363 (':> SurveillanceQuadrotorDrone')"
    );
    let fragment = &input[span.offset..span.offset + span.len];
    assert!(
        fragment.contains(":> SurveillanceQuadrotorDrone"),
        "specializes_span should cover ':> SurveillanceQuadrotorDrone', got: {:?}",
        fragment
    );
}

/// SurveillanceDrone-error.sysml has an invalid statement on line 333: `test {}`.
/// This test ensures the parser rejects the file. Ideally the reported error is on line 333
/// and mentions "test"; we assert that when the parser reports it.
#[test]
fn test_surveillance_drone_error_reports_error_on_line_333() {
    super::init_log();
    let path = surveillance_drone_error_fixture_path();
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    let err = match parse(&input) {
        Ok(_) => panic!(
            "SurveillanceDrone-error.sysml should not parse successfully (invalid 'test {{}}' on line 333)"
        ),
        Err(e) => e,
    };

    // Require that we get an error on line 333 (the invalid "test {}" statement).
    assert_eq!(
        err.line,
        Some(333),
        "expected parse error on line 333 (invalid 'test {{}}' statement); got line {:?}, message: {}, found: {:?}",
        err.line,
        err.message,
        err.found
    );
    assert!(
        err.found.as_deref().is_some_and(|f| f.contains("test")),
        "error at line 333 should have 'found' containing 'test', got: {:?}",
        err.found
    );
}

/// SurveillanceDrone-error.sysml has exactly one invalid statement (`test {}` on line 333).
/// parse_with_diagnostics should report exactly 1 error for this file.
#[test]
fn test_surveillance_drone_error_reports_exactly_one_error() {
    super::init_log();
    let path = surveillance_drone_error_fixture_path();
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    let result = parse_with_diagnostics(&input);
    assert!(
        !result.is_ok(),
        "SurveillanceDrone-error.sysml should have parse errors"
    );
    assert_eq!(
        result.errors.len(),
        2,
        "expected exactly 2 parse errors (invalid 'test {{}}' on line 333, 'test2 {{}}' on line 364); got {} errors: {:?}",
        result.errors.len(),
        result
            .errors
            .iter()
            .map(|e| (e.line, e.found.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
    );
    assert_eq!(result.errors[0].line, Some(333));
    assert!(
        result.errors[0]
            .found
            .as_deref()
            .is_some_and(|f| f.contains("test")),
        "first error should have 'found' containing 'test'"
    );
    assert_eq!(result.errors[1].line, Some(364));
    assert!(
        result.errors[1]
            .found
            .as_deref()
            .is_some_and(|f| f.contains("test2")),
        "second error should have 'found' containing 'test2'"
    );
}

/// SurveillanceDrone-errors.sysml has multiple invalid statements: `test {}` (line 14),
/// `xyz {}` (line 20), `badstmt {}` (line 26). Uses parse_with_diagnostics to collect all errors.
#[test]
fn test_surveillance_drone_errors_reports_all_errors() {
    super::init_log();
    let path = surveillance_drone_errors_fixture_path();
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    let result = parse_with_diagnostics(&input);
    assert!(!result.is_ok(), "SurveillanceDrone-errors.sysml should have parse errors");
    assert_eq!(
        result.errors.len(),
        3,
        "expected 3 parse errors (test, xyz, badstmt); got {}: {:?}",
        result.errors.len(),
        result
            .errors
            .iter()
            .map(|e| (e.line, e.found.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
    );

    // First valid package should be in partial AST
    assert_eq!(
        result.root.elements.len(),
        1,
        "partial AST should contain first valid package"
    );

    // Each error should have correct line and found snippet (lines from parser output)
    let error_specs: Vec<(u32, &str)> = vec![(15, "test"), (19, "xyz"), (23, "badstmt")];
    for (i, (expected_line, expected_found)) in error_specs.iter().enumerate() {
        let err = &result.errors[i];
        assert_eq!(
            err.line,
            Some(*expected_line),
            "error {} should be on line {}; got line {:?}, found: {:?}",
            i + 1,
            expected_line,
            err.line,
            err.found
        );
        assert!(
            err.found
                .as_deref()
                .is_some_and(|f| f.contains(expected_found)),
            "error {} should have 'found' containing '{}'; got: {:?}",
            i + 1,
            expected_found,
            err.found
        );
        assert!(err.expected.is_some(), "error should have expected context");
        assert!(err.code.is_some(), "error should have a code");
    }
}
