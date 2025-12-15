use edgevec::persistence::storage::file::FileBackend;
use edgevec::persistence::{PersistenceError, StorageBackend};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

struct FaultyBackend {
    inner: FileBackend,
    root: PathBuf,
    fail_on_rename: AtomicBool,
}

impl FaultyBackend {
    fn new(path: &Path) -> Self {
        Self {
            inner: FileBackend::new(path),
            root: path.to_path_buf(),
            fail_on_rename: AtomicBool::new(false),
        }
    }
}

impl StorageBackend for FaultyBackend {
    fn append(&mut self, data: &[u8]) -> Result<(), PersistenceError> {
        self.inner.append(data)
    }

    fn read(&self) -> Result<Vec<u8>, PersistenceError> {
        self.inner.read()
    }

    fn atomic_write(&self, key: &str, data: &[u8]) -> Result<(), PersistenceError> {
        // Determine target path for temp file logic
        let target_path = if key.is_empty() {
            self.root.clone()
        } else if let Some(parent) = self.root.parent() {
            parent.join(key)
        } else {
            PathBuf::from(key)
        };

        // Write to temp file to simulate the "Prepare" phase
        let temp_path = target_path.with_extension("tmp_faulty");
        {
            let mut f = fs::File::create(&temp_path).map_err(PersistenceError::Io)?;
            f.write_all(data).map_err(PersistenceError::Io)?;
            f.sync_all().map_err(PersistenceError::Io)?;
        }

        // Check for simulated failure
        if self.fail_on_rename.load(Ordering::SeqCst) {
            return Err(PersistenceError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Simulated Rename Failure",
            )));
        }

        // Cleanup temp and delegate to inner backend (Commit phase)
        let _ = fs::remove_file(temp_path);
        self.inner.atomic_write(key, data)
    }
}

#[test]
fn test_atomic_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("atomic.dat");
    let backend = FileBackend::new(&file_path);

    let data = b"Hello, World!";
    // Using empty key uses the default path provided in new()
    backend.atomic_write("", data).unwrap();

    // Verify file content
    let mut file = fs::File::open(&file_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer, data);
}

#[test]
fn test_atomic_write_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("overwrite.dat");
    let backend = FileBackend::new(&file_path);

    let data1 = b"First write";
    backend.atomic_write("", data1).unwrap();

    let data2 = b"Second write - longer";
    backend.atomic_write("", data2).unwrap();

    let mut file = fs::File::open(&file_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer, data2);
}

#[test]
fn test_atomic_write_no_temp_left() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("clean.dat");
    let backend = FileBackend::new(&file_path);

    let data = b"Clean check";
    backend.atomic_write("", data).unwrap();

    // Check directory for .tmp files
    let mut found_tmp = false;
    for entry in fs::read_dir(dir.path()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "tmp") {
            found_tmp = true;
            println!("Found temp file: {:?}", path);
        }
    }
    assert!(!found_tmp, "Should clean up temp files");
}

#[test]
fn test_atomic_write_with_key() {
    let dir = tempfile::tempdir().unwrap();
    // Default path (ignored if key is absolute, or used as parent if key is relative)
    let default_path = dir.path().join("default.dat");
    let backend = FileBackend::new(&default_path);

    let data = b"Keyed data";
    // Key "custom.dat" should be relative to default_path's parent
    backend.atomic_write("custom.dat", data).unwrap();

    let expected_path = dir.path().join("custom.dat");
    assert!(
        expected_path.exists(),
        "File should exist at {:?}",
        expected_path
    );

    let mut file = fs::File::open(&expected_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer, data);
}

#[test]
fn test_atomic_failure_preserves_data() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("faulty.dat");

    // 1. Setup FaultyBackend
    let backend = FaultyBackend::new(&file_path);

    // 2. Write "Original" (should succeed as fail_on_rename is false)
    let original_data = b"Original Data";
    backend.atomic_write("", original_data).unwrap();

    // Verify Original
    {
        let mut file = fs::File::open(&file_path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, original_data);
    }

    // 3. Configure failure
    backend.fail_on_rename.store(true, Ordering::SeqCst);

    // 4. Atomic Write "New Data"
    let new_data = b"New Data";
    let result = backend.atomic_write("", new_data);

    // 5. Assert returns Err
    assert!(result.is_err(), "Should return error on simulated failure");

    // 6. Assert "Original" preserved
    {
        let mut file = fs::File::open(&file_path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        assert_eq!(buffer, original_data, "Original data should be preserved");
    }

    // 7. Assert Temp file exists
    // My FaultyBackend implementation creates ".tmp_faulty"
    let temp_path = file_path.with_extension("tmp_faulty");
    assert!(
        temp_path.exists(),
        "Temp file should exist due to simulated crash"
    );

    // Optional: Verify temp content
    {
        let mut temp_file = fs::File::open(&temp_path).unwrap();
        let mut temp_buffer = Vec::new();
        temp_file.read_to_end(&mut temp_buffer).unwrap();
        assert_eq!(temp_buffer, new_data, "Temp file should contain new data");
    }
}
