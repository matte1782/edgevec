import { describe, it, expect, vi, beforeEach } from "vitest";
import { Document } from "@langchain/core/documents";
import type { EmbeddingsInterface } from "@langchain/core/embeddings";

// Polyfill IndexedDB for Node.js tests
import "fake-indexeddb/auto";

// Mock edgevec WASM init — must be before store import
vi.mock("edgevec", () => ({
  default: vi.fn().mockResolvedValue(undefined),
}));

// Mock EdgeVecIndex
let addCounter = 0;
let addedVectors: { vector: Float32Array; metadata: Record<string, unknown> }[] = [];
let searchResults: { id: number; score: number; metadata?: Record<string, unknown> }[] = [];
let deletedIds: number[] = [];
let deleteReturnValue = true;
let savedNames: string[] = [];
let loadedNames: string[] = [];

// Store for mock save/load roundtrip
let mockSavedState: {
  vectors: typeof addedVectors;
  counter: number;
} | null = null;

vi.mock("edgevec/edgevec-wrapper.js", () => {
  const MockEdgeVecIndex = vi.fn().mockImplementation(() => ({
    add: (vector: Float32Array, metadata?: Record<string, unknown>) => {
      const id = addCounter++;
      addedVectors.push({ vector, metadata: metadata ?? {} });
      return id;
    },
    search: vi.fn().mockImplementation(() => Promise.resolve(searchResults)),
    delete: (id: number) => {
      deletedIds.push(id);
      return deleteReturnValue;
    },
    save: vi.fn().mockImplementation((name: string) => {
      savedNames.push(name);
      mockSavedState = {
        vectors: [...addedVectors],
        counter: addCounter,
      };
      return Promise.resolve();
    }),
    get size() {
      return addCounter;
    },
  }));

  // Static load method on the constructor
  MockEdgeVecIndex.load = vi.fn().mockImplementation((name: string) => {
    loadedNames.push(name);
    // Return a fresh mock instance (simulating loaded index)
    return Promise.resolve(new MockEdgeVecIndex());
  });

  return { EdgeVecIndex: MockEdgeVecIndex };
});

// Must import AFTER mocks are set up
import { EdgeVecStore } from "../src/store.js";
import { initEdgeVec, _resetForTesting } from "../src/init.js";
import { EdgeVecNotInitializedError } from "../src/init.js";
import { EdgeVecPersistenceError } from "../src/types.js";
import { PAGE_CONTENT_KEY, ID_KEY } from "../src/metadata.js";

/** Write a string value to the edgevec_meta IndexedDB for test setup. */
async function writeIdMapToIDB(key: string, value: string): Promise<void> {
  const db = await new Promise<IDBDatabase>((resolve, reject) => {
    const req = indexedDB.open("edgevec_meta", 1);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains("idmaps")) {
        req.result.createObjectStore("idmaps");
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction("idmaps", "readwrite");
    tx.objectStore("idmaps").put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

/** Read a string value from the edgevec_meta IndexedDB for test assertions. */
async function readIdMapFromIDB(key: string): Promise<string> {
  const db = await new Promise<IDBDatabase>((resolve, reject) => {
    const req = indexedDB.open("edgevec_meta", 1);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
  const result = await new Promise<string>((resolve, reject) => {
    const tx = db.transaction("idmaps", "readonly");
    const req = tx.objectStore("idmaps").get(key);
    req.onsuccess = () => resolve(req.result as string);
    req.onerror = () => reject(req.error);
  });
  db.close();
  return result;
}

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
    deletedIds = [];
    deleteReturnValue = true;
    savedNames = [];
    loadedNames = [];
    mockSavedState = null;
    _resetForTesting();
    await initEdgeVec();

    // Clean up IndexedDB between tests
    const dbs = await indexedDB.databases();
    for (const db of dbs) {
      if (db.name) indexedDB.deleteDatabase(db.name);
    }
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

  describe("delete", () => {
    it("deletes a single document and cleans ID maps", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      await store.delete({ ids: [ids[0]] });

      // Verify EdgeVecIndex.delete was called with the numeric ID
      expect(deletedIds).toEqual([0]);

      // Verify maps are cleaned
      const idMap: Map<string, number> = (store as any).idMap;
      const reverseIdMap: Map<number, string> = (store as any).reverseIdMap;
      expect(idMap.size).toBe(0);
      expect(reverseIdMap.size).toBe(0);
    });

    it("deletes multiple documents", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "a", metadata: {} }),
        new Document({ pageContent: "b", metadata: {} }),
        new Document({ pageContent: "c", metadata: {} }),
      ];
      const ids = await store.addVectors(
        [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
        docs
      );

      // Delete first and last
      await store.delete({ ids: [ids[0], ids[2]] });

      expect(deletedIds).toEqual([0, 2]);

      const idMap: Map<string, number> = (store as any).idMap;
      const reverseIdMap: Map<number, string> = (store as any).reverseIdMap;
      expect(idMap.size).toBe(1);
      expect(reverseIdMap.size).toBe(1);
      // Middle one still exists
      expect(idMap.has(ids[1])).toBe(true);
    });

    it("silently ignores unknown IDs", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });

      // Delete a non-existent ID — should not throw
      await store.delete({ ids: ["non-existent-id"] });
      expect(deletedIds).toHaveLength(0);
    });

    it("handles empty ids array", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      await store.delete({ ids: [] });
      expect(deletedIds).toHaveLength(0);
    });

    it("warns when index.delete returns false and still cleans maps", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      // Make delete return false (simulates already-deleted vector)
      deleteReturnValue = false;
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      await store.delete({ ids: [ids[0]] });

      expect(warnSpy).toHaveBeenCalledOnce();
      expect(warnSpy.mock.calls[0][0]).toMatch(/returned false/);
      warnSpy.mockRestore();

      // Maps should still be cleaned
      const idMap: Map<string, number> = (store as any).idMap;
      const reverseIdMap: Map<number, string> = (store as any).reverseIdMap;
      expect(idMap.size).toBe(0);
      expect(reverseIdMap.size).toBe(0);
    });

    it("throws on delete when WASM not initialized", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(
        store.delete({ ids: ["any-id"] })
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });
  });

  describe("save", () => {
    it("saves index and ID map to IndexedDB", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: { x: 1 } });
      await store.addVectors([[1, 2, 3]], [doc]);

      await store.save("test-index");

      expect(savedNames).toEqual(["test-index"]);

      const parsed = JSON.parse(await readIdMapFromIDB("test-index"));
      expect(parsed.dimensions).toBe(3);
      expect(parsed.metric).toBe("cosine");
      expect(Object.keys(parsed.idMap)).toHaveLength(1);
      expect(Object.keys(parsed.reverseIdMap)).toHaveLength(1);
    });

    it("persists metric for correct normalization on load", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });

      await store.save("test-l2");

      expect(JSON.parse(await readIdMapFromIDB("test-l2")).metric).toBe("l2");
    });

    it("serializes reverseIdMap with string keys", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "a", metadata: {} }),
        new Document({ pageContent: "b", metadata: {} }),
      ];
      const ids = await store.addVectors([[1, 0, 0], [0, 1, 0]], docs);

      await store.save("serial-test");

      const parsed = JSON.parse(await readIdMapFromIDB("serial-test"));
      // reverseIdMap keys must be stringified numeric IDs
      expect(Object.keys(parsed.reverseIdMap).sort()).toEqual(["0", "1"]);
      // Values must be the original string UUIDs
      expect(parsed.reverseIdMap["0"]).toBe(ids[0]);
      expect(parsed.reverseIdMap["1"]).toBe(ids[1]);
    });
  });

  describe("load", () => {
    it("loads index and restores ID maps from IndexedDB", async () => {
      // First save a store
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "hello", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);
      await store.save("load-test");

      // Reset state to simulate fresh environment
      addCounter = 0;
      addedVectors = [];
      loadedNames = [];

      // Load it back
      const loaded = await EdgeVecStore.load("load-test", makeMockEmbeddings());

      expect(loadedNames).toEqual(["load-test"]);
      expect(loaded._vectorstoreType()).toBe("edgevec");

      // Verify ID maps were restored
      const idMap: Map<string, number> = (loaded as any).idMap;
      const reverseIdMap: Map<number, string> = (loaded as any).reverseIdMap;
      expect(idMap.size).toBe(1);
      expect(reverseIdMap.size).toBe(1);
      expect(idMap.has(ids[0])).toBe(true);
    });

    it("restores metric from saved data", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });
      await store.save("metric-test");

      const loaded = await EdgeVecStore.load("metric-test", makeMockEmbeddings());
      const metric: string = (loaded as any).metric;
      expect(metric).toBe("dotproduct");
    });

    it("throws EdgeVecPersistenceError when ID map is missing", async () => {
      // Don't save any ID map data, just try to load
      await expect(
        EdgeVecStore.load("non-existent-db", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("non-existent-db", makeMockEmbeddings())
      ).rejects.toThrow(/No ID map data found/);
    });

    it("throws EdgeVecPersistenceError on corrupted JSON", async () => {
      await writeIdMapToIDB("corrupted-test", "not-valid-json{{{");

      await expect(
        EdgeVecStore.load("corrupted-test", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("corrupted-test", makeMockEmbeddings())
      ).rejects.toThrow(/Failed to parse/);
    });

    it("throws EdgeVecPersistenceError on invalid data structure", async () => {
      await writeIdMapToIDB("invalid-test", JSON.stringify({ foo: "bar" }));

      await expect(
        EdgeVecStore.load("invalid-test", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("invalid-test", makeMockEmbeddings())
      ).rejects.toThrow(/Missing required fields/);
    });
  });

  describe("save/load roundtrip", () => {
    it("loaded store supports add, search, and delete operations", async () => {
      // Save a store with one document
      const store = new EdgeVecStore(makeMockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });
      const doc = new Document({ pageContent: "original", metadata: { v: 1 } });
      const origIds = await store.addVectors([[1, 2, 3]], [doc]);
      await store.save("roundtrip-test");

      // Reset mock state
      addCounter = 0;
      addedVectors = [];
      deletedIds = [];

      // Load it back
      const loaded = await EdgeVecStore.load("roundtrip-test", makeMockEmbeddings());

      // Verify metric was restored
      expect((loaded as any).metric).toBe("l2");

      // Add a new document to the loaded store
      const newDoc = new Document({ pageContent: "new", metadata: { v: 2 } });
      const newIds = await loaded.addVectors([[4, 5, 6]], [newDoc]);
      expect(newIds).toHaveLength(1);
      expect(addedVectors).toHaveLength(1);

      // Search on the loaded store
      searchResults = [
        { id: 0, score: 0.5, metadata: { [PAGE_CONTENT_KEY]: "found", [ID_KEY]: origIds[0] } },
      ];
      const results = await loaded.similaritySearchVectorWithScore([1, 0, 0], 1);
      expect(results).toHaveLength(1);
      // l2 normalization: 1 / (1 + 0.5) = 0.6667
      expect(results[0][1]).toBeCloseTo(1 / 1.5);

      // Delete from the loaded store
      await loaded.delete({ ids: [origIds[0]] });
      const idMap: Map<string, number> = (loaded as any).idMap;
      expect(idMap.has(origIds[0])).toBe(false);
    });
  });

  describe("save WASM guard", () => {
    it("throws on save when WASM not initialized", async () => {
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(store.save("should-fail")).rejects.toThrow(
        EdgeVecNotInitializedError
      );
    });
  });

  describe("load edge cases", () => {
    it("warns and defaults to cosine when metric is invalid", async () => {
      const data = {
        idMap: {},
        reverseIdMap: {},
        metric: "hamming",
        dimensions: 3,
      };
      await writeIdMapToIDB("invalid-metric", JSON.stringify(data));
      const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      const loaded = await EdgeVecStore.load("invalid-metric", makeMockEmbeddings());

      expect(warnSpy).toHaveBeenCalledOnce();
      expect(warnSpy.mock.calls[0][0]).toMatch(/unknown metric/);
      expect((loaded as any).metric).toBe("cosine");
      warnSpy.mockRestore();
    });

    it("throws EdgeVecPersistenceError on non-integer dimensions", async () => {
      const data = {
        idMap: {},
        reverseIdMap: {},
        metric: "cosine",
        dimensions: 3.5,
      };
      await writeIdMapToIDB("float-dims", JSON.stringify(data));

      await expect(
        EdgeVecStore.load("float-dims", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("float-dims", makeMockEmbeddings())
      ).rejects.toThrow(/Missing required fields/);
    });

    it("throws EdgeVecPersistenceError on zero dimensions", async () => {
      const data = {
        idMap: {},
        reverseIdMap: {},
        metric: "cosine",
        dimensions: 0,
      };
      await writeIdMapToIDB("zero-dims", JSON.stringify(data));

      await expect(
        EdgeVecStore.load("zero-dims", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
    });

    it("throws EdgeVecPersistenceError on negative dimensions", async () => {
      const data = {
        idMap: {},
        reverseIdMap: {},
        metric: "cosine",
        dimensions: -5,
      };
      await writeIdMapToIDB("neg-dims", JSON.stringify(data));

      await expect(
        EdgeVecStore.load("neg-dims", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
    });

    it("throws EdgeVecPersistenceError on NaN in idMap values", async () => {
      const data = {
        idMap: { "uuid-1": "not-a-number" },
        reverseIdMap: {},
        metric: "cosine",
        dimensions: 3,
      };
      await writeIdMapToIDB("nan-idmap", JSON.stringify(data));

      await expect(
        EdgeVecStore.load("nan-idmap", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("nan-idmap", makeMockEmbeddings())
      ).rejects.toThrow(/Invalid numeric ID/);
    });

    it("throws EdgeVecPersistenceError on NaN in reverseIdMap keys", async () => {
      const data = {
        idMap: {},
        reverseIdMap: { "abc": "uuid-1" },
        metric: "cosine",
        dimensions: 3,
      };
      await writeIdMapToIDB("nan-reverse", JSON.stringify(data));

      await expect(
        EdgeVecStore.load("nan-reverse", makeMockEmbeddings())
      ).rejects.toThrow(EdgeVecPersistenceError);
      await expect(
        EdgeVecStore.load("nan-reverse", makeMockEmbeddings())
      ).rejects.toThrow(/Invalid numeric key/);
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

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        'category = "gpu"'
      );

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

      const idMap: Map<string, number> = (store as any).idMap;
      const reverseIdMap: Map<number, string> = (store as any).reverseIdMap;

      expect(idMap.size).toBe(3);
      expect(reverseIdMap.size).toBe(3);

      for (let i = 0; i < ids.length; i++) {
        expect(idMap.get(ids[i])).toBe(i);
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
      expect(results[0][1]).toBeCloseTo(0.2);
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
      expect(results[0][1]).toBeCloseTo(0.25);
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
      expect(results[0][1]).toBeCloseTo(0.6);
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
      await EdgeVecStore.fromTexts(
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
      await EdgeVecStore.fromTexts(
        ["a", "b"],
        [{ x: 1 }],
        makeMockEmbeddings(),
        { dimensions: 3 }
      );

      expect(addedVectors).toHaveLength(2);
      expect(addedVectors[0].metadata.x).toBe(1);
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
      const store = new EdgeVecStore(makeMockEmbeddings(), { dimensions: 3 });
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
