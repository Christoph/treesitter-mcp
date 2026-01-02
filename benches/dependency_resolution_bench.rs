//! Performance benchmarks for dependency resolution in view_code
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

fn bench_view_code_signatures_only(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
    });

    c.bench_function("view_code_signatures_only", |b| {
        b.iter(|| treesitter_mcp::analysis::view_code::execute(black_box(&arguments)));
    });
}

fn bench_view_code_full_code(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
    });

    c.bench_function("view_code_full_code", |b| {
        b.iter(|| treesitter_mcp::analysis::view_code::execute(black_box(&arguments)));
    });
}

fn bench_view_code_with_focus(c: &mut Criterion) {
    let file_path = fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "focus_symbol": "add",
    });

    c.bench_function("view_code_with_focus", |b| {
        b.iter(|| treesitter_mcp::analysis::view_code::execute(black_box(&arguments)));
    });
}

fn bench_detail_level_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("detail_level_overhead");

    let file_path = fixture_path("rust", "src/lib.rs");

    // Signatures only
    let sig_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
    });

    group.bench_function("signatures", |b| {
        b.iter(|| treesitter_mcp::analysis::view_code::execute(black_box(&sig_args)));
    });

    // Full code
    let full_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
    });

    group.bench_function("full", |b| {
        b.iter(|| treesitter_mcp::analysis::view_code::execute(black_box(&full_args)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_view_code_signatures_only,
    bench_view_code_full_code,
    bench_view_code_with_focus,
    bench_detail_level_overhead
);
criterion_main!(benches);
