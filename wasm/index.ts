// wasm/index.ts

export { EdgeVecClient, SearchResult, EdgeVecClientConfig } from './EdgeVecClient.js';
export { EdgeVecConfigBuilder } from './EdgeVecConfig.js';
export {
  VectorId,
  DistanceMetric,
  QuantizationMode,
  EdgeVecStats,
  CompactionResult,
  SoftDeleteStats
} from './types.js';

// Re-export raw WASM bindings for advanced users
export * from '../pkg/edgevec.js';
