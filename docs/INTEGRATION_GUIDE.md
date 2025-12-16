# EdgeVec Integration Guide

**Version:** 0.3.0
**Purpose:** Integrate EdgeVec with popular ML/embedding libraries
**Prerequisites:** Basic EdgeVec knowledge (see [Tutorial](./TUTORIAL.md))

---

## Table of Contents

1. [Overview](#overview)
2. [Transformers.js (@xenova/transformers)](#transformersjs-xenovatransformers)
3. [TensorFlow.js](#tensorflowjs)
4. [OpenAI Embeddings](#openai-embeddings)
5. [Cohere Embeddings](#cohere-embeddings)
6. [HuggingFace Inference API](#huggingface-inference-api)
7. [Custom Embedding Models](#custom-embedding-models)
8. [React/Vue/Svelte Integration](#reactvuesvelte-integration)
9. [Worker Thread Integration](#worker-thread-integration)
10. [Best Practices](#best-practices)

---

## Overview

EdgeVec is a **storage and retrieval layer** for vector embeddings. It does **not** generate embeddings itself — you need an embedding model/library for that.

**Typical workflow:**
```
Input (Text/Image)
       ↓
Embedding Model (transformers.js, TensorFlow.js, API)
       ↓
Vector (Float32Array)
       ↓
EdgeVec (Store/Search)
       ↓
Results (IDs + Scores)
```

**Key principle:** Use the **same** embedding model for both indexing and querying.

---

## Transformers.js (@xenova/transformers)

Transformers.js runs ML models entirely in the browser or Node.js — no server required.

### Installation

```bash
npm install @xenova/transformers edgevec
```

### Basic Usage

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    // Initialize both libraries
    await init();
    const embedder = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');

    // all-MiniLM-L6-v2 outputs 384 dimensions
    const config = new EdgeVecConfig(384);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Helper function to get embeddings
    async function embed(text) {
        const result = await embedder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    // Index documents
    const documents = [
        "EdgeVec is a high-performance vector database",
        "Machine learning transforms data into insights",
        "JavaScript enables interactive web applications",
        "WASM brings near-native performance to browsers",
        "Vector search powers semantic search and recommendations"
    ];

    const docIds = {};
    for (const doc of documents) {
        const embedding = await embed(doc);
        const id = index.insert(embedding);
        docIds[id] = doc;
    }

    console.log(`Indexed ${index.liveCount()} documents`);

    // Search
    const query = "fast vector database for browsers";
    const queryEmbedding = await embed(query);
    const results = index.search(queryEmbedding, 3);

    console.log(`\nQuery: "${query}"`);
    console.log("Results:");
    for (const result of results) {
        console.log(`  [${result.score.toFixed(3)}] ${docIds[result.id]}`);
    }
}

main().catch(console.error);
```

**Expected output:**
```
Indexed 5 documents

Query: "fast vector database for browsers"
Results:
  [0.234] EdgeVec is a high-performance vector database
  [0.456] WASM brings near-native performance to browsers
  [0.512] Vector search powers semantic search and recommendations
```

### Model Reference

| Model | Dimensions | Speed | Quality | Use Case |
|:------|:-----------|:------|:--------|:---------|
| `Xenova/all-MiniLM-L6-v2` | 384 | Fast | Good | General text, prototyping |
| `Xenova/all-mpnet-base-v2` | 768 | Medium | High | Production text search |
| `Xenova/gte-small` | 384 | Fast | Good | General purpose |
| `Xenova/bge-small-en-v1.5` | 384 | Fast | High | English text, high quality |
| `Xenova/e5-small-v2` | 384 | Fast | High | Retrieval tasks |

### Complete Semantic Search App

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

class SemanticSearch {
    constructor(modelName = 'Xenova/all-MiniLM-L6-v2') {
        this.modelName = modelName;
        this.embedder = null;
        this.index = null;
        this.documents = new Map();
    }

    async initialize() {
        await init();
        this.embedder = await pipeline('feature-extraction', this.modelName);

        // Determine dimensions from model
        const testEmbed = await this.embed("test");
        const dimensions = testEmbed.length;

        const config = new EdgeVecConfig(dimensions);
        config.metric = 'cosine';
        this.index = new EdgeVec(config);

        console.log(`Initialized with ${dimensions}-dim model`);
    }

    async embed(text) {
        const result = await this.embedder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    async add(id, text) {
        const embedding = await this.embed(text);
        const vectorId = this.index.insert(embedding);
        this.documents.set(vectorId, { id, text });
        return vectorId;
    }

    async addBatch(items) {
        const embeddings = [];
        const metadata = [];

        for (const { id, text } of items) {
            const embedding = await this.embed(text);
            embeddings.push(embedding);
            metadata.push({ id, text });
        }

        const result = this.index.insertBatch(embeddings);

        result.ids.forEach((vectorId, idx) => {
            this.documents.set(Number(vectorId), metadata[idx]);
        });

        return result;
    }

    async search(query, k = 10) {
        const queryEmbedding = await this.embed(query);
        const results = this.index.search(queryEmbedding, k);

        return results.map(r => ({
            score: r.score,
            ...this.documents.get(r.id)
        }));
    }

    async save(name) {
        await this.index.save(name);
        // Note: documents map should be saved separately
    }
}

// Usage
async function demo() {
    const search = new SemanticSearch();
    await search.initialize();

    await search.addBatch([
        { id: 'doc1', text: 'The quick brown fox jumps over the lazy dog' },
        { id: 'doc2', text: 'Machine learning is a subset of artificial intelligence' },
        { id: 'doc3', text: 'Vector databases enable semantic search capabilities' },
        { id: 'doc4', text: 'EdgeVec provides sub-millisecond vector search' },
    ]);

    const results = await search.search('fast similarity search', 2);
    console.log(results);
}

demo();
```

---

## TensorFlow.js

Use TensorFlow.js for Universal Sentence Encoder or custom models.

### Installation

```bash
npm install @tensorflow/tfjs @tensorflow-models/universal-sentence-encoder edgevec
```

### Usage with Universal Sentence Encoder

```javascript
import * as use from '@tensorflow-models/universal-sentence-encoder';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    // Load Universal Sentence Encoder
    const model = await use.load();
    console.log('USE model loaded');

    // USE outputs 512 dimensions
    const config = new EdgeVecConfig(512);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Embed and index documents
    const documents = [
        "EdgeVec is fast",
        "TensorFlow enables machine learning",
        "JavaScript runs everywhere"
    ];

    const embeddings = await model.embed(documents);
    const embeddingData = await embeddings.array();

    for (let i = 0; i < documents.length; i++) {
        const vector = new Float32Array(embeddingData[i]);
        const id = index.insert(vector);
        console.log(`Indexed "${documents[i]}" as ID ${id}`);
    }

    // Search
    const queryEmbedding = await model.embed(["fast database"]);
    const queryData = await queryEmbedding.array();
    const queryVector = new Float32Array(queryData[0]);

    const results = index.search(queryVector, 2);
    console.log('Results:', results);

    // Clean up tensors
    embeddings.dispose();
    queryEmbedding.dispose();
}

main().catch(console.error);
```

### Memory Management Note

TensorFlow.js uses WebGL tensors. Always dispose of them when done:

```javascript
const embedding = await model.embed(["text"]);
try {
    const data = await embedding.array();
    // Use data...
} finally {
    embedding.dispose();  // Always clean up!
}
```

---

## OpenAI Embeddings

For server-side applications using OpenAI's embedding API.

### Installation

```bash
npm install openai edgevec
```

### Usage

```javascript
import OpenAI from 'openai';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const openai = new OpenAI({
        apiKey: process.env.OPENAI_API_KEY
    });

    // text-embedding-3-small outputs 1536 dimensions
    const config = new EdgeVecConfig(1536);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Helper function
    async function embed(text) {
        const response = await openai.embeddings.create({
            model: 'text-embedding-3-small',
            input: text
        });
        return new Float32Array(response.data[0].embedding);
    }

    // Index documents
    const documents = [
        "EdgeVec runs locally with no API calls",
        "OpenAI provides powerful language models",
        "Vector search enables semantic retrieval"
    ];

    for (const doc of documents) {
        const embedding = await embed(doc);
        const id = index.insert(embedding);
        console.log(`Indexed: ${doc.substring(0, 30)}... (ID: ${id})`);
    }

    // Search
    const query = "local vector database";
    const queryEmbedding = await embed(query);
    const results = index.search(queryEmbedding, 2);

    console.log(`\nQuery: "${query}"`);
    for (const result of results) {
        console.log(`  Score: ${result.score.toFixed(3)}, ID: ${result.id}`);
    }
}

main().catch(console.error);
```

### OpenAI Model Reference

| Model | Dimensions | Context | Price |
|:------|:-----------|:--------|:------|
| `text-embedding-3-small` | 1536 | 8191 tokens | Lower |
| `text-embedding-3-large` | 3072 | 8191 tokens | Higher |
| `text-embedding-ada-002` | 1536 | 8191 tokens | Legacy |

**Note:** OpenAI embeddings require network calls and API costs. For fully offline usage, use Transformers.js instead.

---

## Cohere Embeddings

### Installation

```bash
npm install cohere-ai edgevec
```

### Usage

```javascript
import { CohereClient } from 'cohere-ai';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const cohere = new CohereClient({
        token: process.env.COHERE_API_KEY
    });

    // Cohere embed-english-v3.0 outputs 1024 dimensions
    const config = new EdgeVecConfig(1024);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    async function embed(texts, inputType = 'search_document') {
        const response = await cohere.embed({
            texts: texts,
            model: 'embed-english-v3.0',
            inputType: inputType
        });
        return response.embeddings.map(e => new Float32Array(e));
    }

    // Index documents
    const documents = ["Document 1...", "Document 2...", "Document 3..."];
    const embeddings = await embed(documents, 'search_document');

    for (let i = 0; i < embeddings.length; i++) {
        index.insert(embeddings[i]);
    }

    // Search (use 'search_query' for queries)
    const queryEmbedding = (await embed(["search query"], 'search_query'))[0];
    const results = index.search(queryEmbedding, 5);
    console.log(results);
}

main();
```

---

## HuggingFace Inference API

For using HuggingFace's hosted models.

### Installation

```bash
npm install @huggingface/inference edgevec
```

### Usage

```javascript
import { HfInference } from '@huggingface/inference';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

async function main() {
    await init();

    const hf = new HfInference(process.env.HF_TOKEN);

    // sentence-transformers/all-MiniLM-L6-v2 outputs 384 dimensions
    const config = new EdgeVecConfig(384);
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    async function embed(text) {
        const result = await hf.featureExtraction({
            model: 'sentence-transformers/all-MiniLM-L6-v2',
            inputs: text
        });
        return new Float32Array(result);
    }

    // Index and search...
    const embedding = await embed("Hello world");
    index.insert(embedding);
}

main();
```

---

## Custom Embedding Models

EdgeVec accepts any Float32Array embedding.

### Basic Pattern

```javascript
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

// Your custom embedding function
function myEmbedder(text) {
    // Your model logic here
    // Must return Float32Array of consistent dimensions
    const dimensions = 128;
    const embedding = new Float32Array(dimensions);

    // Example: simple hash-based embedding (not for production!)
    for (let i = 0; i < dimensions; i++) {
        let hash = 0;
        for (let j = 0; j < text.length; j++) {
            hash = ((hash << 5) - hash + text.charCodeAt(j) + i) | 0;
        }
        embedding[i] = Math.sin(hash) * 0.5 + 0.5;
    }

    return embedding;
}

async function main() {
    await init();

    const config = new EdgeVecConfig(128);  // Match your model
    config.metric = 'cosine';
    const index = new EdgeVec(config);

    // Use your embedder
    const embedding = myEmbedder("Hello world");
    const id = index.insert(embedding);

    // Search
    const queryEmbedding = myEmbedder("Hello");
    const results = index.search(queryEmbedding, 5);
}

main();
```

### ONNX Runtime Integration

```javascript
import * as ort from 'onnxruntime-web';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

class ONNXEmbedder {
    constructor() {
        this.session = null;
    }

    async load(modelPath) {
        this.session = await ort.InferenceSession.create(modelPath);
    }

    async embed(text) {
        // Tokenize text (implement based on your model)
        const tokens = this.tokenize(text);

        const inputIds = new ort.Tensor('int64', BigInt64Array.from(tokens), [1, tokens.length]);

        const results = await this.session.run({ input_ids: inputIds });
        const embedding = results.embeddings.data;

        return new Float32Array(embedding);
    }

    tokenize(text) {
        // Implement based on your model's tokenizer
        // This is a placeholder
        return text.split(' ').map((_, i) => BigInt(i + 1));
    }
}

async function main() {
    await init();

    const embedder = new ONNXEmbedder();
    await embedder.load('model.onnx');

    // Determine dimensions from first embedding
    const testEmbed = await embedder.embed("test");
    const dimensions = testEmbed.length;

    const config = new EdgeVecConfig(dimensions);
    const index = new EdgeVec(config);

    // Use embedder with EdgeVec...
}
```

---

## React/Vue/Svelte Integration

### React Hook

```javascript
// useEdgeVec.js
import { useState, useEffect, useCallback } from 'react';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

export function useEdgeVec(dimensions, metric = 'cosine') {
    const [index, setIndex] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);

    useEffect(() => {
        let mounted = true;

        async function initialize() {
            try {
                await init();
                const config = new EdgeVecConfig(dimensions);
                config.metric = metric;
                const newIndex = new EdgeVec(config);

                if (mounted) {
                    setIndex(newIndex);
                    setLoading(false);
                }
            } catch (err) {
                if (mounted) {
                    setError(err);
                    setLoading(false);
                }
            }
        }

        initialize();

        return () => {
            mounted = false;
        };
    }, [dimensions, metric]);

    const insert = useCallback((vector) => {
        if (!index) throw new Error('Index not initialized');
        return index.insert(vector);
    }, [index]);

    const search = useCallback((query, k = 10) => {
        if (!index) throw new Error('Index not initialized');
        return index.search(query, k);
    }, [index]);

    return { index, loading, error, insert, search };
}

// Usage in component
function SearchComponent() {
    const { index, loading, search } = useEdgeVec(384);

    if (loading) return <div>Loading EdgeVec...</div>;

    // Use search function...
}
```

### Vue 3 Composable

```javascript
// useEdgeVec.js
import { ref, onMounted } from 'vue';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

export function useEdgeVec(dimensions, metric = 'cosine') {
    const index = ref(null);
    const loading = ref(true);
    const error = ref(null);

    onMounted(async () => {
        try {
            await init();
            const config = new EdgeVecConfig(dimensions);
            config.metric = metric;
            index.value = new EdgeVec(config);
        } catch (err) {
            error.value = err;
        } finally {
            loading.value = false;
        }
    });

    const insert = (vector) => index.value?.insert(vector);
    const search = (query, k = 10) => index.value?.search(query, k);

    return { index, loading, error, insert, search };
}
```

### Svelte Store

```javascript
// edgevec.js
import { writable, derived } from 'svelte/store';
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

function createEdgeVecStore(dimensions, metric = 'cosine') {
    const { subscribe, set } = writable(null);
    const loading = writable(true);
    const error = writable(null);

    async function initialize() {
        try {
            await init();
            const config = new EdgeVecConfig(dimensions);
            config.metric = metric;
            set(new EdgeVec(config));
        } catch (err) {
            error.set(err);
        } finally {
            loading.set(false);
        }
    }

    initialize();

    return {
        subscribe,
        loading,
        error
    };
}

export const edgevec = createEdgeVecStore(384);
```

---

## Worker Thread Integration

For heavy workloads, run EdgeVec in a Web Worker to avoid blocking the main thread.

### Worker Setup

```javascript
// edgevec-worker.js
import init, { EdgeVec, EdgeVecConfig } from 'edgevec';

let index = null;

self.onmessage = async function(e) {
    const { type, payload, id } = e.data;

    try {
        switch (type) {
            case 'init': {
                await init();
                const config = new EdgeVecConfig(payload.dimensions);
                config.metric = payload.metric || 'cosine';
                index = new EdgeVec(config);
                self.postMessage({ id, success: true });
                break;
            }

            case 'insert': {
                const vectorId = index.insert(new Float32Array(payload.vector));
                self.postMessage({ id, success: true, result: vectorId });
                break;
            }

            case 'insertBatch': {
                const vectors = payload.vectors.map(v => new Float32Array(v));
                const result = index.insertBatch(vectors);
                self.postMessage({
                    id,
                    success: true,
                    result: {
                        inserted: result.inserted,
                        total: result.total,
                        ids: Array.from(result.ids)
                    }
                });
                break;
            }

            case 'search': {
                const results = index.search(
                    new Float32Array(payload.query),
                    payload.k
                );
                self.postMessage({ id, success: true, result: results });
                break;
            }

            case 'save': {
                await index.save(payload.name);
                self.postMessage({ id, success: true });
                break;
            }

            case 'load': {
                index = await EdgeVec.load(payload.name);
                self.postMessage({ id, success: true });
                break;
            }

            default:
                throw new Error(`Unknown message type: ${type}`);
        }
    } catch (err) {
        self.postMessage({ id, success: false, error: err.message });
    }
};
```

### Main Thread Client

```javascript
// EdgeVecClient.js
class EdgeVecClient {
    constructor() {
        this.worker = new Worker(
            new URL('./edgevec-worker.js', import.meta.url),
            { type: 'module' }
        );
        this.pendingRequests = new Map();
        this.nextId = 0;

        this.worker.onmessage = (e) => {
            const { id, success, result, error } = e.data;
            const request = this.pendingRequests.get(id);
            if (request) {
                this.pendingRequests.delete(id);
                if (success) {
                    request.resolve(result);
                } else {
                    request.reject(new Error(error));
                }
            }
        };
    }

    send(type, payload) {
        return new Promise((resolve, reject) => {
            const id = this.nextId++;
            this.pendingRequests.set(id, { resolve, reject });
            this.worker.postMessage({ type, payload, id });
        });
    }

    init(dimensions, metric = 'cosine') {
        return this.send('init', { dimensions, metric });
    }

    insert(vector) {
        return this.send('insert', { vector: Array.from(vector) });
    }

    insertBatch(vectors) {
        return this.send('insertBatch', {
            vectors: vectors.map(v => Array.from(v))
        });
    }

    search(query, k = 10) {
        return this.send('search', { query: Array.from(query), k });
    }

    save(name) {
        return this.send('save', { name });
    }

    load(name) {
        return this.send('load', { name });
    }
}

// Usage
const client = new EdgeVecClient();
await client.init(384);
await client.insert(new Float32Array(384).fill(0.1));
const results = await client.search(new Float32Array(384).fill(0.1), 5);
```

---

## Best Practices

### 1. Always Match Dimensions

```javascript
// WRONG - dimensions don't match
const config = new EdgeVecConfig(384);  // 384 dims
const embedding = embed(text);  // Returns 768 dims
index.insert(embedding);  // Error!

// CORRECT
const testEmbed = await embed("test");
const dimensions = testEmbed.length;  // Detect dimensions
const config = new EdgeVecConfig(dimensions);
```

### 2. Use the Same Model for Index and Query

```javascript
// WRONG - different models
const indexEmbedding = await model1.embed(doc);
const queryEmbedding = await model2.embed(query);  // Different model!

// CORRECT - same model
const indexEmbedding = await model.embed(doc);
const queryEmbedding = await model.embed(query);
```

### 3. Normalize When Using Cosine

Most embedding models output normalized vectors, but verify:

```javascript
function normalize(vec) {
    let norm = 0;
    for (let i = 0; i < vec.length; i++) {
        norm += vec[i] * vec[i];
    }
    norm = Math.sqrt(norm);

    const result = new Float32Array(vec.length);
    for (let i = 0; i < vec.length; i++) {
        result[i] = vec[i] / norm;
    }
    return result;
}

// Only normalize if your model doesn't already
// (transformers.js with normalize: true already does this)
```

### 4. Batch When Possible

```javascript
// SLOW - many API calls
for (const doc of documents) {
    const embedding = await embed(doc);  // API call each time
    index.insert(embedding);
}

// FAST - batch embed, then batch insert
const embeddings = await embedBatch(documents);  // Single API call
index.insertBatch(embeddings);
```

### 5. Store Metadata Separately

EdgeVec stores vectors, not metadata. Keep a mapping:

```javascript
const metadata = new Map();

function addDocument(doc) {
    const embedding = await embed(doc.text);
    const vectorId = index.insert(embedding);
    metadata.set(vectorId, {
        title: doc.title,
        url: doc.url,
        text: doc.text
    });
    return vectorId;
}

function searchWithMetadata(query, k) {
    const results = index.search(queryEmbedding, k);
    return results.map(r => ({
        ...r,
        metadata: metadata.get(r.id)
    }));
}
```

### 6. Handle API Rate Limits

```javascript
async function embedWithRetry(text, maxRetries = 3) {
    for (let i = 0; i < maxRetries; i++) {
        try {
            return await embed(text);
        } catch (err) {
            if (err.status === 429 && i < maxRetries - 1) {
                const delay = Math.pow(2, i) * 1000;
                console.log(`Rate limited, retrying in ${delay}ms`);
                await new Promise(r => setTimeout(r, delay));
            } else {
                throw err;
            }
        }
    }
}
```

---

## Troubleshooting Integration

### Dimension Mismatch

```javascript
// Check model's output dimension
const testEmbed = await embed("test");
console.log("Model outputs:", testEmbed.length, "dimensions");
console.log("Config expects:", config.dimensions, "dimensions");
```

### Poor Search Quality

1. Verify same model for index and query
2. Check if model requires specific input formatting
3. Increase `ef_search` for better recall
4. Ensure vectors are normalized for cosine metric

### Memory Issues

1. Use smaller dimensions (384 vs 768)
2. Batch inserts instead of one-by-one
3. Dispose TensorFlow tensors after use
4. Compact index to remove tombstones

---

## See Also

- [Tutorial](./TUTORIAL.md) — Getting started
- [Performance Tuning](./PERFORMANCE_TUNING.md) — Optimization
- [Troubleshooting](./TROUBLESHOOTING.md) — Common issues
- [API Reference](./API_REFERENCE.md) — Full API documentation
