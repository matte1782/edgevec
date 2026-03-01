# Week 43 — Day 6: Documentation

**Date:** 2026-03-12
**Status:** PENDING
**Focus:** Write comprehensive README for `edgevec-langchain` package with usage examples, filter docs, WASM init guide
**Prerequisite:** Day 5 complete (all code + tests done, build verified)
**Reference:** `docs/research/LANGCHAIN_SPIKE.md` Section 5 (Competitive Analysis)

---

## Tasks

| Task | ID | Hours | Status | Dependency |
|:-----|:---|:------|:-------|:-----------|
| Write `pkg/langchain/README.md` with usage examples | W43.6a | 1.5h | PENDING | Day 5 |
| Add quick start: 5-line example showing embed + store + search | W43.6b | 0.5h | PENDING | W43.6a |
| Document filter format with EdgeVec DSL examples | W43.6c | 0.5h | PENDING | W43.6a |
| Document save/load with IndexedDB persistence | W43.6d | 0.5h | PENDING | W43.6a |
| Document WASM initialization requirements | W43.6e | 0.5h | PENDING | W43.6a |
| Run final lint + type check | W43.6f | 0.5h | PENDING | Day 5 |

**Total Estimated Hours:** 4h

---

## Critical Path

```
W43.6a (README structure) → W43.6b + W43.6c + W43.6d + W43.6e (sections, parallel)
W43.6f (lint) — independent
```

W43.6f can run at any time. The README sections (b-e) can be written in parallel after the main structure (a) is established.

---

## Artifacts Produced

| Artifact | Path | Description |
|:---------|:-----|:------------|
| Package README | `pkg/langchain/README.md` | Full documentation |

---

## README Structure

```markdown
# edgevec-langchain

> LangChain.js VectorStore adapter for EdgeVec — in-browser vector search

## Why EdgeVec?
- Comparison table vs Voy (abandoned), MemoryVectorStore (no persistence)

## Installation

## Quick Start (5 lines)

## API Reference
- EdgeVecStore constructor
- addVectors / addDocuments
- similaritySearch / similaritySearchVectorWithScore
- delete
- fromTexts / fromDocuments
- save / load
- asRetriever

## Filter Syntax (EdgeVec DSL)
- Comparison: =, !=, <, >, <=, >=
- Logical: AND, OR, NOT
- Null: IS NULL, IS NOT NULL
- Examples

## WASM Initialization
- Auto-init via factory methods
- Manual init via initEdgeVec()
- Error handling: EdgeVecNotInitializedError

## Persistence (IndexedDB)
- save(directory)
- load(directory, embeddings)
- Data stored: vectors, metadata, ID mapping

## Score Normalization
- How scores are converted from distance to similarity
- Table: metric → formula → range

## Coming Next
- FilterExpression object support (W44)

## License
```

---

## Documentation Standards

| Standard | Requirement |
|:---------|:------------|
| Quick start | Copy-pasteable, works in browser |
| Examples | All examples use realistic, non-trivial data |
| Filter docs | At least 5 DSL examples covering all operators |
| API reference | Every public method documented with signature + description |
| WASM init | Clear diagram: when auto-init vs manual init |
| Error docs | Every custom error type documented |

---

## FilterExpression Deferral Notice

Per the weekly plan: `FilterExpression` object support is deferred to W44. The README must document this clearly:

```markdown
> **Coming in next release:** `FilterExpression` object support for programmatic filter construction.
> Currently, filters are passed as EdgeVec DSL strings.
```

---

## Acceptance Criteria

- [ ] README has: installation, quick start, full API reference, filter examples, WASM init guide
- [ ] Quick start example is copy-pasteable and works (verified by running it)
- [ ] At least 5 filter DSL examples covering: `=`, `!=`, `<`, `>`, `AND`, `OR`, `IS NULL`
- [ ] WASM init section explains both auto-init and manual init paths
- [ ] Persistence section explains IndexedDB mapping
- [ ] Score normalization table included
- [ ] `FilterExpression` deferral clearly noted
- [ ] `npx eslint src/` passes (or equivalent lint)
- [ ] `npx tsc --noEmit` passes (strict mode type check)

---

## Risk Notes

| Risk | Mitigation |
|:-----|:-----------|
| Quick start example becomes stale | Pin to specific LangChain version in example |
| Filter syntax docs incomplete | Reference EdgeVec main docs for full DSL spec |

---

## Exit Criteria

**Day 6 is complete when:**
1. All 6 tasks are DONE
2. README is comprehensive and accurate
3. Lint and type check pass
4. Quick start example verified

**Handoff to Day 7:** Documentation is complete. Day 7 runs hostile review on the entire package.

---

**END OF DAY 6 PLAN**
