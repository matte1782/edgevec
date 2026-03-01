/**
 * WASM Initialization Module for edgevec-langchain
 *
 * Provides singleton initialization of the EdgeVec WASM module.
 * Browser-only — does NOT support SSR/Node.js server-side rendering.
 *
 * ## Usage Pattern
 *
 * **Factory methods** (`fromTexts`, `fromDocuments`) call `initEdgeVec()` automatically.
 * **Constructor** requires WASM to be already initialized — use factory methods for convenience.
 *
 * ```typescript
 * // Option 1: Factory method (auto-init)
 * const store = await EdgeVecStore.fromTexts(texts, metadatas, embeddings, config);
 *
 * // Option 2: Manual init + constructor
 * await initEdgeVec();
 * const store = new EdgeVecStore(embeddings, config);
 * ```
 *
 * @module init
 */

import init from "edgevec";

/** Tracks whether WASM has been initialized */
let initialized = false;

/** Tracks in-flight initialization to prevent double-init races */
let initPromise: Promise<void> | null = null;

/**
 * Initialize the EdgeVec WASM module.
 *
 * Safe to call multiple times — subsequent calls are no-ops.
 * Concurrent calls during initialization will await the same promise.
 *
 * @throws {Error} If WASM initialization fails (e.g., network error loading .wasm)
 */
export async function initEdgeVec(): Promise<void> {
  if (initialized) return;

  if (initPromise) {
    await initPromise;
    return;
  }

  initPromise = (async () => {
    try {
      await init();
      initialized = true;
    } catch (e) {
      initPromise = null;
      throw e;
    }
  })();

  await initPromise;
}

/**
 * Error thrown when EdgeVec WASM module is not initialized.
 *
 * Thrown by `EdgeVecStore` constructor and instance methods
 * if called before `initEdgeVec()` or a factory method.
 */
export class EdgeVecNotInitializedError extends Error {
  constructor() {
    super(
      "EdgeVec WASM module is not initialized. " +
        "Call `await initEdgeVec()` first, or use a factory method like `EdgeVecStore.fromTexts()` which auto-initializes."
    );
    this.name = "EdgeVecNotInitializedError";
  }
}

/**
 * Guard that throws if WASM is not initialized.
 *
 * Must be called at the entry of every public method on `EdgeVecStore`.
 *
 * @throws {EdgeVecNotInitializedError} If WASM not yet initialized
 */
export function ensureInitialized(): void {
  if (!initialized) {
    throw new EdgeVecNotInitializedError();
  }
}

/**
 * Check if WASM is currently initialized (for testing).
 */
export function isInitialized(): boolean {
  return initialized;
}

/**
 * Reset initialization state (for testing only).
 * @internal
 */
export function _resetForTesting(): void {
  initialized = false;
  initPromise = null;
}
