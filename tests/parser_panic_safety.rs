use std::panic::{catch_unwind, AssertUnwindSafe};

use proptest::prelude::*;
use sysml_parser::{parse, parse_with_diagnostics};

fn malformed_inputs() -> Vec<&'static str> {
    vec![
        "",
        "{",
        "}",
        "}}}}",
        "/* unterminated",
        "// no newline",
        "package",
        "package {",
        "package P {",
        "package P { part def A {",
        "package P { part def A {} part def B",
        "package P { part def A { invalid invalid invalid }",
        "package P { requirement def R { \"unterminated }",
        "part def TopLevel;",
        "namespace N { part def X { ;;; }",
        "package P { ////***",
        "package P { /* nested /* comment */",
        "package P { ]]]]]",
        "package P { @@@ ??? ### }",
        "package P { transition then first / * + - }",
        "library standard package LegacyStd {",
        "import Views::*",
        "package P; garbage tokens that should fail strictly",
        "package P { action def A { in x : T out y : U }",
        "package P { requirement def R { subject vehicle : Vehicle attribute mass: Mass } }",
    ]
}

#[test]
fn parse_never_panics_on_known_malformed_inputs() {
    for input in malformed_inputs() {
        let strict = catch_unwind(AssertUnwindSafe(|| parse(input)));
        assert!(
            strict.is_ok(),
            "parse() panicked for malformed input: {:?}",
            input
        );

        let recovered = catch_unwind(AssertUnwindSafe(|| parse_with_diagnostics(input)));
        assert!(
            recovered.is_ok(),
            "parse_with_diagnostics() panicked for malformed input: {:?}",
            input
        );
    }
}

proptest! {
    #[test]
    fn arbitrary_text_never_panics(random in ".*") {
        let strict = catch_unwind(AssertUnwindSafe(|| parse(&random)));
        prop_assert!(
            strict.is_ok(),
            "parse() panicked for generated input of len {}",
            random.len()
        );

        let recovered = catch_unwind(AssertUnwindSafe(|| parse_with_diagnostics(&random)));
        prop_assert!(
            recovered.is_ok(),
            "parse_with_diagnostics() panicked for generated input of len {}",
            random.len()
        );
    }
}
