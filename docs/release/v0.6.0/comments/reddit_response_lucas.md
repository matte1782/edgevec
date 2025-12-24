# Reddit Response to Lucas

**Original Issue:** `docs/release/v0.6.0/comments/reddit_issues_2.txt`
**Date:** 2025-12-24
**Status:** Ready to Post

---

## Lucas's Feedback

> https://matte1782.github.io/edgevec/demo/v060_cyberpunk_demo.html
>
> seems to be down!
>
> PS — I'm a bit confused how to best use the demo! + out of curiosity - is BM25 support on roadmap here? I would love to build pretty simple hybrid search, I have been working mostly with pgvector but it's a bit large for my use! Curious if you've thought about this.
>
> In any case: love your project / this is so cool! Want to dig in more / thanks for building.
>
> Lucas

---

## Response

```
Hi Lucas! Thanks for catching this and for the kind words!

**The broken link is fixed** — the demo is now at:
https://matte1782.github.io/edgevec/demo/cyberpunk.html

Or check the new Demo Hub:
https://matte1782.github.io/edgevec/demo/hub.html

**How to use the demos:**

1. **Filter Playground** (https://matte1782.github.io/edgevec/demo/) —
   Best starting point! Build SQL-like queries visually, see results in
   real-time, copy-paste code into your project.

2. **Cyberpunk Demo** — Compare Binary Quantization (32x memory savings)
   vs full F32, test metadata filtering, monitor memory pressure.

**BM25 / Hybrid Search:**

Great question! It's on the radar but not in v0.7.0 (releasing soon).
EdgeVec focuses on keeping the WASM bundle small (~470KB), and full
BM25 would add significant weight (~150KB for tokenization).

**Current workaround for hybrid search:**

```javascript
import init, { EdgeVec } from 'edgevec';
import BM25 from 'bm25';  // Or any JS BM25 library

// Dense search with EdgeVec
const vectorResults = db.search(embedding, 20);

// Sparse search with BM25
const bm25Results = bm25.search(queryText, 20);

// Combine with Reciprocal Rank Fusion
function rrf(vectorResults, bm25Results, k = 60) {
  const scores = new Map();
  vectorResults.forEach((r, i) => {
    scores.set(r.id, (scores.get(r.id) || 0) + 1 / (k + i + 1));
  });
  bm25Results.forEach((r, i) => {
    scores.set(r.id, (scores.get(r.id) || 0) + 1 / (k + i + 1));
  });
  return [...scores.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 10);
}
```

**Roadmap:**
- v0.8.0: Considering sparse vector support (you compute BM25 vectors externally, EdgeVec stores/searches them)
- v0.9.0+: Possibly full BM25 if demand is high

Compared to pgvector — EdgeVec is definitely smaller (no Postgres!)
and runs entirely in the browser. Trade-off is scale: EdgeVec works
great up to ~100k vectors, pgvector scales to millions.

Thanks for digging in! Let me know if you build something cool with it.
```

---

## Actions Taken

1. **Fixed broken link** — Created redirect at `v060_cyberpunk_demo.html`
2. **Fixed Demo Hub** — All links now work on GitHub Pages
3. **BM25 RFC created** — `docs/rfcs/RFC_BM25_HYBRID_SEARCH.md`
4. **Workaround documented** — Hybrid search pattern with RRF

---

## Post Location

Reply to Lucas on Reddit thread where the issue was reported.

