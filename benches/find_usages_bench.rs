//! Performance benchmarks for find_usages operations
//!
//! Run with: cargo bench --bench find_usages_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use std::path::PathBuf;

fn fixture_path(lang: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}_project", lang))
        .join(file)
}

fn fixture_dir(lang: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}_project", lang))
}

fn bench_find_usages_by_context_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_usages_by_context_lines");

    let file_path = fixture_path("rust", "src/calculator.rs");

    for context_lines in [0, 2, 5, 10] {
        let arguments = json!({
            "symbol": "add",
            "path": file_path.to_str().unwrap(),
            "context_lines": context_lines
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(context_lines),
            &arguments,
            |b, args| {
                b.iter(|| treesitter_mcp::analysis::find_usages::execute(black_box(args)));
            },
        );
    }

    group.finish();
}

fn bench_find_usages_by_language(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_usages_by_language");

    let test_cases = vec![
        ("rust", "src/calculator.rs", "add"),
        ("python", "calculator.py", "add"),
        ("javascript", "calculator.js", "add"),
        ("typescript", "calculator.ts", "add"),
    ];

    for (lang, file, symbol) in test_cases {
        let file_path = fixture_path(lang, file);
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        group.bench_with_input(BenchmarkId::from_parameter(lang), &arguments, |b, args| {
            b.iter(|| treesitter_mcp::analysis::find_usages::execute(black_box(args)));
        });
    }

    group.finish();
}

fn bench_find_usages_cross_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_usages_cross_file");

    let dir_path = fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap(),
        "context_lines": 2
    });

    group.bench_function("rust_project", |b| {
        b.iter(|| treesitter_mcp::analysis::find_usages::execute(black_box(&arguments)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_find_usages_by_context_lines,
    bench_find_usages_by_language,
    bench_find_usages_cross_file
);
criterion_main!(benches);
