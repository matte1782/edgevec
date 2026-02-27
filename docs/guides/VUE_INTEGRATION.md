# Vue 3 Integration Guide

**Version:** EdgeVec v0.8.0
**Last Updated:** 2026-01-08
**Requires:** Vue 3.3+, TypeScript 5.0+ (recommended)

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [useEdgeVec Composable](#useedgevec-composable)
4. [useSearch Composable](#usesearch-composable)
5. [Complete Example](#complete-example)
6. [API Reference](#api-reference)
7. [Advanced Patterns](#advanced-patterns)
8. [TypeScript Setup](#typescript-setup)
9. [Vue vs React Differences](#vue-vs-react-differences)
10. [Troubleshooting](#troubleshooting)

---

## Installation

```bash
npm install edgevec vue
```

Or with yarn/pnpm:

```bash
yarn add edgevec vue
pnpm add edgevec vue
```

---

## Quick Start

```vue
<script setup lang="ts">
import { useEdgeVec } from 'edgevec/vue';

const { db, isReady, isLoading, error, stats } = useEdgeVec({
  dimensions: 384,
  persistName: 'my-vectors',
});
</script>

<template>
  <div v-if="isLoading">Loading EdgeVec...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else-if="isReady">{{ stats?.count }} vectors indexed</div>
</template>
```

---

## useEdgeVec Composable

Initialize an EdgeVec database with automatic WASM loading and lifecycle management.

### Basic Usage

```vue
<script setup lang="ts">
import { useEdgeVec } from 'edgevec/vue';

const { db, isReady, isLoading, error, stats, save, reload } = useEdgeVec({
  dimensions: 384,
  persistName: 'my-vectors',  // Optional: enables IndexedDB persistence
});

// Insert a vector when ready
async function addVector() {
  if (!isReady.value || !db.value) return;

  const vector = new Float32Array(384).map(() => Math.random());
  const id = db.value.insertWithMetadata(vector, {
    category: 'documents',
    timestamp: Date.now(),
  });

  console.log('Inserted vector:', id);
}

// Save to IndexedDB
async function saveDatabase() {
  await save();
  console.log('Database saved');
}
</script>

<template>
  <div v-if="isLoading">Loading EdgeVec...</div>
  <div v-else-if="error">Error: {{ error.message }}</div>
  <div v-else>
    <p>{{ stats?.count }} vectors indexed</p>
    <button @click="addVector">Add Vector</button>
    <button @click="saveDatabase">Save</button>
  </div>
</template>
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `dimensions` | `number` | **required** | Vector dimensions (e.g., 384, 768, 1536) |
| `persistName` | `string` | `undefined` | IndexedDB store name for persistence |
| `efConstruction` | `number` | `200` | HNSW build parameter (higher = better recall, slower build) |
| `m` | `number` | `16` | HNSW max connections per node |

### Return Values

| Property | Type | Description |
|:---------|:-----|:------------|
| `db` | `Ref<EdgeVec \| null>` | Database instance (null until ready) |
| `isReady` | `Ref<boolean>` | True when WASM loaded and db initialized |
| `isLoading` | `Ref<boolean>` | True during initialization |
| `error` | `Ref<Error \| null>` | Initialization error if any |
| `stats` | `Ref<{ count: number } \| null>` | Database statistics |
| `save` | `() => Promise<void>` | Save to IndexedDB |
| `reload` | `() => Promise<void>` | Reload from IndexedDB |

---

## useSearch Composable

Perform reactive searches that automatically update when the query changes.

### Basic Usage

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import { useEdgeVec, useSearch } from 'edgevec/vue';
import { eq, and, gt } from 'edgevec';

const { db, isReady } = useEdgeVec({ dimensions: 384 });
const queryVector = ref<number[] | null>(null);

const { results, isSearching, searchTime, error } = useSearch(db, {
  vector: queryVector,
  k: 10,
  filter: and(eq('category', 'documents'), gt('score', 0.5)),
  enabled: computed(() => isReady.value && queryVector.value !== null),
  debounceMs: 300,
});

// Trigger search
async function search() {
  // Get embedding from your embedding model (see Embedding Guide)
  const embedding = await getEmbedding('search query');
  queryVector.value = embedding;
}

// Placeholder - replace with real embedding function
async function getEmbedding(text: string): Promise<number[]> {
  return Array.from({ length: 384 }, () => Math.random());
}
</script>

<template>
  <div>
    <button @click="search">Search</button>

    <span v-if="isSearching">Searching...</span>
    <span v-else-if="searchTime">Found in {{ searchTime.toFixed(1) }}ms</span>

    <ul>
      <li v-for="result in results" :key="result.id">
        ID: {{ result.id }}, Score: {{ result.score.toFixed(4) }}
      </li>
    </ul>
  </div>
</template>
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `vector` | `Ref<Float32Array \| number[] \| null>` | **required** | Query vector (reactive ref) |
| `k` | `number \| Ref<number>` | `10` | Number of results to return |
| `filter` | `FilterExpression \| string \| Ref` | `undefined` | Filter expression |
| `enabled` | `boolean \| Ref<boolean> \| ComputedRef` | `true` | Enable/disable search |
| `debounceMs` | `number` | `0` | Debounce delay in milliseconds |
| `includeMetadata` | `boolean` | `false` | Include metadata in results |
| `includeVectors` | `boolean` | `false` | Include vectors in results |

### Return Values

| Property | Type | Description |
|:---------|:-----|:------------|
| `results` | `Ref<SearchResult[]>` | Search results array |
| `isSearching` | `Ref<boolean>` | True during search |
| `error` | `Ref<Error \| null>` | Search error if any |
| `searchTime` | `Ref<number \| null>` | Search duration in milliseconds |
| `refetch` | `() => Promise<void>` | Manually trigger search |

---

## Complete Example

A full semantic search component with EdgeVec:

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import { useEdgeVec, useSearch } from 'edgevec/vue';
import { and, eq, gt } from 'edgevec';

// Initialize database
const { db, isReady, stats, save } = useEdgeVec({
  dimensions: 384,
  persistName: 'semantic-search-demo'
});

// Search state
const query = ref('');
const queryVector = ref<number[] | null>(null);
const minRelevance = ref(0.5);
const category = ref('all');

// Dynamic filter based on user selection
const activeFilter = computed(() => {
  if (category.value === 'all') {
    return gt('relevance', minRelevance.value);
  }
  return and(
    eq('category', category.value),
    gt('relevance', minRelevance.value)
  );
});

// Reactive search
const { results, isSearching, searchTime, error } = useSearch(db, {
  vector: queryVector,
  k: 10,
  filter: activeFilter,
  enabled: computed(() => isReady.value && queryVector.value !== null),
  debounceMs: 300
});

// Handle search
async function handleSearch() {
  if (!query.value.trim()) return;

  // Replace with your embedding function
  const embedding = await getEmbedding(query.value);
  queryVector.value = embedding;
}

// Add sample documents
async function addSampleDocs() {
  if (!db.value) return;

  const docs = [
    { text: 'Vue 3 composition API', category: 'programming' },
    { text: 'React hooks tutorial', category: 'programming' },
    { text: 'Best coffee recipes', category: 'food' },
  ];

  for (const doc of docs) {
    const embedding = await getEmbedding(doc.text);
    db.value.insertWithMetadata(new Float32Array(embedding), {
      text: doc.text,
      category: doc.category,
      relevance: 1.0,
    });
  }

  await save();
}

// Placeholder embedding function - replace with real implementation
async function getEmbedding(text: string): Promise<number[]> {
  // Use transformers.js, Ollama, or OpenAI API
  return Array.from({ length: 384 }, () => Math.random());
}
</script>

<template>
  <div class="semantic-search">
    <div v-if="!isReady" class="loading">
      Initializing EdgeVec...
    </div>

    <div v-else class="search-container">
      <!-- Header -->
      <header>
        <h1>Semantic Search</h1>
        <p>{{ stats?.count ?? 0 }} vectors indexed</p>
      </header>

      <!-- Search Form -->
      <div class="search-form">
        <input
          v-model="query"
          type="text"
          placeholder="Search..."
          @keyup.enter="handleSearch"
        />

        <select v-model="category">
          <option value="all">All Categories</option>
          <option value="programming">Programming</option>
          <option value="food">Food</option>
        </select>

        <label>
          Min Relevance: {{ minRelevance }}
          <input v-model.number="minRelevance" type="range" min="0" max="1" step="0.1" />
        </label>

        <button @click="handleSearch" :disabled="isSearching">
          {{ isSearching ? 'Searching...' : 'Search' }}
        </button>
      </div>

      <!-- Results -->
      <div v-if="error" class="error">
        Error: {{ error.message }}
      </div>

      <div v-else-if="results.length > 0" class="results">
        <p class="timing">Found {{ results.length }} results in {{ searchTime?.toFixed(1) }}ms</p>

        <ul>
          <li v-for="result in results" :key="result.id" class="result-item">
            <span class="score">{{ (result.score * 100).toFixed(1) }}%</span>
            <span class="id">ID: {{ result.id }}</span>
          </li>
        </ul>
      </div>

      <div v-else-if="queryVector" class="no-results">
        No results found
      </div>

      <!-- Actions -->
      <div class="actions">
        <button @click="addSampleDocs">Add Sample Docs</button>
        <button @click="save">Save to IndexedDB</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.semantic-search {
  max-width: 600px;
  margin: 0 auto;
  padding: 2rem;
}

.search-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-bottom: 2rem;
}

.results ul {
  list-style: none;
  padding: 0;
}

.result-item {
  display: flex;
  gap: 1rem;
  padding: 0.5rem;
  border-bottom: 1px solid #eee;
}

.score {
  font-weight: bold;
  color: #4caf50;
}

.timing {
  color: #666;
  font-size: 0.9rem;
}

.error {
  color: #f44336;
  padding: 1rem;
  background: #ffebee;
  border-radius: 4px;
}

.loading {
  text-align: center;
  padding: 2rem;
}
</style>
```

---

## API Reference

### useEdgeVec(options)

```typescript
interface UseEdgeVecOptions {
  dimensions: number;
  persistName?: string;
  efConstruction?: number;
  m?: number;
}

interface UseEdgeVecReturn {
  db: Ref<EdgeVec | null>;
  isReady: Ref<boolean>;
  isLoading: Ref<boolean>;
  error: Ref<Error | null>;
  stats: Ref<{ count: number } | null>;
  save: () => Promise<void>;
  reload: () => Promise<void>;
}
```

### useSearch(db, options)

```typescript
interface UseSearchOptions {
  vector: Ref<Float32Array | number[] | null>;
  k?: number | Ref<number>;
  filter?: FilterExpression | string | Ref<FilterExpression | string>;
  enabled?: boolean | Ref<boolean> | ComputedRef<boolean>;
  debounceMs?: number;
  includeMetadata?: boolean;
  includeVectors?: boolean;
}

interface UseSearchReturn {
  results: Ref<SearchResult[]>;
  isSearching: Ref<boolean>;
  error: Ref<Error | null>;
  searchTime: Ref<number | null>;
  refetch: () => Promise<void>;
}

interface SearchResult {
  id: number;
  score: number;
  metadata?: Record<string, unknown>;
  vector?: Float32Array;
}
```

---

## Advanced Patterns

### Debounce Configuration

For real-time search-as-you-type, use debouncing to avoid excessive searches:

```vue
<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useEdgeVec, useSearch } from 'edgevec/vue';

const { db, isReady } = useEdgeVec({ dimensions: 384 });
const searchQuery = ref('');
const queryVector = ref<number[] | null>(null);

// Debounced search with 300ms delay
const { results, isSearching } = useSearch(db, {
  vector: queryVector,
  k: 10,
  debounceMs: 300,  // Wait 300ms after last change
  enabled: computed(() => isReady.value && queryVector.value !== null),
});

// Placeholder - replace with real embedding function
async function getEmbedding(text: string): Promise<number[]> {
  return Array.from({ length: 384 }, () => Math.random());
}

// Convert text to embedding when query changes (with manual debounce)
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(searchQuery, (newQuery) => {
  if (debounceTimer) clearTimeout(debounceTimer);

  debounceTimer = setTimeout(async () => {
    if (newQuery.trim()) {
      queryVector.value = await getEmbedding(newQuery);
    } else {
      queryVector.value = null;
    }
  }, 150);
});
</script>
```

### Error Handling

Handle errors gracefully with error boundaries and retry logic:

```vue
<script setup lang="ts">
import { ref, watch } from 'vue';
import { useEdgeVec, useSearch } from 'edgevec/vue';

const { db, isReady, error: initError } = useEdgeVec({
  dimensions: 384,
  persistName: 'my-db',
});

const queryVector = ref<number[] | null>(null);
const retryCount = ref(0);

const { results, error: searchError, refetch } = useSearch(db, {
  vector: queryVector,
  k: 10,
});

// Auto-retry on transient errors
watch(searchError, async (err) => {
  if (err && retryCount.value < 3) {
    retryCount.value++;
    await new Promise(r => setTimeout(r, 1000 * retryCount.value));
    await refetch();
  }
});

// Reset retry count on success
watch(results, () => {
  if (results.value.length > 0) {
    retryCount.value = 0;
  }
});
</script>

<template>
  <div v-if="initError" class="error">
    Failed to initialize: {{ initError.message }}
    <button @click="location.reload()">Retry</button>
  </div>

  <div v-else-if="searchError" class="error">
    Search failed: {{ searchError.message }}
    <span v-if="retryCount > 0">Retrying... ({{ retryCount }}/3)</span>
  </div>
</template>
```

### Cleanup on Unmount

The composables automatically clean up on component unmount, but for custom cleanup:

```vue
<script setup lang="ts">
import { onUnmounted, watch } from 'vue';
import { useEdgeVec } from 'edgevec/vue';

const { db, save } = useEdgeVec({
  dimensions: 384,
  persistName: 'my-db',
});

// Auto-save on unmount
onUnmounted(async () => {
  if (db.value) {
    await save();
    console.log('Database saved on unmount');
  }
});

// Optional: Save periodically
const saveInterval = setInterval(async () => {
  if (db.value) {
    await save();
  }
}, 60000); // Every minute

onUnmounted(() => {
  clearInterval(saveInterval);
});
</script>
```

### Vue Router Integration

Use EdgeVec across routes with a shared database:

```typescript
// stores/edgevec.ts
import { ref, computed } from 'vue';
import init, { EdgeVec } from 'edgevec';

// Singleton database instance
const db = ref<EdgeVec | null>(null);
const isReady = ref(false);
const isLoading = ref(false);

export async function initEdgeVec() {
  if (db.value || isLoading.value) return;

  isLoading.value = true;
  try {
    await init();
    db.value = new EdgeVec({ dimensions: 384 });
    isReady.value = true;
  } finally {
    isLoading.value = false;
  }
}

export function useSharedEdgeVec() {
  return {
    db: computed(() => db.value),
    isReady: computed(() => isReady.value),
    isLoading: computed(() => isLoading.value),
  };
}
```

```vue
<!-- App.vue -->
<script setup lang="ts">
import { onMounted } from 'vue';
import { initEdgeVec } from './stores/edgevec';

onMounted(() => {
  initEdgeVec();
});
</script>
```

```vue
<!-- Any component -->
<script setup lang="ts">
import { useSharedEdgeVec } from '@/stores/edgevec';

const { db, isReady } = useSharedEdgeVec();
</script>
```

### Pinia Integration

For more complex state management, integrate with Pinia:

```typescript
// stores/vectors.ts
import { defineStore } from 'pinia';
import init, { EdgeVec } from 'edgevec';

export const useVectorStore = defineStore('vectors', {
  state: () => ({
    db: null as EdgeVec | null,
    isReady: false,
    isLoading: false,
    error: null as Error | null,
  }),

  actions: {
    async initialize(dimensions: number = 384) {
      if (this.db || this.isLoading) return;

      this.isLoading = true;
      this.error = null;

      try {
        await init();
        this.db = new EdgeVec({ dimensions });
        this.isReady = true;
      } catch (e) {
        this.error = e as Error;
      } finally {
        this.isLoading = false;
      }
    },

    async search(vector: Float32Array, k: number = 10) {
      if (!this.db) throw new Error('Database not initialized');
      return this.db.search(vector, k);
    },

    async insert(vector: Float32Array, metadata?: Record<string, unknown>) {
      if (!this.db) throw new Error('Database not initialized');
      if (metadata) {
        return this.db.insertWithMetadata(vector, metadata);
      }
      return this.db.insert(vector);
    },
  },
});
```

---

## TypeScript Setup

### tsconfig.json

Ensure your TypeScript configuration includes:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "jsx": "preserve",
    "types": ["vite/client"]
  },
  "include": ["src/**/*", "node_modules/edgevec/types/**/*"]
}
```

### Type Imports

```typescript
import type { EdgeVec, SearchResult, FilterExpression } from 'edgevec';
import type { Ref, ComputedRef } from 'vue';
```

### Custom Type Extensions

```typescript
// types/edgevec.d.ts
declare module 'edgevec/vue' {
  import type { Ref, ComputedRef } from 'vue';
  import type { EdgeVec, FilterExpression, SearchResult } from 'edgevec';

  export interface UseEdgeVecOptions {
    dimensions: number;
    persistName?: string;
    efConstruction?: number;
    m?: number;
  }

  export interface UseEdgeVecReturn {
    db: Ref<EdgeVec | null>;
    isReady: Ref<boolean>;
    isLoading: Ref<boolean>;
    error: Ref<Error | null>;
    stats: Ref<{ count: number } | null>;
    save: () => Promise<void>;
    reload: () => Promise<void>;
  }

  export function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecReturn;

  export interface UseSearchOptions {
    vector: Ref<Float32Array | number[] | null>;
    k?: number | Ref<number>;
    filter?: FilterExpression | string | Ref<FilterExpression | string>;
    enabled?: boolean | Ref<boolean> | ComputedRef<boolean>;
    debounceMs?: number;
    includeMetadata?: boolean;
    includeVectors?: boolean;
  }

  export interface UseSearchReturn {
    results: Ref<SearchResult[]>;
    isSearching: Ref<boolean>;
    error: Ref<Error | null>;
    searchTime: Ref<number | null>;
    refetch: () => Promise<void>;
  }

  export function useSearch(
    db: Ref<EdgeVec | null>,
    options: UseSearchOptions
  ): UseSearchReturn;
}
```

---

## Vue vs React Differences

| Feature | React | Vue |
|:--------|:------|:----|
| State access | Direct values | Use `.value` on refs |
| Computed values | `useMemo` | `computed()` |
| Enabled condition | `enabled: isReady && vector !== null` | `enabled: computed(() => isReady.value && vector.value !== null)` |
| Reactivity | Explicit dependency arrays | Automatic tracking |
| Options | Raw values only | Supports both raw values and refs |
| Effect cleanup | useEffect return | onUnmounted |

### Key Differences in Practice

**React:**
```tsx
const { db, isReady } = useEdgeVec({ dimensions: 384 });
const [vector, setVector] = useState(null);

const { results } = useSearch(db, {
  vector,
  enabled: isReady && vector !== null,
});
```

**Vue:**
```vue
<script setup>
const { db, isReady } = useEdgeVec({ dimensions: 384 });
const vector = ref(null);

const { results } = useSearch(db, {
  vector,  // Pass ref directly
  enabled: computed(() => isReady.value && vector.value !== null),
});
</script>
```

---

## Troubleshooting

### WASM fails to load

**Symptom:** `error.value` shows "Failed to load WASM module"

**Solution:** Ensure your bundler (Vite, Webpack) is configured to handle WASM:

```typescript
// vite.config.ts
export default defineConfig({
  optimizeDeps: {
    exclude: ['edgevec']
  }
});
```

### Search returns empty results

**Symptom:** `results.value` is always empty

**Checklist:**
1. Verify `isReady.value` is `true`
2. Verify `queryVector.value` is not `null`
3. Verify database has vectors: `db.value?.vectorCount()`
4. Check filter syntax is correct
5. Ensure `enabled` computed returns `true`

### Memory issues on large datasets

**Symptom:** Browser becomes unresponsive

**Solutions:**
1. Use Binary Quantization: `db.value.searchBQ(query, k)`
2. Monitor memory: `db.value.getMemoryPressure()`
3. Compact regularly: `db.value.compact()`
4. Limit dataset size to ~100k vectors in browser

---

## See Also

- [Filter Syntax Reference](../api/FILTER_SYNTAX.md)
- [Filter Examples](./FILTER_EXAMPLES.md)
- [React Integration Guide](./REACT_INTEGRATION.md)
- [Embedding Guide](./EMBEDDING_GUIDE.md)
- [TypeScript API Reference](../api/TYPESCRIPT_API.md)
