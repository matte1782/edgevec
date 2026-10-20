/**
 * useEdgeVec - Vue 3 composable for EdgeVec database initialization
 *
 * Creates/loads an EdgeVec index after the application initializes WASM.
 * Call the default `init` export from `edgevec` before mounting components.
 * Handles persistence loading from IndexedDB when persistName is provided.
 *
 * @module edgevec/vue
 * @version 0.8.0
 *
 * @example
 * ```vue
 * <script setup lang="ts">
 * import { useEdgeVec } from 'edgevec/vue';
 *
 * const { db, isReady, isLoading, error, stats } = useEdgeVec({
 *   dimensions: 384,
 *   persistName: 'my-vectors'
 * });
 * </script>
 *
 * <template>
 *   <div v-if="isLoading">Loading EdgeVec...</div>
 *   <div v-else-if="error">Error: {{ error.message }}</div>
 *   <div v-else-if="isReady">Loaded {{ stats?.count }} vectors</div>
 * </template>
 * ```
 */

import { ref, shallowRef, onMounted, onUnmounted } from 'vue';
import { EdgeVecIndex, IndexConfig } from '../edgevec-wrapper.js';
import type { UseEdgeVecOptions, UseEdgeVecResult } from './types.js';
import type { Ref } from 'vue';

/**
 * Vue 3 composable for EdgeVec database initialization.
 *
 * @param options - Configuration options
 * @returns Composable result with db instance and reactive status
 */
export function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecResult {
  const {
    dimensions,
    persistName,
    efConstruction = 200,
    m = 16,
  } = options;

  // Reactive state using Vue refs
  // Use shallowRef for db to avoid deep reactivity on the class instance
  const db: Ref<EdgeVecIndex | null> = shallowRef(null);
  const isLoading = ref(true);
  const isReady = ref(false);
  const error = ref<Error | null>(null);
  const stats = ref<{ count: number; dimensions: number } | null>(null);

  // Track mounted state for cleanup (set in onMounted)
  let isMounted = false;

  async function initialize(): Promise<void> {
    try {
      isLoading.value = true;
      error.value = null;
      isReady.value = false;

      let index: EdgeVecIndex;

      // Try to load from IndexedDB if persistName is provided
      if (persistName) {
        try {
          index = await EdgeVecIndex.load(persistName);
        } catch {
          // If load fails, create new index
          const config: IndexConfig = {
            dimensions,
            efConstruction,
            m,
          };
          index = new EdgeVecIndex(config);
        }
      } else {
        // Create new index
        const config: IndexConfig = {
          dimensions,
          efConstruction,
          m,
        };
        index = new EdgeVecIndex(config);
      }

      // Only update state if still mounted
      if (isMounted) {
        db.value = index;
        stats.value = {
          count: index.size,
          dimensions: index.dimensions || dimensions,
        };
        isReady.value = true;
        isLoading.value = false;
      }
    } catch (err) {
      if (isMounted) {
        error.value = err instanceof Error ? err : new Error(String(err));
        isLoading.value = false;
      }
    }
  }

  async function reload(): Promise<void> {
    db.value = null;
    isReady.value = false;
    stats.value = null;
    error.value = null;
    await initialize();
  }

  async function save(name?: string): Promise<void> {
    const database = db.value;
    if (!database) {
      throw new Error('Database not initialized');
    }
    const saveName = name || persistName;
    if (!saveName) {
      throw new Error('No persist name provided');
    }
    await database.save(saveName);
    // Update stats after save
    if (isMounted) {
      stats.value = {
        count: database.size,
        dimensions: database.dimensions,
      };
    }
  }

  // Initialize on mount
  onMounted(() => {
    isMounted = true;
    initialize();
  });

  // Cleanup on unmount
  onUnmounted(() => {
    isMounted = false;
  });

  return {
    db,
    isReady,
    isLoading,
    error,
    stats,
    reload,
    save,
  };
}
