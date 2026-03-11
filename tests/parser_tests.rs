//! TDD tests: SysML snippets with expected AST.

use sysml_parser::ast::{Identification, Package, PackageBody, PackageBodyElement, RootNamespace};
use sysml_parser::parse;

fn id(name: &str) -> Identification {
    Identification {
        short_name: None,
        name: Some(name.to_string()),
    }
}

/// Build expected AST for `package Foo;`
fn expected_package_foo_semicolon() -> RootNamespace {
    RootNamespace {
        elements: vec![PackageBodyElement::Package(Package {
            identification: id("Foo"),
            body: PackageBody::Semicolon,
        })],
    }
}

/// Build expected AST for `package Bar { }`
fn expected_package_bar_brace() -> RootNamespace {
    RootNamespace {
        elements: vec![PackageBodyElement::Package(Package {
            identification: id("Bar"),
            body: PackageBody::Brace { elements: vec![] },
        })],
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
