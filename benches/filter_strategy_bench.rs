//! Benchmarks for Filter Strategy Selection.
//!
//! Run with: `cargo bench --bench filter_strategy_bench`
//!
//! # Reproducibility
//!
//! These benchmarks are fully deterministic:
//! - All inputs are static (no random data generation)
//! - Filter expressions use fixed string literals
//! - Selectivity values are predefined constants
//!
//! Note: For benchmarks that do use RNG (e.g., selectivity estimation with
//! `estimate_selectivity`), pass `Some(42)` as the seed parameter to ensure
//! reproducibility.
//!
//! # What This Benchmarks
//!
//! 1. **Strategy Selection Overhead**: Time to select a strategy given selectivity
//! 2. **Tautology/Contradiction Detection**: Time to detect always-true/always-false filters
//! 3. **Oversample Calculation**: Time to compute oversample factor from selectivity
//! 4. **Threshold Boundaries**: Performance at critical selectivity decision points
//!
//! # Theoretical Basis for Thresholds
//!
//! The strategy selection thresholds are based on the cost model:
//!
//! - **PostFilter** (selectivity < 0.05): When very few vectors match, it's cheaper
//!   to compute all similarities and filter afterward. The threshold 0.05 means
//!   only 5% of vectors pass the filter.
//!
//! - **PreFilter** (selectivity > 0.80): When most vectors match, it's cheaper to
//!   filter first and search within the filtered subset. The threshold 0.80 means
//!   80% of vectors pass the filter.
//!
//! - **Hybrid** (0.05 ≤ selectivity ≤ 0.80): Oversample by 1/selectivity, search
//!   more neighbors than needed, then filter the results.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use edgevec::filter::strategy::{
    calculate_oversample, is_contradiction, is_tautology, select_strategy, FilterStrategy,
    POSTFILTER_THRESHOLD, PREFILTER_THRESHOLD,
};
use edgevec::filter::{parse, FilterExpr};
use std::hint::black_box;

/// Benchmark: Strategy selection overhead
///
/// Measures the time to parse a filter expression and select a strategy.
fn bench_strategy_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_selection");

    // Test expressions with varying complexity
    let expressions = [
        ("simple_eq", "category = \"gpu\""),
        ("simple_range", "price BETWEEN 100 AND 500"),
        ("compound_and", "category = \"gpu\" AND price < 500"),
        ("compound_or", "category = \"gpu\" OR category = \"cpu\""),
        (
            "complex",
            "(category = \"gpu\" AND price < 500) OR (rating >= 4.5 AND stock > 0)",
        ),
        (
            "deeply_nested",
            "((a = 1 AND b = 2) OR (c = 3 AND d = 4)) AND ((e = 5 OR f = 6) AND g = 7)",
        ),
    ];

    for (name, expr_str) in expressions {
        // Parse expression (includes parse overhead)
        let _expr = parse(expr_str).expect("Valid expression");

        // Benchmark strategy selection with typical selectivity
        group.bench_with_input(
            BenchmarkId::new("parse_and_select", name),
            &0.5f32,
            |b, &selectivity| b.iter(|| black_box(select_strategy(black_box(selectivity)))),
        );
    }

    group.finish();
}

/// Benchmark: Tautology/Contradiction detection
///
/// Measures the time to detect always-true and always-false expressions.
fn bench_tautology_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("tautology_detection");

    // Test expressions that are tautologies
    let tautologies = [
        ("literal_true", FilterExpr::LiteralBool(true)),
        (
            "or_with_not",
            FilterExpr::Or(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::Not(Box::new(FilterExpr::Field(
                    "x".to_string(),
                )))),
            ),
        ),
        (
            "between_all",
            FilterExpr::Between(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(i64::MIN)),
                Box::new(FilterExpr::LiteralInt(i64::MAX)),
            ),
        ),
    ];

    for (name, expr) in &tautologies {
        group.bench_with_input(BenchmarkId::new("is_tautology", name), expr, |b, e| {
            b.iter(|| black_box(is_tautology(black_box(e))))
        });
    }

    // Test expressions that are contradictions
    let contradictions = [
        ("literal_false", FilterExpr::LiteralBool(false)),
        (
            "and_with_not",
            FilterExpr::And(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::Not(Box::new(FilterExpr::Field(
                    "x".to_string(),
                )))),
            ),
        ),
        (
            "impossible_range",
            FilterExpr::Between(
                Box::new(FilterExpr::Field("x".to_string())),
                Box::new(FilterExpr::LiteralInt(100)),
                Box::new(FilterExpr::LiteralInt(50)),
            ),
        ),
    ];

    for (name, expr) in &contradictions {
        group.bench_with_input(BenchmarkId::new("is_contradiction", name), expr, |b, e| {
            b.iter(|| black_box(is_contradiction(black_box(e))))
        });
    }

    group.finish();
}

/// Benchmark: Oversample calculation
///
/// Measures the time to calculate the oversample factor for different selectivities.
fn bench_oversample_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("oversample_calculation");

    let selectivities = [0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99];

    for selectivity in selectivities {
        group.bench_with_input(
            BenchmarkId::new("calculate_oversample", format!("{:.2}", selectivity)),
            &selectivity,
            |b, &s| b.iter(|| black_box(calculate_oversample(black_box(s)))),
        );
    }

    group.finish();
}

/// Benchmark: Strategy selection at threshold boundaries
///
/// Tests performance at the critical selectivity thresholds where
/// strategy decisions change.
fn bench_threshold_boundaries(c: &mut Criterion) {
    let mut group = c.benchmark_group("threshold_boundaries");

    // Test at threshold boundaries
    let boundary_selectivities = [
        ("postfilter_below", POSTFILTER_THRESHOLD - 0.01),
        ("postfilter_at", POSTFILTER_THRESHOLD),
        ("postfilter_above", POSTFILTER_THRESHOLD + 0.01),
        ("hybrid_low", 0.2),
        ("hybrid_mid", 0.5),
        ("hybrid_high", 0.7),
        ("prefilter_below", PREFILTER_THRESHOLD - 0.01),
        ("prefilter_at", PREFILTER_THRESHOLD),
        ("prefilter_above", PREFILTER_THRESHOLD + 0.01),
    ];

    for (name, selectivity) in boundary_selectivities {
        group.bench_with_input(
            BenchmarkId::new("select_strategy", name),
            &selectivity,
            |b, &s| b.iter(|| black_box(select_strategy(black_box(s)))),
        );
    }

    group.finish();
}

/// Benchmark: Strategy selection with Auto mode
///
/// Measures the overhead of automatic strategy selection compared to
/// explicit strategy choices.
fn bench_auto_vs_explicit(c: &mut Criterion) {
    let mut group = c.benchmark_group("auto_vs_explicit");

    // Benchmark explicit strategy creation
    group.bench_function("explicit_postfilter", |b| {
        b.iter(|| black_box(FilterStrategy::PostFilter { oversample: 3.0 }))
    });

    group.bench_function("explicit_prefilter", |b| {
        b.iter(|| black_box(FilterStrategy::PreFilter))
    });

    group.bench_function("explicit_hybrid", |b| {
        b.iter(|| {
            black_box(FilterStrategy::Hybrid {
                oversample_min: 1.5,
                oversample_max: 10.0,
            })
        })
    });

    // Benchmark auto selection (using select_strategy with selectivity)
    group.bench_function("auto_selection", |b| {
        b.iter(|| black_box(select_strategy(black_box(0.5))))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_strategy_selection,
    bench_tautology_detection,
    bench_oversample_calculation,
    bench_threshold_boundaries,
    bench_auto_vs_explicit,
);
criterion_main!(benches);
