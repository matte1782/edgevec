use bytemuck::{Pod, Zeroable};
use core::mem::{align_of, size_of};
use thiserror::Error;

/// Magic number: "EVEC" = [0x45, 0x56, 0x45, 0x43]
pub const MAGIC: [u8; 4] = *b"EVEC";

/// Current major version
pub const VERSION_MAJOR: u8 = 0;

/// Current minor version (bumped to 3 for soft-delete support)
pub const VERSION_MINOR: u8 = 3;

/// Minimum supported minor version for migration
pub const VERSION_MINOR_MIN: u8 = 1;

/// Magic number for metadata section: "META" = [0x4D, 0x45, 0x54, 0x41]
pub const METADATA_MAGIC: [u8; 4] = *b"META";

/// Current metadata section version
pub const METADATA_VERSION: u16 = 1;

/// Serialization format: Postcard (binary, compact)
pub const FORMAT_POSTCARD: u8 = 1;

/// Serialization format: JSON (text, debugging)
pub const FORMAT_JSON: u8 = 2;

/// File format flags
#[allow(non_snake_case)]
pub mod Flags {
    /// Data is compressed
    pub const COMPRESSED: u16 = 1 << 0;
    /// Vectors are quantized
    pub const QUANTIZED: u16 = 1 << 1;
    /// MetadataStore is present (v0.4+)
    pub const HAS_METADATA: u16 = 1 << 2;
}

/// File header for .evec index files.
///
/// # Layout
///
/// Total size: 64 bytes
/// Alignment: 8 bytes
///
/// # Invariants
///
/// - `magic` must be "EVEC"
/// - `version_major` must match current version
///
/// # Thread Safety
///
/// This type is `Send + Sync` as it is a POD struct.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct FileHeader {
    /// Magic number: "EVEC" = [0x45, 0x56, 0x45, 0x43]
    pub magic: [u8; 4], // 0

    /// Format version major part.
    pub version_major: u8, // 4

    /// Format version minor part.
    pub version_minor: u8, // 5

    /// Flags (bit 0: compressed, bit 1: quantized, etc.)
    pub flags: u16, // 6

    /// Number of vectors in file
    pub vector_count: u64, // 8

    /// Byte offset to index section
    pub index_offset: u64, // 16

    /// Byte offset to metadata section (0 if none)
    pub metadata_offset: u64, // 24

    /// RNG Seed for deterministic replay
    pub rng_seed: u64, // 32

    /// Vector dimensionality
    pub dimensions: u32, // 40

    /// CRC32 of header bytes
    pub header_crc: u32, // 44

    /// HNSW M parameter
    pub hnsw_m: u32, // 48

    /// HNSW M0 parameter
    pub hnsw_m0: u32, // 52

    /// CRC32 of data payload
    pub data_crc: u32, // 56

    /// Count of deleted (tombstone) nodes in the index (v0.3+)
    ///
    /// For v0.1/v0.2 files, this field was `reserved` and always 0.
    /// During migration, we recalculate from node data.
    pub deleted_count: u32, // 60
}

// Static assertions for size and alignment
const _: () = assert!(size_of::<FileHeader>() == 64);
const _: () = assert!(align_of::<FileHeader>() == 8);

/// Metadata section header (16 bytes).
///
/// Placed after tombstone bitvec when `Flags::HAS_METADATA` flag is set.
///
/// # Layout
///
/// Total size: 16 bytes
/// Alignment: 4 bytes
///
/// | Offset | Size | Field    | Description                      |
/// |--------|------|----------|----------------------------------|
/// | 0      | 4    | magic    | "META" = [0x4D, 0x45, 0x54, 0x41]|
/// | 4      | 2    | version  | Section format version (1)       |
/// | 6      | 1    | format   | Serialization format (1=Postcard)|
/// | 7      | 1    | reserved | Reserved for future use (0)      |
/// | 8      | 4    | size     | Size of serialized metadata      |
/// | 12     | 4    | crc      | CRC32 of serialized metadata     |
///
/// # Thread Safety
///
/// This type is `Send + Sync` as it is a POD struct.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct MetadataSectionHeader {
    /// Magic number: "META" = [0x4D, 0x45, 0x54, 0x41]
    pub magic: [u8; 4],

    /// Section format version (currently 1)
    pub version: u16,

    /// Serialization format: 1=Postcard, 2=JSON
    pub format: u8,

    /// Reserved for future use (must be 0)
    pub reserved: u8,

    /// Size of serialized metadata in bytes
    pub size: u32,

    /// CRC32 of serialized metadata bytes
    pub crc: u32,
}

// Static assertions for MetadataSectionHeader size and alignment
const _: () = assert!(size_of::<MetadataSectionHeader>() == 16);
const _: () = assert!(align_of::<MetadataSectionHeader>() == 4);

impl MetadataSectionHeader {
    /// The expected magic bytes "META".
    pub const MAGIC: [u8; 4] = METADATA_MAGIC;

    /// The current section version.
    pub const VERSION: u16 = METADATA_VERSION;

    /// Postcard serialization format identifier.
    pub const FORMAT_POSTCARD: u8 = FORMAT_POSTCARD;

    /// JSON serialization format identifier.
    pub const FORMAT_JSON: u8 = FORMAT_JSON;

    /// Creates a new `MetadataSectionHeader` for Postcard-serialized metadata.
    #[must_use]
    pub fn new_postcard(size: u32, crc: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            format: Self::FORMAT_POSTCARD,
            reserved: 0,
            size,
            crc,
        }
    }

    /// Creates a new `MetadataSectionHeader` for JSON-serialized metadata.
    #[must_use]
    pub fn new_json(size: u32, crc: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            format: Self::FORMAT_JSON,
            reserved: 0,
            size,
            crc,
        }
    }

    /// Returns the byte representation of the header.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 16] {
        bytemuck::cast_ref(self)
    }

    /// Parses a `MetadataSectionHeader` from bytes.
    ///
    /// # Requirements
    ///
    /// - `bytes` must be at least 16 bytes
    /// - `bytes` must be 4-byte aligned
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Buffer is less than 16 bytes
    /// - Buffer is not 4-byte aligned
    /// - Magic number is invalid
    /// - Version is unsupported
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MetadataHeaderError> {
        if bytes.len() < 16 {
            return Err(MetadataHeaderError::BufferTooShort(bytes.len()));
        }

        let header = *bytemuck::try_from_bytes::<MetadataSectionHeader>(&bytes[..16])
            .map_err(|_| MetadataHeaderError::UnalignedBuffer)?;

        header.validate_magic()?;
        header.validate_version()?;
        header.validate_format()?;

        Ok(header)
    }

    /// Validates the magic bytes.
    ///
    /// # Errors
    ///
    /// Returns `MetadataHeaderError::InvalidMagic` if magic bytes don't match "META".
    pub fn validate_magic(&self) -> Result<(), MetadataHeaderError> {
        if self.magic != Self::MAGIC {
            return Err(MetadataHeaderError::InvalidMagic(self.magic));
        }
        Ok(())
    }

    /// Validates the version is supported.
    ///
    /// # Errors
    ///
    /// Returns `MetadataHeaderError::UnsupportedVersion` if version is newer than current.
    pub fn validate_version(&self) -> Result<(), MetadataHeaderError> {
        if self.version > Self::VERSION {
            return Err(MetadataHeaderError::UnsupportedVersion(self.version));
        }
        Ok(())
    }

    /// Validates the serialization format is supported.
    ///
    /// # Errors
    ///
    /// Returns `MetadataHeaderError::UnsupportedFormat` if format is not 1 (Postcard) or 2 (JSON).
    pub fn validate_format(&self) -> Result<(), MetadataHeaderError> {
        if self.format != Self::FORMAT_POSTCARD && self.format != Self::FORMAT_JSON {
            return Err(MetadataHeaderError::UnsupportedFormat(self.format));
        }
        Ok(())
    }

    /// Returns true if this header uses Postcard serialization.
    #[must_use]
    pub fn is_postcard(&self) -> bool {
        self.format == Self::FORMAT_POSTCARD
    }

    /// Returns true if this header uses JSON serialization.
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.format == Self::FORMAT_JSON
    }
}

/// Errors that can occur during metadata header parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetadataHeaderError {
    /// Invalid magic number.
    #[error("invalid metadata magic: expected 'META', got {0:?}")]
    InvalidMagic([u8; 4]),

    /// Unsupported version.
    #[error("unsupported metadata version: {0}")]
    UnsupportedVersion(u16),

    /// Unsupported serialization format.
    #[error("unsupported serialization format: {0}")]
    UnsupportedFormat(u8),

    /// Buffer too short.
    #[error("buffer too short: expected 16 bytes, got {0}")]
    BufferTooShort(usize),

    /// Buffer is not 4-byte aligned.
    #[error("buffer is not 4-byte aligned")]
    UnalignedBuffer,

    /// CRC mismatch.
    #[error("CRC mismatch: expected {expected:#x}, got {actual:#x}")]
    CrcMismatch {
        /// Expected CRC (from header)
        expected: u32,
        /// Actual calculated CRC
        actual: u32,
    },
}

/// Errors that can occur during header parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HeaderError {
    /// Invalid magic number.
    #[error("invalid magic number: expected 'EVEC', got {0:?}")]
    InvalidMagic([u8; 4]),

    /// Unsupported version.
    #[error("unsupported version: {0}.{1}")]
    UnsupportedVersion(u8, u8),

    /// Checksum mismatch.
    #[error("checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch {
        /// Expected checksum (from header)
        expected: u32,
        /// Actual calculated checksum
        actual: u32,
    },

    /// Buffer too short.
    #[error("buffer too short: expected 64 bytes, got {0}")]
    BufferTooShort(usize),

    /// Buffer is not 8-byte aligned.
    #[error("buffer is not 8-byte aligned")]
    UnalignedBuffer,
}

impl FileHeader {
    /// The expected magic bytes "EVEC".
    pub const MAGIC: [u8; 4] = MAGIC;
    /// The current major version.
    pub const VERSION_MAJOR: u8 = VERSION_MAJOR;
    /// The current minor version.
    pub const VERSION_MINOR: u8 = VERSION_MINOR;

    /// Creates a new `FileHeader` with default values.
    #[must_use]
    pub fn new(dimensions: u32) -> Self {
        let mut header = Self {
            magic: MAGIC,
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            flags: 0,
            vector_count: 0,
            index_offset: 0,
            metadata_offset: 0,
            rng_seed: 0,
            dimensions,
            header_crc: 0,
            hnsw_m: 16,
            hnsw_m0: 32,
            data_crc: 0,
            deleted_count: 0,
        };
        header.update_checksum();
        header
    }

    /// Returns the byte representation of the header.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 64] {
        bytemuck::cast_ref(self)
    }

    /// Parses a `FileHeader` from bytes.
    ///
    /// # Requirements
    ///
    /// - `bytes` must be at least 64 bytes
    /// - `bytes` must be 8-byte aligned
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Buffer is less than 64 bytes (`BufferTooShort`)
    /// - Buffer is not 8-byte aligned (`UnalignedBuffer`)
    /// - Magic number is invalid (`InvalidMagic`)
    /// - Version is unsupported (`UnsupportedVersion`)
    /// - Checksum mismatch (`ChecksumMismatch`)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < 64 {
            return Err(HeaderError::BufferTooShort(bytes.len()));
        }

        // SAFETY: Length is checked above; alignment is validated via `try_from_bytes`.
        //
        // This function is safe because:
        // - Buffer length is verified to be exactly 64 bytes
        // - FileHeader is Pod + Zeroable (all bit patterns valid)
        // - bytemuck::try_from_bytes validates alignment
        let header = *bytemuck::try_from_bytes::<FileHeader>(&bytes[..64])
            .map_err(|_| HeaderError::UnalignedBuffer)?;

        if header.magic != MAGIC {
            return Err(HeaderError::InvalidMagic(header.magic));
        }

        // Version check: major must match, minor must be >= minimum
        if header.version_major != VERSION_MAJOR {
            return Err(HeaderError::UnsupportedVersion(
                header.version_major,
                header.version_minor,
            ));
        }

        // Support migration from older minor versions (v0.1, v0.2 → v0.3)
        if header.version_minor < VERSION_MINOR_MIN {
            return Err(HeaderError::UnsupportedVersion(
                header.version_major,
                header.version_minor,
            ));
        }

        // Verify checksum
        let mut verify_header = header;
        verify_header.header_crc = 0;
        let calculated_crc = crc32fast::hash(verify_header.as_bytes());

        if header.header_crc != calculated_crc {
            return Err(HeaderError::ChecksumMismatch {
                expected: header.header_crc,
                actual: calculated_crc,
            });
        }

        Ok(header)
    }

    /// Updates the checksum based on current fields.
    pub fn update_checksum(&mut self) {
        self.header_crc = 0;
        self.header_crc = crc32fast::hash(self.as_bytes());
    }

    /// Returns true if this header is from an older format that needs migration.
    ///
    /// Older formats (v0.1, v0.2) had `reserved` instead of `deleted_count`,
    /// and the `HnswNode.pad` field instead of `deleted`.
    ///
    /// Migration consists of:
    /// 1. Interpreting `reserved` (always 0 in old files) as `deleted_count`
    /// 2. Node `pad` byte (always 0) becomes `deleted` (0 = not deleted)
    ///
    /// Since old files had all zeros in these fields, migration is automatic
    /// and no data transformation is needed.
    #[must_use]
    pub fn needs_migration(&self) -> bool {
        self.version_minor < VERSION_MINOR
    }

    /// Returns true if this header supports soft-delete (v0.3+).
    #[must_use]
    pub fn supports_soft_delete(&self) -> bool {
        self.version_minor >= 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_layout() {
        assert_eq!(size_of::<FileHeader>(), 64);
        assert_eq!(core::mem::align_of::<FileHeader>(), 8);
    }

    #[test]
    fn test_new_header_validity() {
        let header = FileHeader::new(128);
        assert_eq!(header.magic, MAGIC);
        assert_eq!(header.dimensions, 128);

        // Checksum should be set
        assert_ne!(header.header_crc, 0);

        // Should be valid
        let bytes = header.as_bytes();
        let decoded = FileHeader::from_bytes(bytes).unwrap();
        assert_eq!(decoded.dimensions, 128);
    }

    #[test]
    fn test_invalid_magic() {
        let mut header = FileHeader::new(128);
        header.magic = [0x00, 0x00, 0x00, 0x00];
        header.update_checksum(); // Recalculate checksum so that's not the error

        let bytes = header.as_bytes();
        let result = FileHeader::from_bytes(bytes);
        assert!(matches!(result, Err(HeaderError::InvalidMagic(_))));
    }

    #[test]
    fn test_checksum_mismatch() {
        let mut header = FileHeader::new(128);
        // Corrupt a field without updating checksum
        header.dimensions = 256;

        let bytes = header.as_bytes();
        let result = FileHeader::from_bytes(bytes);
        assert!(matches!(result, Err(HeaderError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_unaligned_buffer_rejected() {
        let header = FileHeader::new(64);
        let mut buf = Vec::with_capacity(65);
        buf.push(0); // create an offset to force misalignment
        buf.extend_from_slice(header.as_bytes());

        let slice = &buf[1..65];
        let result = FileHeader::from_bytes(slice);
        assert!(matches!(result, Err(HeaderError::UnalignedBuffer)));
    }

    // =========================================================================
    // MetadataSectionHeader tests
    // =========================================================================

    #[test]
    fn test_metadata_header_layout() {
        assert_eq!(size_of::<MetadataSectionHeader>(), 16);
        assert_eq!(align_of::<MetadataSectionHeader>(), 4);
    }

    #[test]
    fn test_metadata_header_new_postcard() {
        let header = MetadataSectionHeader::new_postcard(1024, 0xDEADBEEF);

        assert_eq!(header.magic, *b"META");
        assert_eq!(header.version, 1);
        assert_eq!(header.format, FORMAT_POSTCARD);
        assert_eq!(header.reserved, 0);
        assert_eq!(header.size, 1024);
        assert_eq!(header.crc, 0xDEADBEEF);
        assert!(header.is_postcard());
        assert!(!header.is_json());
    }

    #[test]
    fn test_metadata_header_new_json() {
        let header = MetadataSectionHeader::new_json(2048, 0xCAFEBABE);

        assert_eq!(header.magic, *b"META");
        assert_eq!(header.version, 1);
        assert_eq!(header.format, FORMAT_JSON);
        assert_eq!(header.reserved, 0);
        assert_eq!(header.size, 2048);
        assert_eq!(header.crc, 0xCAFEBABE);
        assert!(!header.is_postcard());
        assert!(header.is_json());
    }

    #[test]
    fn test_metadata_header_roundtrip() {
        let header = MetadataSectionHeader::new_postcard(512, 0x12345678);
        let bytes = header.as_bytes();

        assert_eq!(bytes.len(), 16);

        let decoded = MetadataSectionHeader::from_bytes(bytes).unwrap();
        assert_eq!(decoded.magic, header.magic);
        assert_eq!(decoded.version, header.version);
        assert_eq!(decoded.format, header.format);
        assert_eq!(decoded.size, header.size);
        assert_eq!(decoded.crc, header.crc);
    }

    #[test]
    fn test_metadata_header_invalid_magic() {
        let mut header = MetadataSectionHeader::new_postcard(0, 0);
        header.magic = [0x00, 0x00, 0x00, 0x00];

        let bytes = header.as_bytes();
        let result = MetadataSectionHeader::from_bytes(bytes);
        assert!(matches!(result, Err(MetadataHeaderError::InvalidMagic(_))));
    }

    #[test]
    fn test_metadata_header_unsupported_version() {
        let mut header = MetadataSectionHeader::new_postcard(0, 0);
        header.version = 99; // Future version

        let bytes = header.as_bytes();
        let result = MetadataSectionHeader::from_bytes(bytes);
        assert!(matches!(
            result,
            Err(MetadataHeaderError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn test_metadata_header_unsupported_format() {
        let mut header = MetadataSectionHeader::new_postcard(0, 0);
        header.format = 99; // Unknown format

        let bytes = header.as_bytes();
        let result = MetadataSectionHeader::from_bytes(bytes);
        assert!(matches!(
            result,
            Err(MetadataHeaderError::UnsupportedFormat(99))
        ));
    }

    #[test]
    fn test_metadata_header_buffer_too_short() {
        let bytes = [0u8; 8]; // Only 8 bytes, need 16
        let result = MetadataSectionHeader::from_bytes(&bytes);
        assert!(matches!(
            result,
            Err(MetadataHeaderError::BufferTooShort(8))
        ));
    }

    #[test]
    fn test_flags_constants() {
        assert_eq!(Flags::COMPRESSED, 0b0001);
        assert_eq!(Flags::QUANTIZED, 0b0010);
        assert_eq!(Flags::HAS_METADATA, 0b0100);

        // Flags should be combinable
        let combined = Flags::COMPRESSED | Flags::HAS_METADATA;
        assert_eq!(combined, 0b0101);
    }
}
