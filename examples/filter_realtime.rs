//! Example: Real-time Filtering with Dynamic Queries
//!
//! Demonstrates real-time filtering scenarios:
//! - Building filters programmatically
//! - Comparing different filter strategies
//! - Performance characteristics of filtered search
//! - Strategy selection based on selectivity
//!
//! Run with: `cargo run --example filter_realtime`

use edgevec::filter::{
    estimate_selectivity, parse, FilterStrategy, FilteredSearcher, VectorMetadataStore,
};
use edgevec::metadata::MetadataValue;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    println!("=== EdgeVec Real-time Filtering Example ===\n");

    // 1. Create index with more vectors for realistic timing
    let config = HnswConfig::new(32);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");
    let mut metadata_store = VectorMetadataStore::new();

    // 2. Insert vectors with varied metadata
    let categories = ["A", "B", "C", "D", "E"];
    let n_vectors = 1000;

    println!("Inserting {} vectors with metadata...", n_vectors);
    let start = Instant::now();

    for i in 0..n_vectors {
        // Generate embedding based on category
        let cat_idx = i % categories.len();
        let mut embedding = vec![0.0f32; 32];
        embedding[cat_idx * 6..(cat_idx + 1) * 6]
            .iter_mut()
            .enumerate()
            .for_each(|(j, v)| *v = 0.8 + (j as f32 * 0.02));

        // Add some noise
        for v in embedding.iter_mut() {
            *v += (i as f32 * 0.001) % 0.1;
        }

        let id = index
            .insert(&embedding, &mut storage)
            .expect("Insert failed");

        let mut meta = HashMap::new();
        meta.insert(
            "category".to_string(),
            MetadataValue::String(categories[cat_idx].to_string()),
        );
        meta.insert(
            "score".to_string(),
            MetadataValue::Float((i as f64 % 100.0) / 100.0),
        );
        meta.insert("index".to_string(), MetadataValue::Integer(i as i64));
        meta.insert(
            "active".to_string(),
            MetadataValue::Boolean(i % 3 != 0), // 2/3 are active
        );
        // VectorMetadataStore uses 0-based index, VectorId.0 is 1-based
        metadata_store.set((id.0 - 1) as usize, meta);
    }

    println!("  Inserted in {:?}\n", start.elapsed());

    // 3. Demonstrate different selectivity scenarios
    println!("--- Selectivity Analysis ---\n");

    let filters = vec![
        (r#"category = "A""#, "High selectivity (~20%)"),
        (
            r#"category IN ["A", "B", "C"]"#,
            "Medium selectivity (~60%)",
        ),
        ("active = true", "Low selectivity (~67%)"),
        (
            r#"category = "A" AND score > 0.9"#,
            "Very high selectivity (~2%)",
        ),
        (
            r#"category = "A" OR category = "B""#,
            "Medium selectivity (~40%)",
        ),
    ];

    for (filter_str, description) in &filters {
        let filter = parse(filter_str).expect("Parse failed");
        let selectivity = estimate_selectivity(&filter, &metadata_store, None);

        println!("Filter: {}", filter_str);
        println!("  Description: {}", description);
        println!(
            "  Estimated selectivity: {:.1}%",
            selectivity.selectivity * 100.0
        );
        println!("  Confidence: {:.2}", selectivity.confidence());
        println!();
    }

    // 4. Compare filter strategies
    println!("--- Strategy Comparison ---\n");

    let query = {
        let mut q = vec![0.0f32; 32];
        q[0..6].iter_mut().for_each(|v| *v = 0.85);
        q
    };

    let test_filters = vec![
        r#"category = "A""#,         // ~20% match
        r#"category IN ["A", "B"]"#, // ~40% match
        "active = true",             // ~67% match
    ];

    for filter_str in test_filters {
        println!("Filter: {}", filter_str);
        let filter = parse(filter_str).expect("Parse failed");

        // Test each strategy
        for strategy in [
            FilterStrategy::PreFilter,
            FilterStrategy::PostFilter { oversample: 2.0 },
            FilterStrategy::Hybrid {
                oversample_min: 1.5,
                oversample_max: 10.0,
            },
            FilterStrategy::Auto,
        ] {
            let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);
            let start = Instant::now();

            let result = searcher
                .search_filtered(&query, 10, Some(&filter), strategy)
                .expect("Search failed");

            let elapsed = start.elapsed();

            println!(
                "  {:?}: {} results, {:.3}ms, strategy used: {:?}",
                strategy,
                result.results.len(),
                elapsed.as_secs_f64() * 1000.0,
                result.strategy_used
            );
        }
        println!();
    }

    // 5. Dynamic filter building
    println!("--- Dynamic Filter Building ---\n");

    // Simulate user building a filter interactively
    let mut filter_parts: Vec<String> = vec![];

    // User selects category
    filter_parts.push(r#"category = "A""#.to_string());
    println!("User selects category A");

    // User adds score filter
    filter_parts.push("score > 0.5".to_string());
    println!("User adds score > 0.5");

    // User adds active filter
    filter_parts.push("active = true".to_string());
    println!("User requires active = true");

    // Combine filters with AND
    let combined_filter = filter_parts.join(" AND ");
    println!("\nCombined filter: {}", combined_filter);

    let filter = parse(&combined_filter).expect("Parse failed");
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("Results: {} matches", result.results.len());
    println!("Strategy used: {:?}", result.strategy_used);
    println!(
        "Observed selectivity: {:.1}%",
        result.observed_selectivity * 100.0
    );

    // 6. Unfiltered search for comparison
    println!("\n--- Unfiltered Search (Baseline) ---\n");

    let start = Instant::now();
    let unfiltered = index.search(&query, 10, &storage).expect("Search failed");
    let elapsed = start.elapsed();

    println!(
        "Unfiltered: {} results in {:.3}ms",
        unfiltered.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    // Filtered with same k
    let start = Instant::now();
    let filter = parse(r#"category = "A""#).expect("Parse failed");
    let filtered = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");
    let elapsed = start.elapsed();

    println!(
        "Filtered (category=A): {} results in {:.3}ms",
        filtered.results.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    println!("\n=== Example Complete ===");
}
