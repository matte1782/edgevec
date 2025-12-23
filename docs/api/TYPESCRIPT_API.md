# TypeScript API Reference

**Version:** EdgeVec v0.6.0
**Last Updated:** 2025-12-22

---

## Installation

```bash
npm install edgevec
```

## Quick Start

```typescript
import init, { EdgeVec, EdgeVecConfig, JsMetadataValue } from 'edgevec';

// Initialize WASM module
await init();

// Create index
const config = new EdgeVecConfig(128);  // 128 dimensions
const index = new EdgeVec(config);

// Insert vector
const vector = new Float32Array(128).fill(0.5);
const id = index.insert(vector);

// Add metadata
index.setMetadata(id, 'category', JsMetadataValue.fromString('example'));
index.setMetadata(id, 'score', JsMetadataValue.fromFloat(0.95));

// Search
const query = new Float32Array(128).fill(0.5);
const results = index.search(query, 10);

// Filtered search
const filtered = JSON.parse(index.searchFiltered(query, 10, JSON.stringify({
    filter: 'category = "example" AND score > 0.9',
    strategy: 'auto'
})));
```

---

## Classes

### EdgeVecConfig

Configuration for creating an EdgeVec index.

```typescript
class EdgeVecConfig {
    constructor(dimensions: number);

    dimensions: number;
    set metric(value: string);        // "l2" | "cosine" | "dot"
    set m(value: number);             // Max connections (default: 16)
    set m0(value: number);            // Max connections layer 0 (default: 32)
    set ef_construction(value: number); // Build quality (default: 200)
    set ef_search(value: number);     // Search quality (default: 50)
}
```

**Example:**

```typescript
const config = new EdgeVecConfig(768);
config.metric = "cosine";
config.m = 32;
config.ef_construction = 400;
config.ef_search = 100;

const index = new EdgeVec(config);
```

---

### EdgeVec

Main index class for vector operations.

#### Constructor

```typescript
constructor(config: EdgeVecConfig);
```

#### Vector Operations

```typescript
// Insert a single vector
insert(vector: Float32Array): number;

// Search for k nearest neighbors
search(query: Float32Array, k: number): SearchResult[];

// Filtered search
searchFiltered(query: Float32Array, k: number, options_json: string): string;

// Batch insert (array of Float32Array)
insertBatch(vectors: Array<Float32Array>, config?: BatchInsertConfig): BatchInsertResult;

// Batch insert with progress callback
insertBatchWithProgress(
    vectors: Array<Float32Array>,
    onProgress: (inserted: number, total: number) => void
): BatchInsertResult;
```

#### Soft Delete Operations

```typescript
// Soft delete a single vector
softDelete(vector_id: number): boolean;

// Batch soft delete
softDeleteBatch(ids: Uint32Array): WasmBatchDeleteResult;

// Check if vector is deleted
isDeleted(vector_id: number): boolean;

// Get counts
liveCount(): number;
deletedCount(): number;
tombstoneRatio(): number;
```

#### Compaction

```typescript
// Check if compaction is recommended
needsCompaction(): boolean;

// Get compaction warning message
compactionWarning(): string | undefined;

// Perform compaction
compact(): WasmCompactionResult;

// Get/set threshold
compactionThreshold(): number;
setCompactionThreshold(ratio: number): void;
```

#### Metadata Operations

```typescript
// Set metadata value
setMetadata(vector_id: number, key: string, value: JsMetadataValue): void;

// Get metadata value
getMetadata(vector_id: number, key: string): JsMetadataValue | undefined;

// Get all metadata for a vector
getAllMetadata(vector_id: number): object | undefined;

// Check if metadata key exists
hasMetadata(vector_id: number, key: string): boolean;

// Delete metadata
deleteMetadata(vector_id: number, key: string): boolean;
deleteAllMetadata(vector_id: number): boolean;

// Metadata counts
metadataKeyCount(vector_id: number): number;
totalMetadataCount(): number;
metadataVectorCount(): number;
```

#### Persistence

```typescript
// Save to IndexedDB
save(name: string): Promise<void>;

// Load from IndexedDB
static load(name: string): Promise<EdgeVec>;

// Streaming save (for large databases)
save_stream(chunk_size?: number): PersistenceIterator;
```

---

### JsMetadataValue

Wrapper for metadata values with type safety.

#### Factory Methods

```typescript
static fromString(value: string): JsMetadataValue;
static fromInteger(value: number): JsMetadataValue;
static fromFloat(value: number): JsMetadataValue;
static fromBoolean(value: boolean): JsMetadataValue;
static fromStringArray(value: string[]): JsMetadataValue;
```

#### Type Checking

```typescript
isString(): boolean;
isInteger(): boolean;
isFloat(): boolean;
isBoolean(): boolean;
isStringArray(): boolean;
getType(): 'string' | 'integer' | 'float' | 'boolean' | 'string_array';
```

#### Value Extraction

```typescript
asString(): string | undefined;
asInteger(): number | undefined;
asFloat(): number | undefined;
asBoolean(): boolean | undefined;
asStringArray(): string[] | undefined;
toJS(): string | number | boolean | string[];
```

**Example:**

```typescript
// Setting metadata
index.setMetadata(id, 'title', JsMetadataValue.fromString('My Document'));
index.setMetadata(id, 'page_count', JsMetadataValue.fromInteger(42));
index.setMetadata(id, 'score', JsMetadataValue.fromFloat(0.95));
index.setMetadata(id, 'verified', JsMetadataValue.fromBoolean(true));
index.setMetadata(id, 'tags', JsMetadataValue.fromStringArray(['ai', 'ml']));

// Getting metadata
const title = index.getMetadata(id, 'title');
if (title && title.isString()) {
    console.log('Title:', title.asString());
}

// Get all as plain object
const allMeta = index.getAllMetadata(id);
console.log(allMeta);  // { title: 'My Document', page_count: 42, ... }
```

---

### BatchInsertConfig

Configuration for batch insertion.

```typescript
class BatchInsertConfig {
    constructor();
    validateDimensions: boolean;  // Default: true
}
```

---

### BatchInsertResult

Result from batch insertion.

```typescript
class BatchInsertResult {
    readonly total: number;         // Total vectors attempted
    readonly inserted: number;      // Successfully inserted
    readonly ids: BigUint64Array;   // IDs of inserted vectors
}
```

---

### WasmBatchDeleteResult

Result from batch deletion.

```typescript
class WasmBatchDeleteResult {
    readonly deleted: number;        // Newly deleted
    readonly alreadyDeleted: number; // Already tombstoned
    readonly invalidIds: number;     // IDs not found
    readonly total: number;          // Input count
    readonly uniqueCount: number;    // Unique IDs

    anyDeleted(): boolean;           // At least one deleted
    allValid(): boolean;             // No invalid IDs
}
```

---

### WasmCompactionResult

Result from compaction operation.

```typescript
class WasmCompactionResult {
    readonly tombstones_removed: number;
    readonly new_size: number;
    readonly duration_ms: number;
}
```

---

## Filtered Search

### Search Options

```typescript
interface SearchOptions {
    filter?: string;                // Filter expression
    strategy?: 'auto' | 'pre' | 'post' | 'hybrid';
    oversampleFactor?: number;      // Default: 3.0
    includeMetadata?: boolean;      // Include metadata in results
    includeVectors?: boolean;       // Include vectors in results
}

// Usage
const result = JSON.parse(index.searchFiltered(query, k, JSON.stringify({
    filter: 'category = "gpu" AND price < 500',
    strategy: 'auto',
    includeMetadata: true
})));
```

### Search Result

```typescript
interface FilteredSearchResult {
    results: Array<{
        id: number;
        score: number;
        metadata?: object;
        vector?: number[];
    }>;
    complete: boolean;
    observedSelectivity: number;
    strategyUsed: string;
    vectorsEvaluated: number;
    filterTimeMs?: number;
    totalTimeMs?: number;
}
```

### Filter Strategies

| Strategy | Description | Best For |
|:---------|:------------|:---------|
| `auto` | Automatically select based on estimated selectivity | Default choice |
| `pre` | Filter first, then search | High selectivity (few matches) |
| `post` | Search first, then filter | Low selectivity (many matches) |
| `hybrid` | Oversample during search | Medium selectivity |

---

## Standalone Filter Functions

```typescript
// Parse filter expression (returns AST JSON)
function parse_filter_js(filter_str: string): string;

// Validate filter without full parse
function validate_filter_js(filter_str: string): string;

// Try to parse (returns null on error)
function try_parse_filter_js(filter_str: string): string | null;

// Get filter info (complexity, fields, operators)
function get_filter_info_js(filter_str: string): string;
```

**Example:**

```typescript
import { parse_filter_js, validate_filter_js } from 'edgevec';

// Validate user input
const validation = JSON.parse(validate_filter_js(userInput));
if (validation.valid) {
    // Safe to use
} else {
    console.error('Invalid filter:', validation.errors);
}

// Get filter complexity
const info = JSON.parse(get_filter_info_js('a = 1 AND b > 2'));
console.log('Complexity:', info.complexity);
console.log('Fields:', info.fields);
```

---

## Error Handling

### Error Codes

| Code | Category | Description |
|:-----|:---------|:------------|
| `E001-E099` | Syntax | Parse errors |
| `E101-E199` | Type | Type mismatches |
| `E201-E299` | Evaluation | Runtime errors |
| `E301-E399` | Limit | Resource limits |

### Handling Errors

```typescript
try {
    const result = parse_filter_js('invalid filter');
} catch (e) {
    const error = JSON.parse(e);
    console.log('Code:', error.code);       // "E001"
    console.log('Message:', error.message);
    console.log('Position:', error.position);
    console.log('Suggestion:', error.suggestion);
}
```

---

## Browser Usage

### ES Module

```html
<script type="module">
import init, { EdgeVec, EdgeVecConfig } from './edgevec.js';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // ... use index
}

main();
</script>
```

### IndexedDB Persistence

```typescript
// Save database
await index.save('my-vectors');

// Load database (on page reload)
try {
    const index = await EdgeVec.load('my-vectors');
    console.log('Loaded', index.liveCount(), 'vectors');
} catch (e) {
    // Database doesn't exist, create new
    const index = new EdgeVec(config);
}
```

---

## Node.js Usage

```typescript
// CommonJS
const { default: init, EdgeVec, EdgeVecConfig } = require('edgevec');

// ESM
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const config = new EdgeVecConfig(128);
    const index = new EdgeVec(config);

    // Node.js uses file system for persistence
    // IndexedDB not available in Node
}
```

---

## Performance Tips

1. **Batch insertions** - Use `insertBatch()` for multiple vectors
2. **Reuse queries** - Create query Float32Array once, reuse it
3. **Strategy selection** - Let `auto` strategy choose
4. **Compact strategically** - Don't compact after every deletion
5. **Save periodically** - Avoid data loss on browser close

---

## Complete Example

```typescript
import init, { EdgeVec, EdgeVecConfig, JsMetadataValue } from 'edgevec';

async function main() {
    // Initialize
    await init();

    // Create index
    const config = new EdgeVecConfig(128);
    config.metric = "cosine";
    const index = new EdgeVec(config);

    // Insert vectors with metadata
    const vectors = [
        { data: new Float32Array(128).fill(0.1), category: 'A', price: 100 },
        { data: new Float32Array(128).fill(0.2), category: 'A', price: 200 },
        { data: new Float32Array(128).fill(0.3), category: 'B', price: 150 },
    ];

    for (const v of vectors) {
        const id = index.insert(v.data);
        index.setMetadata(id, 'category', JsMetadataValue.fromString(v.category));
        index.setMetadata(id, 'price', JsMetadataValue.fromInteger(v.price));
    }

    // Filtered search
    const query = new Float32Array(128).fill(0.15);
    const result = JSON.parse(index.searchFiltered(query, 10, JSON.stringify({
        filter: 'category = "A" AND price < 150',
        strategy: 'auto',
        includeMetadata: true
    })));

    console.log('Results:', result.results);
    console.log('Strategy used:', result.strategyUsed);

    // Save to IndexedDB
    await index.save('my-index');

    // Later: Load from IndexedDB
    const loaded = await EdgeVec.load('my-index');
    console.log('Loaded', loaded.liveCount(), 'vectors');
}

main();
```

---

## See Also

- [Filter Syntax Reference](FILTER_SYNTAX.md)
- [Database Operations Guide](DATABASE_OPERATIONS.md)
- [Error Reference](ERROR_REFERENCE.md)
