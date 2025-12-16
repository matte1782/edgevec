# EdgeVec Migration Guide

**Version:** 0.4.0
**Last Updated:** 2025-12-16

This guide covers:
1. [Migration from Competitors](#migration-from-competitors) — hnswlib, FAISS, Pinecone
2. [EdgeVec Version Migration](#edgevec-version-migration) — Between EdgeVec versions

---

## Migration from Competitors

### From hnswlib (Python/C++)

hnswlib is a popular C++ HNSW implementation with Python bindings.

#### Conceptual Differences

| Concept | hnswlib | EdgeVec |
|:--------|:--------|:--------|
| Index creation | `hnswlib.Index(space, dim)` | `new EdgeVec(config)` |
| Insert | `index.add_items(vectors, ids)` | `index.insert(vector)` |
| Search | `index.knn_query(vector, k)` | `index.search(vector, k)` |
| Persistence | `index.save_index(path)` | `index.save(name)` |
| Delete | Not natively supported | `index.softDelete(id)` |
| Browser support | No (native only) | Yes (WASM) |

#### Migration Steps

**Step 1: Export from hnswlib**

```python
import hnswlib
import numpy as np
import json

# Load your existing index
p = hnswlib.Index(space='l2', dim=128)
p.load_index("my_index.bin")

# Get all vectors (hnswlib doesn't expose this directly)
# You need to keep your original vectors somewhere
vectors = np.load("my_vectors.npy")
ids = np.arange(len(vectors))

# Export to JSON for EdgeVec import
export_data = {
    "dimensions": 128,
    "metric": "l2",
    "vectors": vectors.tolist(),
    "ids": ids.tolist()
}
with open("export.json", "w") as f:
    json.dump(export_data, f)
```

**Step 2: Import into EdgeVec**

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function migrateFromHnswlib() {
    await init();

    // Load exported data
    const response = await fetch('export.json');
    const data = await response.json();

    // Create EdgeVec index with matching config
    const config = new EdgeVecConfig(data.dimensions);
    config.metric = data.metric;  // 'l2', 'cosine', or 'dot'
    const index = new EdgeVec(config);

    // Import vectors
    for (let i = 0; i < data.vectors.length; i++) {
        const id = index.insert(new Float32Array(data.vectors[i]));
        // Note: EdgeVec assigns IDs automatically
        // Map: hnswlib_id[i] -> edgevec_id[id]
    }

    // Save to IndexedDB
    await index.save("migrated-index");
    console.log(`Migrated ${data.vectors.length} vectors`);
}
```

#### Key Differences

1. **Auto IDs:** EdgeVec assigns IDs automatically (no manual IDs like hnswlib)
2. **Delete support:** EdgeVec has `softDelete()`, hnswlib doesn't
3. **Browser native:** EdgeVec runs in browsers via WASM
4. **Async init:** EdgeVec requires `await init()` for WASM

---

### From FAISS (Python)

FAISS is Meta's library for efficient similarity search.

#### Conceptual Differences

| Concept | FAISS | EdgeVec |
|:--------|:------|:--------|
| Index type | Multiple (Flat, HNSW, IVF, PQ) | HNSW only |
| Training | May require `index.train()` | No training needed |
| GPU support | Yes | No (CPU/WASM) |
| Quantization | PQ, OPQ, SQ | SQ8 (scalar) |
| Browser support | No | Yes (WASM) |

#### Migration Steps

**Step 1: Export from FAISS**

```python
import faiss
import numpy as np
import json

# Load existing FAISS index
index = faiss.read_index("my_faiss_index.bin")

# Reconstruct vectors (only for Flat or HNSW indices)
# For IVF/PQ, you need original vectors
n = index.ntotal
d = index.d
vectors = np.zeros((n, d), dtype='float32')
for i in range(n):
    vectors[i] = index.reconstruct(i)

# Export
export_data = {
    "dimensions": d,
    "vectors": vectors.tolist()
}
with open("faiss_export.json", "w") as f:
    json.dump(export_data, f)
```

**Step 2: Import into EdgeVec**

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function migrateFromFaiss() {
    await init();

    const response = await fetch('faiss_export.json');
    const data = await response.json();

    const config = new EdgeVecConfig(data.dimensions);
    const index = new EdgeVec(config);

    // Batch insert for better performance
    for (const vector of data.vectors) {
        index.insert(new Float32Array(vector));
    }

    await index.save("migrated-from-faiss");
}
```

#### Important Notes

1. **No GPU:** EdgeVec is CPU/WASM only
2. **HNSW only:** EdgeVec uses HNSW; IVF/PQ indices need vector export
3. **SQ8 quantization:** EdgeVec offers SQ8 (3.6x compression)
4. **Result format:** EdgeVec returns `{ id, score }` objects

---

### From Pinecone (Cloud)

Pinecone is a managed vector database service.

#### Conceptual Differences

| Concept | Pinecone | EdgeVec |
|:--------|:---------|:--------|
| Architecture | Cloud service | Embedded/local |
| Pricing | Per-query/storage | Free (open source) |
| Latency | Network RTT + processing | Sub-millisecond local |
| Metadata | Native support | Store separately |
| Scaling | Managed sharding | Single-node |
| Privacy | Data on servers | Data stays local |

#### Migration Steps

**Step 1: Export from Pinecone**

```python
import pinecone
import json

# Initialize
pinecone.init(api_key="your-api-key", environment="your-env")
index = pinecone.Index("your-index")

# Fetch all vectors (paginated)
# Note: Pinecone doesn't have a direct "get all" API
# You need to iterate through your ID list

all_vectors = []
all_metadata = []
batch_size = 100

# Assuming you have a list of IDs
ids = ["id1", "id2", ...]  # Your vector IDs

for i in range(0, len(ids), batch_size):
    batch_ids = ids[i:i+batch_size]
    response = index.fetch(ids=batch_ids)

    for id, data in response['vectors'].items():
        all_vectors.append({
            "id": id,
            "values": data['values'],
            "metadata": data.get('metadata', {})
        })

# Export
with open("pinecone_export.json", "w") as f:
    json.dump({
        "vectors": all_vectors,
        "dimensions": len(all_vectors[0]['values'])
    }, f)
```

**Step 2: Import into EdgeVec**

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function migrateFromPinecone() {
    await init();

    const response = await fetch('pinecone_export.json');
    const data = await response.json();

    const config = new EdgeVecConfig(data.dimensions);
    const index = new EdgeVec(config);

    // EdgeVec doesn't store metadata natively
    // Store it separately (localStorage, IndexedDB, etc.)
    const metadataStore = {};

    for (const item of data.vectors) {
        const edgevecId = index.insert(new Float32Array(item.values));
        // Map original ID to EdgeVec ID
        metadataStore[edgevecId] = {
            originalId: item.id,
            metadata: item.metadata
        };
    }

    // Save index
    await index.save("migrated-from-pinecone");

    // Save metadata separately
    localStorage.setItem("pinecone-metadata", JSON.stringify(metadataStore));
}
```

#### Key Differences

1. **Local first:** EdgeVec runs entirely on-device (no cloud)
2. **No network latency:** Sub-millisecond search
3. **Privacy:** Data never leaves the device
4. **No native metadata:** Store metadata separately
5. **Manual scaling:** You manage data partitioning if needed

---

### General Migration Tips

#### Data Migration Steps

1. **Export vectors** from source system (JSON, CSV, NumPy)
2. **Convert to Float32Array** if needed
3. **Batch insert** using EdgeVec's API
4. **Verify** by searching for known vectors
5. **Persist** using `index.save()`

#### ID Mapping

EdgeVec uses auto-incrementing IDs. Create a mapping:

```javascript
const idMapping = new Map();  // oldId -> newEdgevecId

for (const [oldId, vector] of yourData) {
    const newId = index.insert(new Float32Array(vector));
    idMapping.set(oldId, newId);
}
```

#### Performance Tuning

After migration, tune these parameters:
- `M`: Connection count (16 default, higher = better recall)
- `efConstruction`: Build quality (200 default)
- `ef`: Search accuracy (set at search time)

See [PERFORMANCE_TUNING.md](./PERFORMANCE_TUNING.md) for detailed guidance.

#### Common Pitfalls

1. **Dimension mismatch:** Ensure vector dimensions match config
2. **Metric mismatch:** Use the same distance metric (L2, cosine, dot)
3. **Normalization:** Some systems expect normalized vectors
4. **Memory:** Large migrations may need chunking

---

## EdgeVec Version Migration

---

## Quick Start: v0.2.x to v0.3.0

**Good news:** v0.3.0 is **fully backward compatible**. Your existing code works without changes.

**Persistence:** v0.2 snapshots are automatically migrated when loaded by v0.3.0.

**New features:** Soft Delete API and Compaction API (opt-in, no migration required).

---

## File Format Versions

| Version | Release | Features |
|:--------|:--------|:---------|
| v0.1 | Pre-alpha | Basic HNSW persistence |
| v0.2 | v0.2.0-alpha | Same as v0.1 |
| v0.3 | **v0.3.0** | Soft delete support, compaction |

---

## v0.2 to v0.3 Migration

### What Changed

**Header (64 bytes):**
- Offset 60-63: `reserved` (always 0) → `deleted_count` (u32)

**Node Structure (16 bytes):**
- Offset 15: `pad` (always 0) → `deleted` (u8)
  - 0 = live vector
  - 1 = deleted (tombstone)

### Migration Process

**Automatic (Default):**

Migration from v0.1/v0.2 to v0.3 is **fully automatic**. When you load an older format file:

1. EdgeVec detects the version mismatch
2. Since `reserved` and `pad` fields were always 0 in older formats:
   - `deleted_count = 0` (no deletions)
   - `deleted = 0` (all nodes live)
3. The index works immediately with soft delete support

**No user action required.**

### Example

```rust
use edgevec::persistence::{read_snapshot, write_snapshot};
use edgevec::persistence::storage::MemoryBackend;

// Load v0.1/v0.2 file — automatic migration
let (index, storage) = read_snapshot(&old_backend)?;

// Now you can use soft delete features
index.soft_delete(VectorId(1))?;
println!("Deleted count: {}", index.deleted_count());

// Save as v0.3 format
let mut new_backend = MemoryBackend::default();
write_snapshot(&index, &storage, &mut new_backend)?;
```

### Backward Compatibility

| Operation | v0.1/v0.2 File | v0.3 File |
|:----------|:--------------:|:---------:|
| Read by v0.3 code | YES (auto-migrated) | YES |
| Read by v0.1/v0.2 code | YES | **NO** |
| Write by v0.3 code | NO | YES |

> **WARNING: Version Downgrade Not Supported**
>
> v0.3 snapshots are **forward-incompatible**. Once a snapshot is written in v0.3 format:
>
> - **It cannot be loaded by v0.2.x or earlier**
> - Older versions will fail with "Unsupported version" error
> - **Always backup before upgrading**
>
> If you accidentally downgrade and need the data:
> 1. Reinstall v0.3.x
> 2. Export data to JSON/CSV
> 3. Re-import to older format (loses soft delete data)

**Important:** Once you save with v0.3 format, older EdgeVec versions cannot read the file. If you need backward compatibility:

1. Keep a backup of the original file before any soft-delete operations
2. Or re-export without soft delete data (requires custom export script)

---

## Detecting Format Version

```rust
use edgevec::persistence::{read_file_header, VERSION_MINOR, VERSION_MINOR_MIN};

let header = read_file_header(&data)?;

// Check version
println!("Format version: 0.{}", header.version_minor);

// Check if migration is needed
if header.needs_migration() {
    println!("File will be migrated from v0.{} to v0.{}",
             header.version_minor, VERSION_MINOR);
}

// Check soft delete support
if header.supports_soft_delete() {
    println!("Soft delete supported, deleted_count: {}", header.deleted_count);
}
```

---

## Troubleshooting

### "Unsupported version" error

This error means the file was created by a newer EdgeVec version than you're running. Update your EdgeVec dependency.

### "Checksum mismatch" error

The file is corrupted or was modified externally. Restore from backup.

### Deleted count mismatch warning

If you see a warning like:
```
Warning: snapshot deleted_count mismatch (header=X, actual=Y). Using actual.
```

This means the header's `deleted_count` doesn't match the actual count of deleted nodes. This can happen if:
- The snapshot was manually edited
- The file was partially corrupted

EdgeVec automatically corrects this by using the actual count.

---

## Version Constants

```rust
use edgevec::persistence::{VERSION_MAJOR, VERSION_MINOR, VERSION_MINOR_MIN};

// Current version
const VERSION_MAJOR: u8 = 0;  // 0.x releases
const VERSION_MINOR: u8 = 3;  // Current minor version

// Minimum supported for migration
const VERSION_MINOR_MIN: u8 = 1;  // Can read v0.1+
```

---

## Future Migrations

Future format changes will follow the same pattern:
1. Increment VERSION_MINOR
2. Add automatic migration from previous versions
3. Document changes in this file

Major version changes (1.0, 2.0) may require explicit migration tools.

---

## New API Summary (v0.3.0)

### Soft Delete (Rust)

```rust
// Delete a vector
let was_deleted = index.soft_delete(vector_id)?;

// Check deletion status
let is_deleted = index.is_deleted(vector_id)?;

// Statistics
let deleted = index.deleted_count();
let live = index.live_count();
let ratio = index.tombstone_ratio();
```

### Soft Delete (JavaScript)

```javascript
// Delete a vector
const wasDeleted = index.softDelete(vectorId);

// Check deletion status
const isDeleted = index.isDeleted(vectorId);

// Statistics
const deleted = index.deletedCount();
const live = index.liveCount();
const ratio = index.tombstoneRatio();
```

### Compaction (Rust)

```rust
// Check if compaction recommended
if index.needs_compaction() {
    let result = index.compact(&mut storage)?;
    println!("Removed {} tombstones", result.tombstones_removed);
}

// Configure threshold
index.set_compaction_threshold(0.4); // 40%
```

### Compaction (JavaScript)

```javascript
// Check if compaction recommended
if (index.needsCompaction()) {
    const result = index.compact();
    console.log(`Removed ${result.tombstones_removed} tombstones`);
}

// Configure threshold
index.setCompactionThreshold(0.4); // 40%
```

---

## See Also

- [API Reference](./API_REFERENCE.md) — Full API documentation
- [CHANGELOG](../CHANGELOG.md) — Version history
- [README](../README.md) — Quick start guide
