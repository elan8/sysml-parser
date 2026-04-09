//! Validation test for `tests/fixtures/KitchenTimer.sysml`.

use std::path::Path;
use sysml_v2_parser::{parse, parse_with_diagnostics, RootElement};

fn kitchen_timer_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("KitchenTimer.sysml")
}

#[test]
fn test_parse_kitchen_timer() {
    super::init_log();
    let path = kitchen_timer_fixture_path();
    let input = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    let strict = parse(&input).expect("strict parse should succeed");
    assert_eq!(strict.elements.len(), 1);

    let result = parse_with_diagnostics(&input);
    assert_eq!(
        result.root.elements.len(),
        1,
        "fixture should produce one root package"
    );
    match &result.root.elements[0].value {
        RootElement::Package(pkg) => {
            assert_eq!(pkg.identification.name.as_deref(), Some("KitchenTimer"));
        }
        other => panic!("expected root element to be package, got {:?}", other),
    }

    assert_eq!(
        result.errors.len(),
        0,
        "KitchenTimer should parse without diagnostics, got {:?}",
        result.errors
    );
}
