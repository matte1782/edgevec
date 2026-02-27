# React Integration Guide

**Version:** EdgeVec v0.8.0
**Last Updated:** 2026-01-08
**Requires:** React 18+, TypeScript 5.0+ (recommended)

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [useEdgeVec Hook](#useedgevec-hook)
4. [useSearch Hook](#usesearch-hook)
5. [Complete Example](#complete-example)
6. [API Reference](#api-reference)
7. [Advanced Patterns](#advanced-patterns)
8. [TypeScript Setup](#typescript-setup)
9. [React vs Vue Differences](#react-vs-vue-differences)
10. [Troubleshooting](#troubleshooting)

---

## Installation

```bash
npm install edgevec react
```

Or with yarn/pnpm:

```bash
yarn add edgevec react
pnpm add edgevec react
```

---

## Quick Start

```tsx
import { useEdgeVec } from 'edgevec/react';

function App() {
  const { db, isReady, isLoading, error, stats } = useEdgeVec({
    dimensions: 384,
    persistName: 'my-vectors',
  });

  if (isLoading) return <div>Loading EdgeVec...</div>;
  if (error) return <div>Error: {error.message}</div>;
  if (!isReady) return null;

  return <div>{stats?.count} vectors indexed</div>;
}
```

---

## useEdgeVec Hook

Initialize an EdgeVec database with automatic WASM loading and lifecycle management.

### Basic Usage

```tsx
import { useEdgeVec } from 'edgevec/react';

function VectorDatabase() {
  const { db, isReady, isLoading, error, stats, save, reload } = useEdgeVec({
    dimensions: 384,
    persistName: 'my-vectors',  // Optional: enables IndexedDB persistence
  });

  // Insert a vector when ready
  const addVector = async () => {
    if (!isReady || !db) return;

    const vector = new Float32Array(384).map(() => Math.random());
    const id = db.insertWithMetadata(vector, {
      category: 'documents',
      timestamp: Date.now(),
    });

    console.log('Inserted vector:', id);
  };

  // Save to IndexedDB
  const saveDatabase = async () => {
    await save();
    console.log('Database saved');
  };

  if (isLoading) return <div>Loading EdgeVec...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <p>{stats?.count} vectors indexed</p>
      <button onClick={addVector}>Add Vector</button>
      <button onClick={saveDatabase}>Save</button>
    </div>
  );
}
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
| `db` | `EdgeVec \| null` | Database instance (null until ready) |
| `isReady` | `boolean` | True when WASM loaded and db initialized |
| `isLoading` | `boolean` | True during initialization |
| `error` | `Error \| null` | Initialization error if any |
| `stats` | `{ count: number } \| null` | Database statistics |
| `save` | `() => Promise<void>` | Save to IndexedDB |
| `reload` | `() => Promise<void>` | Reload from IndexedDB |

---

## useSearch Hook

Perform reactive searches that automatically update when the query changes.

### Basic Usage

```tsx
import { useState } from 'react';
import { useEdgeVec, useSearch } from 'edgevec/react';
import { eq, and, gt } from 'edgevec';

function SearchComponent() {
  const { db, isReady } = useEdgeVec({ dimensions: 384 });
  const [queryVector, setQueryVector] = useState<number[] | null>(null);

  const { results, isSearching, searchTime, error } = useSearch(db, {
    vector: queryVector,
    k: 10,
    filter: and(eq('category', 'documents'), gt('score', 0.5)),
    enabled: isReady && queryVector !== null,
    debounceMs: 300,
  });

  // Trigger search
  const search = async () => {
    // Get embedding from your embedding model (see Embedding Guide)
    const embedding = await getEmbedding('search query');
    setQueryVector(embedding);
  };

  // Placeholder - replace with real embedding function
  async function getEmbedding(text: string): Promise<number[]> {
    return Array.from({ length: 384 }, () => Math.random());
  }

  return (
    <div>
      <button onClick={search}>Search</button>

      {isSearching && <span>Searching...</span>}
      {searchTime && <span>Found in {searchTime.toFixed(1)}ms</span>}

      <ul>
        {results.map(result => (
          <li key={result.id}>
            ID: {result.id}, Score: {result.score.toFixed(4)}
          </li>
        ))}
      </ul>
    </div>
  );
}
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `vector` | `Float32Array \| number[] \| null` | **required** | Query vector |
| `k` | `number` | `10` | Number of results to return |
| `filter` | `FilterExpression \| string` | `undefined` | Filter expression |
| `enabled` | `boolean` | `true` | Enable/disable search |
| `debounceMs` | `number` | `0` | Debounce delay in milliseconds |
| `includeMetadata` | `boolean` | `false` | Include metadata in results |
| `includeVectors` | `boolean` | `false` | Include vectors in results |

### Return Values

| Property | Type | Description |
|:---------|:-----|:------------|
| `results` | `SearchResult[]` | Search results array |
| `isSearching` | `boolean` | True during search |
| `error` | `Error \| null` | Search error if any |
| `searchTime` | `number \| null` | Search duration in milliseconds |
| `refetch` | `() => Promise<void>` | Manually trigger search |

---

## Complete Example

A full semantic search component with EdgeVec:

```tsx
import { useState, useCallback, useMemo } from 'react';
import { useEdgeVec, useSearch } from 'edgevec/react';
import { and, eq, gt, or } from 'edgevec';

export function SemanticSearch() {
  // Initialize database
  const { db, isReady, stats, save } = useEdgeVec({
    dimensions: 384,
    persistName: 'semantic-search-demo'
  });

  // Search state
  const [query, setQuery] = useState('');
  const [queryVector, setQueryVector] = useState<number[] | null>(null);
  const [minRelevance, setMinRelevance] = useState(0.5);
  const [category, setCategory] = useState('all');

  // Dynamic filter based on user selection
  const activeFilter = useMemo(() => {
    if (category === 'all') {
      return gt('relevance', minRelevance);
    }
    return and(
      eq('category', category),
      gt('relevance', minRelevance)
    );
  }, [category, minRelevance]);

  // Reactive search
  const { results, isSearching, searchTime, error } = useSearch(db, {
    vector: queryVector,
    k: 10,
    filter: activeFilter,
    enabled: isReady && queryVector !== null,
    debounceMs: 300
  });

  // Handle search
  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;

    // Replace with your embedding function
    const embedding = await getEmbedding(query);
    setQueryVector(embedding);
  }, [query]);

  // Add sample documents
  const addSampleDocs = useCallback(async () => {
    if (!db) return;

    const docs = [
      { text: 'React hooks tutorial', category: 'programming' },
      { text: 'Vue 3 composition API', category: 'programming' },
      { text: 'Best coffee recipes', category: 'food' },
    ];

    for (const doc of docs) {
      const embedding = await getEmbedding(doc.text);
      db.insertWithMetadata(new Float32Array(embedding), {
        text: doc.text,
        category: doc.category,
        relevance: 1.0,
      });
    }

    await save();
  }, [db, save]);

  // Placeholder embedding function - replace with real implementation
  // NOTE: In production, move this outside the component or memoize it
  async function getEmbedding(text: string): Promise<number[]> {
    // Use transformers.js, Ollama, or OpenAI API
    return Array.from({ length: 384 }, () => Math.random());
  }

  if (!isReady) {
    return <div className="loading">Initializing EdgeVec...</div>;
  }

  return (
    <div className="semantic-search">
      {/* Header */}
      <header>
        <h1>Semantic Search</h1>
        <p>{stats?.count ?? 0} vectors indexed</p>
      </header>

      {/* Search Form */}
      <div className="search-form">
        <input
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
          type="text"
          placeholder="Search..."
        />

        <select value={category} onChange={e => setCategory(e.target.value)}>
          <option value="all">All Categories</option>
          <option value="programming">Programming</option>
          <option value="food">Food</option>
        </select>

        <label>
          Min Relevance: {minRelevance}
          <input
            value={minRelevance}
            onChange={e => setMinRelevance(Number(e.target.value))}
            type="range"
            min="0"
            max="1"
            step="0.1"
          />
        </label>

        <button onClick={handleSearch} disabled={isSearching}>
          {isSearching ? 'Searching...' : 'Search'}
        </button>
      </div>

      {/* Results */}
      {error ? (
        <div className="error">Error: {error.message}</div>
      ) : results.length > 0 ? (
        <div className="results">
          <p className="timing">
            Found {results.length} results in {searchTime?.toFixed(1)}ms
          </p>

          <ul>
            {results.map(result => (
              <li key={result.id} className="result-item">
                <span className="score">{(result.score * 100).toFixed(1)}%</span>
                <span className="id">ID: {result.id}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : queryVector ? (
        <div className="no-results">No results found</div>
      ) : null}

      {/* Actions */}
      <div className="actions">
        <button onClick={addSampleDocs}>Add Sample Docs</button>
        <button onClick={save}>Save to IndexedDB</button>
      </div>
    </div>
  );
}
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
  db: EdgeVec | null;
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
  stats: { count: number } | null;
  save: () => Promise<void>;
  reload: () => Promise<void>;
}
```

### useSearch(db, options)

```typescript
interface UseSearchOptions {
  vector: Float32Array | number[] | null;
  k?: number;
  filter?: FilterExpression | string;
  enabled?: boolean;
  debounceMs?: number;
  includeMetadata?: boolean;
  includeVectors?: boolean;
}

interface UseSearchReturn {
  results: SearchResult[];
  isSearching: boolean;
  error: Error | null;
  searchTime: number | null;
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

```tsx
import { useState, useEffect } from 'react';
import { useEdgeVec, useSearch } from 'edgevec/react';

function LiveSearch() {
  const { db, isReady } = useEdgeVec({ dimensions: 384 });
  const [searchQuery, setSearchQuery] = useState('');
  const [queryVector, setQueryVector] = useState<number[] | null>(null);

  // Debounced search with 300ms delay
  const { results, isSearching } = useSearch(db, {
    vector: queryVector,
    k: 10,
    debounceMs: 300,  // Wait 300ms after last change
    enabled: isReady && queryVector !== null,
  });

  // Placeholder - replace with real embedding function
  async function getEmbedding(text: string): Promise<number[]> {
    return Array.from({ length: 384 }, () => Math.random());
  }

  // Convert text to embedding when query changes (with additional debounce)
  useEffect(() => {
    const timer = setTimeout(async () => {
      if (searchQuery.trim()) {
        const embedding = await getEmbedding(searchQuery);
        setQueryVector(embedding);
      } else {
        setQueryVector(null);
      }
    }, 150);

    return () => clearTimeout(timer);
  }, [searchQuery]);

  return (
    <div>
      <input
        value={searchQuery}
        onChange={e => setSearchQuery(e.target.value)}
        placeholder="Type to search..."
      />
      {isSearching && <span>Searching...</span>}
      <ul>
        {results.map(r => (
          <li key={r.id}>Score: {r.score.toFixed(4)}</li>
        ))}
      </ul>
    </div>
  );
}
```

### Error Boundaries

Wrap EdgeVec components in an error boundary for graceful error handling:

```tsx
import { Component, ReactNode } from 'react';

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class EdgeVecErrorBoundary extends Component<
  { children: ReactNode; fallback?: ReactNode },
  ErrorBoundaryState
> {
  constructor(props: { children: ReactNode; fallback?: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('EdgeVec error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback ?? (
        <div className="error-boundary">
          <h2>Something went wrong with EdgeVec</h2>
          <p>{this.state.error?.message}</p>
          <button onClick={() => this.setState({ hasError: false, error: null })}>
            Try again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

// Usage
function App() {
  return (
    <EdgeVecErrorBoundary fallback={<div>Failed to load vector search</div>}>
      <SemanticSearch />
    </EdgeVecErrorBoundary>
  );
}
```

### Error Handling with Retry Logic

Handle transient errors with automatic retry:

```tsx
import { useState, useEffect, useCallback } from 'react';
import { useEdgeVec, useSearch } from 'edgevec/react';

function SearchWithRetry() {
  const { db, isReady, error: initError } = useEdgeVec({
    dimensions: 384,
    persistName: 'my-db',
  });

  const [queryVector, setQueryVector] = useState<number[] | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  const { results, error: searchError, refetch } = useSearch(db, {
    vector: queryVector,
    k: 10,
    enabled: isReady && queryVector !== null,
  });

  // Auto-retry on transient errors
  useEffect(() => {
    if (searchError && retryCount < 3) {
      const timer = setTimeout(async () => {
        setRetryCount(c => c + 1);
        await refetch();
      }, 1000 * (retryCount + 1)); // Exponential backoff

      return () => clearTimeout(timer);
    }
  }, [searchError, retryCount, refetch]);

  // Reset retry count on success
  useEffect(() => {
    if (results.length > 0) {
      setRetryCount(0);
    }
  }, [results]);

  if (initError) {
    return (
      <div className="error">
        Failed to initialize: {initError.message}
        <button onClick={() => location.reload()}>Retry</button>
      </div>
    );
  }

  if (searchError) {
    return (
      <div className="error">
        Search failed: {searchError.message}
        {retryCount > 0 && <span> Retrying... ({retryCount}/3)</span>}
      </div>
    );
  }

  return (
    <ul>
      {results.map(r => (
        <li key={r.id}>Score: {r.score.toFixed(4)}</li>
      ))}
    </ul>
  );
}
```

### Suspense Integration (Experimental)

For React 18+ Suspense support, create a wrapper:

```tsx
import { Suspense, use, useState } from 'react';
import init, { EdgeVec } from 'edgevec';

// Create a promise that resolves when WASM is loaded
let wasmPromise: Promise<void> | null = null;
function initWasm() {
  if (!wasmPromise) {
    wasmPromise = init();
  }
  return wasmPromise;
}

// Suspense-compatible hook (React 19+)
function useEdgeVecSuspense(dimensions: number) {
  // This will suspend until WASM is loaded
  use(initWasm());

  // Now safe to create EdgeVec
  const [db] = useState(() => new EdgeVec({ dimensions }));
  return db;
}

// Usage with Suspense
function App() {
  return (
    <Suspense fallback={<div>Loading EdgeVec...</div>}>
      <SearchComponent />
    </Suspense>
  );
}

function SearchComponent() {
  const db = useEdgeVecSuspense(384);
  // db is guaranteed to be ready here
  return <div>Ready! {db.vectorCount()} vectors</div>;
}
```

### Context Provider Pattern

Share a single EdgeVec instance across your app:

```tsx
import { createContext, useContext, ReactNode, useState, useEffect } from 'react';
import init, { EdgeVec } from 'edgevec';

interface EdgeVecContextType {
  db: EdgeVec | null;
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
}

const EdgeVecContext = createContext<EdgeVecContextType>({
  db: null,
  isReady: false,
  isLoading: true,
  error: null,
});

export function EdgeVecProvider({
  children,
  dimensions = 384,
  persistName,
}: {
  children: ReactNode;
  dimensions?: number;
  persistName?: string;
}) {
  const [state, setState] = useState<EdgeVecContextType>({
    db: null,
    isReady: false,
    isLoading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;

    async function initialize() {
      try {
        await init();
        if (cancelled) return;

        const db = new EdgeVec({ dimensions });

        if (persistName) {
          try {
            await db.load(persistName);
          } catch {
            // No existing data, that's fine
          }
        }

        setState({
          db,
          isReady: true,
          isLoading: false,
          error: null,
        });
      } catch (error) {
        if (!cancelled) {
          setState({
            db: null,
            isReady: false,
            isLoading: false,
            error: error as Error,
          });
        }
      }
    }

    initialize();

    return () => {
      cancelled = true;
    };
  }, [dimensions, persistName]);

  return (
    <EdgeVecContext.Provider value={state}>
      {children}
    </EdgeVecContext.Provider>
  );
}

export function useEdgeVecContext() {
  const context = useContext(EdgeVecContext);
  if (!context) {
    throw new Error('useEdgeVecContext must be used within EdgeVecProvider');
  }
  return context;
}

// Usage
function App() {
  return (
    <EdgeVecProvider dimensions={384} persistName="my-db">
      <SearchPage />
      <InsertPage />
    </EdgeVecProvider>
  );
}

function SearchPage() {
  const { db, isReady } = useEdgeVecContext();
  // Use shared db instance
}
```

### React Router Integration

Use EdgeVec across routes with persistent state:

```tsx
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { EdgeVecProvider } from './context/EdgeVecContext';

function App() {
  return (
    <EdgeVecProvider dimensions={384} persistName="app-vectors">
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/manage" element={<ManagePage />} />
        </Routes>
      </BrowserRouter>
    </EdgeVecProvider>
  );
}
```

### Redux/Zustand Integration

For complex state management, integrate with Zustand:

```tsx
import { create } from 'zustand';
import init, { EdgeVec } from 'edgevec';

interface VectorState {
  db: EdgeVec | null;
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
  initialize: (dimensions: number) => Promise<void>;
  search: (vector: Float32Array, k: number) => Promise<SearchResult[]>;
  insert: (vector: Float32Array, metadata?: Record<string, unknown>) => number;
}

export const useVectorStore = create<VectorState>((set, get) => ({
  db: null,
  isReady: false,
  isLoading: false,
  error: null,

  initialize: async (dimensions: number) => {
    if (get().db || get().isLoading) return;

    set({ isLoading: true, error: null });

    try {
      await init();
      const db = new EdgeVec({ dimensions });
      set({ db, isReady: true, isLoading: false });
    } catch (error) {
      set({ error: error as Error, isLoading: false });
    }
  },

  search: async (vector, k) => {
    const { db } = get();
    if (!db) throw new Error('Database not initialized');
    return db.search(vector, k);
  },

  insert: (vector, metadata) => {
    const { db } = get();
    if (!db) throw new Error('Database not initialized');
    if (metadata) {
      return db.insertWithMetadata(vector, metadata);
    }
    return db.insert(vector);
  },
}));

// Usage in component
function SearchComponent() {
  const { isReady, search, initialize } = useVectorStore();

  useEffect(() => {
    initialize(384);
  }, [initialize]);

  // ...
}
```

---

## TypeScript Setup

### tsconfig.json

Ensure your TypeScript configuration includes:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["DOM", "DOM.Iterable", "ES2020"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "jsx": "react-jsx",
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*"]
}
```

### Type Imports

```typescript
import type { EdgeVec, SearchResult, FilterExpression } from 'edgevec';
```

### Custom Type Extensions

```typescript
// types/edgevec.d.ts
declare module 'edgevec/react' {
  import type { EdgeVec, FilterExpression, SearchResult } from 'edgevec';

  export interface UseEdgeVecOptions {
    dimensions: number;
    persistName?: string;
    efConstruction?: number;
    m?: number;
  }

  export interface UseEdgeVecReturn {
    db: EdgeVec | null;
    isReady: boolean;
    isLoading: boolean;
    error: Error | null;
    stats: { count: number } | null;
    save: () => Promise<void>;
    reload: () => Promise<void>;
  }

  export function useEdgeVec(options: UseEdgeVecOptions): UseEdgeVecReturn;

  export interface UseSearchOptions {
    vector: Float32Array | number[] | null;
    k?: number;
    filter?: FilterExpression | string;
    enabled?: boolean;
    debounceMs?: number;
    includeMetadata?: boolean;
    includeVectors?: boolean;
  }

  export interface UseSearchReturn {
    results: SearchResult[];
    isSearching: boolean;
    error: Error | null;
    searchTime: number | null;
    refetch: () => Promise<void>;
  }

  export function useSearch(
    db: EdgeVec | null,
    options: UseSearchOptions
  ): UseSearchReturn;
}
```

---

## React vs Vue Differences

| Feature | React | Vue |
|:--------|:------|:----|
| State access | Direct values | Use `.value` on refs |
| Computed values | `useMemo` with deps array | `computed()` auto-tracks |
| Enabled condition | `enabled: isReady && vector !== null` | `enabled: computed(() => isReady.value && vector.value !== null)` |
| Reactivity | Explicit dependency arrays | Automatic tracking |
| Effect cleanup | `useEffect` return function | `onUnmounted` |
| Memoization | `useCallback`, `useMemo` | Not typically needed |

### Key Differences in Practice

**React:**
```tsx
const { db, isReady } = useEdgeVec({ dimensions: 384 });
const [vector, setVector] = useState(null);

const filter = useMemo(
  () => and(eq('category', category), gt('price', minPrice)),
  [category, minPrice]
);

const { results } = useSearch(db, {
  vector,
  filter,
  enabled: isReady && vector !== null,
});
```

**Vue:**
```vue
<script setup>
const { db, isReady } = useEdgeVec({ dimensions: 384 });
const vector = ref(null);

const filter = computed(() =>
  and(eq('category', category.value), gt('price', minPrice.value))
);

const { results } = useSearch(db, {
  vector,  // Pass ref directly
  filter,  // Pass computed directly
  enabled: computed(() => isReady.value && vector.value !== null),
});
</script>
```

---

## Troubleshooting

### WASM fails to load

**Symptom:** `error` shows "Failed to load WASM module"

**Solution:** Configure your bundler to handle WASM files:

**Vite:**
```typescript
// vite.config.ts
export default defineConfig({
  optimizeDeps: {
    exclude: ['edgevec']
  }
});
```

**Create React App:**
```javascript
// craco.config.js
module.exports = {
  webpack: {
    configure: (config) => {
      config.experiments = { asyncWebAssembly: true };
      return config;
    },
  },
};
```

**Next.js:**
```javascript
// next.config.js
module.exports = {
  webpack: (config) => {
    config.experiments = { asyncWebAssembly: true, layers: true };
    return config;
  },
};
```

### Search returns empty results

**Symptom:** `results` is always empty array

**Checklist:**
1. Verify `isReady` is `true`
2. Verify `queryVector` is not `null`
3. Verify database has vectors: `db?.vectorCount()`
4. Check filter syntax is correct
5. Ensure `enabled` is `true`

### Memory issues on large datasets

**Symptom:** Browser becomes unresponsive or crashes

**Solutions:**
1. Use Binary Quantization: `db.searchBQ(query, k)`
2. Monitor memory: `db.getMemoryPressure()`
3. Compact regularly: `db.compact()`
4. Limit dataset size to ~100k vectors in browser

### Hooks called in wrong order

**Symptom:** "Hooks must be called in the same order" error

**Solution:** Ensure hooks are not called conditionally:

```tsx
// WRONG
function Component({ shouldSearch }) {
  const { db } = useEdgeVec({ dimensions: 384 });
  if (shouldSearch) {
    const { results } = useSearch(db, { vector, k: 10 }); // Error!
  }
}

// CORRECT
function Component({ shouldSearch }) {
  const { db } = useEdgeVec({ dimensions: 384 });
  const { results } = useSearch(db, {
    vector,
    k: 10,
    enabled: shouldSearch,  // Use enabled option instead
  });
}
```

### Stale closure in callbacks

**Symptom:** Callbacks see old state values

**Solution:** Use `useCallback` with proper dependencies or refs:

```tsx
function Component() {
  const [count, setCount] = useState(0);
  const { db } = useEdgeVec({ dimensions: 384 });

  // WRONG - count will be stale
  const handleClick = () => {
    console.log(count);
  };

  // CORRECT - always uses latest count
  const handleClick = useCallback(() => {
    console.log(count);
  }, [count]);
}
```

---

## See Also

- [Filter Syntax Reference](../api/FILTER_SYNTAX.md)
- [Filter Examples](./FILTER_EXAMPLES.md)
- [Vue Integration Guide](./VUE_INTEGRATION.md)
- [Embedding Guide](./EMBEDDING_GUIDE.md)
- [TypeScript API Reference](../api/TYPESCRIPT_API.md)
