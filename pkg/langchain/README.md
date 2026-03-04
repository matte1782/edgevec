# edgevec-langchain

> LangChain.js VectorStore adapter for EdgeVec — in-browser vector search with HNSW, persistence, and metadata filtering.

[![npm version](https://img.shields.io/npm/v/edgevec-langchain)](https://www.npmjs.com/package/edgevec-langchain)
[![license](https://img.shields.io/npm/l/edgevec-langchain)](https://github.com/panzerimatteo/edgevec/blob/main/LICENSE)

## Why EdgeVec?

| Feature | edgevec-langchain | MemoryVectorStore | Voy |
|:--------|:-----------------:|:-----------------:|:---:|
| Algorithm | HNSW | Brute-force | IVFFlat |
| Persistence (IndexedDB) | Yes | No | No |
| Metadata filtering (DSL) | Yes | No | No |
| Binary quantization | Yes | No | No |
| Maintained (2026) | Yes | Yes | No (archived) |
| Bundle size | ~8KB (adapter) | Built-in | ~50KB |

## Installation

```bash
npm install edgevec-langchain edgevec @langchain/core
```

**Peer dependencies:**

| Package | Version |
|:--------|:--------|
| `edgevec` | `^0.9.0` |
| `@langchain/core` | `>=0.3.0 <0.5.0` |

**Requires** Node.js >= 20.0.0.

## Quick Start

```typescript
import { EdgeVecStore } from "edgevec-langchain";
import { OpenAIEmbeddings } from "@langchain/openai";

const embeddings = new OpenAIEmbeddings();
const store = await EdgeVecStore.fromTexts(
  ["EdgeVec runs in the browser", "It uses HNSW for fast search"],
  [{ source: "docs" }, { source: "docs" }],
  embeddings,
  { dimensions: 1536 }
);
const results = await store.similaritySearch("browser vector search", 2);
console.log(results);
```

### With FilterExpression

```typescript
import { EdgeVecStore, Filter } from "edgevec-langchain";
import { OpenAIEmbeddings } from "@langchain/openai";

const embeddings = new OpenAIEmbeddings();
const store = await EdgeVecStore.fromTexts(
  ["NVIDIA RTX 4090", "AMD RX 7900 XTX", "Intel Arc A770"],
  [
    { category: "gpu", price: 1599 },
    { category: "gpu", price: 949 },
    { category: "gpu", price: 349 },
  ],
  embeddings,
  { dimensions: 1536 }
);

const results = await store.similaritySearch(
  "fast GPU for gaming",
  5,
  Filter.and(
    Filter.eq("category", "gpu"),
    Filter.lt("price", 500)
  )
);
```

## API Reference

### `EdgeVecStore`

The main class. Extends LangChain's `SaveableVectorStore`.

#### Constructor

```typescript
new EdgeVecStore(embeddings: EmbeddingsInterface, config: EdgeVecStoreConfig)
```

WASM must be initialized before calling the constructor. Use factory methods for auto-initialization.

**Throws:** `EdgeVecNotInitializedError` if WASM is not initialized.

#### `EdgeVecStoreConfig`

Extends `IndexConfig` from `edgevec` with an adapter-level `metric` field:

```typescript
interface EdgeVecStoreConfig extends IndexConfig {
  /** Vector dimensions (required) */
  dimensions: number;
  /** HNSW M parameter — connections per node (optional) */
  m?: number;
  /** HNSW ef_construction parameter (optional) */
  efConstruction?: number;
  /** Enable binary quantization for smaller memory footprint (optional) */
  quantized?: boolean;
  /** Distance metric for score normalization (adapter-only, not passed to EdgeVec). Default: "cosine" */
  metric?: "cosine" | "l2" | "dotproduct";
}
```

#### Static Factory Methods

##### `EdgeVecStore.fromTexts(texts, metadatas, embeddings, config)`

```typescript
static async fromTexts(
  texts: string[],
  metadatas: object[] | object,
  embeddings: EmbeddingsInterface,
  config: EdgeVecStoreConfig
): Promise<EdgeVecStore>
```

Create a store from text strings. Auto-initializes WASM. Pass a single metadata object to apply it to all texts, or an array with one entry per text.

**Throws:** `Error` if `metadatas` is an array whose length does not match `texts`.

##### `EdgeVecStore.fromDocuments(docs, embeddings, config)`

```typescript
static async fromDocuments(
  docs: DocumentInterface[],
  embeddings: EmbeddingsInterface,
  config: EdgeVecStoreConfig
): Promise<EdgeVecStore>
```

Create a store from LangChain `Document` objects. Auto-initializes WASM.

##### `EdgeVecStore.load(directory, embeddings)`

```typescript
static async load(
  directory: string,
  embeddings: EmbeddingsInterface
): Promise<EdgeVecStore>
```

Load a previously saved store from IndexedDB. Auto-initializes WASM.

**Throws:** `EdgeVecPersistenceError` if no data found or data is corrupted.

#### Instance Methods

##### `addDocuments(documents, options?)`

```typescript
async addDocuments(
  documents: DocumentInterface[],
  options?: { ids?: string[] }
): Promise<string[]>
```

Embed and add documents to the store. Returns assigned string IDs.

##### `addVectors(vectors, documents, options?)`

```typescript
async addVectors(
  vectors: number[][],
  documents: DocumentInterface[],
  options?: { ids?: string[] }
): Promise<string[]>
```

Add precomputed embedding vectors with their documents. Returns assigned string IDs.

**Throws:** `Error` on dimension mismatch or metadata serialization failure.

##### `similaritySearch(query, k, filter?)`

```typescript
async similaritySearch(
  query: string,
  k: number,
  filter?: string | FilterExpression
): Promise<DocumentInterface[]>
```

Search for documents similar to a text query. Returns `Document` instances (which implement `DocumentInterface`). Inherited from LangChain `VectorStore`.

##### `similaritySearchVectorWithScore(query, k, filter?)`

```typescript
async similaritySearchVectorWithScore(
  query: number[],
  k: number,
  filter?: string | FilterExpression
): Promise<[DocumentInterface, number][]>
```

Search by embedding vector. Returns `[document, score]` tuples where score is a normalized similarity value (higher = more similar). See [Score Normalization](#score-normalization).

##### `delete(params)`

```typescript
async delete(params: { ids: string[] }): Promise<void>
```

Delete documents by their string IDs. Unknown IDs are silently ignored. Call `save()` after deletion to persist the change to IndexedDB.

##### `save(directory)`

```typescript
async save(directory: string): Promise<void>
```

Persist the store to IndexedDB. The `directory` string is used as the IndexedDB key.

##### `asRetriever(kOrFields?)`

Returns a LangChain `VectorStoreRetriever`. Inherited from `VectorStore`.

```typescript
// Simple form — pass k directly
const retriever = store.asRetriever(5);
const docs = await retriever.invoke("search query");

// Object form — pass k, filter, tags, callbacks, etc.
const retriever = store.asRetriever({ k: 5, filter: 'category = "gpu"' });
const docs = await retriever.invoke("search query");
```

### `initEdgeVec()`

```typescript
async function initEdgeVec(): Promise<void>
```

Initialize the EdgeVec WASM module. Safe to call multiple times — subsequent calls are no-ops. Concurrent calls await the same promise.

Factory methods (`fromTexts`, `fromDocuments`, `load`) call this automatically.

### `EdgeVecNotInitializedError`

Thrown when `EdgeVecStore` is used before WASM initialization. Call `initEdgeVec()` or use a factory method.

### `MetadataSerializationError`

Thrown when metadata contains circular references or values that cannot be serialized.

### `EdgeVecPersistenceError`

Thrown when `save()` or `load()` encounters an IndexedDB error or corrupted data.

### `serializeMetadata(metadata)` / `deserializeMetadata(metadata)`

```typescript
function serializeMetadata(metadata: Record<string, unknown>): Metadata
function deserializeMetadata(metadata: Metadata): Record<string, unknown>
```

Advanced: Convert between LangChain's flexible metadata and EdgeVec's `MetadataValue` types (`string | number | boolean | string[]`). Unsupported types are JSON-stringified and tracked for lossless round-tripping.

Most users do not need these — `EdgeVecStore` handles serialization internally.

## Advanced Usage

### RAG Retrieval Chain

Use `EdgeVecStore` as a filtered retriever in a LangChain RAG pipeline:

```typescript
import { EdgeVecStore, Filter } from "edgevec-langchain";
import { OpenAIEmbeddings } from "@langchain/openai";
import { ChatOpenAI } from "@langchain/openai";
import { createRetrievalChain } from "langchain/chains/retrieval";
import { createStuffDocumentsChain } from "langchain/chains/combine_documents";

// Build a filtered retriever
const embeddings = new OpenAIEmbeddings();
const store = await EdgeVecStore.fromDocuments(docs, embeddings, { dimensions: 1536 });
const retriever = store.asRetriever({
  k: 5,
  filter: Filter.and(
    Filter.eq("status", "published"),
    Filter.ge("updated_at", "2026-01-01")
  ),
});

// Use in a RAG chain
const llm = new ChatOpenAI({ model: "gpt-4" });
const chain = await createRetrievalChain({
  retriever,
  combineDocsChain: await createStuffDocumentsChain({ llm }),
});
const response = await chain.invoke({ input: "How does HNSW work?" });
```

## Filtering

EdgeVec supports two filter styles: **DSL strings** and **`FilterExpression` objects**.

### FilterExpression (Programmatic)

Build filters programmatically using the `Filter` API — no string parsing, full type safety:

```typescript
import { Filter, EdgeVecStore } from "edgevec-langchain";

// Simple equality
const docs = await store.similaritySearch("query", 5, Filter.eq("category", "gpu"));

// AND combination
const docs = await store.similaritySearch(
  "query", 5,
  Filter.and(
    Filter.eq("category", "gpu"),
    Filter.lt("price", 500)
  )
);

// OR combination
const docs = await store.similaritySearch(
  "query", 5,
  Filter.or(
    Filter.eq("brand", "nvidia"),
    Filter.eq("brand", "amd")
  )
);

// Complex: range + NOT + nested logic
const docs = await store.similaritySearch(
  "query", 5,
  Filter.and(
    Filter.between("price", 100, 500),
    Filter.not(Filter.eq("status", "discontinued")),
    Filter.any("tags", "gaming")
  )
);
```

### Filter API Quick Reference

| Method | Description | Example |
|:-------|:------------|:--------|
| `Filter.eq(field, value)` | Equality | `Filter.eq("status", "active")` |
| `Filter.ne(field, value)` | Not equal | `Filter.ne("type", "draft")` |
| `Filter.lt(field, value)` | Less than | `Filter.lt("price", 100)` |
| `Filter.le(field, value)` | Less or equal | `Filter.le("rating", 5)` |
| `Filter.gt(field, value)` | Greater than | `Filter.gt("score", 0.8)` |
| `Filter.ge(field, value)` | Greater or equal | `Filter.ge("count", 10)` |
| `Filter.between(field, lo, hi)` | Range | `Filter.between("price", 10, 50)` |
| `Filter.contains(field, sub)` | String contains | `Filter.contains("name", "pro")` |
| `Filter.startsWith(field, pre)` | String prefix | `Filter.startsWith("sku", "GPU")` |
| `Filter.endsWith(field, suf)` | String suffix | `Filter.endsWith("file", ".pdf")` |
| `Filter.like(field, pat)` | LIKE pattern | `Filter.like("name", "GPU_%")` |
| `Filter.in(field, values)` | Value in set | `Filter.in("cat", ["a","b"])` |
| `Filter.notIn(field, values)` | Value not in set | `Filter.notIn("cat", ["x"])` |
| `Filter.any(field, values)` | Array has any | `Filter.any("tags", "rust")` |
| `Filter.allOf(field, values)` | Array has all | `Filter.allOf("tags", ["a","b"])` |
| `Filter.none(field, values)` | Array has none | `Filter.none("tags", ["old"])` |
| `Filter.isNull(field)` | Field is null | `Filter.isNull("deleted_at")` |
| `Filter.isNotNull(field)` | Field exists | `Filter.isNotNull("email")` |
| `Filter.and(...filters)` | Logical AND | `Filter.and(f1, f2)` |
| `Filter.or(...filters)` | Logical OR | `Filter.or(f1, f2)` |
| `Filter.not(filter)` | Logical NOT | `Filter.not(f1)` |
| `Filter.matchAll` | Tautology (match all, getter) | `Filter.matchAll` |
| `Filter.nothing` | Contradiction (match none, getter) | `Filter.nothing` |
| `Filter.parse(dsl)` | Parse DSL string | `Filter.parse('x = 1')` |
| `Filter.tryParse(dsl)` | Parse, null on error | `Filter.tryParse('x = 1')` |
| `Filter.validate(dsl)` | Validate DSL | `Filter.validate('x = 1')` |

**Available `Filter` methods:** `eq`, `ne`, `lt`, `le`, `gt`, `ge`, `between`, `contains`, `startsWith`, `endsWith`, `like`, `in`, `notIn`, `any`, `allOf`, `none`, `isNull`, `isNotNull`, `and`, `or`, `not`, `parse`, `tryParse`, `validate`. **Getters:** `matchAll`, `nothing`.

### DSL Strings

Filters can also be passed as EdgeVec DSL strings:

```typescript
const docs = await store.similaritySearch("query", 5, 'category = "gpu" AND price < 500');
```

### Operator Reference

| Category | Operators |
|:---------|:----------|
| Comparison | `=`, `!=`, `<`, `>`, `<=`, `>=` |
| String | `CONTAINS`, `STARTS_WITH`, `ENDS_WITH`, `LIKE` |
| Array / Set | `IN`, `NOT IN`, `ANY`, `ALL`, `NONE` |
| Range | `BETWEEN ... AND ...` |
| Logical | `AND`, `OR`, `NOT` |
| Null | `IS NULL`, `IS NOT NULL` |

All string operators are **case-sensitive**.

### Comparison

```typescript
// Equality
const docs = await store.similaritySearch("query", 5, 'category = "gpu"');

// Inequality
const docs = await store.similaritySearch("query", 5, 'category != "cpu"');

// Numeric comparisons
const docs = await store.similaritySearch("query", 5, "price < 500");
const docs = await store.similaritySearch("query", 5, "rating >= 4.5");
```

### Logical operators

```typescript
// AND
const docs = await store.similaritySearch(
  "query", 5,
  'category = "gpu" AND price < 500'
);

// OR
const docs = await store.similaritySearch(
  "query", 5,
  'brand = "nvidia" OR brand = "amd"'
);

// NOT
const docs = await store.similaritySearch(
  "query", 5,
  'NOT (category = "deprecated")'
);

// Combined with parentheses
const docs = await store.similaritySearch(
  "query", 5,
  '(category = "gpu" AND price < 500) OR rating >= 4.5'
);
```

### Null checks

```typescript
// Field exists
const docs = await store.similaritySearch("query", 5, "description IS NOT NULL");

// Field missing
const docs = await store.similaritySearch("query", 5, "deprecated IS NULL");
```

### String operators

```typescript
const docs = await store.similaritySearch("query", 5, 'name CONTAINS "pro"');
const docs = await store.similaritySearch("query", 5, 'name STARTS_WITH "GPU"');
const docs = await store.similaritySearch("query", 5, 'filename ENDS_WITH ".pdf"');
const docs = await store.similaritySearch("query", 5, 'name LIKE "GPU_%"');
```

### Array / set operators

```typescript
// Value in set
const docs = await store.similaritySearch(
  "query", 5,
  'category IN ["gpu", "cpu", "ram"]'
);

// Array field contains any of the values
const docs = await store.similaritySearch(
  "query", 5,
  'tags ANY ["rust", "wasm"]'
);

// Array field contains all values
const docs = await store.similaritySearch(
  "query", 5,
  'tags ALL ["rust", "wasm"]'
);

// Value not in set
const docs = await store.similaritySearch(
  "query", 5,
  'category NOT IN ["deprecated", "archived"]'
);

// Array field contains none of the values
const docs = await store.similaritySearch(
  "query", 5,
  'tags NONE ["deprecated"]'
);
```

### Range

```typescript
const docs = await store.similaritySearch("query", 5, "price BETWEEN 100 AND 500");
```

## WASM Initialization

EdgeVec is a Rust library compiled to WebAssembly. The WASM module must be initialized before use.

### Auto-initialization (recommended)

Factory methods handle WASM init automatically:

```typescript
// fromTexts, fromDocuments, and load all call initEdgeVec() internally
const store = await EdgeVecStore.fromTexts(texts, metadatas, embeddings, config);
```

### Manual initialization

Use when constructing `EdgeVecStore` directly:

```typescript
import { initEdgeVec, EdgeVecStore } from "edgevec-langchain";

await initEdgeVec();
const store = new EdgeVecStore(embeddings, { dimensions: 384 });
await store.addDocuments(docs);
```

### Error handling

If WASM is not initialized, any `EdgeVecStore` method throws `EdgeVecNotInitializedError`:

```typescript
import { EdgeVecNotInitializedError } from "edgevec-langchain";

try {
  const store = new EdgeVecStore(embeddings, { dimensions: 384 });
} catch (e) {
  if (e instanceof EdgeVecNotInitializedError) {
    // Call initEdgeVec() first, or use a factory method
  }
}
```

## Persistence (IndexedDB)

`EdgeVecStore` persists to IndexedDB — the browser's built-in key-value database. Both the HNSW index (vectors, metadata, graph) and the adapter's ID mapping are saved.

### Save

```typescript
await store.save("my-index");
```

### Load

```typescript
import { EdgeVecStore } from "edgevec-langchain";

const store = await EdgeVecStore.load("my-index", embeddings);
// store is fully restored — same vectors, metadata, and IDs
```

### What gets persisted

| Data | Storage |
|:-----|:--------|
| Vectors + HNSW graph | EdgeVec IndexedDB (via `edgevec`) |
| Document metadata | EdgeVec IndexedDB (via `edgevec`) |
| String ID mapping | `edgevec_meta` IndexedDB |
| Metric + dimensions | `edgevec_meta` IndexedDB |

## Score Normalization

Raw distances from EdgeVec are normalized to similarity scores where **higher = more similar**.

| Metric | Formula | Output Range |
|:-------|:--------|:-------------|
| `cosine` | `max(0, min(1, 1 - distance))` | [0, 1] |
| `l2` | `1 / (1 + distance)` | (0, 1] |
| `dotproduct` | `1 / (1 + exp(distance))` | (0, 1) |

For `dotproduct`, EdgeVec returns the negative inner product as the "distance", so this formula yields higher scores for higher dot products (sigmoid of the negated distance).

Non-finite scores (NaN, Infinity) are clamped to `0`.

## License

[MIT](https://github.com/panzerimatteo/edgevec/blob/main/LICENSE)
