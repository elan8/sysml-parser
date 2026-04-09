use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn read_file_to_string(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(content.replace("\r\n", "\n").replace('\r', "\n"))
}

fn bench_parse_with_diagnostics(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_with_diagnostics");

    // Primary fixture from external examples repo.
    let drone = Path::new(r"C:\Git\sysml-examples\drone\sysml\SurveillanceDrone.sysml");
    if let Some(input) = read_file_to_string(drone) {
        group.bench_with_input(
            BenchmarkId::new("SurveillanceDrone", drone.display().to_string()),
            &input,
            |b, input| {
                b.iter(|| {
                    let result = sysml_v2_parser::parse_with_diagnostics(black_box(input));
                    black_box(result.root.elements.len());
                    black_box(result.errors.len());
                })
            },
        );
    }

    // Optional SysML v2 release fixtures (skip if release directory not present).
    let release_root = std::env::var_os("SYSML_V2_RELEASE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release")
        });

    let candidates = [
        release_root
            .join("sysml")
            .join("src")
            .join("validation")
            .join("10-Analysis and Trades")
            .join("10d-Dynamics Analysis.sysml"),
        release_root
            .join("sysml")
            .join("src")
            .join("validation")
            .join("18-Use Case")
            .join("18-Use Case.sysml"),
    ];

    for path in candidates {
        if let Some(input) = read_file_to_string(&path) {
            let id = path
                .strip_prefix(&release_root)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            group.bench_with_input(
                BenchmarkId::new("SysML_v2_release", id),
                &input,
                |b, input| {
                    b.iter(|| {
                        let result = sysml_v2_parser::parse_with_diagnostics(black_box(input));
                        black_box(result.root.elements.len());
                        black_box(result.errors.len());
                    })
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_parse_with_diagnostics);
criterion_main!(benches);
