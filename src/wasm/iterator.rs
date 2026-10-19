use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

/// Iterator for saving the database in chunks.
///
/// Owns a serialized snapshot, so subsequent source mutations or freeing the
/// database cannot invalidate iteration. Memory includes the complete snapshot
/// and one emitted JavaScript chunk. Free the iterator to release the snapshot.
#[wasm_bindgen]
pub struct PersistenceIterator {
    pub(crate) bytes: Vec<u8>,
    pub(crate) position: usize,
    pub(crate) chunk_size: usize,
}

#[wasm_bindgen]
impl PersistenceIterator {
    /// Returns the next chunk of data.
    ///
    /// # Returns
    ///
    /// * `Some(Uint8Array)` - The next chunk of data.
    /// * `None` - If iteration is complete.
    ///
    #[wasm_bindgen]
    pub fn next_chunk(&mut self) -> Option<Uint8Array> {
        if self.position >= self.bytes.len() {
            return None;
        }
        let end = self
            .position
            .saturating_add(self.chunk_size)
            .min(self.bytes.len());
        let chunk = Uint8Array::from(&self.bytes[self.position..end]);
        self.position = end;
        Some(chunk)
    }
}
