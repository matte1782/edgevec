/**
 * edgevec-langchain — LangChain.js VectorStore adapter for EdgeVec
 *
 * @module edgevec-langchain
 * @version 0.1.0
 */

// WASM initialization
export { initEdgeVec, EdgeVecNotInitializedError, isInitialized } from "./init.js";

// Types
export type { EdgeVecStoreConfig, EdgeVecMetric } from "./types.js";
export { MetadataSerializationError, EdgeVecPersistenceError } from "./types.js";

// Metadata serialization
export { serializeMetadata, deserializeMetadata } from "./metadata.js";

// Core store
export { EdgeVecStore } from "./store.js";
