//! Performance benchmarks for parse_file operations
//!
//! Run with: cargo bench --bench parse_bench

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

fn bench_parse_file_by_language(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_file_by_language");

    let test_cases = vec![
        ("rust", "src/calculator.rs"),
        ("python", "calculator.py"),
        ("javascript", "calculator.js"),
        ("typescript", "calculator.ts"),
    ];

    for (lang, file) in test_cases {
        let file_path = fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        group.bench_with_input(BenchmarkId::from_parameter(lang), &arguments, |b, args| {
            b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(args)));
        });
    }

    group.finish();
}

fn bench_parse_file_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_file_by_size");

    // Benchmark files of different sizes
    let test_cases = vec![
        ("small", "rust", "src/models/mod.rs"),  // ~100 lines
        ("medium", "rust", "src/calculator.rs"), // ~200 lines
        ("large", "python", "calculator.py"),    // ~300 lines
    ];

    for (size, lang, file) in test_cases {
        let file_path = fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        group.bench_with_input(BenchmarkId::from_parameter(size), &arguments, |b, args| {
            b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(args)));
        });
    }

    group.finish();
}

fn bench_parse_file_with_code_inclusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_file_code_inclusion");

    let file_path = fixture_path("rust", "src/calculator.rs");

    // Benchmark with code inclusion (default behavior)
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    group.bench_function("with_code", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&arguments)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_file_by_language,
    bench_parse_file_by_size,
    bench_parse_file_with_code_inclusion
);
criterion_main!(benches);
