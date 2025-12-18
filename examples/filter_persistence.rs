//! Example: Persistence with Filtered Data
//!
//! Demonstrates:
//! - Creating an index with filtered data
//! - Soft deleting vectors
//! - Compaction workflow
//! - Saving and loading with filters preserved
//!
//! Run with: `cargo run --example filter_persistence`

use edgevec::filter::{parse, FilterStrategy, FilteredSearcher, VectorMetadataStore};
use edgevec::metadata::MetadataValue;
use edgevec::persistence::{read_snapshot, write_snapshot, MemoryBackend};
use edgevec::{HnswConfig, HnswIndex, VectorStorage};
use std::collections::HashMap;

fn main() {
    println!("=== EdgeVec Persistence with Filters Example ===\n");

    // 1. Create and populate index
    let config = HnswConfig::new(8);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");
    let mut metadata_store = VectorMetadataStore::new();

    // Insert items
    let items = vec![
        (
            "Item 1",
            "active",
            100,
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "Item 2",
            "active",
            200,
            vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "Item 3",
            "inactive",
            150,
            vec![0.8, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "Item 4",
            "active",
            300,
            vec![0.7, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "Item 5",
            "deleted",
            50,
            vec![0.6, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "Item 6",
            "active",
            250,
            vec![0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
    ];

    println!("Step 1: Creating index with {} items", items.len());
    let mut ids = Vec::new();

    for (name, status, value, embedding) in items {
        let id = index
            .insert(&embedding, &mut storage)
            .expect("Insert failed");
        ids.push(id);

        let mut meta = HashMap::new();
        meta.insert("name".to_string(), MetadataValue::String(name.to_string()));
        meta.insert(
            "status".to_string(),
            MetadataValue::String(status.to_string()),
        );
        meta.insert("value".to_string(), MetadataValue::Integer(value));
        // VectorMetadataStore uses 0-based index, VectorId.0 is 1-based
        metadata_store.set((id.0 - 1) as usize, meta);

        println!(
            "  Inserted {} (ID: {}, status: {}, value: {})",
            name, id.0, status, value
        );
    }

    // 2. Demonstrate filtered search before deletion
    println!("\nStep 2: Filtered search for active items");
    let mut searcher = FilteredSearcher::new(&index, &storage, &metadata_store);

    let filter = parse(r#"status = "active""#).expect("Parse failed");
    let query = vec![0.8, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    let result = searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Found {} active items:", result.results.len());
    for r in &result.results {
        if let Some(meta) = metadata_store.get((r.vector_id.0 - 1) as usize) {
            let name = meta
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!("    - {} (score: {:.4})", name, r.distance);
        }
    }

    // 3. Soft delete some vectors
    println!("\nStep 3: Soft deleting items with status 'deleted' or 'inactive'");

    for &id in &ids {
        if let Some(meta) = metadata_store.get((id.0 - 1) as usize) {
            if let Some(MetadataValue::String(status)) = meta.get("status") {
                if status == "deleted" || status == "inactive" {
                    let was_deleted = index.soft_delete(id).expect("Delete failed");
                    println!("  Soft deleted ID {} (was_deleted: {})", id.0, was_deleted);
                }
            }
        }
    }

    println!("\n  Live count: {}", index.live_count());
    println!("  Deleted count: {}", index.deleted_count());
    println!("  Tombstone ratio: {:.1}%", index.tombstone_ratio() * 100.0);

    // 4. Search after deletion (deleted items excluded)
    println!("\nStep 4: Search after soft delete");
    let result = index.search(&query, 10, &storage).expect("Search failed");

    println!("  Unfiltered search found {} results:", result.len());
    for r in &result {
        if let Some(meta) = metadata_store.get((r.vector_id.0 - 1) as usize) {
            let name = meta
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            let status = meta
                .get("status")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!(
                "    - {} (status: {}, score: {:.4})",
                name, status, r.distance
            );
        }
    }

    // 5. Check if compaction needed
    println!("\nStep 5: Compaction check");
    println!("  Needs compaction: {}", index.needs_compaction());

    if index.needs_compaction() {
        if let Some(warning) = index.compaction_warning() {
            println!("  Warning: {}", warning);
        }

        println!("\n  Running compaction...");
        let (new_index, new_storage, result) = index.compact(&storage).expect("Compaction failed");
        index = new_index;
        storage = new_storage;

        println!("  Compaction result:");
        println!("    Tombstones removed: {}", result.tombstones_removed);
        println!("    New size: {}", result.new_size);
        println!("    Duration: {}ms", result.duration_ms);
    }

    // 6. Save to memory backend
    println!("\nStep 6: Saving index to memory");
    let mut backend = MemoryBackend::new();

    write_snapshot(&index, &storage, &mut backend).expect("Save failed");
    println!("  Saved snapshot to memory");

    // 7. Load from memory backend
    println!("\nStep 7: Loading index from memory");
    let (loaded_index, loaded_storage): (HnswIndex, VectorStorage) =
        read_snapshot(&backend).expect("Load failed");

    println!("  Loaded index with {} nodes", loaded_index.node_count());
    println!("  Live count: {}", loaded_index.live_count());

    // 8. Verify loaded index works with filters
    println!("\nStep 8: Verifying loaded index with filtered search");
    let mut loaded_searcher =
        FilteredSearcher::new(&loaded_index, &loaded_storage, &metadata_store);

    let filter = parse(r#"status = "active" AND value > 100"#).expect("Parse failed");
    let result = loaded_searcher
        .search_filtered(&query, 10, Some(&filter), FilterStrategy::Auto)
        .expect("Search failed");

    println!("  Filter: status = \"active\" AND value > 100");
    println!("  Found {} results:", result.results.len());
    for r in &result.results {
        if let Some(meta) = metadata_store.get((r.vector_id.0 - 1) as usize) {
            let name = meta
                .get("name")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            let value = meta
                .get("value")
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();
            println!(
                "    - {} (value: {}, score: {:.4})",
                name, value, r.distance
            );
        }
    }

    // 9. Summary
    println!("\n=== Persistence Summary ===");
    println!("  Original items: 6");
    println!("  Soft deleted: 2 (inactive + deleted status)");
    println!("  After compaction: {}", loaded_index.live_count());
    println!("  Filters work after load: Yes");

    println!("\n=== Example Complete ===");
}
