//! Recall benchmark runner
//!
//! Run with: cargo run --release --bin recall_bench -- --nocapture
//!
//! # Dataset Setup
//!
//! ## SIFT-1M
//! ```bash
//! mkdir -p data/sift
//! cd data/sift
//! wget ftp://ftp.irisa.fr/local/texmex/corpus/sift.tar.gz
//! tar -xzf sift.tar.gz
//! cd ../..
//! ```
//!
//! ## GloVe-100
//! ```bash
//! mkdir -p data/glove
//! cd data/glove
//! # Download GloVe in fvecs format from ann-benchmarks
//! wget http://ann-benchmarks.com/glove-100-angular.hdf5
//! # Convert to fvecs format (or use pre-converted)
//! cd ../..
//! ```
//!
//! ## Running Benchmarks
//! ```bash
//! # With SIFT-1M dataset
//! ANN_BENCHMARK_DATA=./data/sift cargo run --release --bin recall_bench
//!
//! # With GloVe-100 dataset
//! ANN_BENCHMARK_DATA=./data/glove cargo run --release --bin recall_bench -- --glove
//!
//! # With synthetic data (no dataset required)
//! cargo run --release --bin recall_bench -- --synthetic
//!
//! # With SQ8 quantization comparison
//! cargo run --release --bin recall_bench -- --synthetic --sq8
//! ```
//!
//! # Expected Results (SIFT-1M, Float32)
//!
//! | ef_search | k   | Expected Recall |
//! |-----------|-----|-----------------|
//! | 10        | 1   | >0.85           |
//! | 50        | 10  | >0.95           |
//! | 100       | 10  | >0.98           |
//! | 200       | 100 | >0.99           |

mod recall;

use edgevec::storage::StorageType;
use edgevec::{HnswConfig, HnswIndex, QuantizerConfig, VectorStorage};
use recall::{calculate_recall, load_fvecs, load_ivecs, percentile, RecallBenchResult};
use std::path::Path;
use std::time::Instant;

/// Synthetic dataset configuration for testing without SIFT-1M
const SYNTHETIC_DIM: u32 = 128;
const SYNTHETIC_BASE_COUNT: usize = 10_000;
const SYNTHETIC_QUERY_COUNT: usize = 100;
const SYNTHETIC_GT_K: usize = 100;

/// Benchmark sweep configuration
/// ef_search values to test (controls search accuracy vs speed tradeoff)
const EF_SEARCH_VALUES: [u32; 5] = [10, 50, 100, 200, 500];
/// k values (number of neighbors to retrieve) for recall measurement
const K_VALUES: [usize; 3] = [1, 10, 100];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_synthetic = args.iter().any(|a| a == "--synthetic");
    let use_glove = args.iter().any(|a| a == "--glove");
    let use_sq8 = args.iter().any(|a| a == "--sq8");

    println!("=== EdgeVec Recall Benchmark ===\n");

    if use_synthetic {
        println!("Mode: SYNTHETIC (no external dataset required)\n");
        run_synthetic_benchmark(use_sq8);
    } else if use_glove {
        let data_dir =
            std::env::var("ANN_BENCHMARK_DATA").unwrap_or_else(|_| "./data/glove".to_string());
        let data_path = Path::new(&data_dir);

        if !data_path.exists() {
            eprintln!("Data directory not found: {data_dir}");
            eprintln!();
            eprintln!("To run with GloVe-100 dataset:");
            eprintln!("  1. Download GloVe data in fvecs format");
            eprintln!("  2. Extract to: ./data/glove/");
            eprintln!("  3. Set ANN_BENCHMARK_DATA environment variable");
            eprintln!();
            eprintln!(
                "Or run with synthetic data: cargo run --release --bin recall_bench -- --synthetic"
            );
            std::process::exit(1);
        }

        run_glove_benchmark(data_path, use_sq8);
    } else {
        let data_dir =
            std::env::var("ANN_BENCHMARK_DATA").unwrap_or_else(|_| "./data/sift".to_string());

        let data_path = Path::new(&data_dir);

        if !data_path.exists() {
            eprintln!("Data directory not found: {data_dir}");
            eprintln!();
            eprintln!("To run with SIFT-1M dataset:");
            eprintln!("  1. Download from: ftp://ftp.irisa.fr/local/texmex/corpus/sift.tar.gz");
            eprintln!("  2. Extract to: ./data/sift/");
            eprintln!("  3. Set ANN_BENCHMARK_DATA environment variable");
            eprintln!();
            eprintln!(
                "Or run with synthetic data: cargo run --release --bin recall_bench -- --synthetic"
            );
            std::process::exit(1);
        }

        run_sift_benchmark(data_path, use_sq8);
    }
}

/// Run benchmark with synthetic data
fn run_synthetic_benchmark(use_sq8: bool) {
    println!("Generating synthetic dataset...");
    println!("  Vectors: {SYNTHETIC_BASE_COUNT}");
    println!("  Dimension: {SYNTHETIC_DIM}");
    println!("  Queries: {SYNTHETIC_QUERY_COUNT}");
    println!();

    println!("Note: Synthetic random data has no meaningful nearest neighbors.");
    println!("Recall metrics are only meaningful with real datasets (SIFT, GloVe).\n");

    // Generate random vectors
    use rand::{Rng, RngExt};
    let mut rng = rand::rng();

    let base_vectors: Vec<Vec<f32>> = (0..SYNTHETIC_BASE_COUNT)
        .map(|_| {
            (0..SYNTHETIC_DIM as usize)
                .map(|_| rng.random::<f32>())
                .collect()
        })
        .collect();

    let queries: Vec<Vec<f32>> = (0..SYNTHETIC_QUERY_COUNT)
        .map(|_| {
            (0..SYNTHETIC_DIM as usize)
                .map(|_| rng.random::<f32>())
                .collect()
        })
        .collect();

    // Generate ground truth using brute force
    println!("Generating ground truth (brute force)...");
    let ground_truth: Vec<Vec<u32>> = queries
        .iter()
        .map(|q| {
            let mut distances: Vec<(u32, f32)> = base_vectors
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let dist: f32 = q.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum();
                    (i as u32, dist)
                })
                .collect();
            distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            distances
                .iter()
                .take(SYNTHETIC_GT_K)
                .map(|(i, _)| *i)
                .collect()
        })
        .collect();
    println!("  Ground truth generated for {} queries\n", queries.len());

    let mut results = Vec::new();

    // Run Float32 benchmark
    {
        println!("Building HNSW index (Float32)...");
        let config = HnswConfig::new(SYNTHETIC_DIM);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 2000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "Synthetic",
            "float32",
            &mut results,
        );
    }

    // Run SQ8 benchmark if requested
    if use_sq8 {
        println!("\nBuilding HNSW index (SQ8 Quantized)...");
        let config = HnswConfig::new(SYNTHETIC_DIM);
        let mut storage = VectorStorage::new(&config, None);

        // Configure SQ8 quantization (values are 0.0-1.0 range for synthetic data)
        let q_config = QuantizerConfig { min: 0.0, max: 1.0 };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 2000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "Synthetic",
            "sq8",
            &mut results,
        );
    }

    print_results(&results);
    if use_sq8 {
        print_comparison(&results);
    }
}

/// Run benchmark with SIFT-1M dataset
fn run_sift_benchmark(data_dir: &Path, use_sq8: bool) {
    // Load data
    println!("Loading SIFT-1M dataset...");
    let base_path = data_dir.join("sift/sift_base.fvecs");
    let query_path = data_dir.join("sift/sift_query.fvecs");
    let gt_path = data_dir.join("sift/sift_groundtruth.ivecs");

    // Check for alternate paths (dataset might be extracted differently)
    let (base_path, query_path, gt_path) = if base_path.exists() {
        (base_path, query_path, gt_path)
    } else {
        let alt_base = data_dir.join("sift_base.fvecs");
        let alt_query = data_dir.join("sift_query.fvecs");
        let alt_gt = data_dir.join("sift_groundtruth.ivecs");
        if alt_base.exists() {
            (alt_base, alt_query, alt_gt)
        } else {
            eprintln!("Could not find SIFT files in {}", data_dir.display());
            eprintln!("Expected: sift_base.fvecs, sift_query.fvecs, sift_groundtruth.ivecs");
            std::process::exit(1);
        }
    };

    let base_vectors = load_fvecs(&base_path).expect("Failed to load base vectors");
    let queries = load_fvecs(&query_path).expect("Failed to load queries");
    let ground_truth = load_ivecs(&gt_path).expect("Failed to load ground truth");

    println!("  Base vectors: {}", base_vectors.len());
    println!("  Queries: {}", queries.len());
    println!("  Ground truth: {}", ground_truth.len());
    println!();

    // Calculate min/max for quantization from base vectors
    let (min_val, max_val) = calculate_vector_range(&base_vectors);

    let dim = base_vectors.first().map(|v| v.len()).unwrap_or(128) as u32;
    let mut results = Vec::new();

    // Run Float32 benchmark
    {
        println!("Building HNSW index (Float32)...");
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 100_000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "SIFT-1M",
            "float32",
            &mut results,
        );
    }

    // Run SQ8 benchmark if requested
    if use_sq8 {
        println!("\nBuilding HNSW index (SQ8 Quantized)...");
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);

        // Configure SQ8 quantization with actual data range
        let q_config = QuantizerConfig {
            min: min_val,
            max: max_val,
        };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 100_000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "SIFT-1M",
            "sq8",
            &mut results,
        );
    }

    print_results(&results);
    if use_sq8 {
        print_comparison(&results);
    }
}

/// Run recall tests at various ef_search and k values
fn run_recall_tests(
    index: &mut HnswIndex,
    storage: &VectorStorage,
    queries: &[Vec<f32>],
    ground_truth: &[Vec<u32>],
    dataset_name: &str,
    mode: &str,
    results: &mut Vec<RecallBenchResult>,
) {
    println!("Running recall benchmarks ({mode})...\n");

    for ef_search in EF_SEARCH_VALUES {
        // Set ef_search via config (config is pub)
        index.config.ef_search = ef_search;

        for k in K_VALUES {
            let mut recalls = Vec::new();
            let mut latencies = Vec::new();

            for (query, gt) in queries.iter().zip(ground_truth.iter()) {
                let start = Instant::now();
                let search_results = index.search(query, k, storage).expect("Search failed");
                let latency = start.elapsed();

                let result_ids: Vec<u64> = search_results.iter().map(|r| r.vector_id.0).collect();

                let recall = calculate_recall(&result_ids, gt, k);
                recalls.push(recall);
                latencies.push(latency.as_micros() as f64);
            }

            let avg_recall = recalls.iter().sum::<f64>() / recalls.len() as f64;
            let total_time_us: f64 = latencies.iter().sum();
            let qps = queries.len() as f64 / total_time_us * 1_000_000.0;

            latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p50 = percentile(&latencies, 0.5);
            let p99 = percentile(&latencies, 0.99);

            let result = RecallBenchResult {
                dataset: dataset_name.to_string(),
                mode: mode.to_string(),
                k,
                ef_search: ef_search as usize,
                recall: avg_recall,
                queries_per_second: qps,
                latency_p50_us: p50,
                latency_p99_us: p99,
            };

            println!(
                "  ef={:>3}, k={:>3}: recall={:.4}, QPS={:>6.0}, p50={:>5.0}us, p99={:>5.0}us",
                ef_search, k, avg_recall, qps, p50, p99
            );

            results.push(result);
        }
    }
    println!();
}

/// Print results as markdown table
fn print_results(results: &[RecallBenchResult]) {
    println!("=== Summary ===\n");
    println!("| Dataset | Mode | ef_search | k | Recall | QPS | P50 (us) | P99 (us) |");
    println!("|:--------|:-----|----------:|--:|-------:|----:|---------:|---------:|");
    for r in results {
        println!("{}", r.as_table_row());
    }
    println!();

    // Print recall targets check
    println!("=== Recall Target Verification ===\n");
    for r in results {
        let target = match (r.ef_search, r.k) {
            (10, 1) => Some(0.85),
            (50, 10) => Some(0.95),
            (100, 10) => Some(0.98),
            (200, 100) => Some(0.99),
            _ => None,
        };
        if let Some(target) = target {
            let status = if r.recall >= target { "PASS" } else { "FAIL" };
            println!(
                "  ef={:>3}, k={:>3}: {:.4} >= {:.2} ? {}",
                r.ef_search, r.k, r.recall, target, status
            );
        }
    }
}

/// Print Float32 vs SQ8 comparison table
fn print_comparison(results: &[RecallBenchResult]) {
    println!("\n=== Float32 vs SQ8 Comparison ===\n");
    println!(
        "| ef_search | k | Float32 Recall | SQ8 Recall | Delta | Float32 QPS | SQ8 QPS | Speedup |"
    );
    println!(
        "|----------:|--:|---------------:|-----------:|------:|------------:|--------:|--------:|"
    );

    // Group by (ef_search, k) and compare float32 vs sq8
    let float32_results: Vec<_> = results.iter().filter(|r| r.mode == "float32").collect();
    let sq8_results: Vec<_> = results.iter().filter(|r| r.mode == "sq8").collect();

    for f32_r in &float32_results {
        if let Some(sq8_r) = sq8_results
            .iter()
            .find(|r| r.ef_search == f32_r.ef_search && r.k == f32_r.k)
        {
            let delta = sq8_r.recall - f32_r.recall;
            let speedup = sq8_r.queries_per_second / f32_r.queries_per_second;
            println!(
                "| {:>9} | {:>1} | {:>14.4} | {:>10.4} | {:>+5.4} | {:>11.0} | {:>7.0} | {:>6.2}x |",
                f32_r.ef_search,
                f32_r.k,
                f32_r.recall,
                sq8_r.recall,
                delta,
                f32_r.queries_per_second,
                sq8_r.queries_per_second,
                speedup
            );
        }
    }
    println!();
}

/// Calculate min/max range of vector values for quantization
fn calculate_vector_range(vectors: &[Vec<f32>]) -> (f32, f32) {
    let mut min_val = f32::MAX;
    let mut max_val = f32::MIN;

    for vec in vectors {
        for &val in vec {
            if val < min_val {
                min_val = val;
            }
            if val > max_val {
                max_val = val;
            }
        }
    }

    // Add small margin to avoid edge cases
    let margin = (max_val - min_val) * 0.01;
    (min_val - margin, max_val + margin)
}

/// Run benchmark with GloVe-100 dataset
fn run_glove_benchmark(data_dir: &Path, use_sq8: bool) {
    // Load data
    println!("Loading GloVe-100 dataset...");

    // Check for various file naming conventions
    let (base_path, query_path, gt_path) = find_glove_files(data_dir);

    let base_vectors = load_fvecs(&base_path).expect("Failed to load base vectors");
    let queries = load_fvecs(&query_path).expect("Failed to load queries");
    let ground_truth = load_ivecs(&gt_path).expect("Failed to load ground truth");

    println!("  Base vectors: {}", base_vectors.len());
    println!("  Queries: {}", queries.len());
    println!("  Ground truth: {}", ground_truth.len());
    println!(
        "  Dimension: {}",
        base_vectors.first().map(|v| v.len()).unwrap_or(0)
    );
    println!();

    // Calculate min/max for quantization from base vectors
    let (min_val, max_val) = calculate_vector_range(&base_vectors);

    let dim = base_vectors.first().map(|v| v.len()).unwrap_or(100) as u32;
    let mut results = Vec::new();

    // Run Float32 benchmark
    {
        println!("Building HNSW index (Float32)...");
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 100_000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "GloVe-100",
            "float32",
            &mut results,
        );
    }

    // Run SQ8 benchmark if requested
    if use_sq8 {
        println!("\nBuilding HNSW index (SQ8 Quantized)...");
        let config = HnswConfig::new(dim);
        let mut storage = VectorStorage::new(&config, None);

        // Configure SQ8 quantization with actual data range
        let q_config = QuantizerConfig {
            min: min_val,
            max: max_val,
        };
        storage.set_storage_type(StorageType::QuantizedU8(q_config));

        let mut index = HnswIndex::new(config.clone(), &storage).expect("Failed to create index");

        let build_start = Instant::now();
        for (i, vec) in base_vectors.iter().enumerate() {
            index.insert(vec, &mut storage).expect("Failed to insert");
            if (i + 1) % 100_000 == 0 {
                println!("  Inserted {}/{}...", i + 1, base_vectors.len());
            }
        }
        let build_time = build_start.elapsed();
        println!("  Build time: {:.2}s\n", build_time.as_secs_f64());

        run_recall_tests(
            &mut index,
            &storage,
            &queries,
            &ground_truth,
            "GloVe-100",
            "sq8",
            &mut results,
        );
    }

    print_results(&results);
    if use_sq8 {
        print_comparison(&results);
    }
}

/// Find GloVe dataset files in the given directory
fn find_glove_files(
    data_dir: &Path,
) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    // Try different naming conventions
    let candidates = [
        // Standard naming
        (
            "glove_base.fvecs",
            "glove_query.fvecs",
            "glove_groundtruth.ivecs",
        ),
        // Alternate naming
        (
            "glove-100-angular_base.fvecs",
            "glove-100-angular_query.fvecs",
            "glove-100-angular_groundtruth.ivecs",
        ),
        // ANN-benchmarks style
        ("base.fvecs", "query.fvecs", "groundtruth.ivecs"),
    ];

    for (base_name, query_name, gt_name) in candidates {
        let base = data_dir.join(base_name);
        let query = data_dir.join(query_name);
        let gt = data_dir.join(gt_name);

        if base.exists() && query.exists() && gt.exists() {
            return (base, query, gt);
        }
    }

    // Fallback to first candidate with error handling in caller
    let (base_name, query_name, gt_name) = candidates[0];
    (
        data_dir.join(base_name),
        data_dir.join(query_name),
        data_dir.join(gt_name),
    )
}
