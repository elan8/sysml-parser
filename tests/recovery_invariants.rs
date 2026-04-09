use std::panic::{catch_unwind, AssertUnwindSafe};

use sysml_v2_parser::parse_with_diagnostics;

#[test]
fn malformed_view_and_constraint_inputs_do_not_panic() {
    let inputs = [
        "package P { view def V { filter ; render r : Renderer; } }",
        "package P { view v : V { expose ; satisfy VP; } }",
        "package P { constraint def C { in : Real; total >= limit; } }",
        "package P { calc def K { return : Real; result; } }",
    ];

    for input in inputs {
        let recovered = catch_unwind(AssertUnwindSafe(|| parse_with_diagnostics(input)));
        assert!(
            recovered.is_ok(),
            "parse_with_diagnostics should not panic for {:?}",
            input
        );
    }
}
