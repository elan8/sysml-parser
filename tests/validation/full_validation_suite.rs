//! Full validation suite: parse all .sysml files in SysML v2 Release sysml/src/validation.
//!
//! Requires SYSML_V2_RELEASE_DIR (or sysml-v2-release in repo). The full-suite test is
//! `#[ignore]` and runs in CI via the validation job with `--include-ignored`.

use std::fs;
use std::path::{Path, PathBuf};

use sysml_parser::ast::{PackageBody, RootElement, RootNamespace};
use sysml_parser::{parse_root, ParseError};

/// Root of the SysML v2 Release tree (from env or the sysml-v2-release submodule).
fn sysml_v2_release_root() -> PathBuf {
    std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
}

/// Path to the validation directory (SysML v2 Release `sysml/src/validation`).
fn validation_dir() -> PathBuf {
    sysml_v2_release_root()
        .join("sysml")
        .join("src")
        .join("validation")
}

fn find_sysml_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(find_sysml_files(&path)?);
        } else if path.extension().and_then(|s| s.to_str()) == Some("sysml") {
            files.push(path);
        }
    }
    Ok(files)
}

fn count_packages_and_elements(root: &RootNamespace) -> (usize, usize) {
    let mut n_pkgs = 0;
    let mut n_elements = 0;
    for el in &root.elements {
        match &el.value {
            RootElement::Package(p) => {
                n_pkgs += 1;
                if let PackageBody::Brace { elements } = &p.value.body {
                    n_elements += elements.len();
                }
            }
            RootElement::LibraryPackage(lp) => {
                n_pkgs += 1;
                if let PackageBody::Brace { elements } = &lp.value.body {
                    n_elements += elements.len();
                }
            }
            RootElement::Namespace(n) => {
                n_pkgs += 1;
                if let PackageBody::Brace { elements } = &n.value.body {
                    n_elements += elements.len();
                }
            }
            RootElement::Import(_) => {}
        }
    }
    (n_pkgs, n_elements)
}

#[allow(clippy::result_large_err)]
fn parse_file(file_path: &Path) -> Result<(RootNamespace, usize), ParseError> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| ParseError::new(format!("failed to read file: {}", e)))?;
    let n_lines = content.lines().count();
    let root = parse_root(&content)?;
    Ok((root, n_lines))
}

/// Full validation suite: parse all .sysml files in SysML-v2-Release sysml/src/validation.
/// Expects zero parser errors. Skips if validation dir is missing.
///
/// Run with: `cargo test --test validation -- --include-ignored`
/// CI runs it in the validation job with SYSML_V2_RELEASE_DIR set.
#[test]
#[ignore = "slow; requires SysML v2 release; run with: cargo test --test validation -- --include-ignored"]
fn test_full_validation_suite() {
    super::init_log();

    let validation_path = validation_dir();

    if !validation_path.exists() {
        log::debug!("Validation directory not found: {:?}", validation_path);
        log::debug!(
            "Skipping. Run `git submodule update --init sysml-v2-release` or set SYSML_V2_RELEASE_DIR"
        );
        return;
    }

    let files = find_sysml_files(&validation_path).expect("Failed to find validation files");

    assert!(
        !files.is_empty(),
        "No .sysml files found in validation directory"
    );

    let mut failed_files = Vec::new();

    for file in &files {
        let relative_path = file
            .strip_prefix(&validation_path)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();

        match parse_file(file) {
            Ok((root, _n_lines)) => {
                let (n_pkgs, n_elements) = count_packages_and_elements(&root);
                log::info!(
                    "✓ {} ({} pkgs, {} elements)",
                    relative_path,
                    n_pkgs,
                    n_elements
                );
                eprintln!("✓ {} ({} pkgs, {} elements)", relative_path, n_pkgs, n_elements);
            }
            Err(e) => {
                log::debug!("✗ {} - Error: {}", relative_path, e);
                failed_files.push((relative_path, e.to_string()));
            }
        }
    }

    if !failed_files.is_empty() {
        for (file, error) in &failed_files {
            log::info!("  {}: {}", file, error);
            eprintln!("✗ {}: {}", file, error);
        }
        panic!(
            "Validation suite: {} of {} files failed to parse. See stderr for details.",
            failed_files.len(),
            files.len()
        );
    }

    let total_msg = format!(
        "Validation suite passed: {} files parsed successfully",
        files.len()
    );
    log::info!("{}", total_msg);
    eprintln!("{}", total_msg);
}
