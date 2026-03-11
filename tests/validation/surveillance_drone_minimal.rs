//! Minimal parser tests to isolate why SurveillanceDrone.sysml fails.
//! See plan: fix_surveillancedrone_test — step 1.

use std::path::Path;
use sysml_parser::ast::{PackageBody, PackageBodyElement};
use sysml_parser::parse;

/// Parses "package SurveillanceDrone { attribute def Real; }" (no doc/comment before first element).
/// If this passes, the failure with the full fixture is likely due to doc/comment handling.
#[test]
fn test_parse_minimal_package_one_attribute() {
    super::init_log();
    let input = "package SurveillanceDrone { attribute def Real; }";
    let result = parse(input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("minimal parse should succeed: {:?}", e),
    };
    assert_eq!(root.elements.len(), 1, "expected one root package");
    let pkg = match &root.elements[0].value {
        sysml_parser::ast::PackageBodyElement::Package(p) => &p.value,
        other => panic!("expected Package, got {:?}", other),
    };
    let body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(body.len(), 1, "expected one body element");
    match &body[0].value {
        PackageBodyElement::AttributeDef(a) => assert_eq!(a.name, "Real"),
        other => panic!("expected AttributeDef, got {:?}", other),
    }
}

/// Parses package with doc comment and line comment before first element (like the fixture).
#[test]
fn test_parse_package_with_doc_and_line_comment() {
    super::init_log();
    let input = r#"package SurveillanceDrone {
	doc /* Root package for the surveillance quadrotor drone example. */

	// ========== Value types (simplified) ==========
	attribute def Real;
}"#;
    let result = parse(input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("parse with doc/comment should succeed: {:?}", e),
    };
    assert_eq!(root.elements.len(), 1);
    let pkg = match &root.elements[0].value {
        sysml_parser::ast::PackageBodyElement::Package(p) => &p.value,
        other => panic!("expected Package, got {:?}", other),
    };
    let body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(!body.is_empty(), "expected at least one body element (doc + attribute def Real)");
    // First element is doc comment, second is attribute def Real
    match &body[0].value {
        PackageBodyElement::Doc(_) => {}
        other => panic!("expected first element Doc, got {:?}", other),
    }
    let attr = body.iter().find(|e| matches!(&e.value, PackageBodyElement::AttributeDef(a) if a.name == "Real"));
    assert!(attr.is_some(), "expected AttributeDef(Real) in body, got {:?}", body);
}

/// Uses the exact start of SurveillanceDrone.sysml (leading block comment + package + doc + first attribute).
/// Uses a complete package (with closing brace) since truncating the fixture would cut off mid-package.
#[test]
fn test_parse_fixture_exact_start() {
    super::init_log();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("SurveillanceDrone.sysml");
    let full = std::fs::read_to_string(&path).unwrap();
    let input: String = full
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let result = parse(&input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("fixture start should parse: {:?}", e),
    };
    assert_eq!(root.elements.len(), 1);
    let pkg = match &root.elements[0].value {
        sysml_parser::ast::PackageBodyElement::Package(p) => &p.value,
        other => panic!("expected Package, got {:?}", other),
    };
    let body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert!(!body.is_empty(), "expected at least one body element");
    // First element is doc comment, then attribute def Real
    let attr = body.iter().find(|e| matches!(&e.value, PackageBodyElement::AttributeDef(a) if a.name == "Real"));
    assert!(attr.is_some(), "expected AttributeDef(Real) in body, got {:?}", body);
}
