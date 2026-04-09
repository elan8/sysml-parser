use std::path::PathBuf;

use sysml_v2_parser::ast::{
    ActionDefBody, ActionDefBodyElement, ActionUsageBody, ActionUsageBodyElement, Package,
    PackageBody, PackageBodyElement, RootElement, UseCaseDefBody, UseCaseDefBodyElement,
};
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
        .join("10-Analysis and Trades")
        .join("10d-Dynamics Analysis.sysml")
}

fn scan_package(pkg: &Package, for_loops: &mut usize, then_assigns: &mut usize) {
    let PackageBody::Brace { elements } = &pkg.body else {
        return;
    };

    for el in elements {
        match &el.value {
            PackageBodyElement::Package(p) => scan_package(&p.value, for_loops, then_assigns),
            PackageBodyElement::ActionDef(a) => {
                scan_action_def_body(&a.value.body, for_loops, then_assigns)
            }
            PackageBodyElement::ActionUsage(a) => {
                scan_action_usage_body(&a.value.body, for_loops, then_assigns)
            }
            PackageBodyElement::AnalysisCaseDef(a) => {
                scan_use_case_body(&a.value.body, for_loops, then_assigns)
            }
            PackageBodyElement::AnalysisCaseUsage(a) => {
                scan_use_case_body(&a.value.body, for_loops, then_assigns)
            }
            _ => {}
        }
    }
}

fn scan_action_def_body(body: &ActionDefBody, for_loops: &mut usize, then_assigns: &mut usize) {
    let ActionDefBody::Brace { elements } = body else {
        return;
    };
    for el in elements {
        match &el.value {
            ActionDefBodyElement::ForLoop(_) => *for_loops += 1,
            ActionDefBodyElement::Assign(a) if a.value.is_then => *then_assigns += 1,
            ActionDefBodyElement::ActionUsage(u) => {
                scan_action_usage_body(&u.value.body, for_loops, then_assigns)
            }
            _ => {}
        }
    }
}

fn scan_action_usage_body(body: &ActionUsageBody, for_loops: &mut usize, then_assigns: &mut usize) {
    let ActionUsageBody::Brace { elements } = body else {
        return;
    };
    for el in elements {
        match &el.value {
            ActionUsageBodyElement::ForLoop(_) => *for_loops += 1,
            ActionUsageBodyElement::Assign(a) if a.value.is_then => *then_assigns += 1,
            ActionUsageBodyElement::ActionUsage(u) => {
                scan_action_usage_body(&u.value.body, for_loops, then_assigns)
            }
            _ => {}
        }
    }
}

fn scan_use_case_body(body: &UseCaseDefBody, for_loops: &mut usize, then_assigns: &mut usize) {
    let UseCaseDefBody::Brace { elements } = body else {
        return;
    };
    for el in elements {
        match &el.value {
            UseCaseDefBodyElement::ForLoop(fl) => {
                *for_loops += 1;
                scan_action_def_body(&fl.value.body, for_loops, then_assigns);
            }
            UseCaseDefBodyElement::Assign(a) if a.value.is_then => *then_assigns += 1,
            _ => {}
        }
    }
}

#[test]
fn test_action_validation_fixture_has_typed_for_and_then_assign_nodes() {
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

    let mut for_loops = 0usize;
    let mut then_assigns = 0usize;
    for el in &root.elements {
        let RootElement::Package(p) = &el.value else {
            continue;
        };
        scan_package(&p.value, &mut for_loops, &mut then_assigns);
    }

    assert!(
        for_loops > 0,
        "expected at least one `for ... {{ ... }}` to parse as a typed ForLoop"
    );
    assert!(
        then_assigns > 0,
        "expected at least one `then assign ... := ...;` to parse as a typed AssignStmt with is_then=true"
    );
}
