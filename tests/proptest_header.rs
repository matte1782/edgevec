use edgevec::persistence::{read_file_header, FileHeader, PersistenceError};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// PROP-PERSIST-001: Round-trip serialization
    #[test]
    fn test_header_roundtrip(
        dimensions in 1u32..10000,
        vector_count in 0u64..1_000_000,
        _rng_seed in any::<u64>(),
        hnsw_m in any::<u32>(),
        hnsw_m0 in any::<u32>(),
    ) {
        let mut header_struct = FileHeader::new(dimensions);
        header_struct.vector_count = vector_count;
        header_struct.hnsw_m = hnsw_m;
        header_struct.hnsw_m0 = hnsw_m0;
        header_struct.update_checksum();

        let bytes = header_struct.as_bytes();
        let decoded_result = read_file_header(bytes);

        prop_assert!(decoded_result.is_ok(), "Failed to decode valid header: {:?}", decoded_result.err());

        let decoded = decoded_result.unwrap();

        prop_assert_eq!(decoded.magic, FileHeader::MAGIC);
        prop_assert_eq!(decoded.version_major, FileHeader::VERSION_MAJOR);
        prop_assert_eq!(decoded.version_minor, FileHeader::VERSION_MINOR);
        prop_assert_eq!(decoded.vector_count, vector_count);
        prop_assert_eq!(decoded.dimensions, dimensions);
        prop_assert_eq!(decoded.hnsw_m, hnsw_m);
        prop_assert_eq!(decoded.hnsw_m0, hnsw_m0);
    }

    /// PROP-PERSIST-002: Invalid magic must be rejected.
    #[test]
    fn test_invalid_magic_rejected(
        bad_magic in prop::array::uniform4(any::<u8>()),
        dimensions in 1u32..10000,
    ) {
        prop_assume!(bad_magic != FileHeader::MAGIC);

        let header = FileHeader::new(dimensions);
        let mut bytes = header.as_bytes().to_vec();

        // Corrupt magic
        bytes[0..4].copy_from_slice(&bad_magic);

        let result = read_file_header(&bytes);

        let is_match = matches!(result, Err(PersistenceError::InvalidMagic { .. }));
        prop_assert!(is_match);
    }

    /// PROP-PERSIST-003: Unsupported version must be rejected.
    #[test]
    fn test_unsupported_version_rejected(
        other_version in any::<u8>(),
        dimensions in 1u32..10000,
    ) {
        prop_assume!(other_version != FileHeader::VERSION_MAJOR);

        let header = FileHeader::new(dimensions);
        let mut bytes = header.as_bytes().to_vec();

        // Corrupt version
        bytes[4] = other_version;

        // Fix CRC
        let mut hasher = crc32fast::Hasher::new();
        let mut crc_bytes = bytes.clone();
        crc_bytes[44..48].fill(0);
        hasher.update(&crc_bytes);
        let new_crc = hasher.finalize();
        bytes[44..48].copy_from_slice(&new_crc.to_le_bytes());

        let result = read_file_header(&bytes);

        let is_match = matches!(result, Err(PersistenceError::UnsupportedVersion(maj, _)) if maj == other_version);
        prop_assert!(is_match);
    }

    /// PROP-PERSIST-004: Corrupted checksum must be rejected.
    #[test]
    fn test_corrupted_checksum_rejected(
        dimensions in 1u32..10000,
    ) {
        let header = FileHeader::new(dimensions);
        let mut bytes = header.as_bytes().to_vec();

        // Flip a byte in the reserved field
        bytes[60] ^= 0xFF;

        let result = read_file_header(&bytes);
        let is_checksum_error = matches!(result, Err(PersistenceError::ChecksumMismatch { .. }));
        prop_assert!(is_checksum_error);
    }

    /// PROP-PERSIST-005: Short buffers must be rejected.
    #[test]
    fn test_short_buffer_rejected(data in prop::collection::vec(any::<u8>(), 0..64)) {
        prop_assume!(data.len() < 64);
        let result = read_file_header(&data);
        // Extract length to avoid closure reference issues in macro if any
        let len = data.len();
        let is_match = matches!(result, Err(PersistenceError::BufferTooSmall { expected: 64, actual }) if actual == len);
        prop_assert!(is_match);
    }
}
