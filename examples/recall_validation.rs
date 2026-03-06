//! G3 Recall Validation -- real embeddings (50K, 768D, all-mpnet-base-v2)
//!
//! Measures PQ recall@10 and BQ+rescore recall@10 on real sentence embeddings.
//! Validates gate G3 and produces B4 comparison table.
//!
//! # Reproducibility
//!
//! - Dataset: `tests/data/embeddings_768d_50k.bin` (50,000 x 768 x f32, little-endian)
//! - Model: all-mpnet-base-v2, L2-normalized
//! - Query selection: 100 queries, seed=42 (ChaCha8Rng)
//! - Ground truth: brute-force L2 distance, NaN-safe sort via total_cmp
//! - G3 threshold: recall@10 > 0.90
//! - BQ+rescore: Hamming top-100 candidates, L2 rescore to top-10
//!
//! # Usage
//!
//! ```bash
//! cargo run --release --example recall_validation
//! ```

use edgevec::quantization::binary::BinaryQuantizer;
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

/// BQ+rescore pipeline: binarize -> Hamming top-N -> f32 L2 rescore -> top-k.
///
/// Returns (avg_recall, avg_latency_ms).
fn measure_bq_rescore_recall(
    vectors: &[Vec<f32>],
    num_queries: usize,
    k: usize,
    hamming_candidates: usize,
) -> (f64, f64) {
    let quantizer = BinaryQuantizer::new();

    println!(
        "  Binarizing {} vectors (768D -> 96 bytes each)...",
        vectors.len()
    );
    let t_bq = Instant::now();
    let bq_vectors: Vec<_> = vectors.iter().map(|v| quantizer.quantize(v)).collect();
    let bq_secs = t_bq.elapsed().as_secs_f64();
    println!("  Binarization done in {:.1}s", bq_secs);

    // Memory accounting: 50K * 128 bytes (96 data + 32 padding for alignment) = ~6.1MB
    let bq_mem_bytes = bq_vectors.len() * std::mem::size_of_val(&bq_vectors[0]);
    println!(
        "  BQ memory: {:.1} MB ({} bytes/vector)",
        bq_mem_bytes as f64 / (1024.0 * 1024.0),
        std::mem::size_of_val(&bq_vectors[0])
    );

    // Sample query indices deterministically (seed=42) -- SAME as PQ
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = vectors.len();
    let query_indices: Vec<usize> = (0..num_queries).map(|_| rng.gen_range(0..n)).collect();

    println!(
        "  Computing BQ+rescore recall@{} over {} queries (Hamming top-{} -> L2 top-{})...",
        k, num_queries, hamming_candidates, k
    );
    let t_recall = Instant::now();
    let mut total_recall = 0.0f64;

    for (qi_idx, &qi) in query_indices.iter().enumerate() {
        let query = &vectors[qi];
        let query_bq = &bq_vectors[qi];

        // Ground truth: brute-force L2 top-k (same as PQ pipeline)
        let true_topk: HashSet<usize> = brute_force_topk(vectors, query, k).into_iter().collect();

        // Step 1: Hamming distance to all BQ vectors, select top-N candidates
        let mut hamming_dists: Vec<(usize, u32)> = bq_vectors
            .iter()
            .enumerate()
            .map(|(i, bq_v)| (i, query_bq.hamming_distance(bq_v)))
            .collect();
        // NaN-safe sort via total_cmp on u32 (no NaN possible, but consistent style)
        hamming_dists.sort_by(|a, b| a.1.cmp(&b.1));
        let candidates: Vec<usize> = hamming_dists
            .iter()
            .take(hamming_candidates)
            .map(|&(i, _)| i)
            .collect();

        // Step 2: L2 rescore on original f32 vectors for top-N candidates
        let mut rescored: Vec<(usize, f32)> = candidates
            .iter()
            .map(|&idx| {
                let d: f32 = vectors[idx]
                    .iter()
                    .zip(query)
                    .map(|(&a, &b)| (a - b) * (a - b))
                    .sum();
                (idx, d)
            })
            .collect();
        rescored.sort_by(|a, b| a.1.total_cmp(&b.1));
        let rescore_topk: HashSet<usize> = rescored.iter().take(k).map(|&(i, _)| i).collect();

        let overlap = true_topk.intersection(&rescore_topk).count();
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

    let recall_secs = t_recall.elapsed().as_secs_f64();
    let avg_recall = total_recall / num_queries as f64;
    let avg_latency_ms = recall_secs * 1000.0 / num_queries as f64;
    println!(
        "  Recall computation done in {:.1}s ({:.1}ms/query)",
        recall_secs, avg_latency_ms
    );

    (avg_recall, avg_latency_ms)
}

fn main() {
    let path = "tests/data/embeddings_768d_50k.bin";
    let n = 50_000;
    let dims = 768;
    let k = 10;
    let num_queries = 100;
    let g3_threshold = 0.90;
    let hamming_candidates = 100;

    println!("=== G3 Recall Validation + B4 Comparison ===");
    println!("Dataset: {} vectors, {}D (all-mpnet-base-v2)", n, dims);
    println!("Queries: {} (seed=42)", num_queries);
    println!("Metric: recall@{}", k);
    println!("G3 threshold: > {:.2}", g3_threshold);
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

    // =========================================================================
    // PQ Recall (G3)
    // =========================================================================

    // M=8 (primary configuration)
    println!("--- PQ Configuration 1: M=8, Ksub=256, iters=15 ---");
    let recall_m8 = measure_recall(&vectors, 8, 256, 15, num_queries, k);
    println!("  recall@{} = {:.4}", k, recall_m8);
    println!();

    // M=16 (mitigation if M=8 < threshold)
    let recall_m16 = if recall_m8 < g3_threshold {
        println!(
            "M=8 recall ({:.4}) < threshold ({:.2}), trying M=16 mitigation...",
            recall_m8, g3_threshold
        );
        println!();
        println!("--- PQ Configuration 2: M=16, Ksub=256, iters=15 ---");
        let r = measure_recall(&vectors, 16, 256, 15, num_queries, k);
        println!("  recall@{} = {:.4}", k, r);
        println!();
        Some(r)
    } else {
        None
    };

    // =========================================================================
    // BQ+Rescore (B4)
    // =========================================================================

    println!(
        "--- BQ+Rescore: Hamming top-{} -> L2 rescore -> top-{} ---",
        hamming_candidates, k
    );
    let (recall_bq, latency_bq) =
        measure_bq_rescore_recall(&vectors, num_queries, k, hamming_candidates);
    println!("  recall@{} = {:.4}", k, recall_bq);
    println!();

    // =========================================================================
    // G3 Result
    // =========================================================================

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
    if let Some(r16) = recall_m16 {
        println!(
            "M=16: recall@{} = {:.4} -- {}",
            k,
            r16,
            if r16 >= g3_threshold { "PASS" } else { "FAIL" }
        );
    }
    println!();

    if recall_m8 >= g3_threshold {
        println!("Verdict: G3 PASS (M=8 sufficient)");
    } else if recall_m16.is_some_and(|r| r >= g3_threshold) {
        println!(
            "Verdict: G3 CONDITIONAL PASS (M=16 required for >{:.2} recall)",
            g3_threshold
        );
    } else {
        println!("Verdict: G3 FAIL -- both M=8 and M=16 below threshold");
        println!("Action: Escalate per PQ_BENCHMARK_PLAN Section 6.2");
    }
    println!();

    // =========================================================================
    // B4 Comparison Table
    // =========================================================================

    println!(
        "=== B4 Comparison ({}K real embeddings, {}D, {} queries) ===",
        n / 1000,
        dims,
        num_queries
    );
    println!("  PQ (M=8):         recall@{} = {:.4}", k, recall_m8);
    if let Some(r16) = recall_m16 {
        println!("  PQ (M=16):        recall@{} = {:.4}", k, r16);
    }
    println!(
        "  BQ+rescore({}):  recall@{} = {:.4}  ({:.1}ms/query)",
        hamming_candidates, k, recall_bq, latency_bq
    );
    println!();

    // Determine best PQ recall for comparison
    let best_pq = recall_m16.unwrap_or(recall_m8).max(recall_m8);
    let delta = best_pq - recall_bq;
    if delta > 0.0 {
        println!(
            "  Winner: PQ by {:.4} points (best PQ {:.4} vs BQ+rescore {:.4})",
            delta, best_pq, recall_bq
        );
    } else if delta < 0.0 {
        println!(
            "  Winner: BQ+rescore by {:.4} points (BQ+rescore {:.4} vs best PQ {:.4})",
            -delta, recall_bq, best_pq
        );
    } else {
        println!("  Result: TIE ({:.4})", best_pq);
    }

    // Memory summary
    let f32_mem_mb = (n * dims * 4) as f64 / (1024.0 * 1024.0);
    let bq_mem_mb = (n * std::mem::size_of::<edgevec::quantization::binary::QuantizedVector>())
        as f64
        / (1024.0 * 1024.0);
    println!();
    println!(
        "  Memory: f32 base = {:.1}MB, BQ overhead = {:.1}MB, total = {:.1}MB",
        f32_mem_mb,
        bq_mem_mb,
        f32_mem_mb + bq_mem_mb
    );
    assert!(
        f32_mem_mb + bq_mem_mb < 200.0,
        "Memory budget exceeded: {:.1}MB > 200MB",
        f32_mem_mb + bq_mem_mb
    );
    println!(
        "  Memory budget: {:.1}MB < 200MB -- OK",
        f32_mem_mb + bq_mem_mb
    );
}
