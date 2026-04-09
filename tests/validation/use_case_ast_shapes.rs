use std::path::PathBuf;

use sysml_v2_parser::ast::{PackageBody, PackageBodyElement, RootElement, UseCaseDefBodyElement};
use sysml_v2_parser::parse_root;

fn sysml_v2_release_root() -> PathBuf {
    std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
}

fn fixture_path() -> PathBuf {
    sysml_v2_release_root()
        .join("sysml")
        .join("src")
        .join("validation")
        .join("18-Use Case")
        .join("18-Use Case.sysml")
}

#[test]
fn test_use_case_validation_fixture_has_typed_succession_nodes() {
    super::init_log();

    let path = fixture_path();
    if !path.exists() {
        // Allow running without the submodule present.
        return;
    }

    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    let root = parse_root(&input).expect("fixture should parse");
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package root element"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };

    let use_case_body = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::UseCaseUsage(u) => Some(&u.value.body),
            PackageBodyElement::UseCaseDef(u) => Some(&u.value.body),
            _ => None,
        })
        .expect("expected a use case in validation fixture");

    let sysml_v2_parser::ast::UseCaseDefBody::Brace { elements: body } = use_case_body else {
        panic!("expected brace body");
    };

    assert!(
        body.iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::FirstSuccession(_))),
        "expected `first ...;` to parse as FirstSuccession"
    );
    assert!(
        body.iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::ThenIncludeUseCase(_))),
        "expected `then include ... {{}}` to parse as ThenIncludeUseCase"
    );
    assert!(
        body.iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::ThenUseCaseUsage(_))),
        "expected `then use case ... {{}}` to parse as ThenUseCaseUsage"
    );
    assert!(
        body.iter()
            .any(|e| matches!(e.value, UseCaseDefBodyElement::ThenDone(_))),
        "expected `then done;` to parse as ThenDone"
    );
}
