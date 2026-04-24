use std::fs;
use std::path::Path;

use sysml_v2_parser::ast::{
    PackageBody, PackageBodyElement, PartDefBody, PartDefBodyElement, RootElement,
};
use sysml_v2_parser::parse_with_diagnostics;

const MISSION_PACKAGE_FILE: &str = r"C:\Git\apollo-11-sysml-v2\Purpose\MissionPackage.sysml";

#[test]
fn missionpackage_structures_connection_and_comment_members() {
    let path = Path::new(MISSION_PACKAGE_FILE);
    if !path.exists() {
        eprintln!(
            "Skipping integration check; file not found: {}",
            MISSION_PACKAGE_FILE
        );
        return;
    }

    let content = fs::read_to_string(path).expect("mission package file should be readable");
    let parsed = parse_with_diagnostics(&content);
    assert!(
        parsed.errors.is_empty(),
        "unexpected diagnostics while parsing mission package file: {:?}",
        parsed.errors
    );

    let mut structured_connection_count = 0usize;
    let mut structured_comment_count = 0usize;
    let mut opaque_connection_count = 0usize;
    let mut fallback_comment_count = 0usize;

    let package = match &parsed.root.elements[0].value {
        RootElement::Package(package) => &package.value,
        other => panic!("expected package root, got {other:?}"),
    };
    let PackageBody::Brace { elements } = &package.body else {
        panic!("expected package brace body");
    };
    for element in elements {
        let PackageBodyElement::PartDef(part_def) = &element.value else {
            continue;
        };
        let PartDefBody::Brace { elements } = &part_def.value.body else {
            continue;
        };
        for member in elements {
            match &member.value {
                PartDefBodyElement::Connection(_) => structured_connection_count += 1,
                PartDefBodyElement::Comment(_) => structured_comment_count += 1,
                PartDefBodyElement::OpaqueMember(opaque)
                    if opaque.value.keyword == "connection" =>
                {
                    opaque_connection_count += 1
                }
                PartDefBodyElement::Other(text) if text.starts_with("comment ") => {
                    fallback_comment_count += 1
                }
                _ => {}
            }
        }
    }

    assert!(
        structured_connection_count > 0,
        "expected structured connection members in mission package"
    );
    assert!(
        structured_comment_count > 0,
        "expected structured comment members in mission package"
    );
    assert_eq!(
        opaque_connection_count, 0,
        "connection members should not degrade to opaque members"
    );
    assert_eq!(
        fallback_comment_count, 0,
        "comment members should not degrade to fallback Other nodes"
    );
}
