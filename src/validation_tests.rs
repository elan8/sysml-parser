//! Validation tests for SysML v2 parser
//!
//! This module contains tests that validate the parser against
//! the official SysML v2 validation files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{PackageBody, RootElement, RootNamespace};
use crate::parser::parse_root;
use crate::ParseError;

/// Environment variable for the root of a SysML-v2-Release clone (directory that contains `sysml/`).
/// Used by CI; if unset, falls back to the sysml-v2-release submodule in this crate.
pub const SYSML_V2_RELEASE_DIR_ENV: &str = "SYSML_V2_RELEASE_DIR";

/// Root of the SysML v2 Release tree (from env or the sysml-v2-release submodule).
fn sysml_v2_release_root() -> PathBuf {
    std::env::var_os(SYSML_V2_RELEASE_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
}

/// Get the path to the validation directory (SysML v2 Release `sysml/src/validation`).
fn validation_dir() -> PathBuf {
    sysml_v2_release_root()
        .join("sysml")
        .join("src")
        .join("validation")
}

/// Find all .sysml files in a directory recursively.
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

/// Count packages/namespaces and body elements in a RootNamespace (for summary logging).
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

/// Parse a SysML file. Returns (root, line_count) on success.
fn parse_file(file_path: &Path) -> Result<(RootNamespace, usize), ParseError> {
    let content = fs::read_to_string(file_path).map_err(|e| {
        ParseError::new(format!("failed to read file: {}", e))
    })?;
    let n_lines = content.lines().count();
    let root = parse_root(&content)?;
    Ok((root, n_lines))
}

/// Initialize a file logger for tests so debug/info logs are written to
/// `test-logs/sysml-parser-tests.log` under the crate directory.
#[cfg(test)]
fn init_test_logger() {
    let _ = (|| {
        let log_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-logs");
        fs::create_dir_all(&log_dir).ok()?;
        let log_file = fs::File::create(log_dir.join("sysml-parser-tests.log")).ok()?;
        simplelog::WriteLogger::init(
            log::LevelFilter::Debug,
            simplelog::Config::default(),
            log_file,
        )
        .ok()
    })();
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, info};

    /// Full validation suite: parse all .sysml files in SysML-v2-Release sysml/src/validation.
    /// Expects zero parser errors. Ignores missing dir (returns early).
    ///
    /// This test is `#[ignore]` because it parses many files and is slow. Run with:
    ///   cargo test -p sysml-parser -- --ignored
    /// CI runs it via the validation job with SYSML_V2_RELEASE_DIR set.
    #[test]
    #[ignore = "slow; run with: cargo test -p sysml-parser -- --ignored"]
    fn test_full_validation_suite() {
        init_test_logger();

        let validation_path = validation_dir();

        if !validation_path.exists() {
            debug!("Validation directory not found: {:?}", validation_path);
            debug!(
                "Skipping. Run `git submodule update --init sysml-v2-release` or set {} to a SysML-v2-Release clone root",
                super::SYSML_V2_RELEASE_DIR_ENV
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
                    let summary = format!(
                        "✓ {} ({} pkgs, {} elements)",
                        relative_path, n_pkgs, n_elements
                    );
                    info!("{}", summary);
                    eprintln!("{}", summary);
                }
                Err(e) => {
                    debug!("✗ {} - Error: {}", relative_path, e);
                    failed_files.push((relative_path, e.to_string()));
                }
            }
        }

        if !failed_files.is_empty() {
            info!("\nFailed to parse {} file(s):", failed_files.len());
            for (file, error) in &failed_files {
                info!("  {}: {}", file, error);
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
        info!("{}", total_msg);
        eprintln!("{}", total_msg);
    }

    /// Test individual validation files (for easier debugging).
    #[test]
    fn test_parts_tree_basic() {
        init_test_logger();

        let file = validation_dir()
            .join("01-Parts Tree")
            .join("1a-Parts Tree.sysml");

        if file.exists() {
            parse_file(&file).map(|_| ()).expect("Failed to parse 1a-Parts Tree.sysml");
        } else {
            debug!("Test file not found: {:?}", file);
        }
    }
}
