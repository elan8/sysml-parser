use sysml_v2_parser::ast::{
    CalcDefBody, CalcDefBodyElement, PackageBody, PackageBodyElement, RootElement,
};
use sysml_v2_parser::parse_with_diagnostics;

#[test]
fn calc_keeps_library_tolerant_other_nodes_without_new_diagnostics() {
    let input = "package P { calc def K { objective fuelEconomy { } return result: Real; } }";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "accepted calc-library constructs should remain diagnostic-free: {:?}",
        result.errors
    );
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let calc = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::CalcDef(c) => Some(&c.value),
            _ => None,
        })
        .expect("calc def should be present");
    let CalcDefBody::Brace { elements } = &calc.body else {
        panic!("expected calc body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, CalcDefBodyElement::Other(_))),
        "accepted unmodeled calc constructs should remain as Other(...)"
    );
}
