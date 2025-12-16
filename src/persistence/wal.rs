use crate::persistence::entry::WalEntry;
use crate::persistence::storage::StorageBackend;
use crate::persistence::PersistenceError;
use crc32fast::Hasher;
use std::io::{self, Read};
use thiserror::Error;

/// Header size in bytes: sequence(8) + type(1) + pad(3) + len(4)
pub const WAL_HEADER_SIZE: usize = 16;

/// CRC checksum size in bytes
pub const CRC_SIZE: usize = 4;

/// Maximum allowed payload size (16MB) to prevent `DoS` attacks.
pub const MAX_PAYLOAD_SIZE: usize = 16 * 1024 * 1024;

/// Errors that can occur during WAL iteration.
#[derive(Debug, Error)]
pub enum WalError {
    /// I/O error reading from the WAL.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Persistence error.
    #[error("persistence error: {0}")]
    Persistence(#[from] PersistenceError),

    /// CRC32 checksum mismatch.
    #[error("checksum mismatch: expected {expected:#010x}, got {actual:#010x}")]
    ChecksumMismatch {
        /// The expected checksum read from the file.
        expected: u32,
        /// The actual calculated checksum.
        actual: u32,
    },

    /// File ended unexpectedly (truncated).
    #[error("file truncated: expected {expected} bytes, got {actual}")]
    Truncated {
        /// Number of bytes expected to read.
        expected: usize,
        /// Number of bytes actually read.
        actual: usize,
    },

    /// Payload size exceeds maximum allowed limit.
    #[error("payload too large: size {size} exceeds max {max}")]
    PayloadTooLarge {
        /// The requested payload size.
        size: usize,
        /// The maximum allowed size.
        max: usize,
    },
}

/// Iterator over entries in a Write-Ahead Log.
///
/// Reads strictly sequentially. Does not load the whole file into memory.
pub struct WalIterator<R> {
    reader: R,
}

impl<R: Read> WalIterator<R> {
    /// Creates a new `WalIterator` wrapping the given reader.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Helper to read exact bytes or detect clean EOF.
    /// Returns `Ok(true)` if buffer filled, `Ok(false)` if clean EOF (0 bytes read),
    /// `Err(Truncated)` if partial read, or `Err(Io)`.
    fn read_exact_or_eof(&mut self, buf: &mut [u8]) -> Result<bool, WalError> {
        let mut total_read = 0;
        while total_read < buf.len() {
            match self.reader.read(&mut buf[total_read..]) {
                Ok(0) => {
                    if total_read == 0 {
                        return Ok(false); // Clean EOF
                    }
                    return Err(WalError::Truncated {
                        expected: buf.len(),
                        actual: total_read,
                    });
                }
                Ok(n) => total_read += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(WalError::Io(e)),
            }
        }
        Ok(true)
    }
}

impl<R: Read> Iterator for WalIterator<R> {
    type Item = Result<(WalEntry, Vec<u8>), WalError>;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. Read Header
        let mut header_bytes = [0u8; WAL_HEADER_SIZE];
        match self.read_exact_or_eof(&mut header_bytes) {
            Ok(true) => {}            // Got header
            Ok(false) => return None, // Clean EOF
            Err(e) => return Some(Err(e)),
        }

        // 2. Parse WalEntry (Manual Little-Endian Deserialization)
        // Layout: sequence(8) + type(1) + pad(3) + len(4)
        let sequence = u64::from_le_bytes(
            header_bytes[0..8]
                .try_into()
                .expect("slice length is strictly 8 bytes"),
        );
        let entry_type = header_bytes[8];
        // Bytes 9..12 are padding, ignore them
        let payload_len_u32 = u32::from_le_bytes(
            header_bytes[12..16]
                .try_into()
                .expect("slice length is strictly 4 bytes"),
        );

        let entry = WalEntry::new(sequence, entry_type, payload_len_u32);

        let payload_len = entry.payload_len as usize;

        // Validation: Check payload size to prevent DoS (OOM)
        if payload_len > MAX_PAYLOAD_SIZE {
            return Some(Err(WalError::PayloadTooLarge {
                size: payload_len,
                max: MAX_PAYLOAD_SIZE,
            }));
        }

        // 3. Read Payload
        let mut payload = vec![0u8; payload_len];
        match self.read_exact_or_eof(&mut payload) {
            Ok(true) => {}
            Ok(false) => {
                return Some(Err(WalError::Truncated {
                    expected: payload_len,
                    actual: 0,
                }))
            }
            Err(e) => return Some(Err(e)),
        }

        // 4. Read CRC
        let mut crc_bytes = [0u8; CRC_SIZE];
        match self.read_exact_or_eof(&mut crc_bytes) {
            Ok(true) => {}
            Ok(false) => {
                return Some(Err(WalError::Truncated {
                    expected: CRC_SIZE,
                    actual: 0,
                }))
            }
            Err(e) => return Some(Err(e)),
        }
        let stored_crc = u32::from_le_bytes(crc_bytes);

        // 5. Compute CRC (Header + Payload)
        // Note: CRC MUST be computed on the serialized bytes (header_bytes),
        // which matches how they are written.
        let mut hasher = Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(&payload);
        let calculated_crc = hasher.finalize();

        // 6. Verify CRC
        if calculated_crc != stored_crc {
            return Some(Err(WalError::ChecksumMismatch {
                expected: stored_crc,
                actual: calculated_crc,
            }));
        }

        // 7. Return Success
        Some(Ok((entry, payload)))
    }
}

/// Appends entries to the Write-Ahead Log.
pub struct WalAppender {
    backend: Box<dyn StorageBackend>,
    next_sequence: u64,
}

impl WalAppender {
    /// Creates a new `WalAppender` starting at the given sequence number.
    #[must_use]
    pub fn new(backend: Box<dyn StorageBackend>, next_sequence: u64) -> Self {
        Self {
            backend,
            next_sequence,
        }
    }

    /// Appends a new entry to the WAL.
    ///
    /// # Arguments
    ///
    /// * `entry_type` - Type of the entry (0=insert, 1=delete, etc.)
    /// * `payload` - The data to store.
    ///
    /// # Errors
    ///
    /// Returns `WalError::Io` if writing fails, or `PayloadTooLarge` if payload exceeds limit.
    pub fn append(&mut self, entry_type: u8, payload: &[u8]) -> Result<(), WalError> {
        let payload_len = payload.len();
        if payload_len > MAX_PAYLOAD_SIZE {
            return Err(WalError::PayloadTooLarge {
                size: payload_len,
                max: MAX_PAYLOAD_SIZE,
            });
        }

        // SAFETY: MAX_PAYLOAD_SIZE is 16MB, which fits in u32.
        #[allow(clippy::cast_possible_truncation)]
        let payload_len_u32 = payload_len as u32;

        let entry_sequence = self.next_sequence;
        self.next_sequence += 1;

        // Serialize Header (Manual Little-Endian)
        let mut header_bytes = [0u8; WAL_HEADER_SIZE];
        header_bytes[0..8].copy_from_slice(&entry_sequence.to_le_bytes());
        header_bytes[8] = entry_type;
        header_bytes[9..12].fill(0); // Zero padding
        header_bytes[12..16].copy_from_slice(&payload_len_u32.to_le_bytes());

        // Calculate CRC on serialized header + payload
        let mut hasher = Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(payload);
        let crc = hasher.finalize();

        // Combine for single write (backend handles atomicity/sync)
        let mut buffer = Vec::with_capacity(WAL_HEADER_SIZE + payload_len + CRC_SIZE);
        buffer.extend_from_slice(&header_bytes);
        buffer.extend_from_slice(payload);
        buffer.extend_from_slice(&crc.to_le_bytes());

        self.backend.append(&buffer)?;

        Ok(())
    }

    /// Flushes the underlying writer to ensure durability.
    ///
    /// This is now a no-op as `StorageBackend::append` implies durability.
    /// Retained for API compatibility.
    ///
    /// # Errors
    ///
    /// Returns `WalError::Io` if flushing fails.
    pub fn sync(&mut self) -> Result<(), WalError> {
        // Backend append handles sync.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_wal_constants() {
        assert_eq!(WAL_HEADER_SIZE, 16);
        assert_eq!(CRC_SIZE, 4);
    }

    #[test]
    fn test_wal_entry_layout() {
        assert_eq!(size_of::<WalEntry>(), WAL_HEADER_SIZE);
        assert_eq!(align_of::<WalEntry>(), 8);
    }

    #[test]
    fn test_wal_replay_integrity() {
        use crate::persistence::storage::{MemoryBackend, StorageBackend};
        use std::io::Cursor;

        // 1. Setup Backend
        let memory = MemoryBackend::new();
        let backend = Box::new(memory.clone());

        // 2. Write 100 entries
        let mut appender = WalAppender::new(backend, 0);
        #[allow(clippy::cast_sign_loss)]
        for i in 0..100_i32 {
            let payload = (i as u32).to_le_bytes(); // 4 bytes payload
            appender.append(0, &payload).expect("append failed");
        }

        // 3. "Reopen" / Replay
        let read_backend = Box::new(memory); // New Box, same underlying data
        let data = read_backend.read().expect("read failed");

        let cursor = Cursor::new(data);
        let iterator = WalIterator::new(cursor);

        let mut count = 0;
        #[allow(clippy::cast_possible_truncation)]
        for (i, result) in iterator.enumerate() {
            let (entry, payload) = result.expect("replay failed");
            assert_eq!(entry.sequence, i as u64);
            assert_eq!(entry.entry_type, 0);

            // Verify payload
            let expected_payload = (i as u32).to_le_bytes();
            assert_eq!(payload, expected_payload);

            count += 1;
        }

        assert_eq!(count, 100);
    }
}
