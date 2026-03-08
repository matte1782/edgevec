# Reddit Post Drafts — EdgeVec v0.9.0

Status: [DRAFT]
Date: 2026-03-08

---

## Post 1: r/rust

**Title:** I built a WASM-native vector database in Rust — HNSW, SIMD, binary quantization, and entity-enhanced search all running in the browser

**Body:**

Hi r/rust,

I'm Matteo, a BSc AI student at the University of Pavia. For the past few months I've been building EdgeVec, an embedded vector database written in pure Rust that compiles to WebAssembly and runs entirely in the browser. I wanted to share what I've learned and get feedback.

**What it does:** HNSW approximate nearest neighbor search, binary quantization (32x memory reduction), sparse vectors with inverted index, hybrid search via RRF fusion, metadata filtering with SQL-like expressions, and a new MetadataBoost API for entity-enhanced retrieval -- all targeting `wasm32-unknown-unknown`.

**Technical highlights:**

- SIMD128 acceleration on WASM for dot product, L2, and Hamming distance (with AVX2 and NEON paths on native). `unsafe` appears in SIMD intrinsics and one WASM boundary transmute (lifetime erasure for a chunked iterator) -- all wrapped behind safe public APIs with documented safety invariants.
- Product Quantization (experimental) compresses vectors to 8 bytes each. Recall is limited compared to BQ+rescore on real embeddings, but the memory savings are significant.
- MetadataBoost uses a multiplicative formula (`final_distance = raw_distance * (1.0 - boost_factor)`) that's scale-independent across distance metrics. This lets you incorporate NER entity signals (ORG, PERSON, GPE) into search ranking without building a knowledge graph.
- Property testing with `proptest` (distance metric invariants, triangle inequality, NaN handling), fuzz testing with `cargo-fuzz`, and Miri validation for the unsafe SIMD paths. Currently at ~1050 lib tests + 150 integration tests.
- Persistence with a versioned binary format (magic number + checksum). Snapshot/restore works on both native and WASM (IndexedDB).

**What I struggled with:** Getting the HNSW distance semantics right across different metrics was harder than expected. Cosine similarity needed to be converted to a proper distance (`1.0 - dot_product`) so HNSW's "lower is closer" invariant holds. I shipped a bug where dot product returned raw similarity values, which silently inverted search results. Property tests caught the regression once I updated the oracle -- lesson learned about having distance metric proptests from day one.

**Numbers (native, AMD Ryzen 7):** 768D dot product in ~374ns, 10k-vector search in <1ms, Hamming distance at 40 GiB/s throughput. WASM bundle is ~630KB uncompressed (~238KB gzipped).

I'd appreciate any feedback on the architecture, the SIMD dispatch approach, or the quantization strategy. The codebase is MIT/Apache-2.0 dual licensed.

- GitHub: https://github.com/matte1782/edgevec
- crates.io: https://crates.io/crates/edgevec
- npm: https://www.npmjs.com/package/edgevec
- Live demo (entity-enhanced search, runs in your browser): https://matte1782.github.io/edgevec/demo/entity-rag/index.html

---

## Post 2: r/webdev

**Title:** I built an in-browser vector search engine -- ~630KB WASM (~238KB gzipped), zero API calls, works offline

**Body:**

Hey r/webdev,

I'm a university student who's been working on EdgeVec, a vector database that runs entirely in the browser via WebAssembly. No backend, no API keys, no Docker -- just `npm install edgevec` and you have semantic search in your web app.

**The problem I wanted to solve:** Every vector search tutorial starts with "spin up a Pinecone/Qdrant/Weaviate instance." That's fine for server-side apps, but what about offline PWAs, browser extensions, or privacy-sensitive apps where user data shouldn't leave the device? I wanted a vector DB that works the same way SQLite works for relational data -- embedded, local, zero infrastructure.

**What you get:**

- ~630KB WASM bundle (~238KB gzipped)
- HNSW index for fast approximate search, FlatIndex for exact search on small datasets
- Binary quantization: 32x memory reduction, ~95% recall with rescoring
- Metadata filtering with SQL-like syntax: `category = "books" AND price < 50`
- Soft delete, compaction, persistence to IndexedDB
- Entity-enhanced search (MetadataBoost) -- boost results based on NER metadata without a knowledge graph
- LangChain.js integration via `edgevec-langchain`
- Works on Chrome 91+, Firefox 89+, Safari 16.4+, Edge 91+

**Use cases I had in mind:** Offline-capable search in PWAs. Browser extensions that search bookmarks/history semantically without phoning home. Local-first note apps. Small-scale RAG pipelines where you don't want to pay per API call.

**Live demo:** I built an entity-enhanced RAG demo that indexes 1,000 SQuAD paragraphs with 384D embeddings and entity metadata (extracted with spaCy, offline). You can toggle entity boosting on/off to see how it changes results for ambiguous queries like "Apple revenue" or "Jordan river." Everything runs in your browser tab -- open DevTools Network tab and you'll see zero outgoing requests after the initial load.

Try it: https://matte1782.github.io/edgevec/demo/entity-rag/index.html

**Limitations (being honest):** This is for client-side datasets, not millions of vectors. Browser memory caps at ~1GB practically. Single-user, single-tab. If you need distributed multi-user search, use a proper server-side vector DB. EdgeVec is for the cases where those aren't an option.

This is an open-source university project (MIT/Apache-2.0). I'd love feedback on the developer experience, the API surface, or use cases I haven't thought of.

- GitHub: https://github.com/matte1782/edgevec
- npm: https://www.npmjs.com/package/edgevec
- Demo hub: https://matte1782.github.io/edgevec/demo/hub.html

---

*End of drafts. User posts manually.*
