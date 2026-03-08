# Twitter/X Thread Draft — Entity-Enhanced RAG

**Status:** [DRAFT]
**Published:** false
**Author:** Matteo Panzeri
**Target date:** TBD (after GitHub Pages deploy + live demo verification)

---

## Thread (6 tweets)

### Tweet 1 — Hook

> I built entity-enhanced RAG that runs entirely in your browser.
>
> No server. No API keys. No Docker.
>
> Just `npm install edgevec` and a ~630KB WASM bundle.
>
> Here's what I learned. (thread)

_Characters: 199_

---

### Tweet 2 — Problem

> Vector search treats every document as a bag of floats.
>
> Ask "Who founded Apple?" and you get apple orchards and pie recipes. The embedding is close. The answer is wrong.
>
> Entity context disambiguates — vanilla search throws it away.

_Characters: 251_

---

### Tweet 3 — Solution

> GraphRAG fixes this with GPT-4 + Neo4j. Also needs a server, a graph DB, and API costs.
>
> My approach: spaCy NER offline → entities as metadata → boost at search:
>
> `final_distance = raw * (1.0 - boost)`
>
> ~200 lines of Rust.

_Characters: 249_

---

### Tweet 4 — Demo

> Try it live — 1,000 SQuAD paragraphs, 384D embeddings, entity metadata. Searchable in <1ms.
>
> Toggle entity boosting ON/OFF to see disambiguation. Everything runs client-side.
>
> https://matte1782.github.io/edgevec/demo/entity-rag/index.html

_Characters: 268_

---

### Tweet 5 — Comparison

> EdgeVec vs cloud vector DBs:
>
> EdgeVec: offline ✓, in-browser ✓, npm install, ~50K vecs
> Cloud: online only, server-side, Docker+API, millions of vecs
>
> Different tools for different jobs. EdgeVec runs where others can't.

_Characters: 237_

---

### Tweet 6 — CTA

> EdgeVec is open source, MIT/Apache-2.0 dual licensed.
>
> BSc thesis project at University of Pavia.
>
> Star it: https://github.com/matte1782/edgevec
> npm: https://www.npmjs.com/package/edgevec

_Characters: 218_

---

## Notes

- **Demo link:** Verify live demo works on GitHub Pages before posting.
- **No fabricated numbers:** All claims sourced from `docs/blog/entity-enhanced-rag.md` and `README.md`. "<1ms" = 380us at 1K/768D. "~630KB" = 641,684 bytes uncompressed WASM.
- **Comparison is fair:** Explicitly says "different tools for different jobs." No latency comparison (lesson #70).
- **License:** MIT/Apache-2.0 dual licensed (corrected from earlier "MIT licensed" error).
