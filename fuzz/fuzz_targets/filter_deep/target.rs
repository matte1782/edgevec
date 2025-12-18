#![no_main]
//! FUZZ-012: Filter parser fuzzing for deeply nested expressions.
//!
//! Tests that the filter parser handles deeply nested expressions without
//! stack overflow or excessive resource consumption. Uses structured input
//! generation to create nested AND/OR expressions with max depth 50.

use edgevec::filter::parse;
use libfuzzer_sys::fuzz_target;

/// Maximum nesting depth to test
const MAX_DEPTH: usize = 50;

/// Generate a deeply nested filter expression from fuzzer input.
///
/// Interprets bytes as:
/// - Bits 0-1: Operator (0=AND, 1=OR, 2=NOT, 3=leaf)
/// - Bits 2-7: Field/value selector
fn generate_nested_filter(data: &[u8], max_depth: usize) -> String {
    if data.is_empty() || max_depth == 0 {
        // Base case: simple leaf expression
        return "x = 1".to_string();
    }

    let byte = data[0];
    let op = byte & 0b11; // First 2 bits for operator
    let selector = (byte >> 2) & 0b111111; // Next 6 bits for selection

    match op {
        0 => {
            // AND: (left AND right)
            let mid = data.len() / 2;
            let (left_data, right_data) = if data.len() > 1 {
                (&data[1..mid.max(1)], &data[mid..])
            } else {
                (&[][..], &[][..])
            };
            let left = generate_nested_filter(left_data, max_depth - 1);
            let right = generate_nested_filter(right_data, max_depth - 1);
            format!("({} AND {})", left, right)
        }
        1 => {
            // OR: (left OR right)
            let mid = data.len() / 2;
            let (left_data, right_data) = if data.len() > 1 {
                (&data[1..mid.max(1)], &data[mid..])
            } else {
                (&[][..], &[][..])
            };
            let left = generate_nested_filter(left_data, max_depth - 1);
            let right = generate_nested_filter(right_data, max_depth - 1);
            format!("({} OR {})", left, right)
        }
        2 => {
            // NOT: NOT(inner)
            let inner = if data.len() > 1 {
                generate_nested_filter(&data[1..], max_depth - 1)
            } else {
                "x = 1".to_string()
            };
            format!("NOT ({})", inner)
        }
        _ => {
            // Leaf expression: use selector to vary field/operator/value
            let field = match selector % 8 {
                0 => "a",
                1 => "b",
                2 => "c",
                3 => "x",
                4 => "y",
                5 => "count",
                6 => "status",
                _ => "val",
            };
            let operator = match (selector >> 3) % 8 {
                0 => "=",
                1 => "!=",
                2 => ">",
                3 => "<",
                4 => ">=",
                5 => "<=",
                6 => "IS NULL",
                _ => "IS NOT NULL",
            };
            if operator.starts_with("IS") {
                format!("{} {}", field, operator)
            } else {
                let value = selector as i32;
                format!("{} {} {}", field, operator, value)
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // FUZZ-012: parse(deeply_nested_filter) must return Result, never panic.
    // Generate structured nested filter from fuzzer input
    let filter = generate_nested_filter(data, MAX_DEPTH);

    // Attempt to parse - should not panic regardless of structure
    let _ = parse(&filter);
});
