use edgevec::metric::{DotProduct, Hamming, L2Squared, Metric};
use proptest::prelude::*;

// Nvidia Grade float assertion epsilon
const EPSILON: f32 = 1e-5;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// PROP-DIST-005: SIMD Equivalence
    /// Verifies that the implementation (potentially SIMD) matches the scalar reference.
    #[test]
    fn prop_simd_equivalence(
        a in prop::collection::vec(-1.0e5_f32..1.0e5_f32, 1..128),
        b in prop::collection::vec(-1.0e5_f32..1.0e5_f32, 1..128)
    ) {
        let min_len = std::cmp::min(a.len(), b.len());
        let a = &a[..min_len];
        let b = &b[..min_len];

        // Oracle: Scalar Reference (Canonical Implementation)
        // We compute this locally to verify the library implementation (which might be SIMD)
        let oracle_l2: f32 = a.iter().zip(b.iter())
            .map(|(x, y)| {
                let diff = x - y;
                diff * diff
            })
            .sum();

        let oracle_dot_raw: f32 = a.iter().zip(b.iter())
            .map(|(x, y)| x * y)
            .sum();
        // DotProduct::distance returns 1.0 - dot_product (cosine distance for normalized vectors)
        let oracle_dot = 1.0 - oracle_dot_raw;

        // Implementation Under Test
        let impl_l2 = L2Squared::distance(a, b);
        let impl_dot = DotProduct::distance(a, b);

        // Verification
        // Use relative tolerance for large numbers
        let l2_tol = EPSILON.max(oracle_l2.abs() * 1e-4);
        prop_assert!((impl_l2 - oracle_l2).abs() <= l2_tol,
            "L2 Mismatch: Impl={}, Oracle={}, Tol={}", impl_l2, oracle_l2, l2_tol);

        let dot_tol = EPSILON.max(oracle_dot_raw.abs() * 1e-4);
        prop_assert!((impl_dot - oracle_dot).abs() <= dot_tol,
            "Dot Mismatch: Impl={}, Oracle={}, Tol={}", impl_dot, oracle_dot, dot_tol);
    }

    /// PROP-DIST-001: L2 Symmetry
    /// d(a,b) == d(b,a)
    #[test]
    fn prop_l2_symmetry(
        a in prop::collection::vec(any::<f32>(), 1..128),
        b in prop::collection::vec(any::<f32>(), 1..128)
    ) {
        // Ensure equal lengths for validity
        let min_len = std::cmp::min(a.len(), b.len());
        let a = &a[..min_len];
        let b = &b[..min_len];

        let d1 = L2Squared::distance(a, b);
        let d2 = L2Squared::distance(b, a);

        if d1.is_infinite() && d2.is_infinite() {
             // Both infinite is valid symmetry
             return Ok(());
        }

        prop_assert!((d1 - d2).abs() < EPSILON, "Symmetry violation: {} != {}", d1, d2);
    }

    /// PROP-DIST-002: L2 Identity
    /// d(a,a) == 0
    #[test]
    fn prop_l2_identity(
        a in prop::collection::vec(any::<f32>(), 1..128)
    ) {
        let d = L2Squared::distance(&a, &a);
        prop_assert!(d < EPSILON, "Identity violation: {} != 0", d);
        prop_assert!(d >= 0.0, "Non-negativity violation: {} < 0", d);
    }

    /// PROP-DIST-003: Triangle Inequality (Approximate for Squared L2)
    /// Note: L2 Squared does NOT satisfy triangle inequality directly.
    /// d^2(a,c) <= (d(a,b) + d(b,c))^2
    /// We verify this relationship using sqrt(L2Squared) for the check.
    #[test]
    fn prop_l2_triangle_inequality(
        a in prop::collection::vec(-1.0e10_f32..1.0e10_f32, 1..64),
        b in prop::collection::vec(-1.0e10_f32..1.0e10_f32, 1..64),
        c in prop::collection::vec(-1.0e10_f32..1.0e10_f32, 1..64)
    ) {
        let len = a.len().min(b.len()).min(c.len());
        let a = &a[..len];
        let b = &b[..len];
        let c = &c[..len];

        let dist_ab = L2Squared::distance(a, b).sqrt();
        let dist_bc = L2Squared::distance(b, c).sqrt();
        let dist_ac = L2Squared::distance(a, c).sqrt();

        // Dynamic epsilon for large numbers
        let max_val = dist_ab.max(dist_bc).max(dist_ac);
        let tolerance = EPSILON.max(max_val * 1e-5);

        // d(a,c) <= d(a,b) + d(b,c) + tolerance
        prop_assert!(
            dist_ac <= dist_ab + dist_bc + tolerance,
            "Triangle inequality violation: {} > {} + {} (tol: {})",
            dist_ac,
            dist_ab,
            dist_bc,
            tolerance
        );
    }

    /// PROP-DIST-004: Hamming Bounds
    /// 0 <= d(a,b) <= bits(a)
    #[test]
    fn prop_hamming_bounds(
        a in prop::collection::vec(any::<u8>(), 1..128),
        b in prop::collection::vec(any::<u8>(), 1..128)
    ) {
        let len = a.len().min(b.len());
        let a = &a[..len];
        let b = &b[..len];

        let d = Hamming::distance(a, b);
        let max_dist = (len * 8) as f32;

        prop_assert!(d >= 0.0);
        prop_assert!(d <= max_dist);

        // Verify integer nature of Hamming distance
        prop_assert!((d.fract()).abs() < EPSILON, "Hamming distance should be integer");
    }
}

/// Stability Checks (Explicit edge cases)
#[test]
fn test_l2_stability_inf() {
    let a = [f32::INFINITY];
    let b = [0.0];
    // Inf - 0 = Inf, Inf^2 = Inf
    let d = L2Squared::distance(&a, &b);
    assert_eq!(d, f32::INFINITY);
}

#[test]
#[should_panic(expected = "NaN detected")]
fn test_l2_stability_nan_panic() {
    let a = [f32::NAN];
    let b = [0.0];
    L2Squared::distance(&a, &b);
}
