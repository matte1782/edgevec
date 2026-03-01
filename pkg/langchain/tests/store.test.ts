import { describe, it, expect, vi, beforeEach } from "vitest";
import { Document } from "@langchain/core/documents";
import type { EmbeddingsInterface } from "@langchain/core/embeddings";

// Mock edgevec WASM init — must be before store import
vi.mock("edgevec", () => ({
  default: vi.fn().mockResolvedValue(undefined),
}));

// Mock EdgeVecIndex
let addCounter = 0;
let addedVectors: { vector: Float32Array; metadata: Record<string, unknown> }[] = [];
let searchResults: { id: number; score: number; metadata?: Record<string, unknown> }[] = [];

vi.mock("edgevec/edgevec-wrapper.js", () => {
  return {
    EdgeVecIndex: vi.fn().mockImplementation(() => ({
      add: (vector: Float32Array, metadata?: Record<string, unknown>) => {
        const id = addCounter++;
        addedVectors.push({ vector, metadata: metadata ?? {} });
        return id;
      },
      search: vi.fn().mockImplementation(() => Promise.resolve(searchResults)),
      get size() {
        return addCounter;
      },
    })),
  };
});

// Must import AFTER mocks are set up
import { EdgeVecStore } from "../src/store.js";
import { initEdgeVec, _resetForTesting } from "../src/init.js";
import { EdgeVecNotInitializedError } from "../src/init.js";
import { PAGE_CONTENT_KEY, ID_KEY } from "../src/metadata.js";

function makeMockEmbeddings(): EmbeddingsInterface {
  return {
    embedDocuments: vi.fn().mockImplementation((texts: string[]) =>
      Promise.resolve(texts.map(() => [0.1, 0.2, 0.3]))
    ),
    embedQuery: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  } as unknown as EmbeddingsInterface;
}

describe("EdgeVecStore", () => {
  beforeEach(async () => {
    addCounter = 0;
    addedVectors = [];
    searchResults = [];
    _resetForTesting();
    await initEdgeVec();
  });

  describe("constructor", () => {
    it("creates instance when WASM is initialized", () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      expect(store._vectorstoreType()).toBe("edgevec");
    });

    it("throws EdgeVecNotInitializedError when WASM not initialized", () => {
      _resetForTesting();
      expect(
        () => new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 })
      ).toThrow(EdgeVecNotInitializedError);
    });

    it("sets lc_namespace correctly", () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      expect(store.lc_namespace).toEqual([
        "langchain",
        "vectorstores",
        "edgevec",
      ]);
    });
  });

  describe("addVectors", () => {
    it("adds zero documents without error", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const ids = await store.addVectors([], []);
      expect(ids).toEqual([]);
      expect(addedVectors).toHaveLength(0);
    });

    it("adds single document and returns string ID", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "hello", metadata: { source: "test" } });
      const ids = await store.addVectors([[0.1, 0.2, 0.3]], [doc]);

      expect(ids).toHaveLength(1);
      expect(typeof ids[0]).toBe("string");
      // UUID format check
      expect(ids[0]).toMatch(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
      );
    });

    it("adds multiple documents with correct metadata", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "first", metadata: { idx: 0 } }),
        new Document({ pageContent: "second", metadata: { idx: 1 } }),
      ];
      const ids = await store.addVectors(
        [
          [1, 0, 0],
          [0, 1, 0],
        ],
        docs
      );

      expect(ids).toHaveLength(2);
      expect(addedVectors).toHaveLength(2);

      // Check metadata includes pageContent and ID
      expect(addedVectors[0].metadata[PAGE_CONTENT_KEY]).toBe("first");
      expect(addedVectors[0].metadata[ID_KEY]).toBe(ids[0]);
      expect(addedVectors[0].metadata.idx).toBe(0);

      expect(addedVectors[1].metadata[PAGE_CONTENT_KEY]).toBe("second");
      expect(addedVectors[1].metadata[ID_KEY]).toBe(ids[1]);
      expect(addedVectors[1].metadata.idx).toBe(1);
    });

    it("uses provided IDs when options.ids is given", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc], {
        ids: ["custom-id-123"],
      });

      expect(ids).toEqual(["custom-id-123"]);
      expect(addedVectors[0].metadata[ID_KEY]).toBe("custom-id-123");
    });

    it("throws on mismatched vectors/documents length", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });

      await expect(
        store.addVectors([[1, 2, 3], [4, 5, 6]], [doc])
      ).rejects.toThrow(/Mismatched lengths/);
    });

    it("throws on dimension mismatch", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });

      await expect(
        store.addVectors([[1, 2]], [doc])
      ).rejects.toThrow(/2 dimensions, expected 3/);
    });

    it("passes Float32Array to index.add()", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      await store.addVectors([[0.5, 0.6, 0.7]], [doc]);

      expect(addedVectors[0].vector).toBeInstanceOf(Float32Array);
      expect(Array.from(addedVectors[0].vector)).toEqual([
        expect.closeTo(0.5),
        expect.closeTo(0.6),
        expect.closeTo(0.7),
      ]);
    });

    it("serializes complex metadata through serializeMetadata", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({
        pageContent: "test",
        metadata: { nested: { a: 1 }, tags: ["x", "y"] },
      });
      await store.addVectors([[1, 2, 3]], [doc]);

      // nested object should be JSON-stringified
      expect(typeof addedVectors[0].metadata.nested).toBe("string");
      // string[] should pass through
      expect(addedVectors[0].metadata.tags).toEqual(["x", "y"]);
    });
  });

  describe("addDocuments", () => {
    it("embeds documents and delegates to addVectors", async () => {
      const embeddings = makeMockEmbeddings();
      const store = new EdgeVecStore(embeddings, { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "hello", metadata: {} }),
        new Document({ pageContent: "world", metadata: {} }),
      ];

      const ids = await store.addDocuments(docs);

      expect(embeddings.embedDocuments).toHaveBeenCalledWith([
        "hello",
        "world",
      ]);
      expect(ids).toHaveLength(2);
      expect(addedVectors).toHaveLength(2);
    });
  });

  describe("similaritySearchVectorWithScore", () => {
    it("returns empty array when no results", async () => {
      searchResults = [];
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const results = await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5
      );
      expect(results).toEqual([]);
    });

    it("reconstructs Document from search results", async () => {
      searchResults = [
        {
          id: 0,
          score: 0.1,
          metadata: {
            [PAGE_CONTENT_KEY]: "hello world",
            [ID_KEY]: "uuid-1",
            source: "test",
          },
        },
      ];

      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const results = await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        1
      );

      expect(results).toHaveLength(1);
      const [doc, score] = results[0];
      expect(doc.pageContent).toBe("hello world");
      expect(doc.id).toBe("uuid-1");
      expect(doc.metadata.source).toBe("test");
      // Internal keys should be stripped
      expect(doc.metadata[PAGE_CONTENT_KEY]).toBeUndefined();
      expect(doc.metadata[ID_KEY]).toBeUndefined();
    });

    it("falls back to reverseIdMap when ID_KEY not in metadata", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      // Add a document to populate the reverse map
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      // Search result has no ID_KEY in metadata
      searchResults = [
        {
          id: 0,
          score: 0.05,
          metadata: {
            [PAGE_CONTENT_KEY]: "test",
          },
        },
      ];

      const results = await store.similaritySearchVectorWithScore(
        [1, 2, 3],
        1
      );
      expect(results[0][0].id).toBe(ids[0]);
    });

    it("passes filter to search options", async () => {
      searchResults = [];
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      // Access the mock to check it was called with the right args
      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        'category = "gpu"'
      );

      // The mock's search was called — we verify the filter was passed
      const mockIndex = (store as any).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: 'category = "gpu"', includeMetadata: true }
      );
    });

    it("omits filter from search options when undefined", async () => {
      searchResults = [];
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      await store.similaritySearchVectorWithScore([0.1, 0.2, 0.3], 5);

      const mockIndex = (store as any).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { includeMetadata: true }
      );
    });

    it("handles missing metadata gracefully", async () => {
      searchResults = [{ id: 0, score: 0.2, metadata: undefined }];
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      const results = await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        1
      );

      expect(results).toHaveLength(1);
      const [doc] = results[0];
      expect(doc.pageContent).toBe("");
      expect(doc.metadata).toEqual({});
    });
  });

  describe("ID mapping bidirectional correctness", () => {
    it("maps string IDs to numeric IDs from index.add() return", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "a", metadata: {} }),
        new Document({ pageContent: "b", metadata: {} }),
        new Document({ pageContent: "c", metadata: {} }),
      ];

      const ids = await store.addVectors(
        [
          [1, 0, 0],
          [0, 1, 0],
          [0, 0, 1],
        ],
        docs
      );

      // Verify via private maps (access for testing)
      const idMap: Map<string, number> = (store as any).idMap;
      const reverseIdMap: Map<number, string> = (store as any).reverseIdMap;

      expect(idMap.size).toBe(3);
      expect(reverseIdMap.size).toBe(3);

      for (let i = 0; i < ids.length; i++) {
        expect(idMap.get(ids[i])).toBe(i); // addCounter returns 0, 1, 2
        expect(reverseIdMap.get(i)).toBe(ids[i]);
      }
    });
  });

  describe("score normalization", () => {
    it("normalizes cosine: 1 - distance", async () => {
      searchResults = [{ id: 0, score: 0.3, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "cosine",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBeCloseTo(0.7);
    });

    it("normalizes cosine: distance=0 → score=1", async () => {
      searchResults = [{ id: 0, score: 0, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "cosine",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBe(1);
    });

    it("normalizes l2: 1 / (1 + distance)", async () => {
      searchResults = [{ id: 0, score: 4, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBeCloseTo(0.2); // 1 / (1 + 4) = 0.2
    });

    it("normalizes l2: distance=0 → score=1", async () => {
      searchResults = [{ id: 0, score: 0, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBe(1);
    });

    it("normalizes dotproduct: 1 / (1 + |distance|)", async () => {
      searchResults = [{ id: 0, score: -3, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBeCloseTo(0.25); // 1 / (1 + 3) = 0.25
    });

    it("normalizes dotproduct: distance=0 → score=1", async () => {
      searchResults = [{ id: 0, score: 0, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBe(1);
    });

    it("defaults to cosine when no metric specified", async () => {
      searchResults = [{ id: 0, score: 0.4, metadata: {} }];
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBeCloseTo(0.6); // cosine: 1 - 0.4
    });
  });

  describe("fromTexts", () => {
    it("creates store and adds documents from text strings", async () => {
      const embeddings = makeMockEmbeddings();
      const store = await EdgeVecStore.fromTexts(
        ["hello", "world"],
        [{ source: "a" }, { source: "b" }],
        embeddings,
        { dimensions: 3 }
      );

      expect(store._vectorstoreType()).toBe("edgevec");
      expect(addedVectors).toHaveLength(2);
      expect(addedVectors[0].metadata[PAGE_CONTENT_KEY]).toBe("hello");
      expect(addedVectors[0].metadata.source).toBe("a");
      expect(addedVectors[1].metadata[PAGE_CONTENT_KEY]).toBe("world");
      expect(addedVectors[1].metadata.source).toBe("b");
    });

    it("broadcasts single metadata object to all texts", async () => {
      const store = await EdgeVecStore.fromTexts(
        ["one", "two", "three"],
        { shared: true },
        makeMockEmbeddings(),
        { dimensions: 3 }
      );

      expect(addedVectors).toHaveLength(3);
      for (const added of addedVectors) {
        expect(added.metadata.shared).toBe(true);
      }
    });

    it("handles empty metadata array elements with fallback", async () => {
      const store = await EdgeVecStore.fromTexts(
        ["a", "b"],
        [{ x: 1 }], // Only 1 metadata for 2 texts
        makeMockEmbeddings(),
        { dimensions: 3 }
      );

      expect(addedVectors).toHaveLength(2);
      expect(addedVectors[0].metadata.x).toBe(1);
      // Second text gets empty metadata (fallback)
    });
  });

  describe("fromDocuments", () => {
    it("creates store and adds provided documents", async () => {
      const docs = [
        new Document({ pageContent: "doc1", metadata: { idx: 0 } }),
        new Document({ pageContent: "doc2", metadata: { idx: 1 } }),
      ];
      const embeddings = makeMockEmbeddings();
      const store = await EdgeVecStore.fromDocuments(docs, embeddings, {
        dimensions: 3,
      });

      expect(store._vectorstoreType()).toBe("edgevec");
      expect(addedVectors).toHaveLength(2);
      expect(embeddings.embedDocuments).toHaveBeenCalledWith(["doc1", "doc2"]);
    });

    it("returns a functional store instance", async () => {
      searchResults = [
        {
          id: 0,
          score: 0.1,
          metadata: { [PAGE_CONTENT_KEY]: "found", [ID_KEY]: "test-id" },
        },
      ];

      const store = await EdgeVecStore.fromDocuments(
        [new Document({ pageContent: "found", metadata: {} })],
        makeMockEmbeddings(),
        { dimensions: 3 }
      );

      const results = await store.similaritySearchVectorWithScore(
        [1, 2, 3],
        1
      );
      expect(results).toHaveLength(1);
      expect(results[0][0].pageContent).toBe("found");
    });
  });

  describe("ensureInitialized guard", () => {
    it("throws on addVectors when WASM not initialized", async () => {
      // Create store while initialized
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      // Then reset
      _resetForTesting();

      await expect(
        store.addVectors(
          [[1, 2, 3]],
          [new Document({ pageContent: "test", metadata: {} })]
        )
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });

    it("throws on similaritySearchVectorWithScore when WASM not initialized", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(
        store.similaritySearchVectorWithScore([1, 2, 3], 5)
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });
  });
});
