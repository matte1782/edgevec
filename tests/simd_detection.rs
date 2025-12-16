//! SIMD Detection Integration Tests
//!
//! Tests for runtime SIMD feature detection across all platforms.
//! These tests verify that detection works correctly on both x86 and ARM64 CI.

use edgevec::simd::{capabilities, detect_neon, select_backend, SimdBackend, SimdCapabilities};

#[test]
fn test_neon_detection_returns_bool() {
    // Should not panic on any platform - verify function executes without panic
    let _ = detect_neon();
}

#[test]
#[cfg(target_arch = "aarch64")]
fn test_neon_detected_on_arm64() {
    // Most ARM64 CPUs have NEON (mandatory since ARMv8)
    // This may fail on very old ARM64 without NEON (extremely rare)
    assert!(detect_neon(), "Expected NEON on ARM64");
}

#[test]
#[cfg(target_arch = "x86_64")]
fn test_neon_not_detected_on_x86() {
    assert!(!detect_neon(), "NEON should not be detected on x86");
}

#[test]
fn test_backend_selection_no_panic() {
    // Should not panic on any platform
    let backend = select_backend();
    println!("Selected backend: {:?}", backend);

    // Backend should be a valid variant
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
#[cfg(target_arch = "aarch64")]
fn test_arm64_selects_neon_or_portable() {
    let backend = select_backend();
    assert!(
        backend == SimdBackend::Neon || backend == SimdBackend::Portable,
        "ARM64 should select NEON or Portable, got {:?}",
        backend
    );
}

#[test]
#[cfg(target_arch = "x86_64")]
fn test_x86_never_selects_neon() {
    let backend = select_backend();
    assert_ne!(backend, SimdBackend::Neon, "x86 should never select NEON");
}

#[test]
fn test_capabilities_returns_valid_struct() {
    let caps = capabilities();

    // Should return a valid struct without panicking
    let _ = caps.avx2;
    let _ = caps.fma;
    let _ = caps.sse42;
    let _ = caps.neon;
}

#[test]
fn test_capabilities_cached() {
    let caps1 = capabilities();
    let caps2 = capabilities();

    // Should return the same cached instance
    assert!(std::ptr::eq(caps1, caps2));
}

#[test]
#[cfg(target_arch = "aarch64")]
fn test_arm64_capabilities_neon() {
    let caps = capabilities();

    // ARM64 should have NEON and not have x86 features
    assert!(caps.neon, "ARM64 should have NEON");
    assert!(!caps.avx2, "ARM64 should not have AVX2");
    assert!(!caps.sse42, "ARM64 should not have SSE4.2");
}

#[test]
#[cfg(target_arch = "x86_64")]
fn test_x86_capabilities_no_neon() {
    let caps = capabilities();

    // x86 should not have NEON
    assert!(!caps.neon, "x86 should not have NEON");
}

#[test]
fn test_is_optimal_consistent_with_backend() {
    let caps = capabilities();
    let backend = select_backend();

    // If we have an optimal configuration, backend should be SIMD-accelerated
    if caps.is_optimal() {
        assert!(backend.is_simd(), "Optimal config should use SIMD backend");
    }
}

#[test]
fn test_simd_backend_properties() {
    // Test all backend variants
    assert_eq!(SimdBackend::Avx2.name(), "AVX2");
    assert_eq!(SimdBackend::Avx.name(), "AVX");
    assert_eq!(SimdBackend::Sse41.name(), "SSE4.1");
    assert_eq!(SimdBackend::Neon.name(), "NEON");
    assert_eq!(SimdBackend::Portable.name(), "Portable");

    assert!(SimdBackend::Avx2.is_simd());
    assert!(SimdBackend::Avx.is_simd());
    assert!(SimdBackend::Sse41.is_simd());
    assert!(SimdBackend::Neon.is_simd());
    assert!(!SimdBackend::Portable.is_simd());

    // Vector widths
    assert_eq!(SimdBackend::Avx2.vector_width(), 256);
    assert_eq!(SimdBackend::Avx.vector_width(), 256);
    assert_eq!(SimdBackend::Sse41.vector_width(), 128);
    assert_eq!(SimdBackend::Neon.vector_width(), 128);
    assert_eq!(SimdBackend::Portable.vector_width(), 0);

    // Display trait
    assert_eq!(format!("{}", SimdBackend::Avx2), "AVX2");
    assert_eq!(format!("{}", SimdBackend::Avx), "AVX");
    assert_eq!(format!("{}", SimdBackend::Sse41), "SSE4.1");
    assert_eq!(format!("{}", SimdBackend::Neon), "NEON");
    assert_eq!(format!("{}", SimdBackend::Portable), "Portable");
}

#[test]
fn test_simd_backend_equality() {
    assert_eq!(SimdBackend::Avx2, SimdBackend::Avx2);
    assert_eq!(SimdBackend::Neon, SimdBackend::Neon);
    assert_eq!(SimdBackend::Portable, SimdBackend::Portable);

    assert_ne!(SimdBackend::Avx2, SimdBackend::Neon);
    assert_ne!(SimdBackend::Avx2, SimdBackend::Portable);
    assert_ne!(SimdBackend::Neon, SimdBackend::Portable);
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
    assert!(set.contains(&SimdBackend::Avx2));
    assert!(set.contains(&SimdBackend::Avx));
    assert!(set.contains(&SimdBackend::Sse41));
    assert!(set.contains(&SimdBackend::Neon));
    assert!(set.contains(&SimdBackend::Portable));
}

#[test]
fn test_simd_capabilities_default() {
    let caps = SimdCapabilities::default();

    assert!(!caps.avx2);
    assert!(!caps.fma);
    assert!(!caps.sse42);
    assert!(!caps.neon);
}

#[test]
fn test_simd_capabilities_detect_consistent() {
    let caps1 = SimdCapabilities::detect();
    let caps2 = SimdCapabilities::detect();

    // Detection should be consistent
    assert_eq!(caps1, caps2);
}
