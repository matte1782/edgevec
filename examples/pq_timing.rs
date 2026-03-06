//! Quick PQ training timing comparison: max_iters=15 vs max_iters=5
//! Both use early-stop convergence (threshold 1e-4).
//!
//! Run (sequential):  `cargo run --release --example pq_timing`
//! Run (parallel):    `cargo run --release --features parallel --example pq_timing`

use edgevec::quantization::product::PqCodebook;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

fn main() {
    let count = 100_000usize;
    let dims = 768usize;
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    println!("Generating {count} vectors at {dims}D...");
    let vectors: Vec<Vec<f32>> = (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect())
        .collect();
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();
    println!("Data generated.\n");

    // Test 1: max_iters=15 (baseline with early-stop)
    println!("--- max_iters=15 + early-stop (1e-4) ---");
    let start = Instant::now();
    let cb = PqCodebook::train(&refs, 8, 256, 15).unwrap();
    let elapsed15 = start.elapsed().as_secs_f64();
    println!("Time: {elapsed15:.2}s");
    drop(cb);

    // Test 2: max_iters=5 (reduced + early-stop)
    println!("\n--- max_iters=5 + early-stop (1e-4) ---");
    let start = Instant::now();
    let cb = PqCodebook::train(&refs, 8, 256, 5).unwrap();
    let elapsed5 = start.elapsed().as_secs_f64();
    println!("Time: {elapsed5:.2}s");
    drop(cb);

    // Summary
    let baseline = 198.7;
    println!("\n=== SUMMARY ===");
    println!("Baseline (W46, no early-stop, iters=15): {baseline:.1}s");
    println!(
        "iters=15 + early-stop: {elapsed15:.2}s ({:.0}% of baseline)",
        elapsed15 / baseline * 100.0
    );
    println!(
        "iters=5  + early-stop: {elapsed5:.2}s ({:.0}% of baseline)",
        elapsed5 / baseline * 100.0
    );
}
