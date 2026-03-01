/**
 * EdgeVecStore — LangChain.js VectorStore adapter for EdgeVec
 *
 * Core class that bridges EdgeVec's WASM-based vector index with
 * LangChain's VectorStore interface. Handles:
 * - Bidirectional ID mapping (string ↔ numeric)
 * - Metadata serialization via the metadata module
 * - Score normalization (distance → similarity)
 * - Persistence via IndexedDB (save/load with ID map survival)
 *
 * @module store
 */

import { SaveableVectorStore } from "@langchain/core/vectorstores";
import { Document } from "@langchain/core/documents";
import type { EmbeddingsInterface } from "@langchain/core/embeddings";
import type { DocumentInterface } from "@langchain/core/documents";
import { EdgeVecIndex } from "edgevec/edgevec-wrapper.js";
import type { Metadata, SearchOptions } from "edgevec/edgevec-wrapper.js";

import { ensureInitialized, initEdgeVec } from "./init.js";
import { serializeMetadata, deserializeMetadata, PAGE_CONTENT_KEY, ID_KEY } from "./metadata.js";
import type { EdgeVecStoreConfig, EdgeVecMetric } from "./types.js";
import { EdgeVecPersistenceError } from "./types.js";

/** Browser + Node.js compatible UUID generator */
function generateUUID(): string {
  return globalThis.crypto.randomUUID();
}

// --- IndexedDB helpers for ID map persistence ---

const IDMAP_DB_NAME = "edgevec_meta";
const IDMAP_STORE_NAME = "idmaps";

/**
 * Persist a string value to IndexedDB under the given key.
 * Uses a single shared DB (`edgevec_meta`) with an `idmaps` object store.
 */
async function saveToIndexedDB(key: string, value: string): Promise<void> {
  const db = await openMetaDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDMAP_STORE_NAME, "readwrite");
    tx.objectStore(IDMAP_STORE_NAME).put(value, key);
    tx.oncomplete = () => { db.close(); resolve(); };
    tx.onerror = () => { db.close(); reject(tx.error); };
  });
}

/**
 * Load a string value from IndexedDB by key.
 * Returns null if the key does not exist.
 */
async function loadFromIndexedDB(key: string): Promise<string | null> {
  const db = await openMetaDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDMAP_STORE_NAME, "readonly");
    const req = tx.objectStore(IDMAP_STORE_NAME).get(key);
    req.onsuccess = () => {
      db.close();
      resolve(typeof req.result === "string" ? req.result : null);
    };
    req.onerror = () => { db.close(); reject(req.error); };
  });
}

/** Open (or create) the shared metadata IndexedDB. */
function openMetaDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDMAP_DB_NAME, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(IDMAP_STORE_NAME)) {
        db.createObjectStore(IDMAP_STORE_NAME);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

// --- Serialized ID map shape for persistence ---

interface PersistedIdMapData {
  idMap: Record<string, number>;
  reverseIdMap: Record<string, string>; // numeric keys stringified by JSON
  metric: EdgeVecMetric;
  dimensions: number;
}

/**
 * LangChain VectorStore backed by EdgeVec's WASM-based HNSW index.
 *
 * Supports persistence via IndexedDB (`save`/`load`) with full ID mapping survival.
 *
 * ## Usage
 *
 * ```typescript
 * // Option 1: Factory method (auto-initializes WASM)
 * const store = await EdgeVecStore.fromTexts(
 *   ["hello world", "goodbye world"],
 *   [{ source: "a" }, { source: "b" }],
 *   embeddings,
 *   { dimensions: 384 }
 * );
 *
 * // Option 2: Manual init + constructor
 * await initEdgeVec();
 * const store = new EdgeVecStore(embeddings, { dimensions: 384 });
 *
 * // Persistence
 * await store.save("my-index");
 * const restored = await EdgeVecStore.load("my-index", embeddings);
 * ```
 */
export class EdgeVecStore extends SaveableVectorStore {
  declare FilterType: string;

  /** LangChain serialization namespace */
  lc_namespace = ["langchain", "vectorstores", "edgevec"];

  /** The underlying EdgeVec HNSW index */
  private index: EdgeVecIndex;

  /** String ID → numeric ID (as returned by index.add()) */
  private idMap: Map<string, number> = new Map();

  /** Numeric ID → string ID (reverse lookup for search results) */
  private reverseIdMap: Map<number, string> = new Map();

  /** Expected vector dimensions (for validation) */
  private dimensions: number;

  /** Distance metric for score normalization */
  private metric: EdgeVecMetric;

  _vectorstoreType(): string {
    return "edgevec";
  }

  /**
   * Create an EdgeVecStore instance.
   *
   * WASM must be initialized before calling this constructor.
   * Use factory methods (`fromTexts`, `fromDocuments`) for auto-initialization.
   *
   * @param embeddings - LangChain embeddings instance
   * @param config - EdgeVec index configuration
   * @param _internal - Internal: pre-loaded state for `load()`. Do not use directly.
   * @throws {EdgeVecNotInitializedError} If WASM not yet initialized
   */
  constructor(
    embeddings: EmbeddingsInterface,
    config: EdgeVecStoreConfig,
    _internal?: {
      index: EdgeVecIndex;
      idMap: Map<string, number>;
      reverseIdMap: Map<number, string>;
    }
  ) {
    super(embeddings, config);
    ensureInitialized();

    const { metric, ...indexConfig } = config;
    this.metric = metric ?? "cosine";
    this.dimensions = config.dimensions;

    if (_internal) {
      // Restore from load() — use pre-loaded state
      this.index = _internal.index;
      this.idMap = _internal.idMap;
      this.reverseIdMap = _internal.reverseIdMap;
    } else {
      this.index = new EdgeVecIndex(indexConfig);
    }
  }

  /**
   * Add precomputed vectors and their corresponding documents to the store.
   *
   * @param vectors - Array of embedding vectors (number[][])
   * @param documents - Array of LangChain documents (same length as vectors)
   * @param options - Optional: provide string IDs via `options.ids`
   * @returns Array of string IDs assigned to each document
   */
  async addVectors(
    vectors: number[][],
    documents: DocumentInterface[],
    options?: { ids?: string[] }
  ): Promise<string[]> {
    ensureInitialized();

    if (vectors.length !== documents.length) {
      throw new Error(
        `Mismatched lengths: ${vectors.length} vectors vs ${documents.length} documents`
      );
    }

    const ids: string[] = [];

    for (let i = 0; i < vectors.length; i++) {
      if (vectors[i].length !== this.dimensions) {
        throw new Error(
          `Vector at index ${i} has ${vectors[i].length} dimensions, expected ${this.dimensions}`
        );
      }

      const doc = documents[i];
      const stringId =
        options?.ids?.[i] ?? generateUUID();

      // Build metadata: pageContent + ID + user metadata
      const metadata: Metadata = {
        [PAGE_CONTENT_KEY]: doc.pageContent,
        [ID_KEY]: stringId,
        ...serializeMetadata(doc.metadata),
      };

      // index.add() is SYNC and returns the numeric ID
      const numericId = this.index.add(new Float32Array(vectors[i]), metadata);

      this.idMap.set(stringId, numericId);
      this.reverseIdMap.set(numericId, stringId);
      ids.push(stringId);
    }

    return ids;
  }

  /**
   * Add documents to the store (embeds them first via the embeddings instance).
   *
   * @param documents - Array of LangChain documents to embed and store
   * @param options - Optional: provide string IDs via `options.ids`
   * @returns Array of string IDs assigned to each document
   */
  async addDocuments(
    documents: DocumentInterface[],
    options?: { ids?: string[] }
  ): Promise<string[]> {
    const texts = documents.map((doc) => doc.pageContent);
    const vectors = await this.embeddings.embedDocuments(texts);
    return this.addVectors(vectors, documents, options);
  }

  /**
   * Delete documents by their string IDs.
   *
   * Unknown IDs are silently ignored (LangChain convention).
   * If EdgeVec reports the vector was already deleted, a warning is logged
   * and the ID mapping is still cleaned up.
   *
   * @param params - Object with `ids` array of string IDs to delete
   */
  async delete(params: { ids: string[] }): Promise<void> {
    ensureInitialized();

    for (const stringId of params.ids) {
      const numericId = this.idMap.get(stringId);
      if (numericId === undefined) continue; // unknown ID — no-op

      const deleted = this.index.delete(numericId);
      if (!deleted) {
        console.warn(
          `EdgeVecStore: delete(${numericId}) returned false — vector may already be deleted`
        );
      }

      // Clean maps regardless
      this.idMap.delete(stringId);
      this.reverseIdMap.delete(numericId);
    }
  }

  /**
   * Search for documents similar to a query vector.
   *
   * Returns documents paired with normalized similarity scores (higher = more similar).
   *
   * @param query - Query embedding vector
   * @param k - Number of results to return
   * @param filter - Optional EdgeVec filter DSL string
   * @returns Array of [Document, normalizedScore] tuples
   */
  async similaritySearchVectorWithScore(
    query: number[],
    k: number,
    filter?: string
  ): Promise<[DocumentInterface, number][]> {
    ensureInitialized();

    const searchOptions: SearchOptions = {
      includeMetadata: true,
    };
    if (filter !== undefined) {
      searchOptions.filter = filter;
    }

    const results = await this.index.search(
      new Float32Array(query),
      k,
      searchOptions
    );

    return results.map((result) => {
      const rawMetadata = result.metadata ?? {};

      const pageContent =
        typeof rawMetadata[PAGE_CONTENT_KEY] === "string"
          ? rawMetadata[PAGE_CONTENT_KEY]
          : "";

      const stringId =
        typeof rawMetadata[ID_KEY] === "string"
          ? rawMetadata[ID_KEY]
          : this.reverseIdMap.get(result.id);

      // Reconstruct user metadata (strips internal keys)
      const userMetadata = deserializeMetadata(rawMetadata);

      const docInput: { pageContent: string; metadata: Record<string, unknown>; id?: string } = {
        pageContent,
        metadata: userMetadata,
      };
      if (stringId !== undefined) {
        docInput.id = stringId;
      }
      const doc = new Document(docInput);

      return [doc, this.normalizeScore(result.score)];
    });
  }

  /**
   * Save the store to IndexedDB.
   *
   * Persists both the EdgeVec index (vectors, metadata, HNSW graph) and
   * the adapter's ID mapping + metric configuration.
   *
   * The `directory` parameter is used as the IndexedDB key (browser has no filesystem).
   *
   * @param directory - IndexedDB key name for this store
   */
  async save(directory: string): Promise<void> {
    ensureInitialized();

    // Save EdgeVec index (vectors + metadata + graph)
    await this.index.save(directory);

    // Save ID mapping + config separately
    const data: PersistedIdMapData = {
      idMap: Object.fromEntries(this.idMap),
      reverseIdMap: Object.fromEntries(this.reverseIdMap),
      metric: this.metric,
      dimensions: this.dimensions,
    };
    await saveToIndexedDB(directory, JSON.stringify(data));
  }

  /**
   * Load a store from IndexedDB.
   *
   * Reconstructs the EdgeVec index and the adapter's ID mapping from
   * previously saved data. Auto-initializes WASM.
   *
   * @param directory - IndexedDB key name used in `save()`
   * @param embeddings - LangChain embeddings instance
   * @returns Restored EdgeVecStore with full state
   * @throws {EdgeVecPersistenceError} If ID map data is missing or corrupted
   */
  static async load(
    directory: string,
    embeddings: EmbeddingsInterface
  ): Promise<EdgeVecStore> {
    await initEdgeVec();

    // Load EdgeVec index from IndexedDB
    const index = await EdgeVecIndex.load(directory);

    // Load ID map data
    const raw = await loadFromIndexedDB(directory);
    if (raw === null) {
      throw new EdgeVecPersistenceError(
        `No ID map data found for "${directory}". The index may have been saved without the adapter, or the data is corrupted.`
      );
    }

    let data: PersistedIdMapData;
    try {
      data = JSON.parse(raw) as PersistedIdMapData;
    } catch {
      throw new EdgeVecPersistenceError(
        `Failed to parse ID map data for "${directory}". The data may be corrupted.`
      );
    }

    // Validate required fields
    if (
      !data.idMap || typeof data.idMap !== "object" ||
      !data.reverseIdMap || typeof data.reverseIdMap !== "object" ||
      typeof data.dimensions !== "number" ||
      !Number.isFinite(data.dimensions) || data.dimensions <= 0 ||
      data.dimensions !== Math.floor(data.dimensions)
    ) {
      throw new EdgeVecPersistenceError(
        `Invalid ID map data for "${directory}". Missing required fields (idMap, reverseIdMap, dimensions).`
      );
    }

    // Validate metric is one of the allowed values
    const validMetrics: EdgeVecMetric[] = ["cosine", "l2", "dotproduct"];
    let metric: EdgeVecMetric;
    if (validMetrics.includes(data.metric as EdgeVecMetric)) {
      metric = data.metric as EdgeVecMetric;
    } else {
      console.warn(
        `EdgeVecStore.load: unknown metric "${String(data.metric)}" in "${directory}", defaulting to "cosine"`
      );
      metric = "cosine";
    }

    // Reconstruct maps with validation for corrupted numeric values
    const idMap = new Map<string, number>();
    for (const [k, v] of Object.entries(data.idMap)) {
      const num = Number(v);
      if (!Number.isFinite(num) || num !== Math.floor(num)) {
        throw new EdgeVecPersistenceError(
          `Invalid numeric ID "${String(v)}" for key "${k}" in "${directory}". ID map data may be corrupted.`
        );
      }
      idMap.set(k, num);
    }
    const reverseIdMap = new Map<number, string>();
    for (const [k, v] of Object.entries(data.reverseIdMap)) {
      const num = Number(k);
      if (!Number.isFinite(num) || num !== Math.floor(num)) {
        throw new EdgeVecPersistenceError(
          `Invalid numeric key "${k}" in reverseIdMap for "${directory}". ID map data may be corrupted.`
        );
      }
      reverseIdMap.set(num, String(v));
    }

    const config: EdgeVecStoreConfig = {
      dimensions: data.dimensions,
      metric,
    };

    return new EdgeVecStore(embeddings, config, {
      index,
      idMap,
      reverseIdMap,
    });
  }

  /**
   * Normalize a raw distance score to a similarity score in [0, 1].
   *
   * | Metric      | Formula                     | Range   |
   * |:------------|:----------------------------|:--------|
   * | cosine      | `1 - distance`              | [0, 1]  |
   * | l2          | `1 / (1 + distance)`        | (0, 1]  |
   * | dotproduct  | `1 / (1 + |distance|)`      | (0, 1]  |
   */
  private normalizeScore(rawScore: number): number {
    switch (this.metric) {
      case "cosine":
        return 1 - rawScore;
      case "l2":
        return 1 / (1 + rawScore);
      case "dotproduct":
        return 1 / (1 + Math.abs(rawScore));
      default:
        // Defensive: should never happen if metric is validated
        return 1 - rawScore;
    }
  }

  /**
   * Create an EdgeVecStore from text strings (auto-initializes WASM).
   *
   * @param texts - Array of text strings to embed and store
   * @param metadatas - Metadata for each text (array or single object)
   * @param embeddings - LangChain embeddings instance
   * @param config - EdgeVec index configuration
   * @returns Initialized EdgeVecStore with documents added
   */
  static async fromTexts(
    texts: string[],
    metadatas: object[] | object,
    embeddings: EmbeddingsInterface,
    config: EdgeVecStoreConfig
  ): Promise<EdgeVecStore> {
    await initEdgeVec();

    const docs = texts.map(
      (text, i) =>
        new Document({
          pageContent: text,
          metadata: Array.isArray(metadatas) ? metadatas[i] ?? {} : metadatas,
        })
    );

    const store = new EdgeVecStore(embeddings, config);
    await store.addDocuments(docs);
    return store;
  }

  /**
   * Create an EdgeVecStore from documents (auto-initializes WASM).
   *
   * @param docs - Array of LangChain documents
   * @param embeddings - LangChain embeddings instance
   * @param config - EdgeVec index configuration
   * @returns Initialized EdgeVecStore with documents added
   */
  static async fromDocuments(
    docs: DocumentInterface[],
    embeddings: EmbeddingsInterface,
    config: EdgeVecStoreConfig
  ): Promise<EdgeVecStore> {
    await initEdgeVec();

    const store = new EdgeVecStore(embeddings, config);
    await store.addDocuments(docs);
    return store;
  }
}
