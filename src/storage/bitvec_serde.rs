//! Validating deserializer for `BitVec` (tombstone bits in `VectorStorage`).
//!
//! `bitvec` 1.0.1's own `Deserialize` trusts the serialized `bits` length and
//! calls `set_len` in an `unsafe` block: a crafted buffer can panic ("bit-vector
//! capacity exceeded") or expose uninitialized capacity, and its error path is a
//! `todo!()`. It also rejects data whose word width differs from the reader's
//! `usize`, so buffers written on wasm32 cannot be read on 64-bit hosts.
//!
//! This module reads the same wire format (serialization is unchanged and still
//! uses `bitvec`'s impl) and rebuilds the bit vector without `unsafe`:
//!
//! ```text
//! struct BitSeq { order: str, head: BitIdx { width: u8, index: u8 }, bits: u64, data: [word] }
//! ```

use bitvec::prelude::*;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(rename = "BitIdx")]
struct WireBitIdx {
    width: u8,
    index: u8,
}

#[derive(Deserialize)]
#[serde(rename = "BitSeq")]
struct WireBitSeq {
    order: String,
    head: WireBitIdx,
    bits: u64,
    data: Vec<u64>,
}

/// Deserializes a `BitVec<usize, Lsb0>` written by `bitvec`'s `Serialize`,
/// rejecting inconsistent lengths instead of panicking.
///
/// # Errors
///
/// Returns a deserialization error if the word width is not 8/16/32/64, the
/// head index is out of range, a word exceeds the width, or `bits` exceeds the
/// bits available in `data`.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BitVec, D::Error> {
    let wire = WireBitSeq::deserialize(deserializer)?;
    if wire.order != "bitvec::order::Lsb0" {
        return Err(D::Error::custom("unsupported bit-vector bit order"));
    }
    let width = u64::from(wire.head.width);
    if !matches!(width, 8 | 16 | 32 | 64) {
        return Err(D::Error::custom(format!(
            "invalid bit-vector word width {width}"
        )));
    }
    let head = u64::from(wire.head.index);
    if head >= width {
        return Err(D::Error::custom(format!(
            "bit-vector head {head} out of range for width {width}"
        )));
    }
    if width < 64 && wire.data.iter().any(|&w| w >> width != 0) {
        return Err(D::Error::custom("bit-vector word exceeds its width"));
    }
    let available = (wire.data.len() as u64).saturating_mul(width);
    let end = head
        .checked_add(wire.bits)
        .filter(|&end| end <= available)
        .ok_or_else(|| {
            D::Error::custom(format!(
                "bit-vector length {} exceeds {available} available bits",
                wire.bits
            ))
        })?;

    // BitVec has a stricter length ceiling than Vec (its pointer encodes the
    // bit head). Check it before with_capacity, including on 32-bit readers.
    let bits = usize::try_from(wire.bits)
        .ok()
        .filter(|&bits| bits <= BitSlice::<usize, Lsb0>::MAX_BITS)
        .ok_or_else(|| D::Error::custom("bit-vector length exceeds platform limit"))?;
    let mut out = BitVec::with_capacity(bits);
    for i in head..end {
        #[allow(clippy::cast_possible_truncation)]
        let word = wire.data[(i / width) as usize];
        out.push((word >> (i % width)) & 1 == 1);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Holder {
        #[serde(deserialize_with = "deserialize")]
        bits: BitVec,
    }

    fn sample(n: usize) -> BitVec {
        (0..n).map(|i| i % 3 == 0 || i % 7 == 1).collect()
    }

    #[test]
    fn roundtrips_bitvec_wire_format_postcard_and_json() {
        for n in [0, 1, 7, 63, 64, 65, 200] {
            let h = Holder { bits: sample(n) };
            let pc = postcard::to_stdvec(&h).unwrap();
            assert_eq!(
                postcard::from_bytes::<Holder>(&pc).unwrap(),
                h,
                "postcard n={n}"
            );
            let js = serde_json::to_string(&h).unwrap();
            assert_eq!(
                serde_json::from_str::<Holder>(&js).unwrap(),
                h,
                "json n={n}"
            );
        }
    }

    /// Mirror of the wire format to craft arbitrary (including hostile) inputs.
    #[derive(Serialize)]
    #[serde(rename = "BitSeq")]
    struct Craft {
        order: &'static str,
        head: CraftIdx,
        bits: u64,
        data: Vec<u64>,
    }
    #[derive(Serialize)]
    #[serde(rename = "BitIdx")]
    struct CraftIdx {
        width: u8,
        index: u8,
    }

    fn decode(c: &Craft) -> Result<BitVec, postcard::Error> {
        let bytes = postcard::to_stdvec(&(c,)).unwrap();
        postcard::from_bytes::<Holder>(&bytes).map(|h| h.bits)
    }

    fn craft(width: u8, index: u8, bits: u64, data: Vec<u64>) -> Craft {
        Craft {
            order: "bitvec::order::Lsb0",
            head: CraftIdx { width, index },
            bits,
            data,
        }
    }

    #[test]
    fn accepts_32_bit_words_written_on_wasm32() {
        let bv = decode(&craft(32, 0, 40, vec![0x8000_0001, 0x81])).unwrap();
        assert_eq!(bv.len(), 40);
        assert!(bv[0] && bv[31] && bv[32] && bv[39]);
        assert_eq!(bv.count_ones(), 4);
    }

    #[test]
    fn rejects_length_beyond_data_instead_of_panicking() {
        // The exact shape that makes bitvec's own impl panic.
        assert!(decode(&craft(64, 0, 127, vec![u64::MAX])).is_err());
        assert!(decode(&craft(64, 0, u64::MAX, vec![1])).is_err());
        assert!(decode(&craft(64, 1, 64, vec![1])).is_err());
        assert!(decode(&craft(64, 63, u64::MAX - 10, vec![1])).is_err());
    }

    #[test]
    fn rejects_bad_width_head_and_oversized_words() {
        assert!(decode(&craft(12, 0, 1, vec![1])).is_err());
        assert!(decode(&craft(32, 32, 0, vec![])).is_err());
        assert!(decode(&craft(32, 0, 1, vec![1 << 32])).is_err());
        let mut wrong_order = craft(32, 0, 1, vec![1]);
        wrong_order.order = "bitvec::order::Msb0";
        assert!(decode(&wrong_order).is_err());
    }

    #[test]
    fn honours_non_zero_head() {
        let bv = decode(&craft(8, 3, 3, vec![0b0010_1000])).unwrap();
        assert_eq!(
            bv.iter().by_vals().collect::<Vec<_>>(),
            vec![true, false, true]
        );
    }
}
