// wasm/EdgeVecConfig.ts

import type { EdgeVecClientConfig } from './EdgeVecClient.js';

/**
 * Fluent builder for EdgeVec configuration.
 *
 * @example
 * ```typescript
 * const config = new EdgeVecConfigBuilder(128)
 *   .withMetric('cosine')
 *   .build();
 * ```
 */
export class EdgeVecConfigBuilder {
  private config: EdgeVecClientConfig;

  constructor(dimensions: number) {
    if (dimensions <= 0) {
      throw new Error('Dimensions must be positive');
    }
    this.config = { dimensions };
  }

  /**
   * Set the distance metric.
   *
   * @param metric - 'l2' (Euclidean), 'cosine', or 'dot' (inner product)
   */
  withMetric(metric: 'l2' | 'cosine' | 'dot'): this {
    this.config.metric = metric;
    return this;
  }

  /**
   * Set the quantization preference. Only 'none' is currently supported by
   * EdgeVecClient; create() rejects 'sq8' until WASM exposes scalar quantization.
   *
   * @param quantization - 'none' or 'sq8' (8-bit scalar quantization)
   */
  withQuantization(quantization: 'none' | 'sq8'): this {
    this.config.quantization = quantization;
    return this;
  }

  /**
   * Build the configuration object.
   */
  build(): EdgeVecClientConfig {
    return { ...this.config };
  }
}
