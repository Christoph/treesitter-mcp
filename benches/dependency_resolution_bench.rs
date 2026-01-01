//! Performance benchmarks for dependency resolution in parse_file
//!
//! Run with: cargo bench --bench dependency_resolution_bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::path::PathBuf;

fn fixture_path(lang: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}_project", lang))
        .join(file)
}

fn bench_parse_file_without_deps(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    c.bench_function("parse_file_without_deps", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&arguments)));
    });
}

fn bench_parse_file_with_deps(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    c.bench_function("parse_file_with_deps", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&arguments)));
    });
}

fn bench_parse_file_with_deps_and_code(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": true,
    });

    c.bench_function("parse_file_with_deps_and_code", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&arguments)));
    });
}

fn bench_dependency_resolution_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_resolution_overhead");

    let file_path = fixture_path("rust", "src/lib.rs");

    // Without deps
    let no_deps_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    group.bench_function("no_deps", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&no_deps_args)));
    });

    // With deps
    let with_deps_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    group.bench_function("with_deps", |b| {
        b.iter(|| treesitter_mcp::analysis::parse_file::execute(black_box(&with_deps_args)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_file_without_deps,
    bench_parse_file_with_deps,
    bench_parse_file_with_deps_and_code,
    bench_dependency_resolution_overhead
);
criterion_main!(benches);
