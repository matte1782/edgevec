use edgevec::metric::{DotProduct, Hamming, L2Squared, Metric};

#[test]
fn test_l2_squared_basic() {
    let a = [1.0, 2.0, 3.0];
    let b = [4.0, 6.0, 8.0];
    // (1-4)^2 + (2-6)^2 + (3-8)^2
    // 9 + 16 + 25 = 50
    assert_eq!(L2Squared::distance(&a, &b), 50.0);
}

#[test]
fn test_l2_squared_zero() {
    let a = [0.0; 10];
    let b = [0.0; 10];
    assert_eq!(L2Squared::distance(&a, &b), 0.0);
}

#[test]
#[should_panic(expected = "dimension mismatch")]
fn test_l2_squared_mismatch() {
    let a = [1.0, 2.0];
    let b = [1.0, 2.0, 3.0];
    L2Squared::distance(&a, &b);
}

#[test]
#[should_panic(expected = "NaN detected")]
fn test_l2_squared_nan() {
    let a = [f32::NAN, 1.0];
    let b = [0.0, 0.0];
    L2Squared::distance(&a, &b);
}

#[test]
fn test_dot_product_basic() {
    let a = [1.0, 2.0, 3.0];
    let b = [4.0, 5.0, 6.0];
    // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
    // DotProduct::distance returns 1.0 - dot_product (lower = closer)
    assert_eq!(DotProduct::distance(&a, &b), 1.0 - 32.0);
}

#[test]
#[should_panic(expected = "dimension mismatch")]
fn test_dot_product_mismatch() {
    let a = [1.0];
    let b = [1.0, 2.0];
    DotProduct::distance(&a, &b);
}

#[test]
fn test_hamming_basic() {
    // 0b1010_1010 (0xAA)
    // 0b0101_0101 (0x55)
    // XOR = 0xFF (all ones), 8 bits diff per byte
    let a = [0xAA, 0xAA];
    let b = [0x55, 0x55];
    assert_eq!(Hamming::distance(&a, &b), 16.0);
}

#[test]
fn test_hamming_exact_match() {
    let a = [0xFF, 0x00];
    assert_eq!(Hamming::distance(&a, &a), 0.0);
}

#[test]
#[should_panic(expected = "dimension mismatch")]
fn test_hamming_mismatch() {
    let a = [0x00];
    let b = [0x00, 0x00];
    Hamming::distance(&a, &b);
}
