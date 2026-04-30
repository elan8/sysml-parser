use std::fs;
use std::path::PathBuf;

use sysml_v2_parser::ast::{
    PackageBody, PackageBodyElement, PartDefBody, PartDefBodyElement, RequirementDefBody,
    RequirementDefBodyElement, RootElement, UseCaseDefBody, UseCaseDefBodyElement,
};
use sysml_v2_parser::{parse_with_diagnostics, DiagnosticCategory};

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
        Some("unsupported_annotation_syntax"),
        "bad line should be reported as unsupported annotation syntax"
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

#[test]
fn fixture_invalid_qualified_name_separator_reports_specific_fix() {
    let input = fixture("invalid-qualified-name-separator.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(
        err.code.as_deref(),
        Some("invalid_qualified_name_separator")
    );
    assert_eq!(
        err.expected.as_deref(),
        Some("qualified name segments separated by '::'")
    );
    assert!(err
        .suggestion
        .as_deref()
        .is_some_and(|s| s.contains("expose A::B;")));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::ViewUsage(_))));
}

#[test]
fn fixture_incomplete_bind_expression_reports_missing_expression() {
    let input = fixture("incomplete-bind-expression.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(
        err.code.as_deref(),
        Some("missing_expression_after_operator")
    );
    assert_eq!(
        err.expected.as_deref(),
        Some("binding expression after '='")
    );
    assert!(err
        .found
        .as_deref()
        .is_some_and(|found| found.contains("bind status = ;")));
    let action = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::ActionDef(action)
                if action.value.identification.name.as_deref() == Some("ExecutePatrol") =>
            {
                Some(&action.value)
            }
            _ => None,
        })
        .expect("expected action definition");
    assert!(matches!(
        action.body,
        sysml_v2_parser::ast::ActionDefBody::Brace { .. }
    ));
}

#[test]
fn fixture_missing_body_or_semicolon_reports_declaration_terminator_error() {
    let input = fixture("missing-body-or-semicolon.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(2));
    assert_eq!(err.code.as_deref(), Some("missing_body_or_semicolon"));
    assert_eq!(
        err.expected.as_deref(),
        Some("';' or '{' after declaration header")
    );
    assert!(err
        .suggestion
        .as_deref()
        .is_some_and(|s| s.contains("part def Wheel")));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::PartDef(_))));
}

#[test]
fn fixture_unexpected_extra_closing_brace_is_localized() {
    let input = fixture("unexpected-extra-closing-brace.sysml");
    let result = parse_with_diagnostics(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(4));
    assert_eq!(err.code.as_deref(), Some("unexpected_closing_brace"));
    assert_eq!(err.found.as_deref(), Some("}"));
}

#[test]
fn fixture_invalid_typing_operator_reports_specific_fix() {
    let input = fixture("invalid-typing-operator.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(2));
    assert_eq!(err.code.as_deref(), Some("invalid_typing_operator"));
    assert_eq!(
        err.expected.as_deref(),
        Some("':>' specialization operator")
    );
    assert!(err
        .suggestion
        .as_deref()
        .is_some_and(|s| s.contains(":> BaseVehicle")));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, PackageBodyElement::PartDef(_))));
}

#[test]
fn fixture_unexpected_keyword_in_requirement_body_reports_scope_specific_error() {
    let input = fixture("unexpected-keyword-in-requirement-body.sysml");
    let (result, elements) = package_elements(&input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.line, Some(3));
    assert_eq!(err.code.as_deref(), Some("unexpected_keyword_in_scope"));
    assert!(err.message.contains("unexpected keyword `then`"));
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
        .any(|e| matches!(e.value, RequirementDefBodyElement::RequireConstraint(_))));
}

#[test]
fn diagnostics_include_taxonomy_categories() {
    let parse_err = parse_with_diagnostics("package P { part def A { part: Wheel; } }");
    let parse_err_entry = parse_err
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("missing_member_name"))
        .expect("missing member name diagnostic expected");
    assert_eq!(
        parse_err_entry.category,
        Some(DiagnosticCategory::ParseError)
    );

    let unsupported = parse_with_diagnostics("package P { #fmeaspec requirement req1 { } }");
    let unsupported_entry = unsupported
        .errors
        .iter()
        .find(|e| e.code.as_deref() == Some("unsupported_annotation_syntax"))
        .expect("unsupported annotation diagnostic expected");
    assert_eq!(
        unsupported_entry.category,
        Some(DiagnosticCategory::UnsupportedGrammarForm)
    );
}

#[test]
fn invalid_unit_reference_reports_specific_diagnostic() {
    let input = "package P { action def Evaluate { bind measuredMass = []; in result: Real; } }";
    let (result, elements) = package_elements(input);

    assert_eq!(
        result.errors.len(),
        1,
        "unexpected diagnostics: {:?}",
        result.errors
    );
    let err = &result.errors[0];
    assert_eq!(err.code.as_deref(), Some("invalid_unit_reference"));
    assert_eq!(err.expected.as_deref(), Some("unit name inside '[ ]'"));
    assert!(err
        .suggestion
        .as_deref()
        .is_some_and(|s| s.contains("[kg]")));

    let action = elements
        .iter()
        .find_map(|element| match &element.value {
            PackageBodyElement::ActionDef(action)
                if action.value.identification.name.as_deref() == Some("Evaluate") =>
            {
                Some(&action.value)
            }
            _ => None,
        })
        .expect("expected action definition Evaluate");
    let sysml_v2_parser::ast::ActionDefBody::Brace { elements } = &action.body else {
        panic!("expected action definition brace body");
    };
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, sysml_v2_parser::ast::ActionDefBodyElement::Error(_))));
    assert!(elements
        .iter()
        .any(|e| matches!(e.value, sysml_v2_parser::ast::ActionDefBodyElement::InOutDecl(_))));
}
