/**
 * EdgeVec PQ WASM Benchmark Harness
 *
 * Provides benchADC(), benchTraining(), and computeGroundTruth() for
 * measuring PQ performance in the browser via Playwright or DevTools.
 *
 * Usage (from console or Playwright browser_evaluate):
 *   await benchADC(1000, 1, 5)       // 1K codes, 1 query, 5 iterations
 *   await benchTraining(1000, 3)      // 1K vectors, 3 iterations
 *   computeGroundTruth(vectors, queries, 10)  // brute-force top-10
 */

// Seeded PRNG (xorshift128+) for reproducible synthetic data
class SeededRng {
  constructor(seed) {
    this.s0 = seed | 0;
    this.s1 = seed ^ 0xDEADBEEF;
    if (this.s0 === 0) this.s0 = 1;
    if (this.s1 === 0) this.s1 = 1;
    // warm up
    for (let i = 0; i < 20; i++) this.nextU32();
  }

  nextU32() {
    let s1 = this.s0;
    const s0 = this.s1;
    this.s0 = s0;
    s1 ^= s1 << 23;
    s1 ^= s1 >> 17;
    s1 ^= s0;
    s1 ^= s0 >> 26;
    this.s1 = s1;
    return (this.s0 + this.s1) >>> 0;
  }

  // Returns float in [0, 1)
  nextFloat() {
    return this.nextU32() / 4294967296;
  }

  // Returns float in [-1, 1)
  nextFloatSigned() {
    return this.nextFloat() * 2 - 1;
  }
}

/**
 * Generate synthetic vectors for benchmarking.
 * @param {number} n - number of vectors
 * @param {number} dims - dimensionality
 * @param {number} seed - RNG seed
 * @returns {Float32Array} flat array of n * dims floats
 */
function generateSyntheticVectors(n, dims, seed) {
  const rng = new SeededRng(seed);
  const data = new Float32Array(n * dims);
  for (let i = 0; i < data.length; i++) {
    data[i] = rng.nextFloatSigned();
  }
  return data;
}

/**
 * Compute percentile from sorted array using linear interpolation.
 * For small sample sizes (< 100), P99 will interpolate between the
 * highest values rather than always returning the max.
 * @param {number[]} sorted - sorted array of values
 * @param {number} p - percentile (0-100)
 * @returns {number}
 */
function percentile(sorted, p) {
  if (sorted.length === 0) return 0;
  if (sorted.length === 1) return sorted[0];
  const rank = (p / 100) * (sorted.length - 1);
  const lo = Math.floor(rank);
  const hi = Math.ceil(rank);
  const frac = rank - lo;
  return sorted[lo] + frac * (sorted[hi] - sorted[lo]);
}

/**
 * Benchmark ADC (Asymmetric Distance Computation) search.
 *
 * Trains a codebook, encodes nCodes vectors, then measures nQueries
 * ADC scan latency over iterations. Reports median and P99 per-candidate ns.
 *
 * @param {number} nCodes - number of database vectors to encode
 * @param {number} nQueries - number of queries per iteration
 * @param {number} iterations - number of measured iterations (3 warmup added)
 * @param {object} [opts] - options
 * @param {number} [opts.dims=768] - vector dimensionality
 * @param {number} [opts.m=8] - number of subquantizers
 * @param {number} [opts.ksub=256] - centroids per subspace
 * @param {number} [opts.k=10] - top-k results to return
 * @param {number} [opts.maxIters=5] - training iterations
 * @returns {Promise<object>} timing results
 */
async function benchADC(nCodes, nQueries, iterations, opts = {}) {
  if (iterations < 1) throw new Error('iterations must be >= 1');
  if (nCodes < 1) throw new Error('nCodes must be >= 1');
  if (nQueries < 1) throw new Error('nQueries must be >= 1');
  const dims = opts.dims || 768;
  const m = opts.m || 8;
  const ksub = opts.ksub || 256;
  const k = opts.k || 10;
  const maxIters = opts.maxIters || 5;
  const warmup = 3;

  console.log(`[benchADC] nCodes=${nCodes}, nQueries=${nQueries}, iters=${iterations}, dims=${dims}, M=${m}, Ksub=${ksub}`);

  // Generate data
  const data = generateSyntheticVectors(nCodes, dims, 42);
  const queries = [];
  for (let q = 0; q < nQueries; q++) {
    queries.push(generateSyntheticVectors(1, dims, 1000 + q));
  }

  // Train codebook
  console.log(`[benchADC] Training codebook...`);
  const t0 = performance.now();
  const codebook = window.__edgevec.trainPq(data, dims, nCodes, m, ksub, maxIters);
  const trainMs = performance.now() - t0;
  console.log(`[benchADC] Training done in ${trainMs.toFixed(1)}ms`);

  // Encode all vectors
  console.log(`[benchADC] Encoding ${nCodes} vectors...`);
  const encStart = performance.now();
  const allCodes = new Uint8Array(nCodes * m);
  for (let i = 0; i < nCodes; i++) {
    const vec = data.subarray(i * dims, (i + 1) * dims);
    const code = codebook.encodePq(vec);
    allCodes.set(code, i * m);
  }
  const encMs = performance.now() - encStart;
  console.log(`[benchADC] Encoding done in ${encMs.toFixed(1)}ms`);

  // Benchmark ADC search
  const timings = [];

  for (let iter = 0; iter < warmup + iterations; iter++) {
    const iterStart = performance.now();

    for (let q = 0; q < nQueries; q++) {
      codebook.pqSearch(allCodes, nCodes, queries[q], k);
    }

    const iterMs = performance.now() - iterStart;
    if (iter >= warmup) {
      timings.push(iterMs);
    }
  }

  // Compute stats
  const totalCandidates = nQueries * nCodes;
  const nsPerCandidate = timings.map(ms => (ms * 1e6) / totalCandidates);
  nsPerCandidate.sort((a, b) => a - b);

  const result = {
    nCodes,
    nQueries,
    iterations,
    dims,
    m,
    ksub,
    k,
    totalCandidatesPerIter: totalCandidates,
    trainMs: Math.round(trainMs),
    encodeMs: Math.round(encMs),
    median_ns_per_candidate: Math.round(percentile(nsPerCandidate, 50)),
    p99_ns_per_candidate: Math.round(percentile(nsPerCandidate, 99)),
    min_ns_per_candidate: Math.round(nsPerCandidate[0]),
    max_ns_per_candidate: Math.round(nsPerCandidate[nsPerCandidate.length - 1]),
    rawTimingsMs: timings.map(t => Math.round(t * 100) / 100),
  };

  console.log(`[benchADC] Result:`, JSON.stringify(result, null, 2));
  codebook.free();
  return result;
}

/**
 * Benchmark PQ training time.
 *
 * @param {number} nVectors - number of training vectors
 * @param {number} iterations - number of measured iterations (3 warmup added)
 * @param {object} [opts] - options
 * @param {number} [opts.dims=768] - vector dimensionality
 * @param {number} [opts.m=8] - number of subquantizers
 * @param {number} [opts.ksub=256] - centroids per subspace
 * @param {number} [opts.maxIters=5] - training iterations
 * @returns {Promise<object>} timing results
 */
async function benchTraining(nVectors, iterations, opts = {}) {
  if (iterations < 1) throw new Error('iterations must be >= 1');
  if (nVectors < 1) throw new Error('nVectors must be >= 1');
  const dims = opts.dims || 768;
  const m = opts.m || 8;
  const ksub = opts.ksub || 256;
  const maxIters = opts.maxIters || 5;
  const warmup = 3;

  console.log(`[benchTraining] nVectors=${nVectors}, iters=${iterations}, dims=${dims}, M=${m}, Ksub=${ksub}`);

  const timings = [];

  for (let iter = 0; iter < warmup + iterations; iter++) {
    // Generate fresh data each iteration (different seed for variety)
    const data = generateSyntheticVectors(nVectors, dims, 42 + iter);

    const t0 = performance.now();
    const codebook = window.__edgevec.trainPq(data, dims, nVectors, m, ksub, maxIters);
    const elapsedMs = performance.now() - t0;

    codebook.free();

    if (iter >= warmup) {
      timings.push(elapsedMs / 1000); // convert to seconds
    }
  }

  timings.sort((a, b) => a - b);

  const result = {
    nVectors,
    iterations,
    dims,
    m,
    ksub,
    maxIters,
    median_s: Math.round(percentile(timings, 50) * 1000) / 1000,
    p99_s: Math.round(percentile(timings, 99) * 1000) / 1000,
    min_s: Math.round(timings[0] * 1000) / 1000,
    max_s: Math.round(timings[timings.length - 1] * 1000) / 1000,
    rawTimingsS: timings.map(t => Math.round(t * 1000) / 1000),
  };

  console.log(`[benchTraining] Result:`, JSON.stringify(result, null, 2));
  return result;
}

/**
 * Compute brute-force L2 ground truth for recall measurement.
 * Cost excluded from timing per PQ_BENCHMARK_PLAN Section 2.3.
 *
 * @param {Float32Array} vectors - flat array of n * dims
 * @param {number} n - number of vectors
 * @param {number} dims - dimensionality
 * @param {Float32Array[]} queries - array of query vectors
 * @param {number} k - top-k
 * @returns {number[][]} array of top-k index arrays (one per query)
 */
function computeGroundTruth(vectors, n, dims, queries, k) {
  console.log(`[groundTruth] Computing brute-force L2 for ${queries.length} queries over ${n} vectors (${dims}D)...`);
  const t0 = performance.now();

  const results = [];

  for (const query of queries) {
    // Compute L2 distances
    const dists = new Float32Array(n);
    for (let i = 0; i < n; i++) {
      let sum = 0;
      const offset = i * dims;
      for (let d = 0; d < dims; d++) {
        const diff = query[d] - vectors[offset + d];
        sum += diff * diff;
      }
      dists[i] = sum;
    }

    // Find top-k via full sort (acceptable: ground truth cost excluded from timing)
    const indices = Array.from({ length: n }, (_, i) => i);
    indices.sort((a, b) => dists[a] - dists[b]);
    results.push(indices.slice(0, k));
  }

  const elapsed = performance.now() - t0;
  console.log(`[groundTruth] Done in ${elapsed.toFixed(1)}ms`);
  return results;
}

// Expose globally for Playwright browser_evaluate
window.benchADC = benchADC;
window.benchTraining = benchTraining;
window.computeGroundTruth = computeGroundTruth;
window.generateSyntheticVectors = generateSyntheticVectors;
