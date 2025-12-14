//! Scalar implementations for distance metrics.

/// L2 Squared distance for u8 vectors (Scalar fallback).
///
/// # Panics
///
/// Panics if `a` and `b` have different lengths.
#[inline]
#[must_use]
pub fn l2_squared_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    let mut sum: u32 = 0;
    for i in 0..a.len() {
        // Safe upcast to i32 to avoid overflow during subtraction
        let diff = i32::from(a[i]) - i32::from(b[i]);
        // SAFETY: diff*diff is always non-negative, so cast to u32 is safe
        #[allow(clippy::cast_sign_loss)]
        let sq = (diff * diff) as u32;
        sum += sq;
    }
    sum
}

/// Dot Product for u8 vectors (Scalar fallback).
///
/// # Panics
///
/// Panics if `a` and `b` have different lengths.
#[inline]
#[must_use]
pub fn dot_product_u8(a: &[u8], b: &[u8]) -> u32 {
    assert_eq!(a.len(), b.len());
    let mut sum: u32 = 0;
    for i in 0..a.len() {
        sum += u32::from(a[i]) * u32::from(b[i]);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_squared_u8_scalar() {
        let a = vec![1, 2, 3];
        let b = vec![4, 2, 1];
        // (1-4)^2 + (2-2)^2 + (3-1)^2 = 9 + 0 + 4 = 13
        assert_eq!(l2_squared_u8(&a, &b), 13);
    }

    #[test]
    fn test_dot_product_u8_scalar() {
        let a = vec![1, 2, 3];
        let b = vec![4, 2, 1];
        // 1*4 + 2*2 + 3*1 = 4 + 4 + 3 = 11
        assert_eq!(dot_product_u8(&a, &b), 11);
    }

    #[test]
    fn test_overflow_protection() {
        // 255 * 255 = 65025.
        // 100 dimensions -> 6,502,500. Fits in u32 (4B).
        // 100k dimensions -> 6.5e9. Overflow u32 (4.29e9).
        // Max u32 is 4,294,967,295.
        // 65025 * N < 4,294,967,295 => N < 66050.
        // So for vectors longer than ~66k, we might overflow u32.
        // But HNSW usually uses 128-2048 dims. So safe.
        // Constraint in prompt: "Overflow: Result fits in u32 (max 50M < 4B)".
        // 50M refers to max possible value?
        // If dims=2048 (large), max val = 2048 * 255^2 = 133M. Fits comfortably.

        let n = 1000;
        let a = vec![255; n];
        let b = vec![0; n];
        // 255^2 * 1000 = 65,025,000
        assert_eq!(l2_squared_u8(&a, &b), 65_025_000);
    }
}
