/**
 * edgevec-langchain — LangChain.js VectorStore adapter for EdgeVec
 *
 * @module edgevec-langchain
 * @version 0.1.0
 */

// WASM initialization
export { initEdgeVec, EdgeVecNotInitializedError } from "./init.js";

// Types
export type { EdgeVecStoreConfig, EdgeVecMetric } from "./types.js";
export { MetadataSerializationError, EdgeVecPersistenceError } from "./types.js";

// Metadata serialization
export { serializeMetadata, deserializeMetadata } from "./metadata.js";

// Filter API (re-exported from edgevec for convenience)
export { Filter, type FilterExpression } from "edgevec/edgevec-wrapper.js";

// Core store
export { EdgeVecStore } from "./store.js";
