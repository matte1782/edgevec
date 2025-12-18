//! Example: Document Similarity Search with Metadata Filters
//!
//! Demonstrates a document/RAG use case:
//! - Document corpus with metadata (author, date, tags)
//! - Full-text similarity with metadata filtering
//! - String operators (CONTAINS, STARTS_WITH)
//! - NULL checks for optional fields
//!
//! Run with: `cargo run --example filter_documents`

use edgevec::filter::{parse, FilterStrategy, FilteredSearcher, VectorMetadataStore};
use edgevec::metadata::MetadataValue;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::collections::HashMap;

/// Document with metadata
struct Document {
    title: String,
    author: String,
    year: i64,
    department: Option<String>,
    tags: Vec<String>,
    embedding: Vec<f32>,
}

fn main() {
    println!("=== EdgeVec Document Search Example ===\n");

    // 1. Create index (8 dimensions for demo)
    let config = HnswConfig::new(8);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");
    let mut metadata_store = VectorMetadataStore::new();

    // 2. Create document corpus
    let documents = vec![
        Document {
            title: "Introduction to Machine Learning".to_string(),
            author: "Dr. Smith".to_string(),
            year: 2023,
            department: Some("Computer Science".to_string()),
            tags: vec!["ml".to_string(), "ai".to_string(), "tutorial".to_string()],
            embedding: vec![1.0, 0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
        },
        Document {
            title: "Deep Learning for Computer Vision".to_string(),
            author: "Dr. Smith".to_string(),
            year: 2024,
            department: Some("Computer Science".to_string()),
            tags: vec![
                "ml".to_string(),
                "vision".to_string(),
                "deep-learning".to_string(),
            ],
            embedding: vec![0.95, 0.9, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0],
        },
        Document {
            title: "Natural Language Processing Fundamentals".to_string(),
            author: "Dr. Johnson".to_string(),
            year: 2023,
            department: Some("Linguistics".to_string()),
            tags: vec!["nlp".to_string(), "ai".to_string(), "text".to_string()],
            embedding: vec![0.8, 0.7, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0],
        },
        Document {
            title: "Statistical Methods in Research".to_string(),
            author: "Dr. Williams".to_string(),
            year: 2022,
            department: Some("Mathematics".to_string()),
            tags: vec!["statistics".to_string(), "research".to_string()],
            embedding: vec![0.2, 0.3, 0.8, 0.7, 0.1, 0.0, 0.0, 0.0],
        },
        Document {
            title: "Advanced Neural Networks".to_string(),
            author: "Dr. Smith".to_string(),
            year: 2024,
            department: Some("Computer Science".to_string()),
            tags: vec![
                "ml".to_string(),
                "neural-networks".to_string(),
                "advanced".to_string(),
            ],
            embedding: vec![0.9, 0.85, 0.15, 0.05, 0.0, 0.0, 0.0, 0.0],
        },
        Document {
            title: "Data Visualization Techniques".to_string(),
            author: "Prof. Brown".to_string(),
            year: 2021,
            department: None, // Guest lecturer, no department
            tags: vec!["visualization".to_string(), "data".to_string()],
            embedding: vec![0.3, 0.4, 0.6, 0.5, 0.2, 0.1, 0.0, 0.0],
        },
        Document {
            title: "Ethics in AI Research".to_string(),
            author: "Dr. Garcia".to_string(),
            year: 2024,
            department: Some("Philosophy".to_string()),
            tags: vec!["ethics".to_string(), "ai".to_string(), "policy".to_string()],
            embedding: vec![0.5, 0.4, 0.3, 0.2, 0.6, 0.5, 0.0, 0.0],
        },
    ];

    // 3. Index documents
    println!("Indexing {} documents:", documents.len());
    for doc in &documents {
        let id = index
            .insert(&doc.embedding, &mut storage)
            .expect("Insert failed");

        let mut meta = HashMap::new();
        meta.insert(
            "title".to_string(),
            MetadataValue::String(doc.title.clone()),
        );
        meta.insert(
            "author".to_string(),
            MetadataValue::String(doc.author.clone()),
        );
        meta.insert("year".to_string(), MetadataValue::Integer(doc.year));

        // Optional department
        if let Some(ref dept) = doc.department {
            meta.insert(
                "department".to_string(),
                MetadataValue::String(dept.clone()),
            );
        }

        // Tags as string array
        meta.insert(
            "tags".to_string(),
            MetadataValue::StringArray(doc.tags.clone()),
        );

        // VectorMetadataStore uses 0-based index, VectorId.0 is 1-based
        metadata_store.set((id.0 - 1) as usize, meta);

        println!(
            "  [{}] \"{}\" by {} ({})",
            id.0, doc.title, doc.author, doc.year
        );
    }

    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    println!("\n--- Document Search Scenarios ---\n");

    // Scenario 1: Documents by specific author
    println!("Scenario 1: author = \"Dr. Smith\"");
    let filter = parse(r#"author = "Dr. Smith""#).expect("Parse failed");
    let query = vec![0.9, 0.85, 0.15, 0.05, 0.0, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 2: Recent documents (2024)
    println!("\nScenario 2: year = 2024");
    let filter = parse("year = 2024").expect("Parse failed");
    let query = vec![0.5, 0.5, 0.2, 0.1, 0.1, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 3: Title contains keyword
    println!("\nScenario 3: title CONTAINS \"Learning\"");
    let filter = parse(r#"title CONTAINS "Learning""#).expect("Parse failed");
    let query = vec![0.9, 0.85, 0.15, 0.05, 0.0, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 4: Department starts with "Computer"
    println!("\nScenario 4: department STARTS_WITH \"Computer\"");
    let filter = parse(r#"department STARTS_WITH "Computer""#).expect("Parse failed");
    let query = vec![0.9, 0.85, 0.15, 0.05, 0.0, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 5: Documents without department (guest lecturers)
    println!("\nScenario 5: department IS NULL");
    let filter = parse("department IS NULL").expect("Parse failed");
    let query = vec![0.3, 0.4, 0.6, 0.5, 0.2, 0.1, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 6: Documents with department assigned
    println!("\nScenario 6: department IS NOT NULL");
    let filter = parse("department IS NOT NULL").expect("Parse failed");
    let query = vec![0.5, 0.5, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    // Scenario 7: Complex query - CS department, recent, ML-related
    println!(
        "\nScenario 7: department = \"Computer Science\" AND year >= 2023 AND title CONTAINS \"Learning\""
    );
    let filter =
        parse(r#"department = "Computer Science" AND year >= 2023 AND title CONTAINS "Learning""#)
            .expect("Parse failed");
    let query = vec![0.95, 0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    print_results(&result.results, &metadata_store);

    println!("\n=== Example Complete ===");
}

fn print_results(results: &[edgevec::hnsw::SearchResult], metadata_store: &VectorMetadataStore) {
    println!("  Found {} results:", results.len());
    for r in results {
        if let Some(meta) = metadata_store.get((r.vector_id.0 - 1) as usize) {
            let title = match meta.get("title") {
                Some(MetadataValue::String(s)) => s.clone(),
                _ => "Unknown".to_string(),
            };
            let author = match meta.get("author") {
                Some(MetadataValue::String(s)) => s.clone(),
                _ => "Unknown".to_string(),
            };
            let year = match meta.get("year") {
                Some(MetadataValue::Integer(y)) => *y,
                _ => 0,
            };
            println!(
                "    - \"{}\" by {} ({}) [score: {:.4}]",
                title, author, year, r.distance
            );
        }
    }
}
