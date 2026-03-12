//! Tests that AST node spans (offset, line, column, len) are calculated correctly.

use sysml_parser::ast::{
    AstNode, PackageBody, PackageBodyElement, PartUsageBody, PartUsageBodyElement, RootElement,
};
use sysml_parser::parse;

fn assert_span(
    node: &impl AstNode,
    offset: usize,
    line: u32,
    column: usize,
    len: usize,
    msg: &str,
) {
    let s = node.span();
    assert_eq!(s.offset, offset, "{} (offset)", msg);
    assert_eq!(s.line, line, "{} (line)", msg);
    assert_eq!(s.column, column, "{} (column)", msg);
    assert_eq!(s.len, len, "{} (len)", msg);
}

#[test]
fn test_single_line_package_span() {
    let input = "package Foo;";
    let result = parse(input).expect("parse should succeed");
    assert_eq!(result.elements.len(), 1, "one top-level element");

    let elem = &result.elements[0];
    assert_span(elem, 0, 1, 1, 12, "root element covers full input");

    if let RootElement::Package(pkg) = &elem.value {
        assert_span(pkg, 0, 1, 1, 12, "inner package node span");
    } else {
        panic!("expected Package element");
    }
}

#[test]
fn test_multi_line_second_element_span() {
    let line1 = "package Foo;";
    let newline = "\n";
    let line2 = "package Bar;";
    let input = format!("{}{}{}", line1, newline, line2);
    let result = parse(&input).expect("parse should succeed");
    assert_eq!(result.elements.len(), 2, "two top-level elements");

    // First element span covers "package Foo;" only (newline consumed by ws between elements).
    let len1 = line1.len();
    let len2 = line2.len();
    let offset2 = line1.len() + newline.len();

    assert_span(
        &result.elements[0],
        0,
        1,
        1,
        len1,
        "first element ends after first line (no newline in span)",
    );
    assert_span(
        &result.elements[1],
        offset2,
        2,
        1,
        len2,
        "second element starts at line 2, column 1",
    );
}

#[test]
fn test_nested_expression_span() {
    // Part usage with bind; the literal 100 should have a span covering "100".
    let input = "package P { part u : T { bind x = 100; } }";
    let result = parse(input).expect("parse should succeed");
    let elem = &result.elements[0];
    let RootElement::Package(pkg) = &elem.value else {
        panic!("expected Package");
    };
    let body = match &pkg.body {
        PackageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let part_usage_elem = &body[0];
    let PackageBodyElement::PartUsage(part_usage) = &**part_usage_elem else {
        panic!("expected PartUsage");
    };
    let part_body = match &part_usage.body {
        PartUsageBody::Brace { elements } => elements,
        _ => panic!("expected brace body"),
    };
    let bind_elem = &part_body[0];
    let PartUsageBodyElement::Bind(bind) = &**bind_elem else {
        panic!("expected Bind");
    };
    let right = &bind.right;
    let offset_100 = input.find("100").expect("substring 100");
    // Column is 1-based: offset 0 is column 1, so offset_100 is column offset_100 + 1.
    assert_span(right, offset_100, 1, offset_100 + 1, 3, "literal 100 span");
}
