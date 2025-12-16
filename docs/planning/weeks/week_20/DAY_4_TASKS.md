# Day 4: NEON Dot Product & Euclidean Distance

**Date:** 2025-12-26
**Theme:** Implement NEON-optimized similarity functions
**Estimated Hours:** 8
**Status:** PENDING
**Revision:** 2.0 (Post-Hostile-Review Fix)

---

## Objectives

1. Implement NEON dot product using intrinsics
2. Implement NEON euclidean distance using intrinsics
3. Create property tests for both functions
4. Benchmark both against portable implementations

---

## Dependencies

**Requires (BLOCKING):**
- W20.3 complete (NEON Hamming working, pattern established)
- ARM64 CI running

**Blocks:**
- W20.5 (Testing & Analysis) - Needs all NEON functions complete

---

## Tasks

### Task W20.4.1: NEON Dot Product Implementation

**Description:**
Implement NEON-optimized dot product for f32 vectors.

**Acceptance Criteria (ALL BINARY):**
1. [ ] Uses NEON intrinsics: `vld1q_f32`, `vmulq_f32`, `vfmaq_f32`, `vaddvq_f32`
2. [ ] Handles arbitrary input lengths (not just multiples of 4)
3. [ ] `|dot_product_neon(a, b) - dot_product_portable(a, b)| < 1e-6`
4. [ ] Contains safety comment explaining unsafe usage

**Implementation Details:**
- File: `src/simd/neon.rs`

```rust
/// NEON-optimized dot product
///
/// Computes the dot product of two f32 vectors.
///
/// # Safety
/// - Input slices must have the same length
/// - Uses unsafe NEON intrinsics (justified: read-only memory access)
///
/// # Precision
/// - May differ from portable by up to 1e-6 due to FMA operations
#[inline]
#[target_feature(enable = "neon")]
pub unsafe fn dot_product_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Slices must have equal length");

    let len = a.len();
    let chunks = len / 4;

    // Initialize accumulator to zero
    let mut sum = vdupq_n_f32(0.0);

    // Process 4 floats at a time
    for i in 0..chunks {
        let offset = i * 4;
        // SAFETY: offset + 4 <= len verified by chunks calculation
        let va = vld1q_f32(a.as_ptr().add(offset));
        let vb = vld1q_f32(b.as_ptr().add(offset));

        // Fused multiply-add: sum = sum + (va * vb)
        sum = vfmaq_f32(sum, va, vb);
    }

    // Horizontal sum of the vector
    let mut result = vaddvq_f32(sum);

    // Handle tail elements
    let tail_start = chunks * 4;
    for i in tail_start..len {
        result += a[i] * b[i];
    }

    result
}

/// Safe wrapper for NEON dot product
#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Slices must have equal length");
    unsafe { dot_product_neon_unchecked(a, b) }
}
```

**Test Requirements:**
- [ ] Compiles on ARM64 target
- [ ] Safety comment present

**Estimated Complexity:** 2h

**Risk Factors:**
- Risk: FMA precision differences
  Mitigation: Use epsilon comparison (1e-6), not exact equality

---

### Task W20.4.2: NEON Euclidean Distance Implementation

**Description:**
Implement NEON-optimized euclidean distance for f32 vectors.

**Acceptance Criteria (ALL BINARY):**
1. [ ] Uses NEON intrinsics for subtraction and squaring
2. [ ] Handles arbitrary input lengths
3. [ ] `|euclidean_neon(a, b) - euclidean_portable(a, b)| < 1e-6`
4. [ ] Uses standard library sqrt (not NEON approximation)

**Implementation Details:**
- File: `src/simd/neon.rs`

```rust
/// NEON-optimized euclidean distance
///
/// Computes sqrt(sum((a[i] - b[i])^2)) for f32 vectors.
///
/// # Safety
/// - Input slices must have the same length
/// - Uses unsafe NEON intrinsics
///
/// # Precision
/// - Uses std::f32::sqrt for final result (accurate)
/// - May differ from portable by up to 1e-6 due to FMA
#[inline]
#[target_feature(enable = "neon")]
pub unsafe fn euclidean_distance_neon_unchecked(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Slices must have equal length");

    let len = a.len();
    let chunks = len / 4;

    let mut sum_sq = vdupq_n_f32(0.0);

    for i in 0..chunks {
        let offset = i * 4;
        let va = vld1q_f32(a.as_ptr().add(offset));
        let vb = vld1q_f32(b.as_ptr().add(offset));

        // Compute difference
        let diff = vsubq_f32(va, vb);

        // Square and accumulate: sum_sq = sum_sq + (diff * diff)
        sum_sq = vfmaq_f32(sum_sq, diff, diff);
    }

    let mut result = vaddvq_f32(sum_sq);

    // Handle tail
    let tail_start = chunks * 4;
    for i in tail_start..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }

    result.sqrt()
}

/// Safe wrapper for NEON euclidean distance
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Slices must have equal length");
    unsafe { euclidean_distance_neon_unchecked(a, b) }
}
```

**Test Requirements:**
- [ ] Compiles on ARM64 target
- [ ] Safety comment present

**Estimated Complexity:** 2h

**Risk Factors:**
- Risk: sqrt precision
  Mitigation: Use std sqrt, not NEON reciprocal estimate

---

### Task W20.4.3: Property Tests for Dot Product & Euclidean

**Description:**
Create property tests proving NEON implementations match portable within epsilon.

**Acceptance Criteria (ALL BINARY):**
1. [ ] Property tests with 1000+ random inputs for each function
2. [ ] Tests cover: empty, 1, 3, 4, 5, 768, 1024 element vectors
3. [ ] All tests pass on ARM64 CI
4. [ ] Epsilon used: 1e-6 for f32 comparisons

**Implementation Details:**
- File: `tests/simd_neon_similarity.rs`

```rust
use proptest::prelude::*;

const EPSILON: f32 = 1e-6;

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < EPSILON || (a - b).abs() < EPSILON * a.abs().max(b.abs())
}

#[cfg(target_arch = "aarch64")]
mod neon_similarity_tests {
    use super::*;
    use edgevec::simd::{neon, portable};

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        #[test]
        fn dot_product_neon_matches_portable(
            a in prop::collection::vec(-1000.0f32..1000.0f32, 0..1024),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

            let neon_result = neon::dot_product(&a, &b);
            let portable_result = portable::dot_product(&a, &b);

            prop_assert!(
                approx_eq(neon_result, portable_result),
                "NEON ({}) != Portable ({}) for len={}, diff={}",
                neon_result, portable_result, a.len(), (neon_result - portable_result).abs()
            );
        }

        #[test]
        fn euclidean_neon_matches_portable(
            a in prop::collection::vec(-1000.0f32..1000.0f32, 0..1024),
        ) {
            let b: Vec<f32> = a.iter().map(|x| x + 0.5).collect();

            let neon_result = neon::euclidean_distance(&a, &b);
            let portable_result = portable::euclidean_distance(&a, &b);

            prop_assert!(
                approx_eq(neon_result, portable_result),
                "NEON ({}) != Portable ({}) for len={}, diff={}",
                neon_result, portable_result, a.len(), (neon_result - portable_result).abs()
            );
        }
    }

    #[test]
    fn test_dot_product_empty() {
        assert_eq!(neon::dot_product(&[], &[]), 0.0);
    }

    #[test]
    fn test_euclidean_identical() {
        let a = vec![1.0f32; 768];
        let b = a.clone();
        assert!(neon::euclidean_distance(&a, &b) < EPSILON);
    }

    #[test]
    fn test_dot_product_768_dims() {
        // OpenAI embedding dimension
        let a: Vec<f32> = (0..768).map(|i| (i as f32) * 0.001).collect();
        let b: Vec<f32> = (0..768).map(|i| ((768 - i) as f32) * 0.001).collect();

        let neon_result = neon::dot_product(&a, &b);
        let portable_result = portable::dot_product(&a, &b);

        assert!(approx_eq(neon_result, portable_result));
    }
}
```

**Test Requirements:**
- [ ] All property tests pass (1000 cases each)
- [ ] All edge case tests pass

**Estimated Complexity:** 2.5h

**Risk Factors:**
- Risk: Floating-point edge cases (NaN, Inf)
  Mitigation: Test with normal range values, document behavior

---

### Task W20.4.4: Similarity Benchmarks

**Description:**
Create benchmarks for dot product and euclidean distance.

**Acceptance Criteria (ALL BINARY):**
1. [ ] Benchmark both functions at 128, 768, 1536 dimensions
2. [ ] Results show NEON is faster (target: 2x)
3. [ ] Results documented

**Implementation Details:**
- Update `benches/simd_neon_bench.rs` to include similarity functions

**Test Requirements:**
- [ ] Benchmarks compile and run
- [ ] Results captured

**Estimated Complexity:** 1.5h

**Risk Factors:**
- Risk: NEON not faster for small vectors
  Mitigation: Document size threshold where NEON wins

---

## Daily Success Criteria

Day 4 is **COMPLETE** when:

1. [ ] NEON dot_product implemented with intrinsics
2. [ ] NEON euclidean_distance implemented with intrinsics
3. [ ] Property tests pass for both (1000+ cases each)
4. [ ] Edge case tests pass
5. [ ] Benchmark results documented
6. [ ] All unsafe code documented
7. [ ] ARM64 CI passes
8. [ ] x86 CI passes (no regressions)
9. [ ] Hostile review checkpoint passed

---

## Hostile Review Checkpoint

**End of Day 4 Review:**

**Artifacts to Review:**
- `src/simd/neon.rs` (updated with dot product and euclidean)
- `tests/simd_neon_similarity.rs` (property tests)
- `benches/simd_neon_bench.rs` (updated benchmarks)
- CI logs (ARM64 green)

**Review Criteria:**
- [ ] Implementations correct (tests pass)
- [ ] Epsilon comparisons appropriate
- [ ] Safety documentation complete
- [ ] Performance measured
- [ ] No x86 regressions

**Command:** `/review Day 4 NEON Similarity`

**If Review Fails:**
1. Address all critical issues same day
2. Resubmit for review
3. Do NOT proceed to Day 5 until approved

---

## Time Budget

| Task | Estimated | Buffer | Total |
|:-----|:----------|:-------|:------|
| W20.4.1 Dot Product | 1.5h | 0.5h | 2h |
| W20.4.2 Euclidean | 1.5h | 0.5h | 2h |
| W20.4.3 Property Tests | 2h | 0.5h | 2.5h |
| W20.4.4 Benchmarks | 1h | 0.5h | 1.5h |
| **TOTAL** | 6h | 2h | **8h** |

---

**Status:** PENDING
**Requires:** W20.3 (NEON Hamming) complete
**Blocks:** W20.5 (Testing & Analysis)
**Next:** DAY_5_TASKS.md (after hostile review approval)
