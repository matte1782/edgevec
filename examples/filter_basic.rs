//! Example: Basic Filtered Search
//!
//! Demonstrates the core filtered search functionality in EdgeVec:
//! - Creating an index with metadata
//! - Simple equality filters
//! - Range filters
//! - Combined AND/OR filters
//!
//! Run with: `cargo run --example filter_basic`

use edgevec::filter::{parse, FilterStrategy, FilteredSearcher, VectorMetadataStore};
use edgevec::metadata::MetadataValue;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::collections::HashMap;

fn main() {
    println!("=== EdgeVec Basic Filtered Search Example ===\n");

    // 1. Create an HNSW index with 4 dimensions (small for demo)
    let config = HnswConfig::new(4);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

    // Use VectorMetadataStore from the filter module
    let mut metadata_store = VectorMetadataStore::new();

    println!("Created index with 4 dimensions\n");

    // 2. Insert vectors with metadata
    let items = vec![
        ("apple", "fruit", 1.50, vec![1.0, 0.0, 0.0, 0.0]),
        ("banana", "fruit", 0.75, vec![0.9, 0.1, 0.0, 0.0]),
        ("carrot", "vegetable", 0.50, vec![0.0, 1.0, 0.0, 0.0]),
        ("broccoli", "vegetable", 1.25, vec![0.0, 0.9, 0.1, 0.0]),
        ("chicken", "meat", 5.00, vec![0.0, 0.0, 1.0, 0.0]),
        ("beef", "meat", 8.00, vec![0.0, 0.0, 0.9, 0.1]),
        ("salmon", "seafood", 12.00, vec![0.0, 0.0, 0.0, 1.0]),
        ("shrimp", "seafood", 15.00, vec![0.0, 0.0, 0.1, 0.9]),
    ];

    println!("Inserting {} items with metadata:", items.len());
    for (name, category, price, vector) in items {
        let id = index.insert(&vector, &mut storage).expect("Insert failed");

        // Add metadata using VectorMetadataStore
        let mut meta = HashMap::new();
        meta.insert("name".to_string(), MetadataValue::String(name.to_string()));
        meta.insert(
            "category".to_string(),
            MetadataValue::String(category.to_string()),
        );
        meta.insert("price".to_string(), MetadataValue::Float(price));
        // VectorMetadataStore uses 0-based index, VectorId.0 is 1-based
        metadata_store.set((id.0 - 1) as usize, meta);

        println!(
            "  ID {}: {} (category={}, price=${:.2})",
            id.0, name, category, price
        );
    }

    println!("\n--- Running Filtered Searches ---\n");

    // 3. Simple equality filter: category = "fruit"
    println!("Filter: category = \"fruit\"");
    let filter = parse(r#"category = "fruit""#).expect("Parse failed");
    let query = vec![0.95, 0.05, 0.0, 0.0]; // Close to fruits

    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);
    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Found {} results:", result.results.len());
    for r in &result.results {
        let meta = metadata_store.get((r.vector_id.0 - 1) as usize);
        if let Some(m) = meta {
            let name = m
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!("    - {} (score: {:.4})", name, r.distance);
        }
    }

    // 4. Range filter: price < 2.00
    println!("\nFilter: price < 2.00");
    let filter = parse("price < 2.0").expect("Parse failed");
    let query = vec![0.5, 0.5, 0.0, 0.0]; // Between fruits and vegetables

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Found {} results:", result.results.len());
    for r in &result.results {
        let meta = metadata_store.get((r.vector_id.0 - 1) as usize);
        if let Some(m) = meta {
            let name = m
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            let price = m
                .get("price")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!(
                "    - {} (price: {}, score: {:.4})",
                name, price, r.distance
            );
        }
    }

    // 5. Combined filter: category = "meat" OR category = "seafood"
    println!("\nFilter: category = \"meat\" OR category = \"seafood\"");
    let filter = parse(r#"category = "meat" OR category = "seafood""#).expect("Parse failed");
    let query = vec![0.0, 0.0, 0.5, 0.5]; // Between meat and seafood

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Found {} results:", result.results.len());
    for r in &result.results {
        let meta = metadata_store.get((r.vector_id.0 - 1) as usize);
        if let Some(m) = meta {
            let name = m
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            let cat = m
                .get("category")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!(
                "    - {} (category: {}, score: {:.4})",
                name, cat, r.distance
            );
        }
    }

    // 6. Complex filter: (category = "fruit" OR category = "vegetable") AND price < 1.0
    println!("\nFilter: (category = \"fruit\" OR category = \"vegetable\") AND price < 1.0");
    let filter = parse(r#"(category = "fruit" OR category = "vegetable") AND price < 1.0"#)
        .expect("Parse failed");
    let query = vec![0.5, 0.5, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Found {} results:", result.results.len());
    for r in &result.results {
        let meta = metadata_store.get((r.vector_id.0 - 1) as usize);
        if let Some(m) = meta {
            let name = m
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            let price = m
                .get("price")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!(
                "    - {} (price: {}, score: {:.4})",
                name, price, r.distance
            );
        }
    }

    println!("\n=== Example Complete ===");
}
