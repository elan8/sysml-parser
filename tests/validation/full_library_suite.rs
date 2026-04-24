//! Full library suite: parse all .sysml/.kerml files in SysML v2 Release `sysml.library`.
//!
//! Requires SYSML_V2_RELEASE_DIR (or sysml-v2-release in repo). This test is ignored by
//! default because it is slower and intended for compliance/debug runs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sysml_v2_parser::ast::{PackageBody, PackageBodyElement, RootElement, RootNamespace};
use sysml_v2_parser::{parse_with_diagnostics, ParseError};

/// Root of the SysML v2 Release tree (from env or the local sysml-v2-release directory).
fn sysml_v2_release_root() -> PathBuf {
    std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
}

/// Path to the library directory (SysML v2 Release `sysml.library`).
fn library_dir() -> PathBuf {
    sysml_v2_release_root().join("sysml.library")
}

fn find_library_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(find_library_files(&path)?);
        } else if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("sysml") | Some("kerml")
        ) {
            files.push(path);
        }
    }
    Ok(files)
}

fn first_errors_summary(errors: &[ParseError], max_errors: usize) -> String {
    errors
        .iter()
        .take(max_errors)
        .map(|e| {
            format!(
                "[line={:?}, col={:?}, code={:?}, found={:?}] {}",
                e.line, e.column, e.code, e.found, e.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn classify_error(err: &ParseError) -> String {
    let found = err.found.as_deref().unwrap_or("").trim();
    if found.is_empty() {
        return format!("code:{}", err.code.as_deref().unwrap_or("unknown"));
    }
    let lower = found.to_ascii_lowercase();
    let patterns = [
        "abstract action def",
        "action def",
        "abstract allocation",
        "allocation def",
        "abstract analysis def",
        "analysis def",
        "abstract case def",
        "case def",
        "abstract calc def",
        "calc def",
        "abstract connection def",
        "connection def",
        "abstract constraint def",
        "constraint def",
        "abstract flow def",
        "flow def",
        "abstract interface def",
        "interface def",
        "abstract item def",
        "item def",
        "abstract metadata def",
        "metadata def",
        "abstract part def",
        "part def",
        "abstract port def",
        "port def",
        "abstract requirement def",
        "requirement def",
        "private abstract constraint def",
        "abstract state def",
        "state def",
        "use case def",
        "use case ",
        "abstract verification def",
        "verification def",
        "abstract view def",
        "view def",
        "abstract viewpoint def",
        "viewpoint def",
        "abstract rendering def",
        "rendering def",
        "enum def",
    ];
    for pattern in patterns {
        if lower.starts_with(pattern) {
            return pattern.to_string();
        }
    }
    lower
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_bnf_decl_counts(root: &RootNamespace, counts: &mut BTreeMap<String, usize>) {
    for element in &root.elements {
        match &element.value {
            RootElement::Package(p) => collect_bnf_decl_counts_in_body(&p.value.body, counts),
            RootElement::LibraryPackage(p) => {
                collect_bnf_decl_counts_in_body(&p.value.body, counts)
            }
            RootElement::Namespace(n) => collect_bnf_decl_counts_in_body(&n.value.body, counts),
            RootElement::Import(_) => {}
        }
    }
}

fn collect_bnf_decl_counts_in_body(body: &PackageBody, counts: &mut BTreeMap<String, usize>) {
    let PackageBody::Brace { elements } = body else {
        return;
    };
    for element in elements {
        match &element.value {
            PackageBodyElement::FeatureDecl(n) => {
                *counts
                    .entry(format!("dedicated:{}", n.value.keyword))
                    .or_insert(0) += 1;
            }
            PackageBodyElement::ClassifierDecl(n) => {
                *counts
                    .entry(format!("dedicated:{}", n.value.keyword))
                    .or_insert(0) += 1;
            }
            PackageBodyElement::KermlSemanticDecl(n) => {
                *counts
                    .entry(format!("bnf:{}", n.value.bnf_production))
                    .or_insert(0) += 1;
            }
            PackageBodyElement::KermlFeatureDecl(n) => {
                *counts
                    .entry(format!("bnf:{}", n.value.bnf_production))
                    .or_insert(0) += 1;
            }
            PackageBodyElement::ExtendedLibraryDecl(n) => {
                *counts
                    .entry(format!("bnf:{}", n.value.bnf_production))
                    .or_insert(0) += 1;
            }
            PackageBodyElement::Package(n) => {
                collect_bnf_decl_counts_in_body(&n.value.body, counts)
            }
            PackageBodyElement::LibraryPackage(n) => {
                collect_bnf_decl_counts_in_body(&n.value.body, counts)
            }
            _ => {}
        }
    }
}

fn collect_package_body_type_counts(root: &RootNamespace, counts: &mut BTreeMap<String, usize>) {
    for element in &root.elements {
        match &element.value {
            RootElement::Package(p) => collect_body_type_counts(&p.value.body, counts),
            RootElement::LibraryPackage(p) => collect_body_type_counts(&p.value.body, counts),
            RootElement::Namespace(n) => collect_body_type_counts(&n.value.body, counts),
            RootElement::Import(_) => {}
        }
    }
}

fn collect_body_type_counts(body: &PackageBody, counts: &mut BTreeMap<String, usize>) {
    let PackageBody::Brace { elements } = body else {
        return;
    };
    for element in elements {
        let key = match &element.value {
            PackageBodyElement::ExtendedLibraryDecl(_) => "ExtendedLibraryDecl",
            PackageBodyElement::KermlSemanticDecl(_) => "KermlSemanticDecl",
            PackageBodyElement::KermlFeatureDecl(_) => "KermlFeatureDecl",
            PackageBodyElement::FeatureDecl(_) => "FeatureDecl",
            PackageBodyElement::ClassifierDecl(_) => "ClassifierDecl",
            PackageBodyElement::ActionDef(_) => "ActionDef",
            PackageBodyElement::AttributeDef(_) => "AttributeDef",
            PackageBodyElement::CalcDef(_) => "CalcDef",
            PackageBodyElement::CaseDef(_) => "CaseDef",
            PackageBodyElement::ConnectionDef(_) => "ConnectionDef",
            PackageBodyElement::ConstraintDef(_) => "ConstraintDef",
            PackageBodyElement::FlowDef(_) => "FlowDef",
            PackageBodyElement::InterfaceDef(_) => "InterfaceDef",
            PackageBodyElement::ItemDef(_) => "ItemDef",
            PackageBodyElement::MetadataDef(_) => "MetadataDef",
            PackageBodyElement::PartDef(_) => "PartDef",
            PackageBodyElement::PortDef(_) => "PortDef",
            PackageBodyElement::RequirementDef(_) => "RequirementDef",
            PackageBodyElement::StateDef(_) => "StateDef",
            PackageBodyElement::ViewDef(_) => "ViewDef",
            PackageBodyElement::ViewpointDef(_) => "ViewpointDef",
            PackageBodyElement::RenderingDef(_) => "RenderingDef",
            PackageBodyElement::Package(n) => {
                collect_body_type_counts(&n.value.body, counts);
                "Package"
            }
            PackageBodyElement::LibraryPackage(n) => {
                collect_body_type_counts(&n.value.body, counts);
                "LibraryPackage"
            }
            _ => "Other",
        };
        *counts.entry(key.to_string()).or_insert(0) += 1;
    }
}

fn env_threshold(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

fn collect_extended_texts(body: &PackageBody, out: &mut Vec<String>) {
    let PackageBody::Brace { elements } = body else {
        return;
    };
    for element in elements {
        match &element.value {
            PackageBodyElement::ExtendedLibraryDecl(n) => out.push(n.value.text.clone()),
            PackageBodyElement::Package(n) => collect_extended_texts(&n.value.body, out),
            PackageBodyElement::LibraryPackage(n) => collect_extended_texts(&n.value.body, out),
            _ => {}
        }
    }
}

/// Full library suite: parse all SysML/KerML library sources from SysML-v2-Release.
///
/// Run with: `cargo test --test validation test_full_library_suite -- --include-ignored --nocapture`
#[test]
#[ignore = "slow; requires SysML v2 release library sources; run with --include-ignored"]
fn test_full_library_suite() {
    super::init_log();

    let library_path = library_dir();
    if !library_path.exists() {
        log::debug!("Library directory not found: {:?}", library_path);
        log::debug!("Skipping. Run `scripts/fetch-sysml-v2-release.*` or set SYSML_V2_RELEASE_DIR");
        return;
    }

    let mut files = find_library_files(&library_path).expect("Failed to find library files");
    files.sort();

    assert!(
        !files.is_empty(),
        "No .sysml/.kerml files found in sysml.library"
    );

    let mut failed_files = Vec::new();
    let mut files_with_diagnostics = 0usize;

    for file in &files {
        let relative_path = file
            .strip_prefix(&library_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", relative_path, e));

        let result = parse_with_diagnostics(&content);
        if result.errors.is_empty() {
            eprintln!("✓ {}", relative_path);
            continue;
        }

        files_with_diagnostics += 1;

        let has_start_error = result.errors.iter().any(|e| {
            let at_start = e.offset == Some(0)
                || (e.line == Some(1) && e.column == Some(1))
                || e.found
                    .as_deref()
                    .is_some_and(|f| f.starts_with("standard library package"));
            at_start
                && matches!(
                    e.code.as_deref(),
                    Some("expected_keyword") | Some("expected_alt")
                )
        });

        if has_start_error || result.root.elements.is_empty() {
            let sample_errors = result
                .errors
                .iter()
                .take(3)
                .map(|e| {
                    format!(
                        "[line={:?}, col={:?}, code={:?}, found={:?}] {}",
                        e.line, e.column, e.code, e.found, e.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            failed_files.push((relative_path, sample_errors));
        } else {
            eprintln!(
                "⚠ {} (parsed with {} diagnostics)",
                relative_path,
                result.errors.len()
            );
        }
    }

    if !failed_files.is_empty() {
        for (file, details) in &failed_files {
            eprintln!("✗ {}: {}", file, details);
        }
        panic!(
            "Library suite: {} hard failures, {} files with diagnostics, {} files total.",
            failed_files.len(),
            files_with_diagnostics,
            files.len()
        );
    }

    eprintln!(
        "Library suite completed: {} files checked ({} with non-fatal diagnostics).",
        files.len(),
        files_with_diagnostics
    );
}

/// Strict subset suite for fast grammar hardening:
/// parse all files in `Systems Library` and require zero diagnostics.
///
/// Run with:
/// `cargo test --test validation test_systems_library_strict_no_diagnostics -- --include-ignored --nocapture`
#[test]
#[ignore = "strict gate for Systems Library syntax hardening"]
fn test_systems_library_strict_no_diagnostics() {
    super::init_log();

    let systems_path = library_dir().join("Systems Library");
    if !systems_path.exists() {
        log::debug!("Systems Library directory not found: {:?}", systems_path);
        log::debug!("Skipping. Run `scripts/fetch-sysml-v2-release.*` or set SYSML_V2_RELEASE_DIR");
        return;
    }

    let mut files =
        find_library_files(&systems_path).expect("Failed to find Systems Library files");
    files.sort();
    assert!(
        !files.is_empty(),
        "No .sysml/.kerml files found in Systems Library"
    );

    let mut failures = Vec::new();
    let mut bnf_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut pattern_counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in &files {
        let relative_path = file
            .strip_prefix(&systems_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", relative_path, e));
        let result = parse_with_diagnostics(&content);
        collect_bnf_decl_counts(&result.root, &mut bnf_counts);
        if !result.errors.is_empty() {
            for err in &result.errors {
                *pattern_counts.entry(classify_error(err)).or_insert(0) += 1;
            }
            failures.push((
                relative_path,
                result.errors.len(),
                first_errors_summary(&result.errors, 3),
            ));
        } else {
            eprintln!("✓ {}", relative_path);
        }
    }

    if !failures.is_empty() {
        let mut top_bnf = bnf_counts.into_iter().collect::<Vec<_>>();
        top_bnf.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Top modeled BNF declarations:");
        for (pattern, count) in top_bnf.into_iter().take(10) {
            eprintln!("  - {}: {}", pattern, count);
        }
        let mut top_patterns = pattern_counts.into_iter().collect::<Vec<_>>();
        top_patterns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Top diagnostic patterns:");
        for (pattern, count) in top_patterns.into_iter().take(10) {
            eprintln!("  - {}: {}", pattern, count);
        }
        for (path, n, sample) in &failures {
            eprintln!("✗ {} ({} diagnostics): {}", path, n, sample);
        }
        panic!(
            "Systems Library strict suite failed: {} of {} files produced diagnostics.",
            failures.len(),
            files.len()
        );
    }
}

/// Strict full-library gate: require zero diagnostics for all sysml.library files.
///
/// Run with:
/// `cargo test --test validation test_full_library_strict_no_diagnostics -- --include-ignored --nocapture`
#[test]
#[ignore = "strict full-library gate (zero diagnostics)"]
fn test_full_library_strict_no_diagnostics() {
    super::init_log();

    let library_path = library_dir();
    if !library_path.exists() {
        log::debug!("Library directory not found: {:?}", library_path);
        log::debug!("Skipping. Run `scripts/fetch-sysml-v2-release.*` or set SYSML_V2_RELEASE_DIR");
        return;
    }

    let mut files = find_library_files(&library_path).expect("Failed to find library files");
    files.sort();
    assert!(
        !files.is_empty(),
        "No .sysml/.kerml files found in sysml.library"
    );

    let mut failures = Vec::new();
    let mut pattern_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut bnf_counts: BTreeMap<String, usize> = BTreeMap::new();

    for file in &files {
        let relative_path = file
            .strip_prefix(&library_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", relative_path, e));
        let result = parse_with_diagnostics(&content);
        collect_bnf_decl_counts(&result.root, &mut bnf_counts);
        if result.errors.is_empty() {
            eprintln!("✓ {}", relative_path);
            continue;
        }
        for err in &result.errors {
            *pattern_counts.entry(classify_error(err)).or_insert(0) += 1;
        }
        failures.push((
            relative_path,
            result.errors.len(),
            first_errors_summary(&result.errors, 3),
        ));
    }

    if !failures.is_empty() {
        let mut top_bnf = bnf_counts.into_iter().collect::<Vec<_>>();
        top_bnf.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Top modeled BNF declarations:");
        for (pattern, count) in top_bnf.into_iter().take(15) {
            eprintln!("  - {}: {}", pattern, count);
        }
        let mut top_patterns = pattern_counts.into_iter().collect::<Vec<_>>();
        top_patterns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Top diagnostic patterns:");
        for (pattern, count) in top_patterns.into_iter().take(15) {
            eprintln!("  - {}: {}", pattern, count);
        }
        for (path, n, sample) in &failures {
            eprintln!("✗ {} ({} diagnostics): {}", path, n, sample);
        }
        panic!(
            "Full library strict suite failed: {} of {} files produced diagnostics.",
            failures.len(),
            files.len()
        );
    }
}

/// Node-shape quality gate for SysML standard library.
/// This test intentionally fails when `ExtendedLibraryDecl` is still used there.
///
/// Run with:
/// `cargo test --test validation test_systems_library_node_types_no_extended -- --include-ignored --nocapture`
#[test]
#[ignore = "quality gate: ensure systems library maps to dedicated node types"]
fn test_systems_library_node_types_no_extended() {
    super::init_log();

    let systems_path = library_dir().join("Systems Library");
    if !systems_path.exists() {
        log::debug!("Systems Library directory not found: {:?}", systems_path);
        return;
    }

    let mut files =
        find_library_files(&systems_path).expect("Failed to find Systems Library files");
    files.sort();
    assert!(!files.is_empty(), "No systems library files found");

    let mut type_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut extended_by_file = Vec::new();
    let mut sample_texts = Vec::new();

    for file in &files {
        let relative = file
            .strip_prefix(&systems_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", relative, e));
        let result = parse_with_diagnostics(&content);
        collect_package_body_type_counts(&result.root, &mut type_counts);

        let mut file_counts = BTreeMap::new();
        collect_package_body_type_counts(&result.root, &mut file_counts);
        let n_extended = *file_counts.get("ExtendedLibraryDecl").unwrap_or(&0);
        if n_extended > 0 {
            let mut snippets = Vec::new();
            for root in &result.root.elements {
                match &root.value {
                    RootElement::Package(n) => collect_extended_texts(&n.value.body, &mut snippets),
                    RootElement::LibraryPackage(n) => {
                        collect_extended_texts(&n.value.body, &mut snippets)
                    }
                    RootElement::Namespace(n) => {
                        collect_extended_texts(&n.value.body, &mut snippets)
                    }
                    RootElement::Import(_) => {}
                }
            }
            for s in snippets.into_iter().take(2) {
                sample_texts.push((relative.clone(), s));
            }
            extended_by_file.push((relative, n_extended));
        }
    }

    let n_extended_total = *type_counts.get("ExtendedLibraryDecl").unwrap_or(&0);
    let n_semantic_total = *type_counts.get("KermlSemanticDecl").unwrap_or(&0);
    let n_feature_total = *type_counts.get("KermlFeatureDecl").unwrap_or(&0);
    eprintln!("Systems Library node-type counts:");
    let mut sorted_counts = type_counts.into_iter().collect::<Vec<_>>();
    sorted_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (k, v) in sorted_counts {
        eprintln!("  - {}: {}", k, v);
    }

    if n_extended_total > 0 {
        extended_by_file.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Files still mapped as ExtendedLibraryDecl:");
        for (path, n) in extended_by_file.iter().take(10) {
            eprintln!("  - {}: {}", path, n);
        }
    }

    assert_eq!(
        n_extended_total, 0,
        "Systems Library still contains ExtendedLibraryDecl nodes ({} total)",
        n_extended_total
    );
    if let Some(threshold) = env_threshold("SYSTEMS_LIBRARY_KERML_SEMANTIC_MAX") {
        assert!(
            n_semantic_total <= threshold,
            "Systems Library still contains KermlSemanticDecl nodes ({} total, threshold {})",
            n_semantic_total,
            threshold
        );
    }
    if let Some(threshold) = env_threshold("SYSTEMS_LIBRARY_KERML_FEATURE_MAX") {
        assert!(
            n_feature_total <= threshold,
            "Systems Library still contains KermlFeatureDecl nodes ({} total, threshold {})",
            n_feature_total,
            threshold
        );
    }
}

/// Node-shape quality gate for the full SysML standard library.
///
/// Run with:
/// `cargo test --test validation test_full_library_node_types_no_extended -- --include-ignored --nocapture`
#[test]
#[ignore = "quality gate: ensure full std library maps to dedicated node types"]
fn test_full_library_node_types_no_extended() {
    super::init_log();

    let full_path = library_dir();
    if !full_path.exists() {
        log::debug!("Library directory not found: {:?}", full_path);
        return;
    }

    let mut files = find_library_files(&full_path).expect("Failed to find full library files");
    files.sort();
    assert!(!files.is_empty(), "No full library files found");

    let mut type_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut extended_by_file = Vec::new();
    let mut sample_texts = Vec::new();

    for file in &files {
        let relative = file
            .strip_prefix(&full_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", relative, e));
        let result = parse_with_diagnostics(&content);
        collect_package_body_type_counts(&result.root, &mut type_counts);

        let mut file_counts = BTreeMap::new();
        collect_package_body_type_counts(&result.root, &mut file_counts);
        let n_extended = *file_counts.get("ExtendedLibraryDecl").unwrap_or(&0);
        if n_extended > 0 {
            let mut snippets = Vec::new();
            for root in &result.root.elements {
                match &root.value {
                    RootElement::Package(n) => collect_extended_texts(&n.value.body, &mut snippets),
                    RootElement::LibraryPackage(n) => {
                        collect_extended_texts(&n.value.body, &mut snippets)
                    }
                    RootElement::Namespace(n) => {
                        collect_extended_texts(&n.value.body, &mut snippets)
                    }
                    RootElement::Import(_) => {}
                }
            }
            for s in snippets.into_iter().take(2) {
                sample_texts.push((relative.clone(), s));
            }
            extended_by_file.push((relative, n_extended));
        }
    }

    let n_extended_total = *type_counts.get("ExtendedLibraryDecl").unwrap_or(&0);
    let n_semantic_total = *type_counts.get("KermlSemanticDecl").unwrap_or(&0);
    let n_feature_total = *type_counts.get("KermlFeatureDecl").unwrap_or(&0);
    // Staged burn-down support:
    // - historical checkpoints: 1206 -> <=900 -> <=600 -> <=300 -> 0
    // - default is strict hard-0 once migration is complete.
    let threshold = std::env::var("FULL_LIBRARY_EXTENDED_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    eprintln!("Full library node-type counts:");
    let mut sorted_counts = type_counts.into_iter().collect::<Vec<_>>();
    sorted_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (k, v) in sorted_counts {
        eprintln!("  - {}: {}", k, v);
    }

    if n_extended_total > 0 {
        extended_by_file.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        eprintln!("Files still mapped as ExtendedLibraryDecl:");
        for (path, n) in extended_by_file.iter().take(20) {
            eprintln!("  - {}: {}", path, n);
        }
        eprintln!("ExtendedLibraryDecl samples:");
        for (path, sample) in sample_texts.iter().take(20) {
            eprintln!("  - {} => {}", path, sample.replace('\n', " "));
        }
    }

    assert!(
        n_extended_total <= threshold,
        "Full std library still contains ExtendedLibraryDecl nodes ({} total, threshold {})",
        n_extended_total,
        threshold
    );
    if let Some(threshold) = env_threshold("FULL_LIBRARY_KERML_SEMANTIC_MAX") {
        assert!(
            n_semantic_total <= threshold,
            "Full std library still contains KermlSemanticDecl nodes ({} total, threshold {})",
            n_semantic_total,
            threshold
        );
    }
    if let Some(threshold) = env_threshold("FULL_LIBRARY_KERML_FEATURE_MAX") {
        assert!(
            n_feature_total <= threshold,
            "Full std library still contains KermlFeatureDecl nodes ({} total, threshold {})",
            n_feature_total,
            threshold
        );
    }
}
