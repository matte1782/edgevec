# Embedding Integration Guide

> **EdgeVec is embedding-agnostic by design.** This keeps the bundle tiny (227KB) and gives you complete flexibility to choose any embedding provider. This guide shows you exactly how to generate embeddings for use with EdgeVec.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Model Comparison](#model-comparison)
- [Provider Examples](#provider-examples)
  - [Ollama (Local Server)](#ollama-local-server)
  - [Transformers.js (Browser-Native)](#transformersjs-browser-native)
  - [OpenAI API](#openai-api)
  - [Cohere API](#cohere-api)
  - [HuggingFace Inference API](#huggingface-inference-api)
- [Best Practices](#best-practices)
  - [Web Worker Pattern](#web-worker-pattern)
  - [Model Caching](#model-caching)
  - [Batching for Performance](#batching-for-performance)
  - [Dimension Matching](#dimension-matching)
- [Complete Applications](#complete-applications)
  - [Semantic Note Search](#semantic-note-search)
  - [FAQ Bot Widget](#faq-bot-widget)
  - [Image Search with CLIP](#image-search-with-clip)
- [Troubleshooting](#troubleshooting)
- [Choosing Your Approach](#choosing-your-approach)

---

## Quick Start

Here's a complete, copy-paste example using **Transformers.js** with the **all-MiniLM-L6-v2** model (fast, 23MB, runs 100% in browser):

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVecIndex } from 'edgevec';

// Initialize embedding model (one-time, ~23MB download, cached in browser)
const embedder = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');

// Initialize EdgeVec (384D to match MiniLM output)
await init();
const db = new EdgeVecIndex({ dimensions: 384 });

// Helper: Convert text to embedding
async function embed(text) {
    const result = await embedder(text, { pooling: 'mean', normalize: true });
    return new Float32Array(result.data);
}

// Store documents
const docs = [
    "Machine learning is transforming industries",
    "Neural networks process data in layers",
    "The weather today is sunny and warm",
    "Deep learning requires large datasets"
];

for (const doc of docs) {
    const vector = await embed(doc);
    const id = db.add(vector, { text: doc });
    console.log(`Stored: "${doc.substring(0, 30)}..." with ID: ${id}`);
}

// Search for similar documents
const queryVector = await embed("How does AI work?");
const results = await db.search(queryVector, 3);

console.log('Top 3 results:', results);
// Results will show the ML/neural network docs as most similar
```

**What happens:**
1. Transformers.js downloads the model (~23MB, cached in IndexedDB)
2. Each text is converted to a 384-dimensional vector
3. EdgeVec stores and indexes the vectors
4. Search finds the most similar documents by vector distance

---

## Model Comparison

| Model | Dimensions | Size | Speed | Quality | Best For |
|:------|:-----------|:-----|:------|:--------|:---------|
| `Xenova/all-MiniLM-L6-v2` | 384 | ~23MB | Fast | Good | Quick prototypes, demos |
| `Xenova/bge-small-en-v1.5` | 384 | ~33MB | Fast | Better | English-only production |
| `Xenova/nomic-embed-text-v1` | 768 | ~130MB | Medium | Great | Long documents (8k tokens) |
| `Xenova/bge-base-en-v1.5` | 768 | ~110MB | Medium | Excellent | High-quality English search |
| `Xenova/multilingual-e5-small` | 384 | ~118MB | Medium | Great | Multi-language support |

### Model Selection Guide

- **MiniLM-L6-v2** (2019): Fastest loading, good for demos and prototypes. Excellent speed-to-quality ratio.
- **BGE-small** (2023): Better quality than MiniLM, still fast. Best for English-only apps.
- **Nomic-embed** (2024): 8192 token context window. Use for long documents, articles, or code.
- **BGE-base** (2023): Higher quality, larger size. Production-ready for English apps.
- **Multilingual-E5** (2023): Supports 100+ languages. Use for international apps.

> **Pro tip:** Start with MiniLM for development, then upgrade to BGE or Nomic for production.

---

## Provider Examples

### Ollama (Local Server)

**Best for:** Privacy-focused apps, no API costs, local inference with powerful models.

Ollama runs embedding models locally on your machine. No data leaves your device.

**Installation:**
```bash
# macOS / Linux
curl -fsSL https://ollama.com/install.sh | sh

# Windows: Download from https://ollama.com/download

# Pull an embedding model
ollama pull nomic-embed-text
```

**Recommended Models:**

| Model | Dimensions | Size | Use Case |
|:------|:-----------|:-----|:---------|
| `nomic-embed-text` | 768 | 274MB | General purpose, excellent quality |
| `all-minilm` | 384 | 45MB | Smaller, faster |
| `mxbai-embed-large` | 1024 | 670MB | Highest quality |
| `snowflake-arctic-embed` | 1024 | 670MB | High quality, multilingual |

**Code Example:**
```javascript
import init, { EdgeVecIndex } from 'edgevec';

class OllamaEmbeddingService {
    constructor(model = 'nomic-embed-text') {
        this.model = model;
        this.baseUrl = 'http://localhost:11434';
        // nomic-embed-text: 768D, all-minilm: 384D, mxbai-embed-large: 1024D
        this.dimension = model === 'all-minilm' ? 384 :
                         model === 'mxbai-embed-large' ? 1024 : 768;
        this.db = null;
    }

    async initialize() {
        // Check if Ollama is running
        try {
            const health = await fetch(`${this.baseUrl}/api/tags`);
            if (!health.ok) {
                throw new Error('Ollama not responding');
            }
        } catch (error) {
            throw new Error(
                'Ollama is not running. Start it with: ollama serve\n' +
                'Then pull a model: ollama pull nomic-embed-text'
            );
        }

        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });
        console.log(`Ollama service ready with ${this.model} (${this.dimension}D)`);
    }

    async embed(text) {
        const response = await fetch(`${this.baseUrl}/api/embeddings`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                model: this.model,
                prompt: text
            })
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(`Ollama error: ${error}`);
        }

        const data = await response.json();
        return new Float32Array(data.embedding);
    }

    async embedBatch(texts) {
        // Ollama doesn't support native batching, process sequentially
        const embeddings = [];
        for (const text of texts) {
            embeddings.push(await this.embed(text));
        }
        return embeddings;
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    async search(queryEmbedding, k = 10) {
        return await this.db.search(queryEmbedding, k);
    }
}

// Usage
const service = new OllamaEmbeddingService('nomic-embed-text');
await service.initialize();

const vector = await service.embed("Your document text");
const id = service.add(vector, { category: 'notes' });

const queryVector = await service.embed("search query");
const results = await service.search(queryVector, 5);
```

**Benefits:**
- 100% local — no data leaves your machine
- No API costs — run unlimited embeddings for free
- Fast — nomic-embed-text averages ~50ms per embedding
- Powerful models — access to latest open-source models

**Limitations:**
- Requires local installation
- Browser-only apps need a local server or backend proxy
- Resource intensive — needs decent CPU/GPU

---

### Transformers.js (Browser-Native)

**Best for:** Privacy-first apps, offline support, no API costs.

```javascript
import { pipeline, env } from '@xenova/transformers';
import init, { EdgeVecIndex } from 'edgevec';

// Optional: Show download progress
env.allowLocalModels = false;

class EmbeddingService {
    constructor(modelName = 'Xenova/all-MiniLM-L6-v2') {
        this.modelName = modelName;
        this.embedder = null;
        this.db = null;
        this.dimension = modelName.includes('MiniLM') || modelName.includes('small') ? 384 : 768;
    }

    async initialize(onProgress = null) {
        // Load embedding model with progress callback
        this.embedder = await pipeline('feature-extraction', this.modelName, {
            progress_callback: onProgress
        });

        // Initialize EdgeVec
        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });

        console.log(`Initialized with ${this.modelName} (${this.dimension}D)`);
    }

    async embed(text) {
        if (!this.embedder) throw new Error('Call initialize() first');
        const result = await this.embedder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    async embedBatch(texts) {
        // Process multiple texts efficiently
        const embeddings = [];
        for (const text of texts) {
            embeddings.push(await this.embed(text));
        }
        return embeddings;
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    search(queryEmbedding, k = 10, filter = null) {
        if (filter) {
            return this.db.search(queryEmbedding, k, { filter });
        }
        return this.db.search(queryEmbedding, k);
    }
}

// Usage
const service = new EmbeddingService('Xenova/all-MiniLM-L6-v2');

// Show loading progress
await service.initialize((progress) => {
    if (progress.status === 'downloading') {
        console.log(`Downloading: ${Math.round(progress.progress)}%`);
    }
});

// Store and search
const vector = await service.embed("Your document text");
const id = service.add(vector, { category: 'notes', date: '2025-01-15' });

const queryVector = await service.embed("search query");
const results = await service.search(queryVector, 5);
```

---

### OpenAI API

**Best for:** Highest quality embeddings, existing OpenAI integration.

```javascript
import init, { EdgeVecIndex } from 'edgevec';

// OpenAI Embedding Service
class OpenAIEmbeddingService {
    constructor(apiKey, model = 'text-embedding-3-small') {
        this.apiKey = apiKey;
        this.model = model;
        // text-embedding-3-small: 1536D, $0.02/1M tokens
        // text-embedding-3-large: 3072D, $0.13/1M tokens
        this.dimension = model === 'text-embedding-3-large' ? 3072 : 1536;
        this.db = null;
    }

    async initialize() {
        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });
    }

    async embed(text) {
        const response = await fetch('https://api.openai.com/v1/embeddings', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                input: text,
                model: this.model
            })
        });

        if (!response.ok) {
            const error = await response.json();
            throw new Error(`OpenAI API error: ${error.error?.message || response.statusText}`);
        }

        const data = await response.json();
        return new Float32Array(data.data[0].embedding);
    }

    async embedBatch(texts) {
        // OpenAI supports batching up to 2048 inputs
        const response = await fetch('https://api.openai.com/v1/embeddings', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                input: texts,
                model: this.model
            })
        });

        if (!response.ok) {
            const error = await response.json();
            throw new Error(`OpenAI API error: ${error.error?.message}`);
        }

        const data = await response.json();
        return data.data.map(item => new Float32Array(item.embedding));
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    search(queryEmbedding, k = 10) {
        return this.db.search(queryEmbedding, k);
    }
}

// Usage
// SECURITY WARNING: Never expose API keys in client-side code!
// Use a backend proxy in production.
const apiKey = 'sk-...'; // From environment or secure storage
const service = new OpenAIEmbeddingService(apiKey);
await service.initialize();

const vector = await service.embed("Your document");
const id = service.add(vector);

// Cost estimation: ~$0.02 per 1 million tokens
// Average document (500 words) ≈ 650 tokens
// 1000 documents ≈ $0.013
```

> **Security Warning:** Never expose OpenAI API keys in browser code. Use a backend proxy that adds the API key server-side.

---

### Cohere API

**Best for:** Free tier available, good for experimentation.

```javascript
import init, { EdgeVecIndex } from 'edgevec';

class CohereEmbeddingService {
    constructor(apiKey, model = 'embed-english-v3.0') {
        this.apiKey = apiKey;
        this.model = model;
        // embed-english-v3.0: 1024D
        // embed-multilingual-v3.0: 1024D
        this.dimension = 1024;
        this.db = null;
    }

    async initialize() {
        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });
    }

    async embed(text, inputType = 'search_document') {
        const response = await fetch('https://api.cohere.ai/v1/embed', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                texts: [text],
                model: this.model,
                input_type: inputType, // 'search_document' or 'search_query'
                truncate: 'END'
            })
        });

        if (!response.ok) {
            const error = await response.json();
            throw new Error(`Cohere API error: ${error.message}`);
        }

        const data = await response.json();
        return new Float32Array(data.embeddings[0]);
    }

    async embedForStorage(text) {
        return this.embed(text, 'search_document');
    }

    async embedForQuery(text) {
        return this.embed(text, 'search_query');
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    search(queryEmbedding, k = 10) {
        return this.db.search(queryEmbedding, k);
    }
}

// Usage
const service = new CohereEmbeddingService('your-cohere-api-key');
await service.initialize();

// Use different input types for storage vs queries
const docVector = await service.embedForStorage("Document to store");
const id = service.add(docVector);

const queryVector = await service.embedForQuery("Search query");
const results = await service.search(queryVector, 5);
```

---

### HuggingFace Inference API

**Best for:** Free tier, access to many models.

```javascript
import init, { EdgeVecIndex } from 'edgevec';

class HuggingFaceEmbeddingService {
    constructor(apiToken, model = 'sentence-transformers/all-MiniLM-L6-v2') {
        this.apiToken = apiToken;
        this.model = model;
        // Dimension depends on model - check model card
        this.dimension = model.includes('MiniLM-L6') ? 384 : 768;
        this.db = null;
    }

    async initialize() {
        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });
    }

    async embed(text) {
        const response = await fetch(
            `https://api-inference.huggingface.co/pipeline/feature-extraction/${this.model}`,
            {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.apiToken}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    inputs: text,
                    options: { wait_for_model: true }
                })
            }
        );

        if (!response.ok) {
            const error = await response.text();
            throw new Error(`HuggingFace API error: ${error}`);
        }

        const data = await response.json();

        // HF returns nested array for sentence-transformers
        const embedding = Array.isArray(data[0]) ? data[0] : data;

        // Mean pooling if needed (some models return per-token embeddings)
        if (Array.isArray(embedding[0])) {
            const pooled = new Array(embedding[0].length).fill(0);
            for (const token of embedding) {
                for (let i = 0; i < token.length; i++) {
                    pooled[i] += token[i];
                }
            }
            for (let i = 0; i < pooled.length; i++) {
                pooled[i] /= embedding.length;
            }
            return new Float32Array(pooled);
        }

        return new Float32Array(embedding);
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    search(queryEmbedding, k = 10) {
        return this.db.search(queryEmbedding, k);
    }
}

// Usage (free tier: 30,000 characters/month)
const service = new HuggingFaceEmbeddingService('hf_...');
await service.initialize();

const vector = await service.embed("Your text here");
const id = service.add(vector);
```

---

## Best Practices

### Web Worker Pattern

Embedding generation is CPU-intensive. Use a Web Worker to prevent UI freezing:

**embedding-worker.js:**
```javascript
import { pipeline } from '@xenova/transformers';

let embedder = null;

self.onmessage = async (e) => {
    const { type, data } = e.data;

    switch (type) {
        case 'init':
            try {
                embedder = await pipeline('feature-extraction', data.model, {
                    progress_callback: (progress) => {
                        self.postMessage({ type: 'progress', data: progress });
                    }
                });
                self.postMessage({ type: 'ready' });
            } catch (error) {
                self.postMessage({ type: 'error', data: error.message });
            }
            break;

        case 'embed':
            if (!embedder) {
                self.postMessage({ type: 'error', data: 'Not initialized' });
                return;
            }
            try {
                const result = await embedder(data.text, {
                    pooling: 'mean',
                    normalize: true
                });
                self.postMessage({
                    type: 'result',
                    data: {
                        id: data.id,
                        embedding: Array.from(result.data)
                    }
                });
            } catch (error) {
                self.postMessage({ type: 'error', data: error.message });
            }
            break;

        case 'embedBatch':
            if (!embedder) {
                self.postMessage({ type: 'error', data: 'Not initialized' });
                return;
            }
            try {
                const results = [];
                for (let i = 0; i < data.texts.length; i++) {
                    const result = await embedder(data.texts[i], {
                        pooling: 'mean',
                        normalize: true
                    });
                    results.push(Array.from(result.data));
                    self.postMessage({
                        type: 'batchProgress',
                        data: { current: i + 1, total: data.texts.length }
                    });
                }
                self.postMessage({ type: 'batchResult', data: results });
            } catch (error) {
                self.postMessage({ type: 'error', data: error.message });
            }
            break;
    }
};
```

**main.js:**
```javascript
import init, { EdgeVecIndex } from 'edgevec';

class WorkerEmbeddingService {
    constructor(model = 'Xenova/all-MiniLM-L6-v2') {
        this.model = model;
        this.dimension = model.includes('MiniLM') || model.includes('small') ? 384 : 768;
        this.worker = new Worker('embedding-worker.js', { type: 'module' });
        this.pendingRequests = new Map();
        this.requestId = 0;
        this.db = null;
        this.ready = false;

        this.worker.onmessage = (e) => this.handleMessage(e);
    }

    handleMessage(e) {
        const { type, data } = e.data;

        switch (type) {
            case 'ready':
                this.ready = true;
                if (this.onReady) this.onReady();
                break;
            case 'progress':
                if (this.onProgress) this.onProgress(data);
                break;
            case 'result':
                const resolve = this.pendingRequests.get(data.id);
                if (resolve) {
                    resolve(new Float32Array(data.embedding));
                    this.pendingRequests.delete(data.id);
                }
                break;
            case 'error':
                console.error('Worker error:', data);
                break;
        }
    }

    async initialize(onProgress = null) {
        this.onProgress = onProgress;

        await init();
        this.db = new EdgeVecIndex({ dimensions: this.dimension });

        return new Promise((resolve) => {
            this.onReady = resolve;
            this.worker.postMessage({
                type: 'init',
                data: { model: this.model }
            });
        });
    }

    async embed(text) {
        return new Promise((resolve) => {
            const id = this.requestId++;
            this.pendingRequests.set(id, resolve);
            this.worker.postMessage({
                type: 'embed',
                data: { id, text }
            });
        });
    }

    add(embedding, metadata = null) {
        return this.db.add(embedding, metadata);
    }

    search(queryEmbedding, k = 10) {
        return this.db.search(queryEmbedding, k);
    }

    terminate() {
        this.worker.terminate();
    }
}

// Usage - UI stays responsive during embedding
const service = new WorkerEmbeddingService();
await service.initialize((progress) => {
    updateLoadingBar(progress.progress);
});

// This won't freeze the UI
const vector = await service.embed("Long document text...");
const id = service.add(vector);
```

---

### Model Caching

Transformers.js automatically caches models in IndexedDB. To pre-warm the cache:

```javascript
import { pipeline, env } from '@xenova/transformers';

// Check if model is cached
async function isModelCached(modelName) {
    try {
        const cache = await caches.open('transformers-cache');
        const keys = await cache.keys();
        return keys.some(key => key.url.includes(modelName));
    } catch {
        return false;
    }
}

// Pre-load model in background (e.g., on app install)
async function preloadModel(modelName, onProgress) {
    const cached = await isModelCached(modelName);
    if (cached) {
        console.log('Model already cached');
        return;
    }

    console.log('Downloading model to cache...');
    await pipeline('feature-extraction', modelName, {
        progress_callback: onProgress
    });
    console.log('Model cached successfully');
}

// Usage: Pre-load during initial app load
document.addEventListener('DOMContentLoaded', () => {
    preloadModel('Xenova/all-MiniLM-L6-v2', (p) => {
        if (p.status === 'downloading') {
            console.log(`Caching model: ${Math.round(p.progress)}%`);
        }
    });
});
```

---

### Batching for Performance

Process multiple texts efficiently:

```javascript
async function embedBatch(embedder, texts, batchSize = 10) {
    const embeddings = [];

    for (let i = 0; i < texts.length; i += batchSize) {
        const batch = texts.slice(i, i + batchSize);
        const batchEmbeddings = await Promise.all(
            batch.map(async (text) => {
                const result = await embedder(text, {
                    pooling: 'mean',
                    normalize: true
                });
                return new Float32Array(result.data);
            })
        );
        embeddings.push(...batchEmbeddings);

        // Progress callback
        console.log(`Processed ${Math.min(i + batchSize, texts.length)}/${texts.length}`);
    }

    return embeddings;
}

// Usage
const texts = ["doc1", "doc2", "doc3", /* ... hundreds more ... */];
const embeddings = await embedBatch(embedder, texts);

for (let i = 0; i < embeddings.length; i++) {
    db.add(embeddings[i], { docIndex: i });
}
```

---

### Dimension Matching

**Critical:** EdgeVec dimension MUST match embedding model dimension.

```javascript
// ❌ WRONG: Dimension mismatch will cause errors
const db = new EdgeVecIndex({ dimensions: 768 });  // EdgeVec expects 768D
const embedder = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');
// MiniLM outputs 384D — MISMATCH!

// ✅ CORRECT: Dimensions match
const db = new EdgeVecIndex({ dimensions: 384 });  // EdgeVec expects 384D
const embedder = await pipeline('feature-extraction', 'Xenova/all-MiniLM-L6-v2');
// MiniLM outputs 384D — MATCH ✓

// ✅ ALSO CORRECT: Higher dimension model
const db = new EdgeVecIndex({ dimensions: 768 });  // EdgeVec expects 768D
const embedder = await pipeline('feature-extraction', 'Xenova/bge-base-en-v1.5');
// BGE-base outputs 768D — MATCH ✓
```

**Common model dimensions:**
| Model | Dimensions |
|:------|:-----------|
| all-MiniLM-L6-v2 | 384 |
| bge-small-en-v1.5 | 384 |
| bge-base-en-v1.5 | 768 |
| nomic-embed-text-v1 | 768 |
| text-embedding-3-small (OpenAI) | 1536 |
| embed-english-v3.0 (Cohere) | 1024 |

---

## Complete Applications

### Semantic Note Search

A complete personal notes app with semantic search:

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVecIndex } from 'edgevec';

class SemanticNotes {
    constructor() {
        this.embedder = null;
        this.db = null;
        this.notes = new Map(); // id -> note text
    }

    async initialize() {
        // Load embedding model
        this.embedder = await pipeline(
            'feature-extraction',
            'Xenova/all-MiniLM-L6-v2'
        );

        // Initialize EdgeVec
        await init();
        this.db = new EdgeVecIndex({ dimensions: 384 });

        console.log('Semantic Notes ready!');
    }

    async embed(text) {
        const result = await this.embedder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    async addNote(text, metadata = {}) {
        const vector = await this.embed(text);
        const id = this.db.add(vector, {
            ...metadata,
            createdAt: Date.now()
        });
        this.notes.set(id, text);
        return id;
    }

    async search(query, limit = 5) {
        const queryVector = await this.embed(query);
        const results = await this.db.search(queryVector, limit);

        return results.map(r => ({
            id: r.id,
            score: r.score,
            text: this.notes.get(r.id),
            preview: this.notes.get(r.id)?.substring(0, 100) + '...'
        }));
    }

    async searchByDate(query, startDate, endDate, limit = 10) {
        const queryVector = await this.embed(query);
        const filter = `createdAt >= ${startDate} AND createdAt <= ${endDate}`;
        const results = await this.db.search(queryVector, limit, { filter });

        return results.map(r => ({
            id: r.id,
            score: r.score,
            text: this.notes.get(r.id)
        }));
    }
}

// Usage
const notes = new SemanticNotes();
await notes.initialize();

// Add some notes
await notes.addNote("Meeting with team about Q4 planning", { tag: "work" });
await notes.addNote("Remember to buy groceries: milk, eggs, bread", { tag: "personal" });
await notes.addNote("Ideas for the new machine learning project", { tag: "work" });
await notes.addNote("Birthday party planning for next Saturday", { tag: "personal" });

// Search semantically
const results = await notes.search("AI project ideas");
console.log(results);
// Returns the "machine learning project" note as top result
```

---

### FAQ Bot Widget

Match user questions to pre-defined FAQ answers:

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVecIndex } from 'edgevec';

class FAQBot {
    constructor() {
        this.embedder = null;
        this.db = null;
        this.faqs = new Map();
    }

    async initialize(faqData) {
        this.embedder = await pipeline(
            'feature-extraction',
            'Xenova/all-MiniLM-L6-v2'
        );

        await init();
        this.db = new EdgeVecIndex({ dimensions: 384 });

        // Index all FAQ questions
        for (const faq of faqData) {
            const vector = await this.embed(faq.question);
            const id = this.db.add(vector);
            this.faqs.set(id, faq);
        }

        console.log(`FAQ Bot indexed ${faqData.length} questions`);
    }

    async embed(text) {
        const result = await this.embedder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    async answer(userQuestion, threshold = 0.5) {
        const queryVector = await this.embed(userQuestion);
        const results = await this.db.search(queryVector, 3);

        if (results.length === 0 || results[0].score < threshold) {
            return {
                found: false,
                message: "I couldn't find a relevant answer. Please contact support."
            };
        }

        const topMatch = this.faqs.get(results[0].id);
        return {
            found: true,
            confidence: results[0].score,
            question: topMatch.question,
            answer: topMatch.answer,
            related: results.slice(1).map(r => ({
                question: this.faqs.get(r.id).question,
                score: r.score
            }))
        };
    }
}

// Usage
const faqData = [
    {
        question: "How do I reset my password?",
        answer: "Go to Settings > Security > Reset Password. You'll receive an email with instructions."
    },
    {
        question: "What payment methods do you accept?",
        answer: "We accept Visa, MasterCard, American Express, and PayPal."
    },
    {
        question: "How can I contact customer support?",
        answer: "Email us at support@example.com or call 1-800-EXAMPLE (Mon-Fri 9am-5pm EST)."
    },
    {
        question: "What is your return policy?",
        answer: "We offer a 30-day money-back guarantee on all products. No questions asked."
    }
];

const bot = new FAQBot();
await bot.initialize(faqData);

// User asks a question (different wording from FAQs)
const response = await bot.answer("how do I change my password?");
console.log(response);
// {
//   found: true,
//   confidence: 0.89,
//   question: "How do I reset my password?",
//   answer: "Go to Settings > Security > Reset Password...",
//   related: [...]
// }
```

---

### Image Search with CLIP

Search images by text description using CLIP (multi-modal embeddings):

```javascript
import { pipeline } from '@xenova/transformers';
import init, { EdgeVecIndex } from 'edgevec';

class ImageSearch {
    constructor() {
        this.textEncoder = null;
        this.imageProcessor = null;
        this.db = null;
        this.images = new Map(); // id -> image URL
    }

    async initialize() {
        // CLIP models output 512D embeddings
        this.textEncoder = await pipeline(
            'feature-extraction',
            'Xenova/clip-vit-base-patch32',
            { revision: 'main' }
        );

        this.imageProcessor = await pipeline(
            'image-feature-extraction',
            'Xenova/clip-vit-base-patch32'
        );

        await init();
        this.db = new EdgeVecIndex({ dimensions: 512 });

        console.log('Image Search ready!');
    }

    async embedText(text) {
        const result = await this.textEncoder(text, {
            pooling: 'mean',
            normalize: true
        });
        return new Float32Array(result.data);
    }

    async embedImage(imageUrl) {
        const result = await this.imageProcessor(imageUrl);
        return new Float32Array(result.data);
    }

    async indexImage(imageUrl, metadata = {}) {
        const vector = await this.embedImage(imageUrl);
        const id = this.db.add(vector, metadata);
        this.images.set(id, imageUrl);
        return id;
    }

    async searchByText(query, limit = 10) {
        const queryVector = await this.embedText(query);
        const results = await this.db.search(queryVector, limit);

        return results.map(r => ({
            id: r.id,
            score: r.score,
            imageUrl: this.images.get(r.id)
        }));
    }

    async searchByImage(imageUrl, limit = 10) {
        const queryVector = await this.embedImage(imageUrl);
        const results = await this.db.search(queryVector, limit);

        return results.map(r => ({
            id: r.id,
            score: r.score,
            imageUrl: this.images.get(r.id)
        }));
    }
}

// Usage
const imageSearch = new ImageSearch();
await imageSearch.initialize();

// Index some images
await imageSearch.indexImage('https://example.com/beach.jpg', { category: 'nature' });
await imageSearch.indexImage('https://example.com/city.jpg', { category: 'urban' });
await imageSearch.indexImage('https://example.com/dog.jpg', { category: 'animals' });

// Search by text description
const results = await imageSearch.searchByText("a sunny day at the ocean");
console.log(results);
// Returns beach.jpg as top result

// Search by similar image
const similar = await imageSearch.searchByImage('https://example.com/puppy.jpg');
console.log(similar);
// Returns dog.jpg as top result
```

---

## Troubleshooting

| Problem | Cause | Solution |
|:--------|:------|:---------|
| "Dimension mismatch" error | EdgeVec config dim ≠ embedding output dim | Match dimensions: check model output size |
| Model download is slow | Large model (~23-400MB) | Use smaller model (MiniLM) or pre-cache |
| Out of memory error | Model too large for device | Use quantized model or switch to API |
| CORS errors with Transformers.js | CDN blocked by firewall | Self-host model files or use API |
| Search returns wrong results | Embeddings not normalized | Add `normalize: true` to pipeline options |
| "Not initialized" error | Calling methods before init | Always `await initialize()` first |
| Embeddings are all zeros | Text is empty or whitespace | Validate input text before embedding |
| Slow embedding generation | CPU-bound operation | Use Web Worker pattern |
| API rate limit exceeded | Too many requests | Implement batching with delays |

---

## Choosing Your Approach

```
Do you need 100% privacy (no server calls)?
│
├── YES → Running in browser only?
│   │
│   ├── YES → Use Transformers.js (browser-native)
│   │   │
│   │   ├── Need fastest loading?
│   │   │   └── MiniLM-L6-v2 (384D, 23MB)
│   │   │
│   │   ├── Need best English quality?
│   │   │   └── BGE-base-en-v1.5 (768D, 110MB)
│   │   │
│   │   ├── Need long document support?
│   │   │   └── Nomic-embed-text-v1 (768D, 8k tokens)
│   │   │
│   │   └── Need multi-language?
│   │       └── Multilingual-E5-small (384D, 118MB)
│   │
│   └── NO → Use Ollama (local server)
│       │
│       ├── Need general purpose?
│       │   └── nomic-embed-text (768D, 274MB)
│       │
│       ├── Need smallest/fastest?
│       │   └── all-minilm (384D, 45MB)
│       │
│       └── Need highest quality?
│           └── mxbai-embed-large (1024D, 670MB)
│
└── NO → Use an API
    │
    ├── Have OpenAI account?
    │   └── text-embedding-3-small (1536D, $0.02/1M tokens)
    │
    ├── Want free tier?
    │   └── HuggingFace Inference API (30k chars/month free)
    │
    ├── Need multilingual?
    │   └── Cohere embed-multilingual-v3.0 (1024D)
    │
    └── Enterprise requirements?
        └── Cohere, Voyage AI, or self-hosted
```

---

## Summary

EdgeVec + Embeddings = Powerful semantic search in the browser.

**Quick Start Path:**
1. Install: `npm install edgevec @xenova/transformers`
2. Use `Xenova/all-MiniLM-L6-v2` (fast, small, good quality)
3. Match dimension: `new EdgeVecIndex({ dimensions: 384 })`
4. Embed → Insert → Search

**Production Path:**
1. Upgrade to `Xenova/bge-base-en-v1.5` (768D) for better quality
2. Use Web Worker pattern to prevent UI freezing
3. Pre-cache model during app install
4. Consider API (OpenAI/Cohere) for highest quality

---

## Links

- [EdgeVec GitHub](https://github.com/matte1782/edgevec)
- [Ollama](https://ollama.com/) — Local LLM and embedding models
- [Transformers.js Documentation](https://huggingface.co/docs/transformers.js)
- [OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
- [Cohere Embed API](https://docs.cohere.com/reference/embed)
- [HuggingFace Models](https://huggingface.co/models?pipeline_tag=feature-extraction)

---

*Last updated: 2026-01-25 | EdgeVec v0.8.0*
