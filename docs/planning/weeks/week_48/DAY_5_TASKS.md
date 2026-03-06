# Week 48 — Day 5 Tasks (Friday, Apr 18)

**Date:** 2026-04-18
**Focus:** Blog Post + README Update + END-OF-WEEK HOSTILE REVIEW
**Agents:** DOCWRITER, PLANNER, HOSTILE_REVIEWER
**Status:** PENDING

---

## Day Objective

Write the positioning blog post, update README with MetadataBoost section, and pass end-of-week hostile review on all non-code deliverables.

**Success Criteria:**
- `docs/blog/entity-enhanced-rag.md` exists (1500-2000 words)
- README.md updated with MetadataBoost section + demo link
- CHANGELOG.md updated with W48 work
- End-of-week hostile review: GO verdict (0 Critical, 0 Major)

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `src/filter/boost.rs` — MetadataBoost API for code examples
- [ ] `demo/entity-rag/index.html` — demo for screenshot/link reference
- [ ] `README.md` — current structure, where to add new section
- [ ] `CHANGELOG.md` — current format for W48 entry
- [ ] `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` — blog structure spec

---

## Tasks

### W48.5a: Write Blog Post (3h) — DOCWRITER

**Dependency:** Demo works (Day 4), MetadataBoost API stable (Day 1-2)

**File:** `docs/blog/entity-enhanced-rag.md`

**Structure (MUST follow):**

```markdown
# Entity-Enhanced RAG in 300KB: No GPT-4, No Neo4j, No Server

## The Problem: Vector Search Misses Context

[~300 words]
- Naive vector search treats documents as bags of numbers
- Entity context (ORG, PERSON, GPE) is lost
- Example: "Who founded Apple?" — vector search returns fruit articles
- GraphRAG solves this but requires GPT-4 + Neo4j ($$$, latency, complexity)

## The Insight: Metadata as Boost Signals

[~300 words]
- Research citations (VERIFIED):
  - Xu, Z., Cruz, M. J., Guevara, M., Wang, T., Deshpande, M., Wang, X., & Li, Z. (2024). Retrieval-Augmented Generation with Knowledge Graphs for Customer Service Question Answering. arXiv preprint arXiv:2404.17723.
  - Palmonari, M. (2025). Beyond Naive RAG: How Entities and Graphs Enhance Retrieval-Augmented Generation. Invited presentation, UNIMIB Data Semantics course, 2025. Data from Expert.AI/MEI System collaboration on ophthalmic device troubleshooting (Philips/Bosch business cases), presented at European Big Data Value Forum.
- Key finding: retrieval accuracy 0.52 -> 0.71 with entity signals (Palmonari, 2025)
- Idea: don't build a graph. Just BOOST results that match entity metadata.
- Formula: final_distance = raw_distance * (1.0 - boost_factor) — multiplicative, scale-independent

## EdgeVec's MetadataBoost API

[~400 words with code example]
```rust
use edgevec::filter::{MetadataBoost, FilteredSearcher};
use edgevec::metadata::MetadataValue;

// Define boosts: prefer results mentioning organizations
let boosts = vec![
    MetadataBoost::new("entity_type", MetadataValue::String("ORG".into()), 0.3)?,
    MetadataBoost::new("entity_type", MetadataValue::String("PERSON".into()), 0.2)?,
];

// Search with entity boosting (multiplicative — scale-independent)
let results = searcher.search_boosted(&query, 10, &boosts, None, FilterStrategy::Auto)?;
```

- Explain the API: one struct, one method, one error type (`BoostError`)
- Show how it combines with existing FilterExpression (hard + soft filtering)
- Size: ~200 lines of new code, zero new dependencies

## Try It: Live In-Browser Demo

[~200 words]
- Link to demo (hosted on GitHub Pages or similar)
- Screenshot of demo with boost ON vs OFF
- "100% in-browser. Zero API calls. 300KB WASM bundle."
- 1000 SQuAD paragraphs, entity metadata from spaCy NER

## How EdgeVec Compares

[~200 words + comparison table]

| Feature | EdgeVec | Qdrant | Weaviate |
|:--------|:--------|:-------|:---------|
| Bundle size | 300KB WASM | N/A (server) | N/A (server) |
| Runs offline | Yes | No | No |
| In-browser | Yes | No | No |
| Entity boost | MetadataBoost | Custom scorer | Cross-reference |
| Setup | `npm install edgevec` | Docker + API | Docker + API |
| Search latency | <1ms (1K docs, local) | ~5ms (remote) | ~10ms (remote) |

> **Note:** Qdrant/Weaviate latency includes network round-trip overhead.
> The comparison highlights deployment model (in-browser vs client-server),
> not raw engine speed — all three engines are sub-millisecond at 1K docs
> when measured in-process.

## Get Started

[~100 words]
- `cargo add edgevec` / `npm install edgevec`
- Star on GitHub: [link]
- Try the demo: [link]
- Read the docs: [link]
```

**Tone guidelines:**
- Conversational, not academic
- "Show, don't tell" — code examples > theoretical explanations
- Acknowledge limitations honestly (1K docs demo, not 1M)
- No hype: "improves" not "revolutionizes"

**Commands:**
```bash
mkdir -p docs/blog
# Write the blog post
wc -w docs/blog/entity-enhanced-rag.md    # Must be 1500-2000
```

**Acceptance:**
- [ ] 1500-2000 words
- [ ] All 6 sections present
- [ ] Code example compiles (verify with `cargo build`)
- [ ] Research citations are real and fully referenced: Xu et al. (2024, arXiv:2404.17723) and Palmonari (2025, UNIMIB invited talk at European Big Data Value Forum)
- [ ] Comparison table uses verifiable numbers
- [ ] Conversational tone (no academic jargon)
- [ ] No broken links

---

### W48.5b: Update README.md (1h) — DOCWRITER

**Dependency:** W48.5a complete (for consistency)

**Add new section after existing "Features" or "Quick Start":**

```markdown
## Entity-Enhanced RAG

EdgeVec supports **metadata boosting** for entity-enhanced retrieval:

```rust
use edgevec::filter::MetadataBoost;
use edgevec::metadata::MetadataValue;

let boosts = vec![
    MetadataBoost::new("entity_type", MetadataValue::String("ORG".into()), 0.3)?,
];
let results = searcher.search_boosted(&query, 10, &boosts, None, FilterStrategy::Auto)?;
```

[Try the in-browser demo](demo/entity-rag/) — 1000 documents, entity boost ON/OFF toggle, zero API calls.
```

**Acceptance:**
- [ ] New "Entity-Enhanced RAG" section in README
- [ ] Code example is syntactically correct Rust
- [ ] Demo link points to correct path
- [ ] README renders correctly in GitHub markdown preview

---

### W48.5c: Update CHANGELOG.md (0.5h) — PLANNER

**Dependency:** None

**Add W48 section:**
```markdown
## [Unreleased] — Week 48

### Added
- `MetadataBoost` struct for entity-enhanced search (`src/filter/boost.rs`)
- `search_boosted()` method on `FilteredSearcher`
- `searchWithBoost()` WASM export for browser usage
- In-browser entity-RAG demo (`demo/entity-rag/`)
- Blog post: "Entity-Enhanced RAG in 300KB" (`docs/blog/`)
- 11 new MetadataBoost unit tests
```

**Acceptance:**
- [ ] W48 section added to CHANGELOG
- [ ] All new features listed
- [ ] Format matches existing CHANGELOG style

---

### W48.5d: END-OF-WEEK HOSTILE REVIEW (2h) — HOSTILE_REVIEWER

**Dependency:** W48.5a, W48.5b, W48.5c complete

**Scope:** Demo + Blog + README + CHANGELOG

**Attack vectors:**
1. **Demo functionality:** Loads in Chrome without errors, search < 100ms, boost toggle works
2. **Blog accuracy:** Research citations are real and correctly attributed. Verify: Xu et al. (2024, arXiv:2404.17723) and Palmonari (2025, UNIMIB invited talk). The 0.52->0.71 accuracy data is from Expert.AI/MEI System collaboration.
3. **Comparison fairness:** EdgeVec vs Qdrant/Weaviate table uses verifiable, current numbers. No strawman comparisons.
4. **README quality:** Code example compiles. Links are not broken. Renders correctly.
5. **Blog tone:** Conversational, not academic. No over-promising.
6. **No broken promises:** Blog doesn't claim features EdgeVec doesn't have (e.g., "scales to 1M vectors in browser" — not validated)
7. **WASM bundle:** < 500KB verified from Day 4
8. **Data quality:** demo data.json has real embeddings from sentence-transformers, not random noise
9. **CHANGELOG completeness:** All W48 deliverables listed

**Verdict options:**
- **GO:** 0 Critical, 0 Major. Day 6 commit proceeds.
- **CONDITIONAL GO:** 0 Critical, <= 2 Major. Fix before Day 6 commit.
- **NO-GO:** >= 1 Critical. Fix all, resubmit.

**Acceptance:**
- [ ] Written review document at `docs/reviews/2026-04-18_W48_ENDWEEK_REVIEW.md`
- [ ] Verdict: GO or CONDITIONAL GO
- [ ] All findings listed with specific references

---

## Day 5 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~6.5h |
| New files | 1 (docs/blog/entity-enhanced-rag.md) |
| Modified files | 2 (README.md, CHANGELOG.md) |
| Hostile review | 1 (end-of-week) |
| Regressions allowed | 0 (no Rust changes today) |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.5a | 3h | | |
| W48.5b | 1h | | |
| W48.5c | 0.5h | | |
| W48.5d | 2h | | |
| **Total** | **6.5h** | | |

---

## Handoff to Day 6

**Codebase state at EOD:**
- Blog post written and hostile-reviewed
- README updated with MetadataBoost section
- CHANGELOG updated
- Both hostile reviews complete (mid-week + end-of-week)

**Day 6 prerequisites satisfied:**
- [ ] End-of-week hostile review verdict (GO or CONDITIONAL GO)
- [ ] All hostile review findings documented

**Day 6 focus:** Fix findings + ROADMAP v7.3 + commit

**Weekend gap note:** If Day 5 (Friday) -> Day 6 (Saturday or Monday), this handoff must be extra-detailed. Re-read this file and the hostile review document before starting Day 6.

---

**END OF DAY 5 TASKS**
