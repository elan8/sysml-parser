//! Minimal parser tests to isolate why SurveillanceDrone.sysml fails.
//! See plan: fix_surveillancedrone_test — step 1.

use std::path::Path;
use sysml_parser::ast::{PackageBody, PackageBodyElement, RootElement};
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
        sysml_parser::ast::RootElement::Package(p) => &p.value,
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
        sysml_parser::ast::RootElement::Package(p) => &p.value,
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
        sysml_parser::ast::RootElement::Package(p) => &p.value,
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

/// Doc comments inside perform bodies must be parsed as Doc elements, not skipped.
#[test]
fn test_perform_body_doc_comment_parsed_as_element() {
    super::init_log();
    let input = r#"package P {
    part def T { }
    part p : T {
        perform 'act' {
            doc /* allocation comment */
            in x = p.x;
        }
    }
}"#;
    let result = parse(input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("parse should succeed: {:?}", e),
    };
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let pkg_body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let part_usage = pkg_body
        .iter()
        .find_map(|e| match &e.value { PackageBodyElement::PartUsage(p) => Some(&p.value), _ => None })
        .expect("expected part usage p");
    let part_body = match &part_usage.body {
        sysml_parser::ast::PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected part brace body"),
    };
    let perform = part_body
        .iter()
        .find_map(|e| match &e.value { sysml_parser::ast::PartUsageBodyElement::Perform(p) => Some(&p.value), _ => None })
        .expect("expected perform");
    let perform_body = match &perform.body {
        sysml_parser::ast::PerformBody::Brace { elements } => elements,
        _ => panic!("expected perform brace body"),
    };
    assert_eq!(perform_body.len(), 2, "perform body must have Doc then InOut (doc comments are not skipped)");
    match &perform_body[0].value {
        sysml_parser::ast::PerformBodyElement::Doc(d) => assert!(
            d.value.text.contains("allocation comment"),
            "doc text should contain the comment content, got {:?}",
            d.value.text
        ),
        other => panic!("expected first element Doc, got {:?}", other),
    }
    match &perform_body[1].value {
        sysml_parser::ast::PerformBodyElement::InOut(b) => {
            assert_eq!(b.value.direction, sysml_parser::ast::InOut::In);
            assert_eq!(b.value.name, "x");
        }
        other => panic!("expected second element InOut, got {:?}", other),
    }
}

/// KerML: import with FilterPackage form `QualifiedName [ expr ] [ expr ]+ ;`
#[test]
fn test_import_filter_package() {
    super::init_log();
    let input = r#"package P {
    import MyPkg [ 1 ] [ 2 + 3 ];
    attribute def Real;
}"#;
    let result = parse(input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("parse should succeed: {:?}", e),
    };
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected Package"),
    };
    let body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let imp = match &body[0].value {
        PackageBodyElement::Import(i) => &i.value,
        _ => panic!("expected Import"),
    };
    assert!(imp.filter_members.is_some(), "expected filter_members");
    let members = imp.filter_members.as_ref().unwrap();
    assert_eq!(members.len(), 2, "two filter members [ 1 ] [ 2 + 3 ]");
    assert_eq!(imp.target, "MyPkg");
}

/// KerML: top-level namespace declaration (same body as package).
#[test]
fn test_namespace_declaration() {
    super::init_log();
    let input = r#"namespace N {
    attribute def Real;
}"#;
    let result = parse(input);
    let root = match &result {
        Ok(ast) => ast,
        Err(e) => panic!("parse should succeed: {:?}", e),
    };
    let ns = match &root.elements[0].value {
        RootElement::Namespace(n) => &n.value,
        _ => panic!("expected Namespace"),
    };
    assert_eq!(ns.identification.name.as_deref(), Some("N"));
    let body = match &ns.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    assert_eq!(body.len(), 1);
    match &body[0].value {
        PackageBodyElement::AttributeDef(a) => assert_eq!(a.name, "Real"),
        _ => panic!("expected AttributeDef Real"),
    }
}
