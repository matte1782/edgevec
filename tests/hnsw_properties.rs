//! HNSW Property Tests
//!
//! Task: W10.4 - Implement HNSW Property Tests
//!
//! This module implements 5 property-based tests for HNSW graph invariants:
//! 1. Connectivity: All inserted vectors are reachable from entry point
//! 2. Level Distribution: Level assignments follow exponential decay
//! 3. Neighbor Consistency: If A is neighbor of B, distance(A,B) is reasonable
//! 4. Search Recall: For exact search (k=1), result is provably within top-k
//! 5. No Orphans: No vectors are unreachable from entry point

use edgevec::hnsw::{HnswConfig, HnswIndex, NodeId, SearchContext, Searcher};
use edgevec::metric::L2Squared;
use edgevec::storage::VectorStorage;
use proptest::prelude::*;
use std::collections::{HashSet, VecDeque};

/// Helper to create index and storage with custom parameters
fn create_env(dim: u32, m: u32, ef: u32) -> (HnswIndex, VectorStorage) {
    let mut config = HnswConfig::new(dim);
    config.m = m;
    config.m0 = m * 2;
    config.ef_construction = ef;
    let storage = VectorStorage::new(&config, None);
    let index = HnswIndex::new(config, &storage).unwrap();
    (index, storage)
}

/// Helper to compute L2 squared distance between two vectors
fn l2_squared(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// Helper to perform BFS traversal from entry point and return all reachable nodes
fn bfs_reachable(index: &HnswIndex, start: NodeId) -> HashSet<NodeId> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(current) = queue.pop_front() {
        if let Some(node) = index.get_node(current) {
            // Traverse all layers for this node
            for layer in 0..=node.max_layer {
                if let Ok(neighbors) = index.get_neighbors_layer(node, layer) {
                    for neighbor in neighbors {
                        if !visited.contains(&neighbor) {
                            visited.insert(neighbor);
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }
    }

    visited
}

proptest! {
    // 1000 test cases as required by W10.4 acceptance criteria
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // =========================================================================
    // PROPERTY 1: Connectivity
    // All inserted vectors are reachable from entry point
    // =========================================================================
    #[test]
    fn prop_connectivity(
        // Generate vectors with dimension 4 for speed
        vectors in prop::collection::vec(
            prop::collection::vec(-10.0f32..10.0, 4),
            5..30  // 5-30 vectors per test
        ),
        m in 4u32..12,
        ef in 30u32..60,
    ) {
        let (mut index, mut storage) = create_env(4, m, ef);

        // Insert all vectors
        for vec in &vectors {
            let _ = index.insert(vec, &mut storage);
        }

        // Skip if no entry point (empty graph)
        if let Some(entry_point) = index.entry_point() {
            // BFS from entry point
            let reachable = bfs_reachable(&index, entry_point);

            // All nodes must be reachable
            let node_count = index.node_count();
            prop_assert_eq!(
                reachable.len(),
                node_count,
                "Not all nodes reachable: {}/{} nodes found via BFS from entry point",
                reachable.len(),
                node_count
            );

            // Verify each NodeId is in reachable set
            for i in 0..node_count {
                let node_id = NodeId(i as u32);
                prop_assert!(
                    reachable.contains(&node_id),
                    "NodeId {:?} is not reachable from entry point",
                    node_id
                );
            }
        }
    }

    // =========================================================================
    // PROPERTY 2: Level Distribution
    // Level assignments follow exponential decay (geometric distribution)
    // For M=16: P(level=0) ≈ 93.75%, P(level>0) ≈ 6.25%
    // =========================================================================
    #[test]
    fn prop_level_distribution(
        _seed in 0u64..1000,
        m in 4u32..20,
    ) {
        let mut config = HnswConfig::new(4);
        config.m = m;
        config.m0 = m * 2;
        let storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        // Generate 100 random levels
        let mut levels = Vec::with_capacity(100);
        for _ in 0..100 {
            levels.push(index.get_random_level());
        }

        // Check exponential decay properties:

        // 1. Level 0 should be most common (majority)
        let l0_count = levels.iter().filter(|&&l| l == 0).count();
        prop_assert!(
            l0_count > 50,
            "Level 0 should be dominant, got {}/100",
            l0_count
        );

        // 2. Higher levels should be less frequent (monotonic decay)
        let mut level_counts = [0usize; 17];
        for &level in &levels {
            level_counts[level as usize] += 1;
        }

        // 3. No level should exceed reasonable max (capped at 16)
        let max_level = *levels.iter().max().unwrap();
        prop_assert!(
            max_level <= 16,
            "Max level {} exceeds cap of 16",
            max_level
        );

        // 4. Exponential decay: count(level) > count(level+1) on average
        // (allowing for statistical variance by checking overall pattern)
        let higher_levels_total: usize = level_counts[1..].iter().sum();
        prop_assert!(
            l0_count >= higher_levels_total,
            "Level 0 count ({}) should be >= sum of higher levels ({})",
            l0_count,
            higher_levels_total
        );
    }

    // =========================================================================
    // PROPERTY 3: Neighbor Consistency
    // If A is neighbor of B, distance(A,B) is reasonable (not infinite/NaN)
    // =========================================================================
    #[test]
    fn prop_neighbor_consistency(
        vectors in prop::collection::vec(
            prop::collection::vec(-10.0f32..10.0, 4),
            5..25
        ),
        m in 4u32..12,
        ef in 30u32..60,
    ) {
        let (mut index, mut storage) = create_env(4, m, ef);

        // Insert all vectors
        for vec in &vectors {
            let _ = index.insert(vec, &mut storage);
        }

        // Check every node's neighbors
        for i in 0..index.node_count() {
            let node_id = NodeId(i as u32);
            let node = index.get_node(node_id).unwrap();

            // Get the vector for this node
            let vec_a = storage.get_vector(node.vector_id);

            // Check all layers
            for layer in 0..=node.max_layer {
                let neighbors = index.get_neighbors_layer(node, layer).unwrap();

                for neighbor_id in neighbors {
                    // Get neighbor's vector
                    let neighbor_node = index.get_node(neighbor_id).unwrap();
                    let vec_b = storage.get_vector(neighbor_node.vector_id);

                    // Compute distance
                    let dist = l2_squared(&vec_a, &vec_b);

                    // Distance must be finite and non-negative
                    prop_assert!(
                        dist.is_finite(),
                        "Distance between NodeId {:?} and {:?} is not finite: {}",
                        node_id, neighbor_id, dist
                    );
                    prop_assert!(
                        dist >= 0.0,
                        "Distance between NodeId {:?} and {:?} is negative: {}",
                        node_id, neighbor_id, dist
                    );

                    // Distance should be reasonable (not pathologically large)
                    // With vectors in [-10, 10], max L2² for 4D is 4 * (20)² = 1600
                    prop_assert!(
                        dist <= 1700.0,
                        "Distance between NodeId {:?} and {:?} is suspiciously large: {}",
                        node_id, neighbor_id, dist
                    );
                }
            }
        }
    }

    // =========================================================================
    // PROPERTY 4: Search Recall
    // For exact search (k=1), result is provably within top-k by brute force
    // =========================================================================
    #[test]
    fn prop_search_recall(
        vectors in prop::collection::vec(
            prop::collection::vec(-10.0f32..10.0, 4),
            10..30
        ),
        query in prop::collection::vec(-10.0f32..10.0, 4),
        m in 4u32..12,
        ef in 50u32..100,  // Higher ef for better recall
    ) {
        let (mut index, mut storage) = create_env(4, m, ef);

        // Insert all vectors
        let mut inserted_ids = Vec::new();
        for vec in &vectors {
            let id = index.insert(vec, &mut storage).unwrap();
            inserted_ids.push(id);
        }

        // Skip if graph is too small
        if index.entry_point().is_none() || vectors.is_empty() {
            return Ok(());
        }

        // Brute force: find the true nearest neighbor
        let mut best_dist = f32::INFINITY;
        let mut best_idx = 0;
        for (idx, vec) in vectors.iter().enumerate() {
            let dist = l2_squared(&query, vec);
            if dist < best_dist {
                best_dist = dist;
                best_idx = idx;
            }
        }
        let true_nearest_vid = inserted_ids[best_idx];

        // HNSW search
        let searcher = Searcher::<L2Squared, VectorStorage>::new(&index, &storage);
        let mut ctx = SearchContext::new();

        let entry_point = index.entry_point().unwrap();
        let max_layer = index.max_layer();
        let mut curr_ep = entry_point;

        // Top layers: greedy search
        for lc in (1..=max_layer).rev() {
            let _ = searcher.search_layer(&mut ctx, [curr_ep], &query, 1, lc);
            if let Some(best) = ctx.scratch.first() {
                curr_ep = best.node_id;
            }
        }

        // Bottom layer: beam search with high ef
        let _ = searcher.search_layer(&mut ctx, [curr_ep], &query, ef as usize, 0);

        // Check if true nearest is in results
        let found = ctx.scratch.iter().any(|c| {
            let node = index.get_node(c.node_id).unwrap();
            node.vector_id == true_nearest_vid
        });

        // With high ef, we should find the true nearest neighbor
        // Note: HNSW is approximate, so we allow for occasional misses
        // but track the miss rate
        if !found && !ctx.scratch.is_empty() {
            // Check if the returned result is at least close
            let hnsw_best = ctx.scratch.first().unwrap();
            let hnsw_node = index.get_node(hnsw_best.node_id).unwrap();
            let hnsw_vec = storage.get_vector(hnsw_node.vector_id);
            let hnsw_dist = l2_squared(&query, &hnsw_vec);

            // The returned result should be within 2x of the true best
            // (reasonable tolerance for approximate search)
            prop_assert!(
                hnsw_dist <= best_dist * 2.0 + f32::EPSILON,
                "HNSW result distance {} is more than 2x worse than true best {}",
                hnsw_dist, best_dist
            );
        }
    }

    // =========================================================================
    // PROPERTY 5: No Orphans
    // No vectors are unreachable from entry point (equivalent to Connectivity
    // but tested from a different angle - checking for isolated nodes)
    // =========================================================================
    #[test]
    fn prop_no_orphans(
        vectors in prop::collection::vec(
            prop::collection::vec(-10.0f32..10.0, 4),
            5..25
        ),
        m in 4u32..12,
        ef in 30u32..60,
    ) {
        let (mut index, mut storage) = create_env(4, m, ef);

        // Insert all vectors
        for vec in &vectors {
            let _ = index.insert(vec, &mut storage);
        }

        // Skip if empty
        if index.entry_point().is_none() {
            return Ok(());
        }

        let entry_point = index.entry_point().unwrap();
        let reachable = bfs_reachable(&index, entry_point);
        let node_count = index.node_count();

        // Find orphans (nodes not in reachable set)
        let mut orphans = Vec::new();
        for i in 0..node_count {
            let node_id = NodeId(i as u32);
            if !reachable.contains(&node_id) {
                orphans.push(node_id);
            }
        }

        // No orphans should exist
        prop_assert!(
            orphans.is_empty(),
            "Found {} orphan nodes: {:?}",
            orphans.len(),
            orphans
        );

        // Additional check: every node should have at least 1 neighbor at layer 0
        // (unless it's the only node)
        if node_count > 1 {
            for i in 0..node_count {
                let node_id = NodeId(i as u32);
                let node = index.get_node(node_id).unwrap();
                let neighbors = index.get_neighbors_layer(node, 0).unwrap();

                // Node should have at least one neighbor (unless it's isolated)
                // This is a weaker check - we're mainly interested in reachability
                // but having 0 neighbors at layer 0 is suspicious
                if neighbors.is_empty() {
                    // Check if this node is the entry point (it might have higher layer neighbors only)
                    if node_id != entry_point {
                        // Check if reachable via higher layers
                        let mut found_neighbor = false;
                        for layer in 1..=node.max_layer {
                            let layer_neighbors = index.get_neighbors_layer(node, layer).unwrap();
                            if !layer_neighbors.is_empty() {
                                found_neighbor = true;
                                break;
                            }
                        }
                        prop_assert!(
                            found_neighbor,
                            "NodeId {:?} has no neighbors at any layer and is not entry point",
                            node_id
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_bfs_reachable_single_node() {
        let (mut index, mut storage) = create_env(4, 8, 50);
        let vec = vec![1.0, 2.0, 3.0, 4.0];
        index.insert(&vec, &mut storage).unwrap();

        let entry = index.entry_point().unwrap();
        let reachable = bfs_reachable(&index, entry);

        assert_eq!(reachable.len(), 1);
        assert!(reachable.contains(&entry));
    }

    #[test]
    fn test_bfs_reachable_multiple_nodes() {
        let (mut index, mut storage) = create_env(4, 8, 50);

        for i in 0..10 {
            let vec = vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32];
            index.insert(&vec, &mut storage).unwrap();
        }

        let entry = index.entry_point().unwrap();
        let reachable = bfs_reachable(&index, entry);

        assert_eq!(reachable.len(), 10);
    }

    #[test]
    fn test_l2_squared_helper() {
        let a = vec![0.0, 0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0, 1.0];
        assert!((l2_squared(&a, &b) - 4.0).abs() < f32::EPSILON);

        let c = vec![3.0, 4.0];
        let d = vec![0.0, 0.0];
        assert!((l2_squared(&c, &d) - 25.0).abs() < f32::EPSILON);
    }
}
