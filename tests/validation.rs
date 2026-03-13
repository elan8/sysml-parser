//! Integration tests that parse SysML validation fixture files.
//!
//! Each validation .sysml file has a corresponding test module under `validation/`
//! for easier maintenance.
//!
//! Logging defaults to WARN so test output stays small. Use `RUST_LOG=debug` (or
//! `RUST_LOG=sysml_parser=debug`) and `--nocapture` when debugging parser behavior.

#[path = "validation/parts_tree_1a.rs"]
mod parts_tree_1a;

/// Initialize the logger. Default level is WARN so failures don't flood with DEBUG.
/// Set `RUST_LOG=debug` (or `RUST_LOG=sysml_parser=debug`) when debugging.
pub(crate) fn init_log() {
    let mut builder = env_logger::Builder::from_default_env();
    if std::env::var("RUST_LOG").is_err() {
        builder.filter_level(log::LevelFilter::Warn);
    }
    let _ = builder.try_init();
}

/// Asserts that parsed and expected ASTs are equal. Normalizes parsed (strips optional
/// spans) so comparison matches hand-built expected AST. On failure, panics with a short
/// message (first difference position and snippet) instead of dumping full ASTs.
pub(crate) fn assert_ast_eq(
    parsed: &sysml_parser::ast::RootNamespace,
    expected: &sysml_parser::ast::RootNamespace,
    msg: &str,
) {
    let normalized = parsed.normalize_for_test_comparison();
    if normalized == *expected {
        return;
    }
    let pa = format!("{:?}", normalized);
    let pe = format!("{:?}", expected);
    let pos = pa
        .chars()
        .zip(pe.chars())
        .position(|(a, b)| a != b)
        .unwrap_or(pa.len().min(pe.len()));
    let snippet: String = pa
        .chars()
        .skip(pos.saturating_sub(80))
        .take(160)
        .collect();
    panic!(
        "{}: AST mismatch at char {} (parsed {} chars, expected {} chars). Snippet: ...{}... \
         Set RUST_LOG=debug and run with --nocapture for full parser trace.",
        msg, pos, pa.len(), pe.len(), snippet
    );
}

#[path = "validation/parts_interconnection_2a.rs"]
mod parts_interconnection_2a;

#[path = "validation/function_based_behavior_3a.rs"]
mod function_based_behavior_3a;

#[path = "validation/functional_allocation_4a.rs"]
mod functional_allocation_4a;

#[path = "validation/full_validation_suite.rs"]
mod full_validation_suite;

#[path = "validation/surveillance_drone.rs"]
mod surveillance_drone;

#[path = "validation/surveillance_drone_minimal.rs"]
mod surveillance_drone_minimal;

#[path = "validation/traffic_light_intersection.rs"]
mod traffic_light_intersection;
