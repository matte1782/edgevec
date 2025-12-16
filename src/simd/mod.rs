//! SIMD capability detection and runtime optimization.
//!
//! This module provides runtime detection of CPU SIMD features
//! to enable performance warnings and feature-appropriate code paths.
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
//!     // Warn user about performance impact
//!     warn_if_suboptimal();
//! }
//! ```
//!
//! # Backend Selection
//!
//! ```rust
//! use edgevec::simd::{select_backend, SimdBackend};
//!
//! let backend = select_backend();
//! match backend {
//!     SimdBackend::Avx2 => println!("Using AVX2 (256-bit)"),
//!     SimdBackend::Avx => println!("Using AVX (256-bit)"),
//!     SimdBackend::Sse41 => println!("Using SSE4.1 (128-bit)"),
//!     SimdBackend::Neon => println!("Using NEON (128-bit)"),
//!     SimdBackend::Portable => println!("Using portable fallback"),
//! }
//! ```

pub mod detect;

// NEON module - conditionally compiled for ARM64
#[cfg(target_arch = "aarch64")]
pub mod neon;

pub use detect::{capabilities, warn_if_suboptimal, SimdCapabilities};

/// Available SIMD backends for runtime dispatch.
///
/// The backend is selected at runtime based on CPU feature detection.
/// Priority order: AVX2 > AVX > SSE4.1 > NEON > Portable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimdBackend {
    /// AVX2 (256-bit vectors) - x86_64 only
    Avx2,
    /// AVX (256-bit vectors, no integer ops) - x86_64 only
    Avx,
    /// SSE4.1 (128-bit vectors) - x86_64 only
    Sse41,
    /// ARM NEON (128-bit vectors) - aarch64 only
    Neon,
    /// Portable fallback - all platforms
    Portable,
}

impl SimdBackend {
    /// Returns a human-readable name for this backend.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Avx2 => "AVX2",
            Self::Avx => "AVX",
            Self::Sse41 => "SSE4.1",
            Self::Neon => "NEON",
            Self::Portable => "Portable",
        }
    }

    /// Returns true if this is a SIMD-accelerated backend.
    #[must_use]
    pub fn is_simd(&self) -> bool {
        !matches!(self, Self::Portable)
    }

    /// Returns the vector width in bits for this backend.
    #[must_use]
    pub fn vector_width(&self) -> usize {
        match self {
            Self::Avx2 | Self::Avx => 256,
            Self::Sse41 | Self::Neon => 128,
            Self::Portable => 0,
        }
    }
}

impl std::fmt::Display for SimdBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Detect NEON SIMD support at runtime.
///
/// Returns `true` on ARM64 platforms with NEON support (most ARM64 CPUs).
/// Returns `false` on all other platforms.
///
/// # Example
///
/// ```rust
/// use edgevec::simd::detect_neon;
///
/// if detect_neon() {
///     println!("NEON SIMD available!");
/// }
/// ```
#[inline]
#[must_use]
pub fn detect_neon() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("neon")
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        false // NEON only exists on ARM
    }
}

/// Select the best available SIMD backend for the current CPU.
///
/// This function performs runtime detection and returns the fastest
/// available backend:
///
/// - **x86_64 with AVX2**: Returns `SimdBackend::Avx2`
/// - **aarch64 with NEON**: Returns `SimdBackend::Neon`
/// - **All other platforms**: Returns `SimdBackend::Portable`
///
/// # Example
///
/// ```rust
/// use edgevec::simd::{select_backend, SimdBackend};
///
/// let backend = select_backend();
/// println!("Using {} backend", backend);
/// ```
#[must_use]
pub fn select_backend() -> SimdBackend {
    #[cfg(target_arch = "x86_64")]
    {
        // Priority: AVX2 > AVX > SSE4.1 > Portable
        if is_x86_feature_detected!("avx2") {
            return SimdBackend::Avx2;
        }
        if is_x86_feature_detected!("avx") {
            return SimdBackend::Avx;
        }
        if is_x86_feature_detected!("sse4.1") {
            return SimdBackend::Sse41;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if detect_neon() {
            return SimdBackend::Neon;
        }
    }

    SimdBackend::Portable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_neon_returns_bool() {
        // Should not panic on any platform - verify function executes without panic
        let _ = detect_neon();
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_neon_not_detected_on_x86() {
        assert!(!detect_neon(), "NEON should not be detected on x86");
    }

    #[test]
    fn test_select_backend_no_panic() {
        // Should not panic on any platform
        let backend = select_backend();
        println!("Selected backend: {:?}", backend);
        // Backend should be one of the valid variants
        assert!(matches!(
            backend,
            SimdBackend::Avx2
                | SimdBackend::Avx
                | SimdBackend::Sse41
                | SimdBackend::Neon
                | SimdBackend::Portable
        ));
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_x86_does_not_select_neon() {
        let backend = select_backend();
        assert_ne!(backend, SimdBackend::Neon, "x86 should never select NEON");
    }

    #[test]
    fn test_simd_backend_name() {
        assert_eq!(SimdBackend::Avx2.name(), "AVX2");
        assert_eq!(SimdBackend::Avx.name(), "AVX");
        assert_eq!(SimdBackend::Sse41.name(), "SSE4.1");
        assert_eq!(SimdBackend::Neon.name(), "NEON");
        assert_eq!(SimdBackend::Portable.name(), "Portable");
    }

    #[test]
    fn test_simd_backend_is_simd() {
        assert!(SimdBackend::Avx2.is_simd());
        assert!(SimdBackend::Avx.is_simd());
        assert!(SimdBackend::Sse41.is_simd());
        assert!(SimdBackend::Neon.is_simd());
        assert!(!SimdBackend::Portable.is_simd());
    }

    #[test]
    fn test_simd_backend_vector_width() {
        assert_eq!(SimdBackend::Avx2.vector_width(), 256);
        assert_eq!(SimdBackend::Avx.vector_width(), 256);
        assert_eq!(SimdBackend::Sse41.vector_width(), 128);
        assert_eq!(SimdBackend::Neon.vector_width(), 128);
        assert_eq!(SimdBackend::Portable.vector_width(), 0);
    }

    #[test]
    fn test_simd_backend_display() {
        assert_eq!(format!("{}", SimdBackend::Avx2), "AVX2");
        assert_eq!(format!("{}", SimdBackend::Avx), "AVX");
        assert_eq!(format!("{}", SimdBackend::Sse41), "SSE4.1");
        assert_eq!(format!("{}", SimdBackend::Neon), "NEON");
        assert_eq!(format!("{}", SimdBackend::Portable), "Portable");
    }

    #[test]
    fn test_simd_backend_clone() {
        let backend = SimdBackend::Avx2;
        let cloned = backend;
        assert_eq!(backend, cloned);
    }

    #[test]
    fn test_simd_backend_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SimdBackend::Avx2);
        set.insert(SimdBackend::Avx);
        set.insert(SimdBackend::Sse41);
        set.insert(SimdBackend::Neon);
        set.insert(SimdBackend::Portable);
        assert_eq!(set.len(), 5);
    }
}
