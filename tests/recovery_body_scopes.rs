use sysml_v2_parser::ast::{
    PackageBody, PackageBodyElement, RequirementDefBody, RequirementDefBodyElement, RootElement,
    StateDefBody, StateDefBodyElement, UseCaseDefBody, UseCaseDefBodyElement,
};
use sysml_v2_parser::parse_with_diagnostics;

#[test]
fn requirement_recovery_keeps_later_members() {
    let input = "package P {\nrequirement def R {\nsubject laptop: ;\nrequire constraint { }\n}\n}";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let req = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::RequirementDef(r) => Some(&r.value),
            _ => None,
        })
        .expect("requirement def should be present");
    let RequirementDefBody::Brace { elements } = &req.body else {
        panic!("expected requirement body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, RequirementDefBodyElement::Error(_))),
        "malformed requirement member should be preserved as an error node"
    );
    assert!(
        elements.iter().any(|e| matches!(
            e.value,
            RequirementDefBodyElement::RequireConstraint(_)
        )),
        "later requirement members should still parse"
    );
}

#[test]
fn use_case_recovery_keeps_later_members() {
    let input = "package P {\nuse case def U {\nactor user: ;\nobjective { }\n}\n}";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let use_case = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::UseCaseDef(u) => Some(&u.value),
            _ => None,
        })
        .expect("use case def should be present");
    let UseCaseDefBody::Brace { elements } = &use_case.body else {
        panic!("expected use case body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::Error(_))),
        "malformed use case member should be preserved as an error node"
    );
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::Objective(_))),
        "later use case members should still parse"
    );
}

#[test]
fn state_recovery_keeps_later_members() {
    let input =
        "package P {\nstate def Machine {\nstate: Mode;\ntransition t then Ready;\n}\n}";
    let result = parse_with_diagnostics(input);
    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let state_def = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::StateDef(s) => Some(&s.value),
            _ => None,
        })
        .expect("state def should be present");
    let StateDefBody::Brace { elements } = &state_def.body else {
        panic!("expected state body");
    };
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, StateDefBodyElement::Error(_))),
        "malformed state member should be preserved as an error node"
    );
    assert!(
        elements
            .iter()
            .any(|e| matches!(e.value, StateDefBodyElement::Transition(_))),
        "later state members should still parse"
    );
}

