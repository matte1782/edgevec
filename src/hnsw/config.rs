use serde::{Deserialize, Serialize};

/// HNSW algorithm parameters.
///
/// # Size
/// 32 bytes, aligned to 4
///
/// # Parameter Guidelines (from paper)
/// - M: 12-48 for high recall, 4-8 for speed
/// - `ef_construction`: Higher = better quality, slower build
/// - `ef_search`: Higher = better recall, slower search
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct HnswConfig {
    /// Max connections per node in layers > 0
    /// Typical: 16
    pub m: u32, // offset 0, size 4

    /// Max connections per node in layer 0 (typically 2*M)
    /// Typical: 32
    pub m0: u32, // offset 4, size 4

    /// Construction-time candidate list size
    /// Typical: 200
    pub ef_construction: u32, // offset 8, size 4

    /// Search-time candidate list size
    /// Typical: 50
    pub ef_search: u32, // offset 12, size 4

    /// Vector dimensionality
    pub dimensions: u32, // offset 16, size 4

    /// Distance metric (0 = L2, 1 = Cosine, 2 = Dot)
    pub metric: u32, // offset 20, size 4

    /// Reserved for future use
    pub _reserved: [u32; 2], // offset 24, size 8
}

impl HnswConfig {
    /// Metric code for L2 Squared distance.
    pub const METRIC_L2_SQUARED: u32 = 0;
    /// Metric code for Cosine Similarity.
    pub const METRIC_COSINE: u32 = 1;
    /// Metric code for Dot Product.
    pub const METRIC_DOT_PRODUCT: u32 = 2;
    /// Metric code for Hamming distance (for binary vectors).
    pub const METRIC_HAMMING: u32 = 3;

    /// Creates a default configuration.
    #[must_use]
    pub fn new(dimensions: u32) -> Self {
        Self {
            m: 12,
            m0: 24,
            ef_construction: 100,
            ef_search: 50,
            dimensions,
            metric: Self::METRIC_L2_SQUARED,
            _reserved: [0; 2],
        }
    }
}

// Verify size and alignment
const _: () = assert!(core::mem::size_of::<HnswConfig>() == 32);
const _: () = assert!(core::mem::align_of::<HnswConfig>() == 4);
