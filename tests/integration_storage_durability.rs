use edgevec::hnsw::{HnswConfig, VectorId};
use edgevec::persistence::storage::file::FileBackend;
use edgevec::persistence::wal::WalAppender;
use edgevec::storage::VectorStorage;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

// INT-DUR-001: Standard Persistence Cycle
#[test]
fn test_storage_durability_cycle() {
    // Setup
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("wal.log");
    let config = HnswConfig::new(4); // 4D vectors

    // Action 1: Initialize Storage with WAL and Insert
    {
        // Use FileBackend instead of raw File
        let backend = Box::new(FileBackend::new(&wal_path));
        let wal = WalAppender::new(backend, 0);
        let mut storage = VectorStorage::new(&config, Some(wal));

        for i in 0..100 {
            let vec = vec![i as f32; 4];
            let id = storage.insert(&vec).expect("Insert should succeed");
            assert_eq!(id.0, (i + 1) as u64);
        }
    } // Storage dropped here

    // Action 3: Recover
    let backend = Box::new(FileBackend::new(&wal_path));
    let recovered_storage =
        VectorStorage::recover(backend, &config).expect("Recovery should succeed");

    // Assert
    assert_eq!(recovered_storage.len(), 100);

    for i in 0..100 {
        let expected_vec = vec![i as f32; 4];
        let id = VectorId((i + 1) as u64);
        let actual_vec = recovered_storage.get_vector(id);
        assert_eq!(actual_vec, expected_vec.as_slice());
    }
}

// INT-DUR-002: Truncated WAL Recovery
#[test]
fn test_recovery_truncated_wal() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("wal_trunc.log");
    let config = HnswConfig::new(4);

    // 1. Create valid WAL with 100 entries
    {
        let backend = Box::new(FileBackend::new(&wal_path));
        let wal = WalAppender::new(backend, 0);
        let mut storage = VectorStorage::new(&config, Some(wal));
        for i in 0..100 {
            storage.insert(&[i as f32; 4]).unwrap();
        }
    }

    // 2. Truncate the file (cut off half of the last entry)
    {
        let file = OpenOptions::new().write(true).open(&wal_path).unwrap();
        let len = file.metadata().unwrap().len();
        // Entry size approx: Header(16) + ID(8) + Vec(16) + CRC(4) = 44 bytes
        // Cut 10 bytes from end
        file.set_len(len - 10).unwrap();
    }

    // 3. Recover
    let backend = Box::new(FileBackend::new(&wal_path));
    let recovered = VectorStorage::recover(backend, &config).expect("Should recover valid prefix");

    // 4. Verify we lost the last entry but kept the first 99
    assert_eq!(recovered.len(), 99, "Should recover 99 entries");
    // Verify last valid entry (i=98, ID=99)
    let vec99 = recovered.get_vector(VectorId(99));
    assert_eq!(&vec99[..], &[98.0, 98.0, 98.0, 98.0]);
}

// INT-DUR-003: Corrupted WAL Recovery (Checksum Mismatch)
#[test]
fn test_recovery_checksum_fail_tail() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("wal_corr.log");
    let config = HnswConfig::new(4);

    // 1. Create valid WAL with 100 entries
    {
        let backend = Box::new(FileBackend::new(&wal_path));
        let wal = WalAppender::new(backend, 0);
        let mut storage = VectorStorage::new(&config, Some(wal));
        for i in 0..100 {
            storage.insert(&[i as f32; 4]).unwrap();
        }
    }

    // 2. Corrupt the last entry (flip a bit in the payload)
    {
        let mut file = OpenOptions::new().write(true).open(&wal_path).unwrap();
        // Seek to somewhere in the last entry's payload
        // Last entry starts at: Total - 44. Payload starts at +16.
        file.seek(SeekFrom::End(-20)).unwrap();
        file.write_all(&[0xFF]).unwrap(); // Corrupt a byte
    }

    // 3. Recover
    let backend = Box::new(FileBackend::new(&wal_path));
    let recovered = VectorStorage::recover(backend, &config).expect("Should recover valid prefix");

    // 4. Verify we lost the last entry due to checksum mismatch
    assert_eq!(recovered.len(), 99, "Should recover 99 entries");
}
