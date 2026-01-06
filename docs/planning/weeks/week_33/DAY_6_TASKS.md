# Day 6: Documentation

**Date:** 2026-01-18
**Focus:** W33.3 — Documentation & Examples
**Hours:** 2h

---

## Objectives

1. Document filter functions in README
2. Document React hooks in README
3. Create complete example component

---

## Tasks

### Task 6.1: Add Filter Functions to README (30 min)

**Update:** `pkg/README.md`

Add section after existing filter documentation:

```markdown
### Functional Filter API

For functional composition style, use the standalone filter functions:

​```typescript
import { filter, and, or, eq, gt, lt, between, contains } from 'edgevec';

// Simple conditions
const byCategory = eq('category', 'electronics');
const expensive = gt('price', 1000);

// Compose with and/or
const query = filter(
  and(
    eq('category', 'electronics'),
    gt('price', 100),
    lt('price', 1000)
  )
);

// Nested logic
const complex = filter(
  and(
    eq('status', 'active'),
    or(
      eq('brand', 'Apple'),
      eq('brand', 'Samsung'),
      eq('brand', 'Google')
    )
  )
);

// Use in search
const results = await index.search(embedding, 10, { filter: query });
​```

Available functions:
- **Comparison:** `eq`, `ne`, `gt`, `lt`, `ge`, `le`, `between`
- **String:** `contains`, `startsWith`, `endsWith`, `like`
- **Array:** `inArray`, `notInArray`, `any`, `all`, `none`
- **Null:** `isNull`, `isNotNull`
- **Logic:** `and`, `or`, `not`, `filter`
```

---

### Task 6.2: Add React Hooks Section to README (1h)

**Update:** `pkg/README.md`

Add new section:

```markdown
## React Integration

EdgeVec provides React hooks for seamless integration with React 18+ applications.

### Installation

​```bash
npm install edgevec react
​```

### useEdgeVec

Initialize an EdgeVec database with automatic WASM loading:

​```tsx
import { useEdgeVec } from 'edgevec/react';

function App() {
  const { db, isReady, isLoading, error } = useEdgeVec({
    dimensions: 384,
    persistName: 'my-vectors',  // Optional: enables IndexedDB persistence
    metric: 'cosine',           // 'cosine' | 'euclidean' | 'dot'
  });

  if (isLoading) return <div>Loading EdgeVec...</div>;
  if (error) return <div>Error: {error.message}</div>;
  if (!isReady) return null;

  return <SearchUI db={db} />;
}
​```

### useSearch

Perform reactive searches that automatically update when the query changes:

​```tsx
import { useEdgeVec, useSearch } from 'edgevec/react';
import { eq } from 'edgevec';

function SearchComponent() {
  const { db, isReady } = useEdgeVec({ dimensions: 384 });
  const [queryVector, setQueryVector] = useState<number[] | null>(null);

  const { results, isSearching, searchTime } = useSearch(db, {
    vector: queryVector,
    k: 10,
    filter: eq('category', 'documents'),
    enabled: isReady && queryVector !== null,
    debounceMs: 300,  // Debounce rapid changes
  });

  return (
    <div>
      {isSearching && <span>Searching...</span>}
      {searchTime && <span>Found in {searchTime.toFixed(1)}ms</span>}
      <ul>
        {results.map(result => (
          <li key={result.id}>
            Score: {result.score.toFixed(4)}
          </li>
        ))}
      </ul>
    </div>
  );
}
​```

### Complete Example

​```tsx
import { useState, useCallback } from 'react';
import { useEdgeVec, useSearch } from 'edgevec/react';
import { and, eq, gt } from 'edgevec';

// Assume you have an embedding function
async function getEmbedding(text: string): Promise<number[]> {
  // Your embedding logic here (e.g., transformers.js, API call)
  return new Array(384).fill(0).map(() => Math.random());
}

export function SemanticSearch() {
  const { db, isReady, stats } = useEdgeVec({
    dimensions: 384,
    persistName: 'semantic-search-demo'
  });

  const [query, setQuery] = useState('');
  const [queryVector, setQueryVector] = useState<number[] | null>(null);

  const { results, isSearching, searchTime } = useSearch(db, {
    vector: queryVector,
    k: 10,
    filter: and(
      eq('type', 'document'),
      gt('relevance', 0.5)
    ),
    enabled: isReady,
    debounceMs: 300
  });

  const handleSearch = useCallback(async () => {
    if (query.trim()) {
      const embedding = await getEmbedding(query);
      setQueryVector(embedding);
    }
  }, [query]);

  if (!isReady) {
    return <div>Initializing vector database...</div>;
  }

  return (
    <div>
      <div>
        <input
          value={query}
          onChange={e => setQuery(e.target.value)}
          placeholder="Search..."
        />
        <button onClick={handleSearch}>Search</button>
      </div>

      <div>
        {stats && <span>{stats.count} vectors indexed</span>}
        {isSearching && <span>Searching...</span>}
        {searchTime && <span>{searchTime.toFixed(1)}ms</span>}
      </div>

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
​```

### Hook API Reference

#### useEdgeVec(options)

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| dimensions | number | required | Vector dimensions |
| persistName | string | undefined | IndexedDB store name |
| metric | string | 'cosine' | Distance metric |
| efConstruction | number | 200 | HNSW build parameter |
| m | number | 16 | HNSW connections |
| enableBQ | boolean | false | Enable binary quantization |

Returns: `{ db, isReady, isLoading, error, stats, reload }`

#### useSearch(db, options)

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| vector | Float32Array \| number[] \| null | required | Query vector |
| k | number | 10 | Number of results |
| filter | FilterExpression \| string | undefined | Filter expression |
| enabled | boolean | true | Enable/disable search |
| debounceMs | number | 0 | Debounce delay |

Returns: `{ results, isSearching, error, searchTime, refetch }`
```

---

### Task 6.3: Create Example Component File (30 min)

**Create:** `examples/react-search/SearchDemo.tsx`

A standalone example file that can be copy-pasted.

---

## Verification

- [ ] Filter functions documented with examples
- [ ] React hooks documented with complete example
- [ ] API reference tables complete
- [ ] All code examples verified to compile
- [ ] README renders correctly in markdown preview

---

## Notes

_Fill during work:_

---

**Status:** PENDING
