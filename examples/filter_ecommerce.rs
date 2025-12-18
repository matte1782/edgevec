//! Example: E-commerce Product Search with Filters
//!
//! Demonstrates a realistic e-commerce use case:
//! - Product catalog with rich metadata
//! - Price range filtering
//! - Category filtering
//! - Rating filtering
//! - Combined multi-attribute queries
//!
//! Run with: `cargo run --example filter_ecommerce`

use edgevec::filter::{parse, FilterStrategy, FilteredSearcher, VectorMetadataStore};
use edgevec::metadata::MetadataValue;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::collections::HashMap;

/// Product data structure
struct Product {
    name: String,
    category: String,
    brand: String,
    price: f64,
    rating: f64,
    in_stock: bool,
    embedding: Vec<f32>,
}

fn main() {
    println!("=== EdgeVec E-commerce Product Search Example ===\n");

    // 1. Create index (using 8 dimensions for demo)
    let config = HnswConfig::new(8);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");
    let mut metadata_store = VectorMetadataStore::new();

    // 2. Create product catalog
    let products = vec![
        Product {
            name: "RTX 4090 Gaming GPU".to_string(),
            category: "gpu".to_string(),
            brand: "nvidia".to_string(),
            price: 1599.99,
            rating: 4.8,
            in_stock: true,
            embedding: vec![1.0, 0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Product {
            name: "RTX 4080 Super".to_string(),
            category: "gpu".to_string(),
            brand: "nvidia".to_string(),
            price: 999.99,
            rating: 4.7,
            in_stock: true,
            embedding: vec![0.95, 0.85, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Product {
            name: "RX 7900 XTX".to_string(),
            category: "gpu".to_string(),
            brand: "amd".to_string(),
            price: 899.99,
            rating: 4.5,
            in_stock: true,
            embedding: vec![0.9, 0.8, 0.15, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Product {
            name: "Intel Core i9-14900K".to_string(),
            category: "cpu".to_string(),
            brand: "intel".to_string(),
            price: 549.99,
            rating: 4.6,
            in_stock: true,
            embedding: vec![0.0, 0.0, 0.0, 1.0, 0.9, 0.1, 0.0, 0.0],
        },
        Product {
            name: "AMD Ryzen 9 7950X".to_string(),
            category: "cpu".to_string(),
            brand: "amd".to_string(),
            price: 499.99,
            rating: 4.7,
            in_stock: true,
            embedding: vec![0.0, 0.0, 0.0, 0.95, 0.85, 0.15, 0.0, 0.0],
        },
        Product {
            name: "Budget Gaming GPU".to_string(),
            category: "gpu".to_string(),
            brand: "amd".to_string(),
            price: 249.99,
            rating: 4.0,
            in_stock: false,
            embedding: vec![0.7, 0.5, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Product {
            name: "32GB DDR5 RAM Kit".to_string(),
            category: "memory".to_string(),
            brand: "corsair".to_string(),
            price: 129.99,
            rating: 4.8,
            in_stock: true,
            embedding: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.9],
        },
        Product {
            name: "64GB DDR5 RAM Kit".to_string(),
            category: "memory".to_string(),
            brand: "gskill".to_string(),
            price: 249.99,
            rating: 4.9,
            in_stock: true,
            embedding: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.95, 0.95],
        },
    ];

    // 3. Index products
    println!("Indexing {} products:", products.len());
    for product in &products {
        let id = index
            .insert(&product.embedding, &mut storage)
            .expect("Insert failed");

        let mut meta = HashMap::new();
        meta.insert(
            "name".to_string(),
            MetadataValue::String(product.name.clone()),
        );
        meta.insert(
            "category".to_string(),
            MetadataValue::String(product.category.clone()),
        );
        meta.insert(
            "brand".to_string(),
            MetadataValue::String(product.brand.clone()),
        );
        meta.insert("price".to_string(), MetadataValue::Float(product.price));
        meta.insert("rating".to_string(), MetadataValue::Float(product.rating));
        meta.insert(
            "in_stock".to_string(),
            MetadataValue::Boolean(product.in_stock),
        );
        // VectorMetadataStore uses 0-based index, VectorId.0 is 1-based
        metadata_store.set((id.0 - 1) as usize, meta);

        println!(
            "  {} - ${:.2} ({}, {:.1} stars)",
            product.name, product.price, product.category, product.rating
        );
    }

    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    println!("\n--- E-commerce Search Scenarios ---\n");

    // Scenario 1: High-end GPUs under $1200
    println!("Scenario 1: category = \"gpu\" AND price < 1200");
    let filter = parse(r#"category = "gpu" AND price < 1200"#).expect("Parse failed");
    let query = vec![1.0, 0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0]; // GPU-like query

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    // Scenario 2: AMD products with rating >= 4.5
    println!("\nScenario 2: brand = \"amd\" AND rating >= 4.5");
    let filter = parse(r#"brand = "amd" AND rating >= 4.5"#).expect("Parse failed");
    let query = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.0, 0.0]; // General query

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    // Scenario 3: In-stock items under $300
    println!("\nScenario 3: in_stock = true AND price < 300");
    let filter = parse("in_stock = true AND price < 300").expect("Parse failed");
    let query = vec![0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5];

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    // Scenario 4: GPUs or CPUs, highly rated
    println!("\nScenario 4: (category = \"gpu\" OR category = \"cpu\") AND rating >= 4.6");
    let filter =
        parse(r#"(category = "gpu" OR category = "cpu") AND rating >= 4.6"#).expect("Parse failed");
    let query = vec![0.5, 0.5, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 5, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    // Scenario 5: Price range with BETWEEN
    println!("\nScenario 5: price BETWEEN 200 AND 600");
    let filter = parse("price BETWEEN 200 AND 600").expect("Parse failed");
    let query = vec![0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25, 0.25];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    // Scenario 6: Category IN list
    println!("\nScenario 6: category IN [\"gpu\", \"cpu\"] AND in_stock = true");
    let filter = parse(r#"category IN ["gpu", "cpu"] AND in_stock = true"#).expect("Parse failed");
    let query = vec![0.5, 0.5, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store, "");

    println!("\n=== Example Complete ===");
}

fn print_results(
    results: &[edgevec::hnsw::SearchResult],
    metadata_store: &VectorMetadataStore,
    prefix: &str,
) {
    println!("{}  Found {} results:", prefix, results.len());
    for r in results {
        if let Some(meta) = metadata_store.get((r.vector_id.0 - 1) as usize) {
            let name = match meta.get("name") {
                Some(MetadataValue::String(s)) => s.clone(),
                _ => "Unknown".to_string(),
            };
            let price = match meta.get("price") {
                Some(MetadataValue::Float(p)) => *p,
                _ => 0.0,
            };
            let rating = match meta.get("rating") {
                Some(MetadataValue::Float(r)) => *r,
                _ => 0.0,
            };
            println!(
                "{}    - {} (${:.2}, {:.1} stars, score: {:.4})",
                prefix, name, price, rating, r.distance
            );
        }
    }
}
