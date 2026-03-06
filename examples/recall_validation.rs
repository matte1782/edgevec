//! G3 Recall Validation -- real embeddings (50K, 768D, all-mpnet-base-v2)
//!
//! Measures PQ recall@10 on real sentence embeddings to validate gate G3.
//!
//! # Reproducibility
//!
//! - Dataset: `tests/data/embeddings_768d_50k.bin` (50,000 x 768 x f32, little-endian)
//! - Model: all-mpnet-base-v2, L2-normalized
//! - Query selection: 100 queries, seed=42 (ChaCha8Rng)
//! - Ground truth: brute-force L2 distance, NaN-safe sort via total_cmp
//! - G3 threshold: recall@10 > 0.90
//!
//! # Usage
//!
//! ```bash
//! cargo run --release --example recall_validation
//! ```

use edgevec::quantization::product::PqCodebook;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;
use std::fs;
use std::time::Instant;

/// Load raw f32 embeddings from a binary file (no header, little-endian).
fn load_embeddings(path: &str, n: usize, dims: usize) -> Vec<Vec<f32>> {
    let bytes = fs::read(path).expect("failed to read embeddings file");
    let expected = n * dims * 4;
    assert_eq!(
        bytes.len(),
        expected,
        "unexpected file size: got {} bytes, expected {} ({} x {} x 4)",
        bytes.len(),
        expected,
        n,
        dims
    );

    let mut vectors = Vec::with_capacity(n);
    for i in 0..n {
        let offset = i * dims * 4;
        let vec: Vec<f32> = (0..dims)
            .map(|d| {
                let start = offset + d * 4;
                f32::from_le_bytes([
                    bytes[start],
                    bytes[start + 1],
                    bytes[start + 2],
                    bytes[start + 3],
                ])
            })
            .collect();
        vectors.push(vec);
    }
    vectors
}

/// Brute-force top-k by L2 distance. NaN-safe via total_cmp (lesson #71).
fn brute_force_topk(base: &[Vec<f32>], query: &[f32], k: usize) -> Vec<usize> {
    let mut dists: Vec<(usize, f32)> = base
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let d: f32 = v.iter().zip(query).map(|(&a, &b)| (a - b) * (a - b)).sum();
            (i, d)
        })
        .collect();
    dists.sort_by(|a, b| a.1.total_cmp(&b.1));
    dists.iter().take(k).map(|&(i, _)| i).collect()
}

/// Train PQ, encode all vectors, then measure recall@k over sampled queries.
fn measure_recall(
    vectors: &[Vec<f32>],
    m: usize,
    ksub: usize,
    max_iters: usize,
    num_queries: usize,
    k: usize,
) -> f64 {
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    println!(
        "  Training PQ codebook (M={}, Ksub={}, iters={})...",
        m, ksub, max_iters
    );
    let t0 = Instant::now();
    let codebook = PqCodebook::train(&refs, m, ksub, max_iters).expect("training failed");
    let train_secs = t0.elapsed().as_secs_f64();
    println!("  Training done in {:.1}s", train_secs);

    println!("  Encoding {} vectors...", vectors.len());
    let t1 = Instant::now();
    let codes: Vec<_> = vectors
        .iter()
        .map(|v| codebook.encode(v).expect("encode failed"))
        .collect();
    let encode_secs = t1.elapsed().as_secs_f64();
    println!("  Encoding done in {:.1}s", encode_secs);

    // Sample query indices deterministically (seed=42)
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = vectors.len();
    let query_indices: Vec<usize> = (0..num_queries).map(|_| rng.gen_range(0..n)).collect();

    println!("  Computing recall@{} over {} queries...", k, num_queries);
    let t2 = Instant::now();
    let mut total_recall = 0.0f64;

    for (qi_idx, &qi) in query_indices.iter().enumerate() {
        let query = &vectors[qi];

        // Ground truth: brute-force L2 top-k
        let true_topk: HashSet<usize> = brute_force_topk(vectors, query, k).into_iter().collect();

        // PQ ADC search
        let dt = codebook
            .compute_distance_table(query)
            .expect("distance table failed");
        let pq_results = dt.scan_topk(&codes, k);
        let pq_topk: HashSet<usize> = pq_results.iter().map(|r| r.index).collect();

        let overlap = true_topk.intersection(&pq_topk).count();
        let query_recall = overlap as f64 / k as f64;
        total_recall += query_recall;

        // Progress every 25 queries
        if (qi_idx + 1) % 25 == 0 {
            println!(
                "    [{}/{}] running avg recall = {:.4}",
                qi_idx + 1,
                num_queries,
                total_recall / (qi_idx + 1) as f64
            );
        }
    }

    let recall_secs = t2.elapsed().as_secs_f64();
    let avg_recall = total_recall / num_queries as f64;
    println!(
        "  Recall computation done in {:.1}s ({:.1}ms/query)",
        recall_secs,
        recall_secs * 1000.0 / num_queries as f64
    );

    avg_recall
}

fn main() {
    let path = "tests/data/embeddings_768d_50k.bin";
    let n = 50_000;
    let dims = 768;
    let k = 10;
    let num_queries = 100;
    let g3_threshold = 0.90;

    println!("=== G3 Recall Validation ===");
    println!("Dataset: {} vectors, {}D (all-mpnet-base-v2)", n, dims);
    println!("Queries: {} (seed=42)", num_queries);
    println!("Metric: recall@{}", k);
    println!("Threshold: > {:.2}", g3_threshold);
    println!();

    // Load embeddings
    println!("Loading embeddings from {}...", path);
    let t_load = Instant::now();
    let vectors = load_embeddings(path, n, dims);
    println!(
        "Loaded {} vectors in {:.1}s",
        vectors.len(),
        t_load.elapsed().as_secs_f64()
    );
    println!();

    // M=8 (primary configuration)
    println!("--- Configuration 1: M=8, Ksub=256, iters=15 ---");
    let recall_m8 = measure_recall(&vectors, 8, 256, 15, num_queries, k);
    println!("  recall@{} = {:.4}", k, recall_m8);
    println!();

    // M=16 (mitigation if M=8 < threshold)
    if recall_m8 < g3_threshold {
        println!(
            "M=8 recall ({:.4}) < threshold ({:.2}), trying M=16 mitigation...",
            recall_m8, g3_threshold
        );
        println!();
        println!("--- Configuration 2: M=16, Ksub=256, iters=15 ---");
        let recall_m16 = measure_recall(&vectors, 16, 256, 15, num_queries, k);
        println!("  recall@{} = {:.4}", k, recall_m16);
        println!();

        println!("=== G3 RESULT ===");
        println!(
            "M=8:  recall@{} = {:.4} -- {}",
            k,
            recall_m8,
            if recall_m8 >= g3_threshold {
                "PASS"
            } else {
                "FAIL"
            }
        );
        println!(
            "M=16: recall@{} = {:.4} -- {}",
            k,
            recall_m16,
            if recall_m16 >= g3_threshold {
                "PASS"
            } else {
                "FAIL"
            }
        );

        if recall_m16 >= g3_threshold {
            println!();
            println!(
                "Verdict: G3 CONDITIONAL PASS (M=16 required for >{:.2} recall)",
                g3_threshold
            );
        } else {
            println!();
            println!("Verdict: G3 FAIL -- both M=8 and M=16 below threshold");
            println!("Action: Escalate per PQ_BENCHMARK_PLAN Section 6.2");
        }
    } else {
        println!("=== G3 RESULT ===");
        println!("M=8: recall@{} = {:.4} -- PASS", k, recall_m8);
        println!();
        println!("Verdict: G3 PASS (M=8 sufficient)");
    }
}
