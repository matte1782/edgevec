import { describe, it, expect, vi, beforeEach } from "vitest";
import { Document } from "@langchain/core/documents";
import { Embeddings } from "@langchain/core/embeddings";

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
import { EdgeVecPersistenceError, MetadataSerializationError } from "../src/types.js";
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
    req.onsuccess = () => {
      if (req.result === undefined) {
        reject(new Error(`Key "${key}" not found in edgevec_meta IndexedDB`));
      } else {
        resolve(String(req.result));
      }
    };
    req.onerror = () => reject(req.error);
  });
  db.close();
  return result;
}

class MockEmbeddings extends Embeddings {
  private dims: number;
  constructor(dims = 3) {
    super({});
    this.dims = dims;
  }
  async embedDocuments(texts: string[]): Promise<number[][]> {
    return texts.map((_, i) =>
      Array.from({ length: this.dims }, (__, d) => (i + 1) * 0.1 + d * 0.01)
    );
  }
  async embedQuery(_text: string): Promise<number[]> {
    return Array.from({ length: this.dims }, (_, d) => 0.1 + d * 0.01);
  }
}

/** Typed test accessor for EdgeVecStore private fields — avoids `as any`. */
interface StoreTestAccess {
  idMap: Map<string, number>;
  reverseIdMap: Map<number, string>;
  metric: string;
  index: { search: ReturnType<typeof vi.fn> };
}

function testInternals(store: EdgeVecStore): StoreTestAccess {
  return store as unknown as StoreTestAccess;
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

    // Clean up IndexedDB between tests (properly awaited)
    const dbs = await indexedDB.databases();
    await Promise.all(
      dbs.filter((db) => db.name).map((db) =>
        new Promise<void>((resolve) => {
          const req = indexedDB.deleteDatabase(db.name!);
          req.onsuccess = () => resolve();
          req.onerror = () => resolve();
          req.onblocked = () => resolve();
        })
      )
    );
  });

  describe("constructor", () => {
    it("creates instance when WASM is initialized", () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      expect(store._vectorstoreType()).toBe("edgevec");
    });

    it("throws EdgeVecNotInitializedError when WASM not initialized", () => {
      _resetForTesting();
      expect(
        () => new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 })
      ).toThrow(EdgeVecNotInitializedError);
    });

    it("sets lc_namespace correctly", () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      expect(store.lc_namespace).toEqual([
        "langchain",
        "vectorstores",
        "edgevec",
      ]);
    });
  });

  describe("addVectors", () => {
    it("adds zero documents without error", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const ids = await store.addVectors([], []);
      expect(ids).toEqual([]);
      expect(addedVectors).toHaveLength(0);
    });

    it("adds single document and returns string ID", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc], {
        ids: ["custom-id-123"],
      });

      expect(ids).toEqual(["custom-id-123"]);
      expect(addedVectors[0].metadata[ID_KEY]).toBe("custom-id-123");
    });

    it("throws on mismatched vectors/documents length", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });

      await expect(
        store.addVectors([[1, 2, 3], [4, 5, 6]], [doc])
      ).rejects.toThrow(/Mismatched lengths/);
    });

    it("throws on dimension mismatch", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });

      await expect(
        store.addVectors([[1, 2]], [doc])
      ).rejects.toThrow(/2 dimensions, expected 3/);
    });

    it("passes Float32Array to index.add()", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const embeddings = new MockEmbeddings();
      const embedSpy = vi.spyOn(embeddings, "embedDocuments");
      const store = new EdgeVecStore(embeddings, { dimensions: 3 });
      const docs = [
        new Document({ pageContent: "hello", metadata: {} }),
        new Document({ pageContent: "world", metadata: {} }),
      ];

      const ids = await store.addDocuments(docs);

      expect(embedSpy).toHaveBeenCalledWith([
        "hello",
        "world",
      ]);
      expect(ids).toHaveLength(2);
      expect(addedVectors).toHaveLength(2);
    });
  });

  describe("delete", () => {
    it("deletes a single document and cleans ID maps", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "test", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      await store.delete({ ids: [ids[0]] });

      // Verify EdgeVecIndex.delete was called with the numeric ID
      expect(deletedIds).toEqual([0]);

      // Verify maps are cleaned
      const { idMap, reverseIdMap } = testInternals(store);
      expect(idMap.size).toBe(0);
      expect(reverseIdMap.size).toBe(0);
    });

    it("deletes multiple documents", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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

      const { idMap, reverseIdMap } = testInternals(store);
      expect(idMap.size).toBe(1);
      expect(reverseIdMap.size).toBe(1);
      // Middle one still exists
      expect(idMap.has(ids[1])).toBe(true);
    });

    it("silently ignores unknown IDs", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      // Delete a non-existent ID — should not throw
      await store.delete({ ids: ["non-existent-id"] });
      expect(deletedIds).toHaveLength(0);
    });

    it("handles empty ids array", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      await store.delete({ ids: [] });
      expect(deletedIds).toHaveLength(0);
    });

    it("warns when index.delete returns false and still cleans maps", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const { idMap, reverseIdMap } = testInternals(store);
      expect(idMap.size).toBe(0);
      expect(reverseIdMap.size).toBe(0);
    });

    it("throws on delete when WASM not initialized", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(
        store.delete({ ids: ["any-id"] })
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });
  });

  describe("save", () => {
    it("saves index and ID map to IndexedDB", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });

      await store.save("test-l2");

      expect(JSON.parse(await readIdMapFromIDB("test-l2")).metric).toBe("l2");
    });

    it("serializes reverseIdMap with string keys", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const doc = new Document({ pageContent: "hello", metadata: {} });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);
      await store.save("load-test");

      // Reset state to simulate fresh environment
      addCounter = 0;
      addedVectors = [];
      loadedNames = [];

      // Load it back
      const loaded = await EdgeVecStore.load("load-test", new MockEmbeddings());

      expect(loadedNames).toEqual(["load-test"]);
      expect(loaded._vectorstoreType()).toBe("edgevec");

      // Verify ID maps were restored
      const { idMap, reverseIdMap } = testInternals(loaded);
      expect(idMap.size).toBe(1);
      expect(reverseIdMap.size).toBe(1);
      expect(idMap.has(ids[0])).toBe(true);
    });

    it("restores metric from saved data", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });
      await store.save("metric-test");

      const loaded = await EdgeVecStore.load("metric-test", new MockEmbeddings());
      const { metric } = testInternals(loaded);
      expect(metric).toBe("dotproduct");
    });

    it("throws EdgeVecPersistenceError when ID map is missing", async () => {
      // Don't save any ID map data, just try to load
      const err = await EdgeVecStore.load("non-existent-db", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/No ID map data found/);
    });

    it("throws EdgeVecPersistenceError on corrupted JSON", async () => {
      await writeIdMapToIDB("corrupted-test", "not-valid-json{{{");

      const err = await EdgeVecStore.load("corrupted-test", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/Failed to parse/);
    });

    it("throws EdgeVecPersistenceError on invalid data structure", async () => {
      await writeIdMapToIDB("invalid-test", JSON.stringify({ foo: "bar" }));

      const err = await EdgeVecStore.load("invalid-test", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/Missing required fields/);
    });
  });

  describe("save/load roundtrip", () => {
    it("loaded store supports add, search, and delete operations", async () => {
      // Save a store with one document
      const store = new EdgeVecStore(new MockEmbeddings(), {
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
      const loaded = await EdgeVecStore.load("roundtrip-test", new MockEmbeddings());

      // Verify metric was restored
      expect(testInternals(loaded).metric).toBe("l2");

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
      const { idMap } = testInternals(loaded);
      expect(idMap.has(origIds[0])).toBe(false);
    });
  });

  describe("save WASM guard", () => {
    it("throws on save when WASM not initialized", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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

      const loaded = await EdgeVecStore.load("invalid-metric", new MockEmbeddings());

      expect(warnSpy).toHaveBeenCalledOnce();
      expect(warnSpy.mock.calls[0][0]).toMatch(/unknown metric/);
      expect(testInternals(loaded).metric).toBe("cosine");
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

      const err = await EdgeVecStore.load("float-dims", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/Missing required fields/);
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
        EdgeVecStore.load("zero-dims", new MockEmbeddings())
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
        EdgeVecStore.load("neg-dims", new MockEmbeddings())
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

      const err = await EdgeVecStore.load("nan-idmap", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/Invalid numeric ID/);
    });

    it("throws EdgeVecPersistenceError on NaN in reverseIdMap keys", async () => {
      const data = {
        idMap: {},
        reverseIdMap: { "abc": "uuid-1" },
        metric: "cosine",
        dimensions: 3,
      };
      await writeIdMapToIDB("nan-reverse", JSON.stringify(data));

      const err = await EdgeVecStore.load("nan-reverse", new MockEmbeddings())
        .catch((e: unknown) => e);
      expect(err).toBeInstanceOf(EdgeVecPersistenceError);
      expect((err as Error).message).toMatch(/Invalid numeric key/);
    });
  });

  describe("similaritySearchVectorWithScore", () => {
    it("returns empty array when no results", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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

      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        'category = "gpu"'
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: 'category = "gpu"', includeMetadata: true }
      );
    });

    it("omits filter from search options when undefined", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      await store.similaritySearchVectorWithScore([0.1, 0.2, 0.3], 5);

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { includeMetadata: true }
      );
    });

    it("handles missing metadata gracefully", async () => {
      searchResults = [{ id: 0, score: 0.2, metadata: undefined }];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
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

      const { idMap, reverseIdMap } = testInternals(store);

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
      const store = new EdgeVecStore(new MockEmbeddings(), {
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
      const store = new EdgeVecStore(new MockEmbeddings(), {
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
      const store = new EdgeVecStore(new MockEmbeddings(), {
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
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "l2",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBe(1);
    });

    it("normalizes dotproduct via sigmoid: 1 / (1 + exp(distance))", async () => {
      searchResults = [{ id: 0, score: -3, metadata: {} }];
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      // sigmoid(-3) = 1 / (1 + exp(-3)) ≈ 0.9526
      expect(results[0][1]).toBeCloseTo(1 / (1 + Math.exp(-3)));
    });

    it("normalizes dotproduct: distance=0 → score=0.5 (sigmoid midpoint)", async () => {
      searchResults = [{ id: 0, score: 0, metadata: {} }];
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      // sigmoid(0) = 0.5
      expect(results[0][1]).toBe(0.5);
    });

    it("normalizes dotproduct: preserves ordering (more negative = more similar)", async () => {
      searchResults = [
        { id: 0, score: -5, metadata: {} },
        { id: 1, score: -1, metadata: {} },
        { id: 2, score: 3, metadata: {} },
      ];
      const store = new EdgeVecStore(new MockEmbeddings(), {
        dimensions: 3,
        metric: "dotproduct",
      });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        3
      );
      // More negative raw score → higher similarity
      expect(results[0][1]).toBeGreaterThan(results[1][1]);
      expect(results[1][1]).toBeGreaterThan(results[2][1]);
    });

    it("defaults to cosine when no metric specified", async () => {
      searchResults = [{ id: 0, score: 0.4, metadata: {} }];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const results = await store.similaritySearchVectorWithScore(
        [1, 0, 0],
        1
      );
      expect(results[0][1]).toBeCloseTo(0.6);
    });
  });

  describe("fromTexts", () => {
    it("creates store and adds documents from text strings", async () => {
      const embeddings = new MockEmbeddings();
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
        new MockEmbeddings(),
        { dimensions: 3 }
      );

      expect(addedVectors).toHaveLength(3);
      for (const added of addedVectors) {
        expect(added.metadata.shared).toBe(true);
      }
    });

    it("throws on mismatched metadata array length", async () => {
      await expect(
        EdgeVecStore.fromTexts(
          ["a", "b"],
          [{ x: 1 }],
          new MockEmbeddings(),
          { dimensions: 3 }
        )
      ).rejects.toThrow(/Mismatched lengths/);
    });
  });

  describe("fromDocuments", () => {
    it("creates store and adds provided documents", async () => {
      const docs = [
        new Document({ pageContent: "doc1", metadata: { idx: 0 } }),
        new Document({ pageContent: "doc2", metadata: { idx: 1 } }),
      ];
      const embeddings = new MockEmbeddings();
      const embedSpy = vi.spyOn(embeddings, "embedDocuments");
      const store = await EdgeVecStore.fromDocuments(docs, embeddings, {
        dimensions: 3,
      });

      expect(store._vectorstoreType()).toBe("edgevec");
      expect(addedVectors).toHaveLength(2);
      expect(embedSpy).toHaveBeenCalledWith(["doc1", "doc2"]);
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
        new MockEmbeddings(),
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
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(
        store.addVectors(
          [[1, 2, 3]],
          [new Document({ pageContent: "test", metadata: {} })]
        )
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });

    it("throws on similaritySearchVectorWithScore when WASM not initialized", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      _resetForTesting();

      await expect(
        store.similaritySearchVectorWithScore([1, 2, 3], 5)
      ).rejects.toThrow(EdgeVecNotInitializedError);
    });
  });

  describe("addVectors batch", () => {
    it("adds 100 documents with all unique IDs", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const docs = Array.from({ length: 100 }, (_, i) =>
        new Document({ pageContent: `doc-${i}`, metadata: { idx: i } })
      );
      const vectors = docs.map(() => [0.1, 0.2, 0.3]);

      const ids = await store.addVectors(vectors, docs);

      expect(ids).toHaveLength(100);
      expect(new Set(ids).size).toBe(100); // all unique
      expect(addedVectors).toHaveLength(100);
    });
  });

  describe("search k exceeds count", () => {
    it("returns min(k, count) when k exceeds stored count", async () => {
      // Only 3 results available, but k=10
      searchResults = [
        { id: 0, score: 0.1, metadata: { [PAGE_CONTENT_KEY]: "a" } },
        { id: 1, score: 0.2, metadata: { [PAGE_CONTENT_KEY]: "b" } },
        { id: 2, score: 0.3, metadata: { [PAGE_CONTENT_KEY]: "c" } },
      ];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const results = await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        10
      );

      // Mock returns 3 results regardless of k — store returns what engine gives
      expect(results).toHaveLength(3);
      expect(testInternals(store).index.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        10,
        { includeMetadata: true }
      );
    });
  });

  describe("metadata edge cases (store-level)", () => {
    it("throws MetadataSerializationError on circular reference in metadata", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const circular: Record<string, unknown> = { name: "loop" };
      circular.self = circular;

      const doc = new Document({ pageContent: "test", metadata: circular });

      await expect(
        store.addVectors([[1, 2, 3]], [doc])
      ).rejects.toThrow(MetadataSerializationError);
    });

    it("handles 10KB+ pageContent through add and search roundtrip", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const largeContent = "x".repeat(10_240); // 10KB

      const doc = new Document({ pageContent: largeContent, metadata: { size: "large" } });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      expect(ids).toHaveLength(1);
      // Verify the large pageContent was stored in metadata
      expect(addedVectors[0].metadata[PAGE_CONTENT_KEY]).toBe(largeContent);

      // Simulate search returning this document
      searchResults = [
        {
          id: 0,
          score: 0.05,
          metadata: {
            [PAGE_CONTENT_KEY]: largeContent,
            [ID_KEY]: ids[0],
            size: "large",
          },
        },
      ];

      const results = await store.similaritySearchVectorWithScore([1, 2, 3], 1);
      expect(results).toHaveLength(1);
      expect(results[0][0].pageContent).toBe(largeContent);
      expect(results[0][0].pageContent.length).toBe(10_240);
      expect(results[0][0].metadata.size).toBe("large");
    });

    it("preserves unicode and emoji in metadata through add and search roundtrip", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
      const unicodeMetadata = {
        title: "日本語テスト 🎌",
        tags: ["émoji", "über", "中文"],
        emoji: "🚀🔥💯",
      };

      const doc = new Document({ pageContent: "unicode test 🦀", metadata: unicodeMetadata });
      const ids = await store.addVectors([[1, 2, 3]], [doc]);

      expect(ids).toHaveLength(1);
      expect(addedVectors[0].metadata[PAGE_CONTENT_KEY]).toBe("unicode test 🦀");
      expect(addedVectors[0].metadata.title).toBe("日本語テスト 🎌");
      expect(addedVectors[0].metadata.tags).toEqual(["émoji", "über", "中文"]);
      expect(addedVectors[0].metadata.emoji).toBe("🚀🔥💯");

      // Simulate search returning this document
      searchResults = [
        {
          id: 0,
          score: 0.1,
          metadata: {
            [PAGE_CONTENT_KEY]: "unicode test 🦀",
            [ID_KEY]: ids[0],
            title: "日本語テスト 🎌",
            tags: ["émoji", "über", "中文"],
            emoji: "🚀🔥💯",
          },
        },
      ];

      const results = await store.similaritySearchVectorWithScore([1, 2, 3], 1);
      expect(results[0][0].pageContent).toBe("unicode test 🦀");
      expect(results[0][0].metadata.title).toBe("日本語テスト 🎌");
      expect(results[0][0].metadata.emoji).toBe("🚀🔥💯");
    });
  });

  describe("invalid filter syntax", () => {
    it("propagates error from index.search on invalid filter syntax", async () => {
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      // Make mock search reject with a filter error
      testInternals(store).index.search.mockRejectedValueOnce(
        new Error("Invalid filter syntax: unexpected token '!!!'")
      );

      await expect(
        store.similaritySearchVectorWithScore([1, 2, 3], 5, "!!invalid!!!")
      ).rejects.toThrow(/Invalid filter syntax/);
    });
  });

  describe("FilterExpression support", () => {
    // Note: edgevec WASM module is fully mocked in this test suite. The Filter
    // class from edgevec/filter.js is a pure JS object (not WASM-dependent) but
    // its constructor references the WASM-backed parser. We test pass-through
    // behavior using structurally typed objects that satisfy the FilterExpression
    // interface, which is sufficient to verify that EdgeVecStore correctly forwards
    // filter objects to the underlying index.search() without transformation.

    /** Helper: create a mock FilterExpression that satisfies the interface contract */
    function mockFilterExpression(json: string, str: string): import("edgevec/edgevec-wrapper.js").FilterExpression {
      return {
        _json: json,
        toString: () => str,
        toJSON: () => JSON.parse(json),
        isTautology: false,
        isContradiction: false,
        complexity: 1,
      };
    }

    it("accepts FilterExpression object as filter parameter", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const filterExpr = mockFilterExpression(
        '{"op":"eq","field":"category","value":"gpu"}',
        'category = "gpu"'
      );

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        filterExpr
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: filterExpr, includeMetadata: true }
      );
    });

    it("accepts AND-combined FilterExpression objects", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const andFilter = mockFilterExpression(
        '{"op":"and","children":[{"op":"eq","field":"category","value":"gpu"},{"op":"gt","field":"price","value":100}]}',
        'category = "gpu" AND price > 100'
      );

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        andFilter
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: andFilter, includeMetadata: true }
      );
    });

    it("accepts OR-combined FilterExpression objects", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const orFilter = mockFilterExpression(
        '{"op":"or","children":[{"op":"eq","field":"status","value":"active"},{"op":"eq","field":"status","value":"featured"}]}',
        'status = "active" OR status = "featured"'
      );

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        orFilter
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: orFilter, includeMetadata: true }
      );
    });

    it("accepts between FilterExpression", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      const betweenFilter = mockFilterExpression(
        '{"op":"between","field":"price","low":100,"high":500}',
        "price >= 100 AND price <= 500"
      );

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        betweenFilter
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: betweenFilter, includeMetadata: true }
      );
    });

    it("still accepts string filters (backward compatibility)", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      await store.similaritySearchVectorWithScore(
        [0.1, 0.2, 0.3],
        5,
        'status = "active"'
      );

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { filter: 'status = "active"', includeMetadata: true }
      );
    });

    it("handles undefined filter (no filter)", async () => {
      searchResults = [];
      const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

      await store.similaritySearchVectorWithScore([0.1, 0.2, 0.3], 5);

      const mockIndex = testInternals(store).index;
      expect(mockIndex.search).toHaveBeenCalledWith(
        expect.any(Float32Array),
        5,
        { includeMetadata: true }
      );
    });

    describe("FilterExpression edge cases", () => {
      it("handles empty AND filter (tautology)", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const tautologyFilter = {
          ...mockFilterExpression(
            '{"op":"and","children":[]}',
            "TRUE"
          ),
          isTautology: true,
        };

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          tautologyFilter
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: tautologyFilter, includeMetadata: true }
        );
      });

      it("handles deeply nested filters (depth 3+)", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const nestedJson = JSON.stringify({
          op: "and",
          children: [
            {
              op: "or",
              children: [
                { op: "eq", field: "status", value: "active" },
                { op: "ne", field: "status", value: "deleted" },
              ],
            },
            {
              op: "and",
              children: [
                { op: "gt", field: "score", value: 0.5 },
                { op: "lt", field: "score", value: 0.95 },
              ],
            },
          ],
        });

        const deepFilter = mockFilterExpression(
          nestedJson,
          '(status = "active" OR status != "deleted") AND (score > 0.5 AND score < 0.95)'
        );

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          deepFilter
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: deepFilter, includeMetadata: true }
        );
      });

      it("handles unicode field names", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const unicodeFilter = mockFilterExpression(
          '{"op":"eq","field":"citt\u00e0","value":"Milano"}',
          'citt\u00e0 = "Milano"'
        );

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          unicodeFilter
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: unicodeFilter, includeMetadata: true }
        );
      });

      it("handles special characters in values", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const specialValue = 'he said \\"hello\\\\world\\"';
        const specialFilter = mockFilterExpression(
          '{"op":"eq","field":"message","value":"he said \\\\\\"hello\\\\\\\\world\\\\\\""}',
          `message = "${specialValue}"`
        );

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          specialFilter
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: specialFilter, includeMetadata: true }
        );
      });

      it("handles numeric edge cases (zero, negative)", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        // Test zero value
        const zeroFilter = mockFilterExpression(
          '{"op":"eq","field":"score","value":0}',
          "score = 0"
        );

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          zeroFilter
        );

        let mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: zeroFilter, includeMetadata: true }
        );
        // Verify the filter object itself has the zero value, not undefined/null
        expect(zeroFilter.toJSON().value).toBe(0);

        // Test negative value
        const negativeFilter = mockFilterExpression(
          '{"op":"lt","field":"temperature","value":-40.5}',
          "temperature < -40.5"
        );

        const store2 = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });
        await store2.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          negativeFilter
        );

        mockIndex = testInternals(store2).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: negativeFilter, includeMetadata: true }
        );
        expect(negativeFilter.toJSON().value).toBe(-40.5);
      });
    });

    // Real-world filter patterns that users will write. These verify that complex
    // nested FilterExpression objects are passed through correctly to the WASM layer.
    describe("FilterExpression real-world patterns", () => {
      it("e-commerce: category + price range filter", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const filterExpr: import("edgevec/edgevec-wrapper.js").FilterExpression = {
          ...mockFilterExpression(
            '{"op":"and","children":[{"op":"eq","field":"category","value":"electronics"},{"op":"ge","field":"price","value":100},{"op":"le","field":"price","value":500}]}',
            'category = "electronics" AND price >= 100 AND price <= 500'
          ),
          complexity: 3,
        };

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          10,
          filterExpr
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          10,
          { filter: filterExpr, includeMetadata: true }
        );
      });

      it("multi-tenant: tenant isolation with status filter", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const filterExpr: import("edgevec/edgevec-wrapper.js").FilterExpression = {
          ...mockFilterExpression(
            '{"op":"and","children":[{"op":"eq","field":"tenant_id","value":"org_123"},{"op":"or","children":[{"op":"eq","field":"status","value":"active"},{"op":"eq","field":"status","value":"pending"}]}]}',
            'tenant_id = "org_123" AND (status = "active" OR status = "pending")'
          ),
          complexity: 4,
        };

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          10,
          filterExpr
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          10,
          { filter: filterExpr, includeMetadata: true }
        );
      });

      it("date range: time-bounded search", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const filterExpr: import("edgevec/edgevec-wrapper.js").FilterExpression = {
          ...mockFilterExpression(
            '{"op":"and","children":[{"op":"ge","field":"created_at","value":"2026-01-01"},{"op":"lt","field":"created_at","value":"2026-04-01"}]}',
            'created_at >= "2026-01-01" AND created_at < "2026-04-01"'
          ),
          complexity: 2,
        };

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          10,
          filterExpr
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          10,
          { filter: filterExpr, includeMetadata: true }
        );
      });

      it("negation: document type with NOT deleted", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const filterExpr: import("edgevec/edgevec-wrapper.js").FilterExpression = {
          ...mockFilterExpression(
            '{"op":"and","children":[{"op":"eq","field":"type","value":"document"},{"op":"not","child":{"op":"eq","field":"deleted","value":true}}]}',
            'type = "document" AND NOT (deleted = true)'
          ),
          complexity: 3,
        };

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          10,
          filterExpr
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          10,
          { filter: filterExpr, includeMetadata: true }
        );
      });
    });

    // Type safety tests verifying that the adapter correctly relies on TypeScript
    // types for compile-time safety and WASM for runtime validation.
    describe("FilterExpression type safety", () => {
      it("passes through object without _json field (WASM validates)", async () => {
        // The adapter does not validate filter shape -- WASM is responsible for
        // validation. This documents the pass-through behavior.
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const noJsonField = {
          toString: () => "test",
          toJSON: () => ({}),
          isTautology: false,
          isContradiction: false,
          complexity: 0,
        } as any;

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          noJsonField
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: noJsonField, includeMetadata: true }
        );
      });

      it("passes through object with non-standard toJSON return (WASM validates)", async () => {
        // Malformed FilterExpression objects are forwarded to WASM which will
        // throw its own validation error.
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        const badToJson = {
          _json: '{"op":"eq"}',
          toString: () => "malformed",
          toJSON: () => "this-is-a-string-not-an-object",
          isTautology: false,
          isContradiction: false,
          complexity: 1,
        } as any;

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          badToJson
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: badToJson, includeMetadata: true }
        );
      });

      it("TypeScript prevents plain object as FilterExpression at compile time", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        // @ts-expect-error - Plain objects are not assignable to string | FilterExpression
        await store.similaritySearchVectorWithScore([1, 2, 3], 5, { foo: "bar" });

        // Runtime does not prevent it -- only TypeScript's type system does.
        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: { foo: "bar" }, includeMetadata: true }
        );
      });
    });

    // W45.1b: Null/undefined/empty filter handling tests
    // These tests document pass-through behavior. The adapter does not validate
    // filters — WASM handles validation. Null, empty, and whitespace filters are
    // forwarded as-is.
    describe("null/undefined/empty filter handling", () => {
      it("passes null filter through to WASM (JS interop)", async () => {
        // TypeScript type is `string | FilterExpression | undefined`, but plain JS
        // callers can pass null. The `!== undefined` guard means null is forwarded.
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          null as any
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: null, includeMetadata: true }
        );
      });

      it("passes empty string filter through to WASM", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          ""
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: "", includeMetadata: true }
        );
      });

      it("passes whitespace-only filter through to WASM", async () => {
        searchResults = [];
        const store = new EdgeVecStore(new MockEmbeddings(), { dimensions: 3 });

        await store.similaritySearchVectorWithScore(
          [0.1, 0.2, 0.3],
          5,
          "   "
        );

        const mockIndex = testInternals(store).index;
        expect(mockIndex.search).toHaveBeenCalledWith(
          expect.any(Float32Array),
          5,
          { filter: "   ", includeMetadata: true }
        );
      });
    });
  });
});
