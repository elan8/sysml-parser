use sysml_v2_parser::ast::{
    PackageBody, PackageBodyElement, RootElement, ViewBody, ViewBodyElement, ViewDefBody,
    ViewDefBodyElement,
};
use sysml_v2_parser::parse_with_diagnostics;

#[test]
fn view_def_recovery_inserts_error_node_and_keeps_later_render() {
    let input = "package P { view def V { filter ; render r : Renderer; } }";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let view = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::ViewDef(v) => Some(&v.value),
            _ => None,
        })
        .expect("view def should be present");
    let ViewDefBody::Brace { elements } = &view.body else {
        panic!("expected view def body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, ViewDefBodyElement::Error(_))),
        "malformed view-def member should be preserved as an error node"
    );
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, ViewDefBodyElement::ViewRendering(_))),
        "later render member should still parse"
    );
}

#[test]
fn view_usage_recovery_inserts_error_node_and_keeps_later_satisfy() {
    let input = "package P { view v : V { expose ; satisfy VP; } }";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let view = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::ViewUsage(v) => Some(&v.value),
            _ => None,
        })
        .expect("view usage should be present");
    let ViewBody::Brace { elements } = &view.body else {
        panic!("expected view body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, ViewBodyElement::Error(_))),
        "malformed view member should be preserved as an error node"
    );
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, ViewBodyElement::Satisfy(_))),
        "later satisfy member should still parse"
    );
}
