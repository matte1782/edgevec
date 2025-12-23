# WasmIndex / EdgeVec API Reference

**Version:** EdgeVec v0.6.0
**Last Updated:** 2025-12-22

The `EdgeVec` class (exported as `WasmIndex` in some contexts) is the main entry point for EdgeVec in JavaScript/TypeScript.

---

## Constructor

```typescript
new EdgeVec(config: EdgeVecConfig): EdgeVec
```

### Configuration Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `dimensions` | `number` | Required | Vector dimension (must be divisible by 8 for BQ) |
| `metric` | `string` | `"l2"` | Distance metric: `"l2"`, `"cosine"`, `"dot"` |
| `m` | `number` | `16` | HNSW max connections per node |
| `m0` | `number` | `32` | HNSW max connections layer 0 |
| `ef_construction` | `number` | `200` | HNSW construction search width |
| `ef_search` | `number` | `50` | HNSW search width |

### Example

```typescript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

await init();

const config = new EdgeVecConfig(768);
config.metric = "cosine";
config.m = 32;
config.ef_construction = 400;
config.ef_search = 100;

const index = new EdgeVec(config);
```

---

## Insert Methods

### insert(vector)

Insert a vector without metadata.

```typescript
insert(vector: Float32Array): number
```

**Parameters:**
- `vector` - Float32Array of dimension matching config

**Returns:** `number` - VectorId assigned to the inserted vector

**Example:**

```typescript
const vector = new Float32Array(768).map(() => Math.random());
const id = index.insert(vector);
console.log('Inserted vector with ID:', id);
```

---

### insertWithMetadata(vector, metadata)

Insert a vector with associated metadata. **(v0.6.0)**

```typescript
insertWithMetadata(
  vector: Float32Array,
  metadata: Record<string, MetadataValue>
): number
```

**Parameters:**
- `vector` - Float32Array of dimension matching config
- `metadata` - Object with string keys and supported value types

**Metadata Value Types:**
- `string` - Text values
- `number` - Numeric values (integer or float)
- `boolean` - Boolean flags
- `string[]` - Array of strings (for tags)

**Returns:** `number` - VectorId assigned to the inserted vector

**Example:**

```typescript
const vector = new Float32Array(768).map(() => Math.random());
const id = index.insertWithMetadata(vector, {
    category: "electronics",
    price: 299.99,
    inStock: true,
    tags: ["featured", "sale"]
});
```

---

### insertBatch(vectors)

Batch insert multiple vectors.

```typescript
insertBatch(vectors: Float32Array[]): BatchInsertResult
```

**Returns:** `BatchInsertResult` with:
- `total: number` - Total vectors attempted
- `inserted: number` - Successfully inserted
- `ids: BigUint64Array` - IDs of inserted vectors

---

## Search Methods

### search(query, k)

Standard F32 vector search using HNSW.

```typescript
search(query: Float32Array, k: number): SearchResult[]
```

**Parameters:**
- `query` - Query vector (Float32Array)
- `k` - Number of nearest neighbors to return

**Returns:** Array of `SearchResult`:
```typescript
interface SearchResult {
    id: number;      // Vector ID
    score: number;   // Distance/similarity score
}
```

**Example:**

```typescript
const query = new Float32Array(768).map(() => Math.random());
const results = index.search(query, 10);

for (const result of results) {
    console.log(`ID: ${result.id}, Score: ${result.score}`);
}
```

---

### searchFiltered(query, filter, k)

Search with metadata filter expression. **(v0.6.0)**

```typescript
searchFiltered(
  query: Float32Array,
  filter: string,
  k: number
): SearchResult[]
```

**Parameters:**
- `query` - Query vector
- `filter` - Filter expression string (see [Filter Syntax](./FILTER_SYNTAX.md))
- `k` - Number of results

**Returns:** Array of `SearchResult`

**Example:**

```typescript
// Simple equality
const results = index.searchFiltered(query, 'category = "electronics"', 10);

// Comparison
const results = index.searchFiltered(query, 'price > 100 AND price < 500', 10);

// Array membership
const results = index.searchFiltered(query, 'tags ANY ["featured"]', 10);

// Complex expression
const results = index.searchFiltered(query,
    '(category = "electronics" OR category = "computers") AND inStock = true',
    10
);
```

---

### searchBQ(query, k)

Fast binary quantization search using Hamming distance. **(v0.6.0)**

```typescript
searchBQ(query: Float32Array, k: number): SearchResult[]
```

**Requirements:**
- Dimension must be divisible by 8
- BQ is auto-enabled for compatible dimensions

**Performance Characteristics:**
- ~5x faster than F32 search
- ~85% recall
- 32x memory reduction

**Example:**

```typescript
const results = index.searchBQ(query, 10);
```

---

### searchBQRescored(query, k, rescoreFactor)

BQ search with F32 rescoring for higher recall. **(v0.6.0)**

```typescript
searchBQRescored(
  query: Float32Array,
  k: number,
  rescoreFactor: number
): SearchResult[]
```

**Parameters:**
- `query` - Query vector
- `k` - Number of final results
- `rescoreFactor` - Multiplier for BQ candidates (higher = better recall, slower)

**Rescore Factor Guide:**

| Factor | Recall | Speed |
|--------|--------|-------|
| 1 | ~70% | 5x |
| 3 | ~90% | 3x |
| 5 | ~95% | 2.5x |
| 10 | ~98% | 2x |

**Example:**

```typescript
// High recall, good speed
const results = index.searchBQRescored(query, 10, 5);  // ~95% recall
```

---

### searchHybrid(query, options)

Flexible search combining BQ speed with metadata filtering. **(v0.6.0)**

```typescript
searchHybrid(
  query: Float32Array,
  options: HybridSearchOptions
): SearchResult[]

interface HybridSearchOptions {
  k: number;
  filter?: string;        // Optional filter expression
  useBQ?: boolean;        // Default: true if BQ enabled
  rescoreFactor?: number; // Default: 3
}
```

**Example:**

```typescript
const results = index.searchHybrid(query, {
    k: 10,
    filter: 'category = "news" AND score > 0.5',
    useBQ: true,
    rescoreFactor: 5
});
```

---

## Metadata Methods

### getMetadata(id)

Get metadata for a vector. **(v0.6.0)**

```typescript
getMetadata(id: number): Record<string, MetadataValue> | null
```

**Returns:** Metadata object or `null` if vector not found or deleted.

**Example:**

```typescript
const metadata = index.getMetadata(id);
if (metadata) {
    console.log('Category:', metadata.category);
    console.log('Price:', metadata.price);
}
```

---

### setMetadata(id, key, value)

Set a single metadata field.

```typescript
setMetadata(id: number, key: string, value: JsMetadataValue): void
```

**Example:**

```typescript
import { JsMetadataValue } from 'edgevec';

index.setMetadata(id, 'category', JsMetadataValue.fromString('updated'));
index.setMetadata(id, 'price', JsMetadataValue.fromFloat(199.99));
```

---

### getAllMetadata(id)

Get all metadata for a vector as a plain JavaScript object.

```typescript
getAllMetadata(id: number): object | null
```

---

### deleteMetadata(id, key)

Delete a single metadata field.

```typescript
deleteMetadata(id: number, key: string): boolean
```

---

## Memory Methods (v0.6.0)

### getMemoryPressure()

Get current memory usage and pressure level.

```typescript
getMemoryPressure(): MemoryPressure

interface MemoryPressure {
  level: 'normal' | 'warning' | 'critical';
  usedBytes: number;
  totalBytes: number;
  usagePercent: number;
}
```

**Example:**

```typescript
const pressure = index.getMemoryPressure();
console.log(`Memory: ${pressure.usagePercent.toFixed(1)}% (${pressure.level})`);
```

---

### canInsert()

Check if inserts are allowed based on memory pressure.

```typescript
canInsert(): boolean
```

**Returns:** `false` when memory is at critical level (>95% by default).

**Example:**

```typescript
if (index.canInsert()) {
    index.insert(vector);
} else {
    console.warn('Memory critical, cannot insert');
}
```

---

### setMemoryConfig(config)

Configure memory pressure thresholds.

```typescript
setMemoryConfig(config: MemoryConfig): void

interface MemoryConfig {
  warningThreshold?: number;   // Default: 0.70 (70%)
  criticalThreshold?: number;  // Default: 0.90 (90%)
  blockInsertsAtCritical?: boolean;  // Default: true
}
```

---

### getMemoryRecommendation()

Get actionable memory management guidance.

```typescript
getMemoryRecommendation(): string
```

**Returns:** Human-readable suggestion based on current memory state.

---

## Delete Methods

### softDelete(id)

Mark a vector as deleted (soft delete). O(1) operation.

```typescript
softDelete(id: number): boolean
```

**Returns:** `true` if newly deleted, `false` if already deleted.

---

### isDeleted(id)

Check if a vector is deleted.

```typescript
isDeleted(id: number): boolean
```

---

### compact()

Remove deleted vectors and reclaim memory.

```typescript
compact(): WasmCompactionResult

interface WasmCompactionResult {
  tombstones_removed: number;
  new_size: number;
  duration_ms: number;
}
```

**Warning:** This is a blocking operation for large indices.

---

### needsCompaction()

Check if tombstone ratio exceeds threshold.

```typescript
needsCompaction(): boolean
```

---

## Persistence Methods

### save(name)

Save to IndexedDB (browser) or filesystem (Node.js).

```typescript
save(name: string): Promise<void>
```

---

### load(name)

Load from storage.

```typescript
static load(name: string): Promise<EdgeVec>
```

**Example:**

```typescript
// Save
await index.save('my-vector-db');

// Load on page reload
try {
    const index = await EdgeVec.load('my-vector-db');
    console.log('Loaded', index.liveCount(), 'vectors');
} catch (e) {
    // Database doesn't exist
    const index = new EdgeVec(config);
}
```

---

### createSnapshot()

Create a binary snapshot for manual persistence.

```typescript
createSnapshot(): Uint8Array
```

---

### loadSnapshot(data)

Load from a binary snapshot.

```typescript
loadSnapshot(data: Uint8Array): void
```

---

## Utility Methods

### vectorCount()

Total vectors including deleted.

```typescript
vectorCount(): number
```

---

### liveCount()

Vectors excluding deleted.

```typescript
liveCount(): number
```

---

### deletedCount()

Count of tombstoned vectors.

```typescript
deletedCount(): number
```

---

### tombstoneRatio()

Ratio of deleted to total vectors.

```typescript
tombstoneRatio(): number  // 0.0 to 1.0
```

---

### hasBQ()

Check if binary quantization is enabled.

```typescript
hasBQ(): boolean
```

---

### enableBQ()

Enable binary quantization on this index. **(v0.6.0)**

Binary quantization reduces memory usage by 32x (from 32 bits to 1 bit per dimension)
while maintaining ~85-95% recall. BQ is required for `searchBQ()` and `searchBQRescored()`.

```typescript
enableBQ(): void
```

**Requirements:**
- Dimensions must be divisible by 8

**Example:**

```typescript
const db = new EdgeVec(config);
db.enableBQ();  // Enable BQ for faster search

// Insert vectors (BQ codes computed automatically)
db.insert(vector);

// Use BQ search
if (db.hasBQ()) {
    const results = db.searchBQ(query, 10);
}
```

---

## Error Handling

EdgeVec methods may throw errors for invalid operations:

```typescript
try {
    const results = index.searchFiltered(query, 'invalid filter syntax', 10);
} catch (e) {
    console.error('Search failed:', e.message);
}
```

See [Error Reference](./ERROR_REFERENCE.md) for error codes and solutions.

---

## See Also

- [Filter Syntax Reference](./FILTER_SYNTAX.md)
- [Memory Management Guide](./MEMORY.md)
- [TypeScript API](./TYPESCRIPT_API.md)
- [Error Reference](./ERROR_REFERENCE.md)
