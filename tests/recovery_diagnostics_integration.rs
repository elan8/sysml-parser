use std::fs;
use std::path::PathBuf;

use sysml_v2_parser::ast::{
    PackageBody, PackageBodyElement, PartDefBody, PartDefBodyElement, RequirementDefBody,
    RequirementDefBodyElement, RootElement, UseCaseDefBody, UseCaseDefBodyElement,
};
use sysml_v2_parser::parse_with_diagnostics;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    fs::read_to_string(path)
        .expect("fixture should be readable")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn package_elements(
    input: &str,
) -> (
    sysml_v2_parser::ParseResult,
    Vec<sysml_v2_parser::ast::Node<PackageBodyElement>>,
) {
    let result = parse_with_diagnostics(input);
    let elements = {
        let pkg = match &result.root.elements[0].value {
            RootElement::Package(p) => &p.value,
            _ => panic!("expected package"),
        };
        let PackageBody::Brace { elements } = &pkg.body else {
            panic!("expected brace body");
        };
        elements.clone()
    };
    (result, elements)
}

#[test]
fn fixture_missing_semicolon_reports_specific_diagnostic_and_keeps_siblings() {
    let input = fixture("missing-semicolon-true-positive.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(err.code.as_deref(), Some("missing_semicolon"));
    assert!(err
        .found
        .as_deref()
        .is_some_and(|found| found.contains("exhibit state s : S")));
    let part = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::PartDef(part)
                if part.value.identification.name.as_deref() == Some("A") =>
            {
                Some(&part.value)
            }
            _ => None,
        })
        .expect("expected part definition A");
    let PartDefBody::Brace { elements } = &part.body else {
        panic!("expected part definition brace body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::Error(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::PartUsage(_))));
}

#[test]
fn fixture_missing_name_does_not_fall_back_to_missing_semicolon() {
    let input = fixture("missing-semicolon-false-positive-name.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(err.code.as_deref(), Some("missing_member_name"));
    assert_ne!(err.code.as_deref(), Some("missing_semicolon"));
    let use_case = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::UseCaseDef(use_case) => Some(&use_case.value),
            _ => None,
        })
        .expect("expected use case definition");
    let UseCaseDefBody::Brace { elements } = &use_case.body else {
        panic!("expected use case brace body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, UseCaseDefBodyElement::Error(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, UseCaseDefBodyElement::Objective(_))));
}

#[test]
fn fixture_missing_type_does_not_fall_back_to_missing_semicolon() {
    let input = fixture("missing-semicolon-false-positive-type.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(err.code.as_deref(), Some("missing_type_reference"));
    assert_ne!(err.code.as_deref(), Some("missing_semicolon"));
    let requirement = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::RequirementDef(requirement) => Some(&requirement.value),
            _ => None,
        })
        .expect("expected requirement definition");
    let RequirementDefBody::Brace { elements } = &requirement.body else {
        panic!("expected requirement brace body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, RequirementDefBodyElement::Error(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, RequirementDefBodyElement::RequireConstraint(_))));
}

#[test]
fn fixture_single_bad_line_does_not_cascade_into_later_valid_lines() {
    let input = fixture("cascade-single-bad-line.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(2));
    assert_eq!(
        err.code.as_deref(),
        Some("recovered_package_body_element"),
        "bad line should stay a generic local recovery"
    );
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::PartDef(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::ActionDef(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::RequirementDef(_))));
}

#[test]
fn fixture_nested_bad_block_recovers_inside_part_and_keeps_outer_siblings() {
    let input = fixture("cascade-bad-block-then-valid-siblings.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(err.code.as_deref(), Some("missing_type_reference"));

    let broken = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::PartDef(part)
                if part.value.identification.name.as_deref() == Some("Broken") =>
            {
                Some(&part.value)
            }
            _ => None,
        })
        .expect("expected Broken part");
    let PartDefBody::Brace { elements } = &broken.body else {
        panic!("expected Broken brace body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::Error(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::Ref(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PartDefBodyElement::Ref(_))));
    assert!(result
        .root
        .elements
        .iter()
        .any(|e| matches!(e.value, RootElement::Package(_))));
    assert!(package_elements(&input)
        .1
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::ActionDef(_))));
}

#[test]
fn fixture_unmatched_brace_reports_local_eof_error_without_extra_recovery_noise() {
    let input = fixture("unmatched-brace-locality.sysml");
    let result = parse_with_diagnostics(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.code.as_deref(), Some("missing_closing_brace"));
    assert!(
        err.line.is_some_and(|line| line >= 5),
        "EOF brace diagnostic should stay near the end: {:?}",
        err
    );
    assert!(
        result.root.elements.is_empty()
            || result
                .root
                .elements
                .iter()
                .any(|e| matches!(e.value, RootElement::Package(_)))
    );
}
