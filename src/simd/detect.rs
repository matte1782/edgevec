//! Runtime SIMD capability detection
//!
//! Detects CPU SIMD features at runtime to provide performance warnings
//! and enable feature-appropriate code paths.
//!
//! # Example
//!
//! ```rust
//! use edgevec::simd::{capabilities, warn_if_suboptimal, SimdCapabilities};
//!
//! // Get detected capabilities
//! let caps = capabilities();
//! println!("AVX2 available: {}", caps.avx2);
//!
//! // Check if configuration is optimal
//! if !caps.is_optimal() {
//!     warn_if_suboptimal();
//! }
//! ```

use std::sync::OnceLock;

/// SIMD capabilities detected at runtime
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct SimdCapabilities {
    /// AVX2 (256-bit vectors) available
    pub avx2: bool,
    /// FMA (fused multiply-add) available
    pub fma: bool,
    /// SSE 4.2 available
    pub sse42: bool,
    /// NEON (ARM) available
    pub neon: bool,
}

impl SimdCapabilities {
    /// Detect SIMD capabilities for current CPU
    #[must_use]
    pub fn detect() -> Self {
        // x86_64 (64-bit Intel/AMD)
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                avx2: is_x86_feature_detected!("avx2"),
                fma: is_x86_feature_detected!("fma"),
                sse42: is_x86_feature_detected!("sse4.2"),
                neon: false,
            }
        }

        // x86 (32-bit) - [FIX M1: Added for completeness]
        #[cfg(target_arch = "x86")]
        {
            Self {
                avx2: is_x86_feature_detected!("avx2"),
                fma: is_x86_feature_detected!("fma"),
                sse42: is_x86_feature_detected!("sse4.2"),
                neon: false,
            }
        }

        // aarch64 (ARM 64-bit)
        // NOTE: std::arch::is_aarch64_feature_detected! stable since Rust 1.61 (MSRV 1.70 OK)
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                avx2: false,
                fma: false,
                sse42: false,
                neon: std::arch::is_aarch64_feature_detected!("neon"),
            }
        }

        // WASM (compile-time SIMD, not runtime detectable)
        #[cfg(target_arch = "wasm32")]
        {
            Self::default()
        }

        // Fallback for all other architectures
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "wasm32"
        )))]
        {
            Self::default()
        }
    }

    /// Check if optimal performance features are available
    #[must_use]
    pub fn is_optimal(&self) -> bool {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            self.avx2 && self.fma
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.neon
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
        {
            true
        } // No expectations for other platforms
    }

    /// Get human-readable performance warning if suboptimal
    #[must_use]
    pub fn performance_warning(&self) -> Option<String> {
        if self.is_optimal() {
            return None;
        }

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            let mut missing = Vec::new();
            if !self.avx2 {
                missing.push("AVX2");
            }
            if !self.fma {
                missing.push("FMA");
            }

            Some(format!(
                "EdgeVec: Suboptimal SIMD configuration detected. Missing: {}. \
                 Expected 60-78% performance loss. \
                 Add `rustflags = [\"-C\", \"target-cpu=native\"]` to .cargo/config.toml",
                missing.join(", ")
            ))
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            None
        }
    }
}

/// Global cached capabilities (detected once)
static CAPABILITIES: OnceLock<SimdCapabilities> = OnceLock::new();

/// Get cached SIMD capabilities
///
/// This function detects SIMD capabilities once and caches the result.
/// Subsequent calls return the cached value.
///
/// # Example
///
/// ```rust
/// use edgevec::simd::capabilities;
///
/// let caps = capabilities();
/// println!("AVX2: {}, FMA: {}", caps.avx2, caps.fma);
/// ```
#[must_use]
pub fn capabilities() -> &'static SimdCapabilities {
    CAPABILITIES.get_or_init(SimdCapabilities::detect)
}

/// Log performance warning if SIMD is suboptimal
///
/// Call this at library initialization to warn users about
/// potential performance issues due to missing SIMD features.
///
/// # Example
///
/// ```rust
/// use edgevec::simd::warn_if_suboptimal;
///
/// // Call at application startup
/// warn_if_suboptimal();
/// ```
pub fn warn_if_suboptimal() {
    if let Some(warning) = capabilities().performance_warning() {
        // Print to stderr for all users
        eprintln!("{warning}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid_capabilities() {
        let caps = SimdCapabilities::detect();
        // Should not panic and should return a valid struct
        // We can't assert specific values since they depend on the CPU
        let _ = caps.avx2;
        let _ = caps.fma;
        let _ = caps.sse42;
        let _ = caps.neon;
    }

    #[test]
    fn test_default_all_false() {
        let caps = SimdCapabilities::default();
        assert!(!caps.avx2);
        assert!(!caps.fma);
        assert!(!caps.sse42);
        assert!(!caps.neon);
    }

    #[test]
    fn test_capabilities_returns_same_instance() {
        let caps1 = capabilities();
        let caps2 = capabilities();
        // Should return the same cached instance
        assert!(std::ptr::eq(caps1, caps2));
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_is_optimal_with_full_x86_features() {
        let caps = SimdCapabilities {
            avx2: true,
            fma: true,
            sse42: true,
            neon: false,
        };
        assert!(caps.is_optimal());
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_is_optimal_missing_avx2() {
        let caps = SimdCapabilities {
            avx2: false,
            fma: true,
            sse42: true,
            neon: false,
        };
        assert!(!caps.is_optimal());
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_is_optimal_missing_fma() {
        let caps = SimdCapabilities {
            avx2: true,
            fma: false,
            sse42: true,
            neon: false,
        };
        assert!(!caps.is_optimal());
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_performance_warning_none_when_optimal() {
        let caps = SimdCapabilities {
            avx2: true,
            fma: true,
            sse42: true,
            neon: false,
        };
        assert!(caps.performance_warning().is_none());
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_performance_warning_present_when_suboptimal() {
        let caps = SimdCapabilities {
            avx2: false,
            fma: false,
            sse42: true,
            neon: false,
        };
        let warning = caps.performance_warning();
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("AVX2"));
        assert!(msg.contains("FMA"));
        assert!(msg.contains("60-78%"));
        assert!(msg.contains("target-cpu=native"));
    }

    #[test]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    fn test_performance_warning_missing_only_avx2() {
        let caps = SimdCapabilities {
            avx2: false,
            fma: true,
            sse42: true,
            neon: false,
        };
        let warning = caps.performance_warning();
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("AVX2"));
        assert!(!msg.contains("FMA"));
    }

    #[test]
    fn test_warn_if_suboptimal_does_not_panic() {
        // Should not panic regardless of CPU features
        warn_if_suboptimal();
    }

    #[test]
    fn test_simd_capabilities_clone() {
        let caps = SimdCapabilities {
            avx2: true,
            fma: false,
            sse42: true,
            neon: false,
        };
        let cloned = caps;
        assert_eq!(caps, cloned);
    }

    #[test]
    fn test_simd_capabilities_debug() {
        let caps = SimdCapabilities::default();
        let debug_str = format!("{caps:?}");
        assert!(debug_str.contains("SimdCapabilities"));
        assert!(debug_str.contains("avx2"));
        assert!(debug_str.contains("fma"));
    }
}
