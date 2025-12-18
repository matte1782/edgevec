# Database Operations Guide

**Version:** EdgeVec v0.5.0
**Last Updated:** 2025-12-18

---

## Overview

EdgeVec provides database-level operations beyond basic vector search:

- **Soft Delete** - Mark vectors as deleted without removing them
- **Compaction** - Reclaim space from deleted vectors
- **Persistence** - Save and load the database

---

## Soft Delete

Soft delete marks vectors as "tombstones" without removing them from the index. Deleted vectors remain for graph routing but are excluded from search results.

### API

```typescript
// Delete a single vector
const wasDeleted: boolean = index.softDelete(vectorId);

// Check if a vector is deleted
const isDeleted: boolean = index.isDeleted(vectorId);

// Get deletion counts
const liveCount: number = index.liveCount();
const deletedCount: number = index.deletedCount();
const tombstoneRatio: number = index.tombstoneRatio();
```

### Behavior

| Method | Returns | Notes |
|:-------|:--------|:------|
| `softDelete(id)` | `true` | Vector was deleted |
| `softDelete(id)` | `false` | Vector was already deleted (idempotent) |
| `softDelete(id)` | Error | Vector ID doesn't exist |
| `isDeleted(id)` | `true` | Vector is tombstoned |
| `isDeleted(id)` | `false` | Vector is live |
| `isDeleted(id)` | Error | Vector ID doesn't exist |

### Example: Single Delete

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Create index and insert vectors
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);

const id1 = index.insert(new Float32Array(128).fill(0.1));
const id2 = index.insert(new Float32Array(128).fill(0.2));
const id3 = index.insert(new Float32Array(128).fill(0.3));

console.log('Live count:', index.liveCount());  // 3

// Soft delete vector
const wasDeleted = index.softDelete(id2);
console.log('Was deleted:', wasDeleted);        // true
console.log('Is deleted:', index.isDeleted(id2)); // true

// Check counts
console.log('Live count:', index.liveCount());     // 2
console.log('Deleted count:', index.deletedCount()); // 1

// Search - deleted vectors are excluded
const results = index.search(query, 10);
// id2 will not appear in results
```

### Example: Batch Delete

```javascript
// Delete multiple vectors efficiently
const ids = new Uint32Array([1, 3, 5, 7, 9]);
const result = index.softDeleteBatch(ids);

console.log('Deleted:', result.deleted);           // Number newly deleted
console.log('Already deleted:', result.alreadyDeleted); // Already tombstoned
console.log('Invalid IDs:', result.invalidIds);    // IDs not found
console.log('All valid:', result.allValid());      // true if no invalid IDs
```

### Why Soft Delete?

| Benefit | Explanation |
|:--------|:------------|
| **O(1) deletion** | No graph restructuring needed |
| **Search integrity** | Graph connectivity maintained for routing |
| **Undo capability** | Can restore before compaction |
| **Batch efficiency** | No per-delete graph repair |

---

## Compaction

Compaction rebuilds the index without tombstones, reclaiming memory.

### API

```typescript
// Check if compaction is recommended
const needsCompaction: boolean = index.needsCompaction();

// Get/set compaction threshold
const threshold: number = index.compactionThreshold(); // Default: 0.3 (30%)
index.setCompactionThreshold(0.5); // Trigger at 50% deleted

// Get warning message
const warning: string | undefined = index.compactionWarning();

// Perform compaction
const result: WasmCompactionResult = index.compact();
```

### Compaction Result

```typescript
interface WasmCompactionResult {
    tombstones_removed: number;  // Deleted vectors removed
    new_size: number;            // Live vectors remaining
    duration_ms: number;         // Time taken
}
```

### When to Compact

| Scenario | Recommended Action |
|:---------|:-------------------|
| `tombstoneRatio() > 0.3` | Compact (default threshold) |
| Memory pressure | Compact to free space |
| After bulk deletions | Compact to reclaim space |
| Before persistence | Optional (reduces snapshot size) |

### Example: Compaction Workflow

```javascript
// Check if compaction is needed
if (index.needsCompaction()) {
    console.log('Tombstone ratio:', index.tombstoneRatio());
    console.log('Warning:', index.compactionWarning());

    // Perform compaction
    const result = index.compact();

    console.log('Removed:', result.tombstones_removed, 'tombstones');
    console.log('New size:', result.new_size, 'vectors');
    console.log('Duration:', result.duration_ms, 'ms');
}
```

### Compaction Considerations

| Factor | Impact |
|:-------|:-------|
| **Blocking** | Compaction is synchronous; UI may freeze |
| **Memory** | Temporarily uses 2x memory during rebuild |
| **ID Preservation** | Vector IDs are preserved after compaction |
| **Metadata** | Metadata is preserved after compaction |

**Recommendation:** For large indices (>10k vectors), warn users before compacting or perform during idle time.

---

## Persistence

EdgeVec supports saving and loading the database to/from IndexedDB (browser) or files (Node.js).

### API

```typescript
// Save to IndexedDB (browser)
await index.save('my-database');

// Load from IndexedDB (browser)
const loaded = await EdgeVec.load('my-database');

// Streaming save (for large databases)
const iterator = index.save_stream(10_000_000); // 10MB chunks
let chunk;
while ((chunk = iterator.next_chunk()) !== undefined) {
    // Process chunk (e.g., send to server, write to file)
}
```

### Browser Persistence (IndexedDB)

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

// Create and populate index
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);
// ... insert vectors ...

// Save to IndexedDB
await index.save('my-vectors-db');
console.log('Saved to IndexedDB');

// Later: Load from IndexedDB
const loadedIndex = await EdgeVec.load('my-vectors-db');
console.log('Loaded', loadedIndex.liveCount(), 'vectors');
```

### Streaming Save (Large Databases)

For databases larger than available memory, use streaming:

```javascript
// Stream chunks to storage
const chunkSize = 10 * 1024 * 1024; // 10MB
const iterator = index.save_stream(chunkSize);

const chunks = [];
let chunk;
while ((chunk = iterator.next_chunk()) !== undefined) {
    chunks.push(chunk);
    console.log('Chunk:', chunk.length, 'bytes');
}

// Combine chunks for storage
const totalSize = chunks.reduce((sum, c) => sum + c.length, 0);
console.log('Total size:', totalSize, 'bytes');
```

### Persistence Format

The snapshot format includes:

| Component | Contents |
|:----------|:---------|
| **Header** | Magic number, version, checksum |
| **Config** | Dimensions, HNSW parameters |
| **Vectors** | Raw vector data |
| **Graph** | HNSW node and neighbor data |
| **Metadata** | Key-value pairs per vector |
| **Tombstones** | Soft delete markers |

### Example: Complete Workflow

```javascript
import init, { EdgeVec, EdgeVecConfig, JsMetadataValue } from 'edgevec';

await init();

// 1. Create database
const config = new EdgeVecConfig(128);
const index = new EdgeVec(config);

// 2. Insert vectors with metadata
for (let i = 0; i < 1000; i++) {
    const vector = new Float32Array(128).fill(Math.random());
    const id = index.insert(vector);
    index.setMetadata(id, 'category', JsMetadataValue.fromString('test'));
    index.setMetadata(id, 'index', JsMetadataValue.fromInteger(i));
}

// 3. Soft delete some vectors
for (let i = 0; i < 100; i++) {
    index.softDelete(i + 1);
}

// 4. Compact before saving (optional but reduces size)
if (index.needsCompaction()) {
    const result = index.compact();
    console.log('Compacted:', result.tombstones_removed, 'tombstones');
}

// 5. Save to IndexedDB
await index.save('my-database');

// 6. Later: Load and verify
const loaded = await EdgeVec.load('my-database');
console.log('Vectors:', loaded.liveCount());
console.log('Metadata entries:', loaded.totalMetadataCount());
```

---

## Best Practices

### Soft Delete

1. **Batch deletions** - Use `softDeleteBatch()` for multiple vectors
2. **Monitor tombstone ratio** - Check `tombstoneRatio()` periodically
3. **Compact strategically** - Don't compact after every deletion

### Compaction

1. **Set appropriate threshold** - Default 30% is good for most cases
2. **Warn users** - Compaction can freeze UI for large indices
3. **Compact before persistence** - Reduces snapshot size

### Persistence

1. **Save regularly** - Prevent data loss on browser close
2. **Handle errors** - IndexedDB can fail (quota, corruption)
3. **Use unique names** - Avoid overwriting unrelated databases
4. **Check before load** - Handle missing databases gracefully

---

## Error Handling

### Soft Delete Errors

```javascript
try {
    index.softDelete(999999); // Non-existent ID
} catch (e) {
    console.error('Delete failed:', e);
    // Handle: Vector ID doesn't exist
}
```

### Compaction Errors

```javascript
try {
    const result = index.compact();
} catch (e) {
    console.error('Compaction failed:', e);
    // Handle: Memory allocation error, etc.
}
```

### Persistence Errors

```javascript
try {
    await index.save('my-db');
} catch (e) {
    console.error('Save failed:', e);
    // Handle: IndexedDB quota exceeded, etc.
}

try {
    const loaded = await EdgeVec.load('my-db');
} catch (e) {
    console.error('Load failed:', e);
    // Handle: Database not found, corrupted, etc.
}
```

---

## See Also

- [Filter Syntax Reference](FILTER_SYNTAX.md)
- [TypeScript API](TYPESCRIPT_API.md)
- [Error Reference](ERROR_REFERENCE.md)
