use edgevec::persistence::storage::file::FileBackend;
use edgevec::persistence::storage::{load_snapshot, StorageBackend};
use edgevec::persistence::{FileHeader, HeaderError, PersistenceError};
use proptest::prelude::*;
use tempfile::NamedTempFile;

fn create_valid_file(data_len: usize) -> (NamedTempFile, Vec<u8>) {
    let file = NamedTempFile::new().unwrap();
    let mut header = FileHeader::new(128);
    let data: Vec<u8> = (0..data_len).map(|i| (i % 255) as u8).collect();

    // Calculate CRC
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&data);
    header.data_crc = hasher.finalize();
    header.update_checksum();

    let mut buffer = Vec::with_capacity(64 + data.len());
    buffer.extend_from_slice(header.as_bytes());
    buffer.extend_from_slice(&data);

    let backend = FileBackend::new(file.path());
    backend.atomic_write("", &buffer).unwrap();
    (file, data)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Test 1: Bit Rot in Data Payload
    ///
    /// Corrupts a single bit in the data section and verifies that `load_snapshot()` returns `ChecksumMismatch`.
    #[test]
    fn test_bit_rot_data(
        data_len in 10usize..1000,
        bit_idx_seed in 0usize..10000
    ) {
        let (file, _data) = create_valid_file(data_len);
        let path = file.path();

        let mut file_bytes = std::fs::read(path).unwrap();
        // Target data section (offset 64+)
        let data_offset = 64;
        // Ensure we are within data bounds
        let target_byte = data_offset + (bit_idx_seed % data_len);
        let target_bit = (bit_idx_seed / data_len) % 8;

        // Flip bit
        file_bytes[target_byte] ^= 1 << target_bit;

        std::fs::write(path, &file_bytes).unwrap();

        let res = {
            let backend = FileBackend::new(path);
            load_snapshot(&backend)
        };
        match res {
            Err(PersistenceError::ChecksumMismatch { .. }) => {}, // Pass
            Err(e) => panic!("Expected ChecksumMismatch, got {:?}", e),
            Ok(_) => panic!("Load succeeded with corrupted data"),
        }
    }

    /// Test 2: File Truncation
    ///
    /// Truncates the file and verifies correct error handling.
    /// - < 64 bytes: BufferTooSmall
    /// - >= 64 bytes (data truncated): ChecksumMismatch
    #[test]
    fn test_truncation(
        data_len in 10usize..1000,
        cut_amount in 1usize..500
    ) {
        let (file, _data) = create_valid_file(data_len);
        let path = file.path();

        let file_bytes = std::fs::read(path).unwrap();
        let original_len = file_bytes.len();
        let cut = core::cmp::min(cut_amount, original_len - 1); // Leave at least 1 byte
        let new_len = original_len - cut;

        let truncated = &file_bytes[0..new_len];
        std::fs::write(path, truncated).unwrap();

        let res = {
            let backend = FileBackend::new(path);
            load_snapshot(&backend)
        };

        if new_len < 64 {
             match res {
                Err(PersistenceError::BufferTooSmall { .. }) => {}, // Pass
                Err(e) => panic!("Expected BufferTooSmall, got {:?}", e),
                Ok(_) => panic!("Load succeeded with truncated header"),
            }
        } else {
            // Data truncated -> Checksum mismatch (CRC won't match partial data)
             match res {
                Err(PersistenceError::ChecksumMismatch { .. }) => {}, // Pass
                Err(e) => panic!("Expected ChecksumMismatch, got {:?}", e),
                Ok(_) => panic!("Load succeeded with truncated data"),
            }
        }
    }

    /// Test 3: Header Damage
    ///
    /// Corrupts random bits in the header (first 64 bytes).
    /// Expected errors: InvalidMagic, UnsupportedVersion, or Header(ChecksumMismatch).
    #[test]
    fn test_header_damage(
        bit_idx in 0usize..512 // 64 bytes * 8 bits
    ) {
        let (file, _data) = create_valid_file(100);
        let path = file.path();

        let mut file_bytes = std::fs::read(path).unwrap();
        let byte_idx = bit_idx / 8;
        let bit_offset = bit_idx % 8;

        file_bytes[byte_idx] ^= 1 << bit_offset;

        std::fs::write(path, &file_bytes).unwrap();

        let res = {
            let backend = FileBackend::new(path);
            load_snapshot(&backend)
        };

        match res {
            Err(PersistenceError::Header(h_err)) => {
                match h_err {
                    HeaderError::InvalidMagic(_) => {},
                    HeaderError::UnsupportedVersion(_, _) => {},
                    HeaderError::ChecksumMismatch { .. } => {},
                    HeaderError::BufferTooShort(_) => panic!("Did not truncate, but got BufferTooShort"),
                    #[allow(deprecated)]
                    HeaderError::UnalignedBuffer => panic!("Did not misalign, but got UnalignedBuffer"),
                }
            }
             Err(e) => panic!("Expected Header error, got {:?}", e),
            Ok(_) => panic!("Load succeeded with corrupted header"),
        }
    }
}
