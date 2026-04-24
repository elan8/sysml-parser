use sysml_v2_parser::ast::{PackageBody, PackageBodyElement, RootElement};
use sysml_v2_parser::{parse, parse_with_diagnostics};

#[test]
fn package_recovery_inserts_error_node_and_keeps_later_sibling() {
    let input = "package P {\n#fmeaspec requirement req1 { }\npart def Good;\n}";
    let root = parse(input).expect("package should parse with local recovery");
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::Error(_))),
        "expected package body error node"
    );
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, PackageBodyElement::PartDef(_))),
        "later valid sibling should still parse"
    );
}

#[test]
fn package_recovery_diagnostic_is_specific() {
    let input = "package P {\n#fmeaspec requirement req1 { }\npart def Good;\n}";
    let result = parse_with_diagnostics(input);
    let err = result
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("recovered_package_body_element"))
        .expect("expected package recovery diagnostic");
    assert!(
        err.message.contains("annotation"),
        "annotation recovery should explain the failure"
    );
    assert!(
        err.found
            .as_deref()
            .is_some_and(|f| f.contains("#fmeaspec")),
        "diagnostic should preserve recovered snippet"
    );
}
