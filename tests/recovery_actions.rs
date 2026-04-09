use sysml_v2_parser::ast::{ActionDefBody, ActionDefBodyElement, PackageBody, PackageBodyElement, RootElement};
use sysml_v2_parser::parse_with_diagnostics;

#[test]
fn action_recovery_inserts_error_node_and_keeps_later_sibling() {
    let input = r#"package P {
action def A {
  badstmt {};
  action good { };
}
}"#;
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let action = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::ActionDef(a) => Some(&a.value),
            _ => None,
        })
        .expect("action def should be present");
    let ActionDefBody::Brace { elements } = &action.body else {
        panic!("expected action body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, ActionDefBodyElement::Error(_))),
        "malformed action member should be preserved as an error node"
    );
    assert!(
        elements.iter().any(|e| matches!(
            e.value,
            ActionDefBodyElement::ActionUsage(_)
        )),
        "later action members should still parse"
    );
}

