# Entity-Enhanced RAG in 300KB: No GPT-4, No Neo4j, No Server

You have a pile of documents. You embed them, store them in a vector database, and search by similarity. It works great — until it doesn't.

Ask "Who founded Apple?" and your vector search cheerfully returns articles about apple orchards, apple pie recipes, and the Apple Music subscription page. The embeddings are close enough. The answer is wrong enough to matter.

This is the fundamental problem with naive vector search: it treats every document as a bag of floating-point numbers. The rich structure that humans rely on — the fact that "Apple" in one document refers to a trillion-dollar company and in another refers to a fruit — is flattened into a 768-dimensional smear. And once that structure is gone, no amount of clever ANN indexing will bring it back.

## The Problem: Vector Search Misses Context

Embedding models are remarkable at capturing semantic similarity. But they compress *everything* into a single vector, which means they lose the ability to distinguish between different *types* of similarity. A document about Steve Jobs founding Apple and a document about Johnny Appleseed planting apple trees might end up surprisingly close in embedding space, because both involve people and apples and founding/planting.

The entity context — the fact that "Apple" is an ORG in one document and a fruit in another, that "Jobs" is a PERSON and not a job listing — is exactly the signal that would disambiguate these results. But vanilla vector search throws it away.

The industry's answer to this is GraphRAG: build a knowledge graph over your documents using GPT-4 to extract entities and relationships, store it in Neo4j or a similar graph database, then combine graph traversal with vector retrieval. Microsoft's GraphRAG paper demonstrated impressive results. The approach works.

It also requires GPT-4 API calls for entity extraction (dollars per thousand documents), a Neo4j instance (infrastructure), and a non-trivial orchestration layer to combine graph and vector results. For a production deployment at scale, that tradeoff makes sense. For a client-side application running in a browser tab — where you have no server, no database process, and maybe 500KB of budget for your entire search stack — it is a non-starter.

What if you could get *some* of the benefit — the entity disambiguation part, specifically — without any of that infrastructure?

## The Insight: Metadata as Boost Signals

Recent research has been quantifying just how much entity-aware retrieval improves over naive approaches.

Xu, Z., Cruz, M. J., Guevara, M., Wang, T., Deshpande, M., Wang, X., & Li, Z. (2024). *Retrieval-Augmented Generation with Knowledge Graphs for Customer Service Question Answering.* arXiv preprint arXiv:2404.17723. This work demonstrated that incorporating knowledge graph entities into the retrieval step significantly improves answer quality in customer service QA, compared to embedding-only retrieval.

Palmonari, M. (2025). *Beyond Naive RAG: How Entities and Graphs Enhance Retrieval-Augmented Generation.* Invited presentation, UNIMIB Data Semantics course, 2025. Data from Expert.AI/MEI System collaboration on ophthalmic device troubleshooting (Philips/Bosch business cases), presented at European Big Data Value Forum. In this work, entity-enhanced retrieval improved retrieval accuracy from 0.52 to 0.71 — a 37% relative improvement — on technical troubleshooting documents where entity disambiguation was critical.

The insight these works share is that entities act as *structural anchors* for retrieval. You don't always need a full knowledge graph to exploit them. Sometimes, just knowing that a document mentions "Apple" as an ORG (not a fruit) or "Berlin" as a GPE (not a person's name) is enough to break ties between otherwise-similar embeddings.

So here is the idea: skip the graph entirely. Run NER (Named Entity Recognition) at indexing time — spaCy does this in milliseconds per document, entirely offline, no API calls required. Store the extracted entities as metadata alongside each vector. Then, at search time, *boost* results whose entity metadata matches the query's context. You get entity-aware retrieval without building, storing, or querying a knowledge graph.

The formula is simple and multiplicative:

```
final_distance = raw_distance * (1.0 - boost_factor)
```

A boost factor of 0.3 reduces distance by 30%. This is scale-independent: it works the same whether your L2 distances are 0.001 or 50,000. No tuning needed when you switch distance metrics or embedding models.

## EdgeVec's MetadataBoost API

EdgeVec implements this in about 200 lines of Rust. Zero new dependencies. One struct, one method, one error type.

```rust
use edgevec::filter::{MetadataBoost, FilteredSearcher, FilterStrategy};
use edgevec::metadata::MetadataValue;

// Step 1: Define boosts — what entity types should influence ranking?
let boosts = vec![
    // Promote results that mention organizations (+30% closer)
    MetadataBoost::new(
        "entity_type".to_string(),
        MetadataValue::String("ORG".to_string()),
        0.3,
    )?,
    // Also promote results mentioning specific people (+20% closer)
    MetadataBoost::new(
        "entity_type".to_string(),
        MetadataValue::String("PERSON".to_string()),
        0.2,
    )?,
];

// Step 2: Search with boosting
// search_boosted oversamples candidates, applies boost reranking,
// and returns results sorted by boosted distance.
let results = searcher.search_boosted(
    &query,           // query vector
    10,               // k results
    &boosts,          // metadata boosts
    None,             // optional FilterExpr (hard filter)
    FilterStrategy::Auto,  // let EdgeVec pick pre/post filter
)?;
```

`MetadataBoost::new` takes a field name (`String`), a target value (`MetadataValue`), and a weight (`f32`). The weight must be finite — NaN and infinity are rejected with a `BoostError::InvalidWeight`. Positive weights promote matching results (reduce distance); negative weights penalize them (increase distance). The total boost factor from multiple boosts is additively stacked and clamped to `[-1.0, 0.99]`, so distance can never go to zero or flip negative.

The real power comes from combining boosts with EdgeVec's existing `FilterExpression` system. Boosts are *soft* signals — they rerank results. Filters are *hard* constraints — they exclude results. Use them together:

```rust
use edgevec::filter::parse;

// Hard filter: only documents from 2024
let filter = parse("year = 2024")?;

// Soft boost: prefer ORG entities within those results
let results = searcher.search_boosted(
    &query, 10, &boosts,
    Some(&filter),       // hard filter applied first
    FilterStrategy::Auto,
)?;
```

For `StringArray` metadata fields — common when a document mentions multiple entities — the boost matches if the target value appears anywhere in the array. A document with `entities: ["NASA", "SpaceX", "ESA"]` will match a boost targeting `"NASA"`. This means you can store all entities extracted by NER as a single array field and boost against any of them individually.

The WASM boundary exposes this as `searchBoosted()`, accepting JSON boost configs directly from JavaScript. Same API, same formula, same results — running entirely in the browser.

## Try It: Live In-Browser Demo

See it in action: **[Entity-RAG Demo](https://matte1782.github.io/edgevec/demo/entity-rag/)**

The demo indexes 1,000 SQuAD paragraphs with 384-dimensional embeddings (generated by `all-MiniLM-L6-v2`) and entity metadata extracted by spaCy NER. Toggle entity boosting on and off to see how it changes search results in real time.

Everything runs in your browser. Zero API calls. The WASM bundle is roughly 500KB uncompressed. No server, no Docker, no API keys.

A toggle lets you switch between "boost ON" and "boost OFF" for the same query, so you can see exactly which results get promoted when entity signals are factored in. Try queries that are ambiguous without context — "Apple revenue" or "Jordan river" — and watch the entity boost disambiguate.

To be clear: this is a 1,000-document demo, not a million-document benchmark. We chose SQuAD because it has diverse, well-studied paragraphs that cover a wide range of entities — organizations, locations, people, dates. The entity metadata was extracted with spaCy's `en_core_web_sm` model, which is fast and free. No GPT-4 calls were harmed in the making of this demo.

EdgeVec is designed for client-side use cases: local-first apps, offline search, privacy-sensitive retrieval, browser extensions. For server-side deployments at scale, dedicated vector databases are the right tool.

## How EdgeVec Compares

EdgeVec is not competing with Qdrant or Weaviate on raw throughput. They are production-grade server-side databases designed for millions of vectors across distributed clusters. EdgeVec is designed for a different deployment model entirely: the browser.

| Feature | EdgeVec | Qdrant | Weaviate |
|:--------|:--------|:-------|:---------|
| Bundle size | ~500KB WASM | N/A (server) | N/A (server) |
| Runs offline | Yes | No | No |
| In-browser | Yes | No | No |
| Entity boost | MetadataBoost | Custom scorer | Cross-reference |
| Setup | `npm install edgevec` | Docker + API | Docker + API |
| Search latency* | <1ms (1K docs) | ~5ms (remote) | ~10ms (remote) |

> **\*Note:** EdgeVec latency is measured in-process (no network overhead). Qdrant and Weaviate latency figures include network round-trip. This comparison highlights deployment model differences, not raw engine speed. All three engines are sub-millisecond at 1K documents when measured in-process.

The point is not that EdgeVec is "faster." The point is that EdgeVec runs *where the others cannot*. Consider the use cases: an offline-capable PWA that works on an airplane. A browser extension that searches your bookmarks semantically without sending them to any server. A local-first note-taking app where your data never leaves your device. A privacy-sensitive medical reference tool that runs entirely on the clinician's tablet.

In all of these scenarios, "spin up a Docker container" is not an option. A 500KB WASM bundle with entity boosting, metadata filtering, and sub-millisecond search is.

## Get Started

Install EdgeVec and try metadata boosting in under five minutes:

**Rust:**
```bash
cargo add edgevec
```

**JavaScript / TypeScript:**
```bash
npm install edgevec
```

Try the live demo: [Entity-RAG Demo](https://matte1782.github.io/edgevec/demo/entity-rag/)

Browse the source: [GitHub](https://github.com/matte1782/edgevec)

Read the filter guide: [Filter Syntax Documentation](https://github.com/matte1782/edgevec/blob/main/docs/api/FILTER_SYNTAX.md)

---

*EdgeVec is an open-source embedded vector database built in Rust with first-class WebAssembly support. MIT / Apache-2.0 dual licensed.*
