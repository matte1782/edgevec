//! Memory Pressure Monitoring for EdgeVec WASM.
//!
//! Provides visibility into WASM heap usage and enables graceful degradation
//! when memory is constrained.
//!
//! # RFC-002 Specification
//!
//! - Warning threshold: 80% usage
//! - Critical threshold: 95% usage
//! - Real-time usage stats
//! - Guidance for graceful degradation

use serde::{Deserialize, Serialize};
use std::cell::Cell;
use wasm_bindgen::prelude::*;

// =============================================================================
// MEMORY PRESSURE TYPES
// =============================================================================

/// Memory pressure levels.
///
/// Used to indicate the current state of WASM heap usage and guide
/// application behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryPressureLevel {
    /// < 80% usage — all operations allowed
    Normal,
    /// 80-95% usage — consider reducing data
    Warning,
    /// > 95% usage — risk of OOM, stop inserts
    Critical,
}

/// Memory pressure statistics.
///
/// Returned by `getMemoryPressure()` to provide visibility into WASM heap usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPressure {
    /// Current pressure level
    pub level: MemoryPressureLevel,
    /// Bytes currently allocated (estimated)
    pub used_bytes: usize,
    /// Total WASM heap size
    pub total_bytes: usize,
    /// Usage as percentage (0-100)
    pub usage_percent: f64,
}

impl MemoryPressure {
    /// Calculate memory pressure from current WASM heap.
    ///
    /// Uses WASM memory introspection to determine heap size and estimates
    /// used bytes based on allocation tracking.
    #[must_use]
    pub fn current() -> Self {
        Self::current_with_thresholds(80.0, 95.0)
    }

    /// Calculate memory pressure with custom thresholds.
    #[must_use]
    pub fn current_with_thresholds(warning_threshold: f64, critical_threshold: f64) -> Self {
        // Get WASM memory size
        let memory = wasm_bindgen::memory();
        let buffer = memory
            .unchecked_ref::<js_sys::WebAssembly::Memory>()
            .buffer();
        // buffer() returns JsValue, cast to ArrayBuffer to get byte_length
        let array_buffer = buffer.unchecked_ref::<js_sys::ArrayBuffer>();
        let total_bytes = array_buffer.byte_length() as usize;

        // Get estimated used bytes from our tracker
        let used_bytes = ALLOCATION_ESTIMATE.with(Cell::get);

        #[allow(clippy::cast_precision_loss)]
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let level = if usage_percent >= critical_threshold {
            MemoryPressureLevel::Critical
        } else if usage_percent >= warning_threshold {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        };

        Self {
            level,
            used_bytes,
            total_bytes,
            usage_percent,
        }
    }
}

// =============================================================================
// MEMORY CONFIGURATION
// =============================================================================

/// Memory pressure configuration.
///
/// Allows customization of thresholds and automatic behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConfig {
    /// Warning threshold percentage (default: 80)
    #[serde(default = "default_warning_threshold")]
    pub warning_threshold: f64,
    /// Critical threshold percentage (default: 95)
    #[serde(default = "default_critical_threshold")]
    pub critical_threshold: f64,
    /// Auto-compact when warning threshold reached
    #[serde(default)]
    pub auto_compact_on_warning: bool,
    /// Block inserts when critical threshold reached
    #[serde(default = "default_block_inserts")]
    pub block_inserts_on_critical: bool,
}

fn default_warning_threshold() -> f64 {
    80.0
}

fn default_critical_threshold() -> f64 {
    95.0
}

fn default_block_inserts() -> bool {
    true
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            auto_compact_on_warning: false,
            block_inserts_on_critical: true,
        }
    }
}

// =============================================================================
// MEMORY RECOMMENDATION
// =============================================================================

/// Memory recommendation based on current state.
///
/// Provides actionable guidance to the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecommendation {
    /// Recommended action: "none", "compact", or "reduce"
    pub action: String,
    /// Human-readable message
    pub message: String,
    /// Whether inserts are currently allowed
    pub can_insert: bool,
    /// Whether compaction would help
    pub suggest_compact: bool,
}

// =============================================================================
// ALLOCATION TRACKING
// =============================================================================

// Thread-local estimation of used bytes
// Updated when we know about allocations (vectors, metadata, etc.)
thread_local! {
    static ALLOCATION_ESTIMATE: Cell<usize> = const { Cell::new(0) };
}

/// Update the allocation estimate when we allocate memory.
///
/// Call this when vectors are inserted to track WASM heap usage.
/// Used by `getMemoryPressure()` to provide visibility into memory state.
#[inline]
pub fn track_allocation(bytes: usize) {
    ALLOCATION_ESTIMATE.with(|e| {
        e.set(e.get().saturating_add(bytes));
    });
}

/// Update the allocation estimate when we free memory.
///
/// Call this when vectors are deleted to track WASM heap usage.
/// Note: Currently used for future soft-delete tracking (W28.5+).
#[inline]
#[allow(dead_code)]
pub fn track_deallocation(bytes: usize) {
    ALLOCATION_ESTIMATE.with(|e| {
        e.set(e.get().saturating_sub(bytes));
    });
}

/// Get the current allocation estimate.
///
/// Used internally by `MemoryPressure::current()`.
#[must_use]
#[inline]
#[allow(dead_code)]
pub fn get_allocation_estimate() -> usize {
    ALLOCATION_ESTIMATE.with(Cell::get)
}

/// Track a vector insertion (dimensions × 4 bytes for F32).
///
/// Convenience function for vector operations.
#[inline]
pub fn track_vector_insert(dimensions: u32) {
    let bytes = (dimensions as usize) * std::mem::size_of::<f32>();
    track_allocation(bytes);
}

/// Track a batch of vector insertions.
#[inline]
pub fn track_batch_insert(count: usize, dimensions: u32) {
    let bytes_per_vector = (dimensions as usize) * std::mem::size_of::<f32>();
    track_allocation(count * bytes_per_vector);
}

/// Reset the allocation estimate (for testing).
#[cfg(test)]
pub fn reset_allocation_estimate() {
    ALLOCATION_ESTIMATE.with(|e| e.set(0));
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pressure_level_serialize() {
        let normal = MemoryPressureLevel::Normal;
        let json = serde_json::to_string(&normal).unwrap();
        assert_eq!(json, "\"normal\"");

        let warning = MemoryPressureLevel::Warning;
        let json = serde_json::to_string(&warning).unwrap();
        assert_eq!(json, "\"warning\"");

        let critical = MemoryPressureLevel::Critical;
        let json = serde_json::to_string(&critical).unwrap();
        assert_eq!(json, "\"critical\"");
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert!((config.warning_threshold - 80.0).abs() < f64::EPSILON);
        assert!((config.critical_threshold - 95.0).abs() < f64::EPSILON);
        assert!(!config.auto_compact_on_warning);
        assert!(config.block_inserts_on_critical);
    }

    #[test]
    fn test_memory_config_deserialize() {
        let json = r#"{
            "warningThreshold": 70,
            "criticalThreshold": 90,
            "autoCompactOnWarning": true,
            "blockInsertsOnCritical": false
        }"#;

        let config: MemoryConfig = serde_json::from_str(json).unwrap();
        assert!((config.warning_threshold - 70.0).abs() < f64::EPSILON);
        assert!((config.critical_threshold - 90.0).abs() < f64::EPSILON);
        assert!(config.auto_compact_on_warning);
        assert!(!config.block_inserts_on_critical);
    }

    #[test]
    fn test_allocation_tracking() {
        reset_allocation_estimate();

        assert_eq!(get_allocation_estimate(), 0);

        track_allocation(1000);
        assert_eq!(get_allocation_estimate(), 1000);

        track_allocation(500);
        assert_eq!(get_allocation_estimate(), 1500);

        track_deallocation(300);
        assert_eq!(get_allocation_estimate(), 1200);

        // Saturating subtraction - can't go below 0
        track_deallocation(2000);
        assert_eq!(get_allocation_estimate(), 0);
    }

    #[test]
    fn test_memory_recommendation_serialize() {
        let rec = MemoryRecommendation {
            action: "compact".to_string(),
            message: "Memory usage high".to_string(),
            can_insert: true,
            suggest_compact: true,
        };

        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("\"action\":\"compact\""));
        assert!(json.contains("\"canInsert\":true"));
        assert!(json.contains("\"suggestCompact\":true"));
    }
}
