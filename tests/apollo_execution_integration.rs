use std::fs;
use std::path::Path;

use sysml_v2_parser::ast::{
    OccurrenceBodyElement, OccurrenceUsageBody, PackageBody, PackageBodyElement, PartDefBody,
    PartDefBodyElement, PartUsageBody, PartUsageBodyElement, RootElement,
};
use sysml_v2_parser::parse_with_diagnostics;

const APOLLO_EXECUTION_FILE: &str =
    r"C:\Git\apollo-11-sysml-v2\Execution\Apollo11MissionExecutionPackage.sysml";

fn walk_occurrence(
    body: &OccurrenceUsageBody,
    structured_assert_count: &mut usize,
    degraded_assert_count: &mut usize,
) {
    let OccurrenceUsageBody::Brace { elements } = body else {
        return;
    };

    for element in elements {
        match &element.value {
            OccurrenceBodyElement::AssertConstraint(_) => *structured_assert_count += 1,
            OccurrenceBodyElement::Other(text) if text == "assert constraint" => {
                *degraded_assert_count += 1;
            }
            OccurrenceBodyElement::OccurrenceUsage(occurrence) => {
                walk_occurrence(
                    &occurrence.value.body,
                    structured_assert_count,
                    degraded_assert_count,
                );
            }
            _ => {}
        }
    }
}

#[test]
fn apollo_execution_file_preserves_assert_constraints_structurally() {
    let path = Path::new(APOLLO_EXECUTION_FILE);
    if !path.exists() {
        eprintln!(
            "Skipping integration check; file not found: {}",
            APOLLO_EXECUTION_FILE
        );
        return;
    }

    let content = fs::read_to_string(path).expect("apollo execution file should be readable");
    let parsed = parse_with_diagnostics(&content);
    assert!(
        parsed.errors.is_empty(),
        "unexpected diagnostics while parsing apollo execution file: {:?}",
        parsed.errors
    );

    let mut structured_assert_count = 0usize;
    let mut degraded_assert_count = 0usize;

    let package = match &parsed.root.elements[0].value {
        RootElement::Package(package) => &package.value,
        other => panic!("expected package root, got {other:?}"),
    };
    let PackageBody::Brace { elements } = &package.body else {
        panic!("expected package brace body");
    };
    for element in elements {
        match &element.value {
            PackageBodyElement::PartUsage(part_usage) => {
                if let PartUsageBody::Brace { elements } = &part_usage.value.body {
                    for body_element in elements {
                        if let PartUsageBodyElement::OccurrenceUsage(occurrence) =
                            &body_element.value
                        {
                            walk_occurrence(
                                &occurrence.value.body,
                                &mut structured_assert_count,
                                &mut degraded_assert_count,
                            );
                        }
                    }
                }
            }
            PackageBodyElement::PartDef(part_def) => {
                if let PartDefBody::Brace { elements } = &part_def.value.body {
                    for body_element in elements {
                        if let PartDefBodyElement::OccurrenceUsage(occurrence) = &body_element.value
                        {
                            walk_occurrence(
                                &occurrence.value.body,
                                &mut structured_assert_count,
                                &mut degraded_assert_count,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    assert!(
        structured_assert_count > 0,
        "expected at least one structured assert-constraint member in apollo execution file"
    );
    assert_eq!(
        degraded_assert_count, 0,
        "assert constraints should not degrade into OccurrenceBodyElement::Other"
    );
}
