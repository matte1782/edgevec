/**
 * Integration tests for edgevec-langchain
 *
 * Tests multi-step workflows, LangChain default method behavior,
 * and end-to-end RAG pipeline with deterministic embeddings.
 *
 * W43.5a-e: Integration + default method verification
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { Document } from "@langchain/core/documents";
import { Embeddings } from "@langchain/core/embeddings";

// Polyfill IndexedDB for Node.js tests
import "fake-indexeddb/auto";

// --- Mock WASM init ---
vi.mock("edgevec", () => ({
  default: vi.fn().mockResolvedValue(undefined),
}));

// --- Smart mock EdgeVecIndex that tracks added data for realistic search ---
let addCounter = 0;
let storedEntries: Array<{
  id: number;
  vector: Float32Array;
  metadata: Record<string, unknown>;
  deleted: boolean;
}> = [];

/**
 * Compute cosine similarity between two vectors.
 * Used by mock search to return meaningful results.
 */
function cosineSimilarity(a: Float32Array, b: Float32Array): number {
  let dot = 0;
  let normA = 0;
  let normB = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  if (denom === 0) return 0;
  return dot / denom;
}

vi.mock("edgevec/edgevec-wrapper.js", () => {
  const MockEdgeVecIndex = vi.fn().mockImplementation(() => ({
    add: (vector: Float32Array, metadata?: Record<string, unknown>) => {
      const id = addCounter++;
      storedEntries.push({ id, vector: new Float32Array(vector), metadata: metadata ?? {}, deleted: false });
      return id;
    },
    search: vi.fn().mockImplementation(
      (query: Float32Array, k: number, _options?: Record<string, unknown>) => {
        // Smart search: rank non-deleted entries by cosine similarity, return top-k
        const active = storedEntries.filter((e) => !e.deleted);
        const scored = active.map((entry) => ({
          id: entry.id,
          // Return cosine distance (0 = identical, 2 = opposite)
          score: 1 - cosineSimilarity(query, entry.vector),
          metadata: entry.metadata,
        }));
        scored.sort((a, b) => a.score - b.score); // lower distance = more similar
        return Promise.resolve(scored.slice(0, k));
      }
    ),
    delete: vi.fn().mockImplementation((numericId: number) => {
      const entry = storedEntries.find((e) => e.id === numericId);
      if (entry && !entry.deleted) {
        entry.deleted = true;
        return true;
      }
      return false;
    }),
    save: vi.fn().mockResolvedValue(undefined),
    get size() {
      return storedEntries.filter((e) => !e.deleted).length;
    },
  }));

  MockEdgeVecIndex.load = vi.fn().mockImplementation(() => {
    return Promise.resolve(new MockEdgeVecIndex());
  });

  return { EdgeVecIndex: MockEdgeVecIndex };
});

// Must import AFTER mocks
import { EdgeVecStore } from "../src/store.js";
import { initEdgeVec, _resetForTesting } from "../src/init.js";

// --- Deterministic Embeddings: text-dependent vectors ---

class DeterministicEmbeddings extends Embeddings {
  private dims: number;

  constructor(dims = 32) {
    super({});
    this.dims = dims;
  }

  async embedDocuments(texts: string[]): Promise<number[][]> {
    return texts.map((text) => this.hashToVector(text));
  }

  async embedQuery(text: string): Promise<number[]> {
    return this.hashToVector(text);
  }

  /**
   * Deterministic hash-based vector generation.
   * Same text always produces same vector. Different texts produce different vectors.
   */
  private hashToVector(text: string): number[] {
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
      hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
    }
    return Array.from({ length: this.dims }, (_, d) =>
      Math.sin(hash + d * 2654435761) * 0.5 + 0.5
    );
  }
}

// --- Test helpers ---

/** Delete all IndexedDB databases with proper awaiting. */
async function cleanupIDB(): Promise<void> {
  const dbs = await indexedDB.databases();
  await Promise.all(
    dbs
      .filter((db) => db.name)
      .map(
        (db) =>
          new Promise<void>((resolve) => {
            const req = indexedDB.deleteDatabase(db.name!);
            req.onsuccess = () => resolve();
            req.onerror = () => resolve(); // treat errors as ok in cleanup
            req.onblocked = () => resolve();
          })
      )
  );
}

// --- Test suite ---

describe("Integration: RAG Pipeline", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  describe("end-to-end pipeline", () => {
    it("embeds texts, stores, queries, and retrieves correct documents", async () => {
      const embeddings = new DeterministicEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 32 });

      const docs = [
        new Document({ pageContent: "TypeScript is great for web development", metadata: { topic: "typescript" } }),
        new Document({ pageContent: "Rust is great for systems programming", metadata: { topic: "rust" } }),
        new Document({ pageContent: "Python is great for data science", metadata: { topic: "python" } }),
        new Document({ pageContent: "TypeScript and JavaScript share syntax", metadata: { topic: "typescript" } }),
      ];

      const ids = await store.addDocuments(docs);

      expect(ids).toHaveLength(4);
      expect(new Set(ids).size).toBe(4);
      ids.forEach((id) => expect(typeof id).toBe("string"));

      // Search for query similar to TypeScript content
      const results = await store.similaritySearchVectorWithScore(
        await embeddings.embedQuery("TypeScript web development"),
        2
      );

      // Exactly 2 results (4 docs in store, k=2)
      expect(results).toHaveLength(2);

      for (const [doc, score] of results) {
        expect(typeof doc.pageContent).toBe("string");
        expect(doc.pageContent.length).toBeGreaterThan(0);
        expect(doc.metadata).toBeDefined();
        expect(score).toBeGreaterThanOrEqual(0);
        expect(score).toBeLessThanOrEqual(1);
      }
    });

    it("returns results ordered by similarity (most similar first)", async () => {
      const embeddings = new DeterministicEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 32 });

      await store.addDocuments([
        new Document({ pageContent: "TypeScript is great for web development", metadata: {} }),
        new Document({ pageContent: "Rust is great for systems programming", metadata: {} }),
        new Document({ pageContent: "Python is great for data science", metadata: {} }),
      ]);

      const results = await store.similaritySearchVectorWithScore(
        await embeddings.embedQuery("TypeScript web development"),
        3
      );

      expect(results).toHaveLength(3);

      // Scores must be monotonically non-increasing (most similar first)
      for (let i = 1; i < results.length; i++) {
        expect(results[i - 1][1]).toBeGreaterThanOrEqual(results[i][1]);
      }
    });

    it("preserves document IDs through the pipeline", async () => {
      const embeddings = new DeterministicEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 32 });

      const customIds = ["doc-alpha", "doc-beta", "doc-gamma"];
      const docs = [
        new Document({ pageContent: "Alpha document", metadata: { idx: 0 } }),
        new Document({ pageContent: "Beta document", metadata: { idx: 1 } }),
        new Document({ pageContent: "Gamma document", metadata: { idx: 2 } }),
      ];

      const returnedIds = await store.addDocuments(docs, { ids: customIds });
      expect(returnedIds).toEqual(customIds);

      const results = await store.similaritySearchVectorWithScore(
        await embeddings.embedQuery("Alpha"),
        3
      );

      expect(results).toHaveLength(3);
      for (const [doc] of results) {
        expect(doc.id).toBeDefined();
        expect(customIds).toContain(doc.id);
      }
    });

    it("returns results with correct metadata types after roundtrip", async () => {
      const embeddings = new DeterministicEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 32 });

      await store.addDocuments([
        new Document({
          pageContent: "Document with complex metadata",
          metadata: {
            count: 42,
            active: true,
            tags: ["a", "b"],
            nested: { key: "value" },
            label: "test",
          },
        }),
      ]);

      const results = await store.similaritySearchVectorWithScore(
        await embeddings.embedQuery("complex metadata"),
        1
      );

      expect(results).toHaveLength(1);
      const [doc] = results[0];
      expect(doc.metadata.count).toBe(42);
      expect(doc.metadata.active).toBe(true);
      expect(doc.metadata.tags).toEqual(["a", "b"]);
      expect(doc.metadata.nested).toEqual({ key: "value" });
      expect(doc.metadata.label).toBe("test");
    });

    it("handles empty store gracefully", async () => {
      const embeddings = new DeterministicEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 32 });

      const results = await store.similaritySearchVectorWithScore(
        await embeddings.embedQuery("anything"),
        5
      );

      expect(results).toEqual([]);
    });
  });
});

// W43.5c: similaritySearch default method
describe("Integration: Default similaritySearch", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("accepts string query, calls embedQuery, returns Document[] without scores", async () => {
    const embeddings = new DeterministicEmbeddings();
    const embedQuerySpy = vi.spyOn(embeddings, "embedQuery");

    const store = new EdgeVecStore(embeddings, { dimensions: 32 });
    await store.addDocuments([
      new Document({ pageContent: "Hello world", metadata: { src: "a" } }),
      new Document({ pageContent: "Goodbye world", metadata: { src: "b" } }),
    ]);

    const results = await store.similaritySearch("Hello", 2);

    // Verify embedQuery was called with the string query
    expect(embedQuerySpy).toHaveBeenCalledWith("Hello");

    // Exactly 2 results (2 docs, k=2)
    expect(results).toHaveLength(2);

    for (const doc of results) {
      expect(doc).toBeInstanceOf(Document);
      expect(typeof doc.pageContent).toBe("string");
      expect(doc.metadata).toBeDefined();
    }

    embedQuerySpy.mockRestore();
  });

  it("respects k parameter (returns exactly min(k, count))", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    for (let i = 0; i < 5; i++) {
      await store.addDocuments([
        new Document({ pageContent: `Document number ${i}`, metadata: { i } }),
      ]);
    }

    const results = await store.similaritySearch("Document", 2);
    expect(results).toHaveLength(2);
  });
});

// W43.5c2: addDocuments
describe("Integration: addDocuments", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("calls embedDocuments then addVectors, returns IDs", async () => {
    const embeddings = new DeterministicEmbeddings();
    const embedDocsSpy = vi.spyOn(embeddings, "embedDocuments");

    const store = new EdgeVecStore(embeddings, { dimensions: 32 });
    const docs = [
      new Document({ pageContent: "First", metadata: {} }),
      new Document({ pageContent: "Second", metadata: {} }),
    ];

    const ids = await store.addDocuments(docs);

    expect(embedDocsSpy).toHaveBeenCalledWith(["First", "Second"]);
    expect(ids).toHaveLength(2);
    expect(new Set(ids).size).toBe(2);
    expect(storedEntries).toHaveLength(2);

    embedDocsSpy.mockRestore();
  });

  it("passes custom IDs through to addVectors", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    const ids = await store.addDocuments(
      [new Document({ pageContent: "Test", metadata: {} })],
      { ids: ["custom-id-1"] }
    );

    expect(ids).toEqual(["custom-id-1"]);
  });
});

// W43.5d: similaritySearchWithScore default method
describe("Integration: Default similaritySearchWithScore", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("accepts string query, returns [Document, score][] with normalized scores", async () => {
    const embeddings = new DeterministicEmbeddings();
    const embedQuerySpy = vi.spyOn(embeddings, "embedQuery");

    const store = new EdgeVecStore(embeddings, { dimensions: 32 });
    await store.addDocuments([
      new Document({ pageContent: "Machine learning is exciting", metadata: {} }),
      new Document({ pageContent: "Deep learning uses neural networks", metadata: {} }),
    ]);

    const results = await store.similaritySearchWithScore("machine learning", 2);

    expect(embedQuerySpy).toHaveBeenCalledWith("machine learning");
    expect(results).toHaveLength(2);

    for (const entry of results) {
      expect(Array.isArray(entry)).toBe(true);
      expect(entry).toHaveLength(2);

      const [doc, score] = entry;
      expect(doc).toBeInstanceOf(Document);
      expect(typeof score).toBe("number");
      expect(score).toBeGreaterThanOrEqual(0);
      expect(score).toBeLessThanOrEqual(1);
    }

    embedQuerySpy.mockRestore();
  });
});

// W43.5e: asRetriever()
describe("Integration: asRetriever", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("returns a functional VectorStoreRetriever", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    await store.addDocuments([
      new Document({ pageContent: "Retriever test document one", metadata: { n: 1 } }),
      new Document({ pageContent: "Retriever test document two", metadata: { n: 2 } }),
      new Document({ pageContent: "Retriever test document three", metadata: { n: 3 } }),
    ]);

    const retriever = store.asRetriever({ k: 2 });
    const results = await retriever.invoke("Retriever test");

    expect(results).toHaveLength(2);

    for (const doc of results) {
      expect(doc).toBeInstanceOf(Document);
      expect(typeof doc.pageContent).toBe("string");
      expect(doc.pageContent.length).toBeGreaterThan(0);
      expect(doc.metadata).toBeDefined();
    }
  });

  it("asRetriever with no args defaults to k=4", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    for (let i = 0; i < 6; i++) {
      await store.addDocuments([
        new Document({ pageContent: `Test doc ${i}`, metadata: {} }),
      ]);
    }

    const retriever = store.asRetriever();
    const results = await retriever.invoke("test");

    // Default k=4, 6 docs in store
    expect(results).toHaveLength(4);
  });

  it("retriever results have pageContent and metadata", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    await store.addDocuments([
      new Document({
        pageContent: "Important document with metadata",
        metadata: { importance: "high", category: "test" },
      }),
    ]);

    const retriever = store.asRetriever({ k: 1 });
    const results = await retriever.invoke("important");

    expect(results).toHaveLength(1);
    expect(results[0].pageContent).toBe("Important document with metadata");
    expect(results[0].metadata.importance).toBe("high");
    expect(results[0].metadata.category).toBe("test");
  });
});

// Score normalization: concrete expected values, not just range checks
describe("Integration: Score normalization correctness", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("cosine: self-query returns score = 1.0 (distance 0 → similarity 1)", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32, metric: "cosine" });

    await store.addDocuments([
      new Document({ pageContent: "exact match test", metadata: {} }),
    ]);

    // Query with the EXACT same text → mock returns cosine distance ≈ 0
    const results = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("exact match test"),
      1
    );

    expect(results).toHaveLength(1);
    // cosine distance of identical vectors = 0, normalized = 1 - 0 = 1.0
    expect(results[0][1]).toBeCloseTo(1.0, 5);
  });

  it("l2: self-query returns score = 1.0 (distance 0 → 1/(1+0) = 1)", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32, metric: "l2" });

    await store.addDocuments([
      new Document({ pageContent: "exact match test", metadata: {} }),
    ]);

    const results = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("exact match test"),
      1
    );

    expect(results).toHaveLength(1);
    // l2 distance of identical vectors = 0, normalized = 1/(1+0) = 1.0
    expect(results[0][1]).toBeCloseTo(1.0, 5);
  });

  it("dotproduct: self-query returns score > 0.5 (negative distance → sigmoid > 0.5)", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32, metric: "dotproduct" });

    await store.addDocuments([
      new Document({ pageContent: "exact match test", metadata: {} }),
    ]);

    // Mock returns cosine distance (not raw dot product), so identical → distance 0
    // sigmoid(0) = 0.5. For real EdgeVec with dot product, self-query would have
    // very negative distance → sigmoid → close to 1. Here we verify the formula works.
    const results = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("exact match test"),
      1
    );

    expect(results).toHaveLength(1);
    // sigmoid(0) = 0.5 exactly
    expect(results[0][1]).toBeCloseTo(0.5, 5);
  });

  it("cosine: different texts produce lower scores than self-query", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32, metric: "cosine" });

    await store.addDocuments([
      new Document({ pageContent: "exact match test", metadata: {} }),
      new Document({ pageContent: "completely different unrelated content", metadata: {} }),
    ]);

    const results = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("exact match test"),
      2
    );

    expect(results).toHaveLength(2);
    // Self-match should have higher score than non-match
    expect(results[0][1]).toBeGreaterThan(results[1][1]);
    // Self-match should be very close to 1.0
    expect(results[0][1]).toBeCloseTo(1.0, 5);
    // Non-match should be strictly less than 1.0
    expect(results[1][1]).toBeLessThan(1.0);
  });
});

// DeterministicEmbeddings correctness
describe("Integration: DeterministicEmbeddings", () => {
  it("produces identical vectors for identical texts", async () => {
    const emb = new DeterministicEmbeddings();
    const v1 = await emb.embedQuery("hello world");
    const v2 = await emb.embedQuery("hello world");
    expect(v1).toEqual(v2);
  });

  it("produces distinct vectors for distinct texts", async () => {
    const emb = new DeterministicEmbeddings();
    const v1 = await emb.embedQuery("hello");
    const v2 = await emb.embedQuery("goodbye");
    expect(v1).not.toEqual(v2);
  });
});

// Multi-step workflow — add, search, delete, search again
describe("Integration: Multi-step workflows", () => {
  beforeEach(async () => {
    addCounter = 0;
    storedEntries = [];
    _resetForTesting();
    await initEdgeVec();
    await cleanupIDB();
  });

  it("add → search → delete → search returns fewer results", async () => {
    const embeddings = new DeterministicEmbeddings();
    const store = new EdgeVecStore(embeddings, { dimensions: 32 });

    const ids = await store.addDocuments([
      new Document({ pageContent: "Keep this document", metadata: {} }),
      new Document({ pageContent: "Delete this document", metadata: {} }),
    ]);

    // First search: both documents exist
    const before = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("document"),
      10
    );
    expect(before).toHaveLength(2);

    // Delete one — mock now properly marks entry as deleted
    await store.delete({ ids: [ids[1]] });

    // Second search: only one remains (mock filters deleted entries)
    const after = await store.similaritySearchVectorWithScore(
      await embeddings.embedQuery("document"),
      10
    );
    expect(after).toHaveLength(1);
  });

  it("fromTexts factory: creates store, embeds, and allows search", async () => {
    const store = await EdgeVecStore.fromTexts(
      ["First text", "Second text", "Third text"],
      [{ src: "a" }, { src: "b" }, { src: "c" }],
      new DeterministicEmbeddings(),
      { dimensions: 32 }
    );

    const results = await store.similaritySearch("First", 2);
    expect(results).toHaveLength(2);
    expect(results[0]).toBeInstanceOf(Document);
  });

  it("fromDocuments factory: creates store from Document objects", async () => {
    const docs = [
      new Document({ pageContent: "Doc A", metadata: { type: "a" } }),
      new Document({ pageContent: "Doc B", metadata: { type: "b" } }),
    ];

    const store = await EdgeVecStore.fromDocuments(
      docs,
      new DeterministicEmbeddings(),
      { dimensions: 32 }
    );

    const results = await store.similaritySearchWithScore("Doc A", 1);
    expect(results).toHaveLength(1);
    expect(results[0][0]).toBeInstanceOf(Document);
    expect(typeof results[0][1]).toBe("number");
  });
});
