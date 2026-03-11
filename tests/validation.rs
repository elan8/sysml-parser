//! Integration tests that parse SysML validation fixture files.
//!
//! Each validation .sysml file has a corresponding test module under `validation/`
//! for easier maintenance.
//!
//! Logging is initialized with default level DEBUG. Use `--nocapture` to see log output when running tests.
//! Set `RUST_LOG` to override (e.g. `RUST_LOG=trace`).

#[path = "validation/parts_tree_1a.rs"]
mod parts_tree_1a;

/// Initialize the logger so that `log` statements in library and tests are visible.
/// Default level is DEBUG when `RUST_LOG` is not set.
pub(crate) fn init_log() {
    let mut builder = env_logger::Builder::from_default_env();
    if std::env::var("RUST_LOG").is_err() {
        builder.filter_level(log::LevelFilter::Debug);
    }
    let _ = builder.try_init();
}

#[path = "validation/parts_interconnection_2a.rs"]
mod parts_interconnection_2a;

#[path = "validation/function_based_behavior_3a.rs"]
mod function_based_behavior_3a;

#[path = "validation/functional_allocation_4a.rs"]
mod functional_allocation_4a;

#[path = "validation/surveillance_drone.rs"]
mod surveillance_drone;

#[path = "validation/surveillance_drone_minimal.rs"]
mod surveillance_drone_minimal;
