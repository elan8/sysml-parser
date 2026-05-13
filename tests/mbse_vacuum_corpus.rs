use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use sysml_v2_parser::{parse_with_diagnostics, ParseError};

const MAX_DIAGNOSTICS_PER_FILE: usize = 25;

fn collect_sysml_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sysml_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "sysml") {
            files.push(path);
        }
    }
}

#[test]
#[ignore = "requires MBSE_VACUUM_EXAMPLE_DIR to point at the MBSE vacuum-cleaner corpus"]
fn mbse_vacuum_corpus_parse_with_diagnostics_is_bounded_and_panic_free() {
    let Some(root) = std::env::var_os("MBSE_VACUUM_EXAMPLE_DIR").map(PathBuf::from) else {
        eprintln!("MBSE_VACUUM_EXAMPLE_DIR is not set; skipping optional corpus regression");
        return;
    };
    assert!(
        root.is_dir(),
        "MBSE_VACUUM_EXAMPLE_DIR should be a directory: {}",
        root.display()
    );

    let mut files = Vec::new();
    collect_sysml_files(&root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "expected at least one .sysml file under {}",
        root.display()
    );

    let mut primary_codes = BTreeMap::<String, usize>::new();
    let mut per_file_counts = Vec::new();
    let mut over_limit = Vec::<(PathBuf, usize, BTreeMap<String, usize>, Vec<ParseError>)>::new();
    for path in files {
        let input = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let result = catch_unwind(AssertUnwindSafe(|| parse_with_diagnostics(&input)))
            .unwrap_or_else(|_| panic!("parse_with_diagnostics panicked for {}", path.display()));
        let diagnostic_count = result.errors.len();
        per_file_counts.push((path.clone(), diagnostic_count));
        for err in &result.errors {
            let code = err.code.clone().unwrap_or_else(|| "unknown".to_string());
            *primary_codes.entry(code).or_default() += 1;
        }
        if diagnostic_count > MAX_DIAGNOSTICS_PER_FILE {
            let mut file_codes = BTreeMap::<String, usize>::new();
            for err in &result.errors {
                let code = err.code.clone().unwrap_or_else(|| "unknown".to_string());
                *file_codes.entry(code).or_default() += 1;
            }
            over_limit.push((
                path.clone(),
                diagnostic_count,
                file_codes,
                result.errors.into_iter().take(25).collect(),
            ));
        }
    }

    per_file_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    eprintln!("MBSE vacuum corpus per-file diagnostic counts:");
    for (path, count) in &per_file_counts {
        eprintln!("  {count:>4}  {}", path.display());
    }
    eprintln!("MBSE vacuum corpus primary diagnostic codes: {primary_codes:#?}");
    if !over_limit.is_empty() {
        eprintln!("MBSE vacuum corpus over-limit detail:");
        for (path, count, file_codes, sample_errors) in &over_limit {
            eprintln!("  {count:>4}  {}", path.display());
            eprintln!("    codes: {file_codes:#?}");
            for err in sample_errors {
                eprintln!(
                    "    line {:?}, col {:?}, code {:?}, found {:?}: {}",
                    err.line, err.column, err.code, err.found, err.message
                );
            }
        }
    }
    assert!(
        over_limit.is_empty(),
        "diagnostic count should stay bounded at {MAX_DIAGNOSTICS_PER_FILE}; over-limit files: {over_limit:#?}"
    );
}
