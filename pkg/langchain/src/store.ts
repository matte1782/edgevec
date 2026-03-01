/**
 * EdgeVecStore — LangChain.js VectorStore adapter for EdgeVec
 *
 * Core class that bridges EdgeVec's WASM-based vector index with
 * LangChain's VectorStore interface. Handles:
 * - Bidirectional ID mapping (string ↔ numeric)
 * - Metadata serialization via the metadata module
 * - Score normalization (distance → similarity)
 *
 * @module store
 */

import { VectorStore } from "@langchain/core/vectorstores";
import { Document } from "@langchain/core/documents";
import type { EmbeddingsInterface } from "@langchain/core/embeddings";
import type { DocumentInterface } from "@langchain/core/documents";
import { EdgeVecIndex } from "edgevec/edgevec-wrapper.js";
import type { Metadata, SearchOptions } from "edgevec/edgevec-wrapper.js";

import { randomUUID } from "node:crypto";

import { ensureInitialized, initEdgeVec } from "./init.js";
import { serializeMetadata, deserializeMetadata, PAGE_CONTENT_KEY, ID_KEY } from "./metadata.js";
import type { EdgeVecStoreConfig, EdgeVecMetric } from "./types.js";

/**
 * LangChain VectorStore backed by EdgeVec's WASM-based HNSW index.
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
 * ```
 */
export class EdgeVecStore extends VectorStore {
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
   * @throws {EdgeVecNotInitializedError} If WASM not yet initialized
   */
  constructor(embeddings: EmbeddingsInterface, config: EdgeVecStoreConfig) {
    super(embeddings, config);
    ensureInitialized();

    const { metric, ...indexConfig } = config;
    this.metric = metric ?? "cosine";
    this.dimensions = config.dimensions;
    this.index = new EdgeVecIndex(indexConfig);
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
        options?.ids?.[i] ?? randomUUID();

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

      // Extract page content
      const pageContent =
        typeof rawMetadata[PAGE_CONTENT_KEY] === "string"
          ? (rawMetadata[PAGE_CONTENT_KEY] as string)
          : "";

      // Extract string ID from metadata or reverse map
      const stringId =
        typeof rawMetadata[ID_KEY] === "string"
          ? (rawMetadata[ID_KEY] as string)
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
