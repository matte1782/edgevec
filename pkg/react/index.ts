/**
 * EdgeVec React Hooks
 *
 * React hooks for seamless EdgeVec integration in React 18+ applications.
 *
 * @module edgevec/react
 * @version 0.8.0
 *
 * @example
 * ```tsx
 * import { useState } from 'react';
 * import { useEdgeVec, useSearch } from 'edgevec/react';
 * import { eq, and, gt } from 'edgevec';
 *
 * function SearchComponent() {
 *   const { db, isReady, stats } = useEdgeVec({
 *     dimensions: 384,
 *     persistName: 'my-vectors'
 *   });
 *
 *   // queryEmbedding: your embedding vector from an embedding model
 *   const [queryEmbedding, setQueryEmbedding] = useState<number[] | null>(null);
 *
 *   const { results, isSearching, searchTime } = useSearch(db, {
 *     vector: queryEmbedding,
 *     k: 10,
 *     filter: and(eq('category', 'docs'), gt('score', 0.5)),
 *     enabled: isReady && queryEmbedding !== null,
 *     debounceMs: 300
 *   });
 *
 *   if (!isReady) return <div>Loading...</div>;
 *
 *   return (
 *     <div>
 *       <p>{stats?.count} vectors indexed</p>
 *       {isSearching && <p>Searching...</p>}
 *       {searchTime && <p>Found in {searchTime.toFixed(1)}ms</p>}
 *       <ul>
 *         {results.map(r => (
 *           <li key={r.id}>ID: {r.id}, Score: {r.score.toFixed(4)}</li>
 *         ))}
 *       </ul>
 *     </div>
 *   );
 * }
 * ```
 */

export { useEdgeVec } from './useEdgeVec.js';
export { useSearch } from './useSearch.js';

export type {
  UseEdgeVecOptions,
  UseEdgeVecResult,
  UseSearchOptions,
  UseSearchResult,
} from './types.js';
