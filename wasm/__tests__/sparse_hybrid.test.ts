// wasm/__tests__/sparse_hybrid.test.ts
// W41.1.5: WASM integration test for sparse/hybrid search (v0.9.0)

import { EdgeVecClient } from '../EdgeVecClient';
import { EdgeVecConfigBuilder } from '../EdgeVecConfig';

describe('Sparse & Hybrid Search Integration (v0.9.0)', () => {
  let client: EdgeVecClient;

  beforeEach(async () => {
    const config = new EdgeVecConfigBuilder(128)
      .withMetric('cosine')
      .build();
    client = await EdgeVecClient.create(config);
  });

  describe('Sparse Storage', () => {
    it('should initialize sparse storage', () => {
      // Initially no sparse storage
      expect(client.inner.hasSparseStorage()).toBe(false);

      // Initialize
      client.inner.initSparseStorage();
      expect(client.inner.hasSparseStorage()).toBe(true);
      expect(client.inner.sparseCount()).toBe(0);
    });

    it('should insert and search sparse vectors', () => {
      client.inner.initSparseStorage();

      // Insert sparse vectors (simulating BM25 term scores)
      const indices1 = new Uint32Array([0, 5, 10]);
      const values1 = new Float32Array([1.0, 2.0, 3.0]);
      const id1 = client.inner.insertSparse(indices1, values1, 1000);
      expect(id1).toBeGreaterThanOrEqual(0);

      const indices2 = new Uint32Array([5, 10, 20]);
      const values2 = new Float32Array([1.5, 1.0, 2.5]);
      const id2 = client.inner.insertSparse(indices2, values2, 1000);
      expect(id2).toBeGreaterThanOrEqual(0);

      expect(client.inner.sparseCount()).toBe(2);

      // Search sparse
      const queryIndices = new Uint32Array([5, 10]);
      const queryValues = new Float32Array([2.0, 1.0]);
      const resultsJson = client.inner.searchSparse(queryIndices, queryValues, 1000, 2);
      const results = JSON.parse(resultsJson);

      expect(results.length).toBe(2);
      // Results should be sorted by score (descending)
      expect(results[0].score).toBeGreaterThanOrEqual(results[1].score);
    });
  });

  describe('Hybrid Search', () => {
    it('should perform hybrid search combining dense and sparse', () => {
      // Insert dense vectors
      const vectors = Array.from({ length: 20 }, (_, i) => {
        const vec = new Float32Array(128);
        for (let j = 0; j < 128; j++) {
          vec[j] = Math.sin(i * 0.1 + j * 0.01);
        }
        return vec;
      });

      for (const vec of vectors) {
        client.insert(vec);
      }

      // Initialize and insert sparse vectors
      client.inner.initSparseStorage();
      for (let i = 0; i < 20; i++) {
        const indices = new Uint32Array([i, i + 100]);
        const values = new Float32Array([1.0 + i * 0.1, 0.5]);
        client.inner.insertSparse(indices, values, 1000);
      }

      // Hybrid search
      const denseQuery = new Float32Array(128);
      for (let j = 0; j < 128; j++) {
        denseQuery[j] = Math.sin(5 * 0.1 + j * 0.01);
      }
      const sparseIndices = new Uint32Array([5, 105]);
      const sparseValues = new Float32Array([1.5, 0.5]);

      const options = JSON.stringify({
        dense_k: 10,
        sparse_k: 10,
        k: 5,
        fusion: 'rrf'
      });

      const resultsJson = client.inner.hybridSearch(
        denseQuery,
        sparseIndices,
        sparseValues,
        1000,
        options
      );
      const results = JSON.parse(resultsJson);

      expect(results.length).toBeGreaterThan(0);
      expect(results.length).toBeLessThanOrEqual(5);
      // Each result should have id and score
      for (const r of results) {
        expect(r.id).toBeDefined();
        expect(r.score).toBeDefined();
        expect(typeof r.score).toBe('number');
      }
    });
  });

  describe('Binary Vector Search', () => {
    it('should insert and search binary vectors with correct Hamming distances', () => {
      // Insert binary vectors with known bit patterns
      // Vector 1: all bits set (0xFF per byte, 128 bits set)
      const v1 = new Uint8Array(16);
      v1.fill(0xFF);
      const id1 = client.inner.insertBinary(v1);
      expect(id1).toBeGreaterThanOrEqual(0);

      // Vector 2: all bits zero (0x00 per byte, 0 bits set)
      const v2 = new Uint8Array(16);
      v2.fill(0x00);
      const id2 = client.inner.insertBinary(v2);
      expect(id2).toBeGreaterThanOrEqual(0);

      // Vector 3: alternating pattern (0xAA = 10101010, 64 bits set)
      const v3 = new Uint8Array(16);
      v3.fill(0xAA);
      const id3 = client.inner.insertBinary(v3);
      expect(id3).toBeGreaterThanOrEqual(0);

      // Search with all-zero query: expected Hamming distances:
      //   v2 (0x00): distance = 0 (exact match)
      //   v3 (0xAA): distance = 64 (half bits differ)
      //   v1 (0xFF): distance = 128 (all bits differ)
      const query = new Uint8Array(16);
      query.fill(0x00);
      const results = client.inner.searchBinary(query, 3);

      expect(results).toBeDefined();
      expect(typeof results).toBe('string');

      const parsed = JSON.parse(results);
      expect(parsed.length).toBe(3);

      // Results should be sorted by ascending Hamming distance
      for (let i = 1; i < parsed.length; i++) {
        expect(parsed[i - 1].distance).toBeLessThanOrEqual(parsed[i].distance);
      }

      // First result should be exact match (distance 0)
      expect(parsed[0].distance).toBe(0);

      // Second result should be the alternating pattern (distance 64)
      expect(parsed[1].distance).toBe(64);

      // Third result should be all-ones (distance 128)
      expect(parsed[2].distance).toBe(128);
    });

    it('should return empty results for empty index', () => {
      const query = new Uint8Array(16);
      query.fill(0x00);
      const results = client.inner.searchBinary(query, 5);
      expect(results).toBeDefined();
      const parsed = JSON.parse(results);
      expect(parsed.length).toBe(0);
    });
  });
});
