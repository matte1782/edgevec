//! Persistence module for EdgeVec.
//!
//! Handles file formats, serialization, recovery, and logging.

/// Storage chunking logic.
pub mod chunking;
/// WAL entry definitions.
pub mod entry;
/// File header definitions.
pub mod header;
/// Persistence reader.
pub mod reader;
/// Snapshot management.
pub mod snapshot;
/// Storage backend.
pub mod storage;
/// Write-Ahead Log implementation.
pub mod wal;
/// Persistence writer.
pub mod writer;

pub use chunking::{ChunkedWriter, MIN_CHUNK_SIZE};
pub use header::{
    FileHeader, Flags, HeaderError, MetadataHeaderError, MetadataSectionHeader, FORMAT_JSON,
    FORMAT_POSTCARD, MAGIC, METADATA_MAGIC, METADATA_VERSION, VERSION_MAJOR, VERSION_MINOR,
    VERSION_MINOR_MIN,
};
pub use reader::{read_file_header, read_index_header};
pub use snapshot::{read_snapshot, write_snapshot};
pub use storage::{MemoryBackend, StorageBackend};
pub use writer::write_empty_index;

use thiserror::Error;

/// Errors that can occur during persistence operations.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Buffer too small.
    #[error("buffer too small: expected {expected}, got {actual}")]
    BufferTooSmall {
        /// Expected size in bytes.
        expected: usize,
        /// Actual size in bytes.
        actual: usize,
    },

    /// Invalid magic number.
    #[error("invalid magic number: expected {expected:?}, got {actual:?}")]
    InvalidMagic {
        /// Expected magic bytes.
        expected: [u8; 4],
        /// Actual magic bytes.
        actual: [u8; 4],
    },

    /// Unsupported version.
    #[error("unsupported version: {0}.{1}")]
    UnsupportedVersion(u8, u8),

    /// Checksum mismatch.
    #[error("checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch {
        /// Expected CRC32.
        expected: u32,
        /// Actual CRC32.
        actual: u32,
    },

    /// Header error.
    #[error("header error: {0}")]
    Header(#[from] HeaderError),

    /// Corrupted data.
    #[error("corrupted data: {0}")]
    Corrupted(String),

    /// Unsupported operation.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// Component not initialized.
    #[error("not initialized")]
    NotInitialized,

    /// Truncated data (unexpected end of snapshot).
    #[error("truncated data: expected more bytes")]
    TruncatedData,

    /// Serialization error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error.
    #[error("deserialization error: {0}")]
    DeserializationError(String),
}
