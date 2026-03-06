# Week 48 — Day 3 Tasks (Wednesday, Apr 16)

**Date:** 2026-04-16
**Focus:** Demo Data Preparation (Python — SQuAD + Embeddings + NER)
**Agents:** BENCHMARK_SCIENTIST
**Status:** PENDING

---

## Day Objective

Generate `demo/entity-rag/data.json` containing 1000 SQuAD paragraphs with 384D embeddings and entity metadata. This file is the data backbone for the Day 4 in-browser demo.

**Success Criteria:**
- `demo/entity-rag/prepare_data.py` exists and is reproducible
- `demo/entity-rag/data.json` exists with 1000 documents
- Each document has: text, 384D embedding, entity metadata
- File size < 5MB
- At least 80% of documents have >= 1 entity
- At least 3 distinct entity types present

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` — data format spec
- [ ] `src/metadata/types.rs` — MetadataValue types (String, Integer, Float, Boolean, StringArray) — demo metadata must be compatible
- [ ] `src/filter/boost.rs` — MetadataBoost struct — demo boosts must match field/value format

---

## Tasks

### W48.3a: Create `demo/entity-rag/prepare_data.py` (3h) — BENCHMARK_SCIENTIST

**Dependency:** None (independent of Rust code)

**Pre-requisite check:**
```bash
# Create directory
mkdir -p demo/entity-rag

# Check Python
python --version    # Python 3.8+ required

# Install dependencies (in virtualenv recommended)
pip install sentence-transformers spacy datasets numpy
python -m spacy download en_core_web_sm
```

**Decision Tree:**
- If `datasets` (HuggingFace) install fails -> download SQuAD v2 JSON directly from https://rajpurkar.github.io/SQuAD-explorer/
- If `sentence-transformers` install fails on Windows -> use WSL2 or conda
- If `en_core_web_sm` download fails -> use regex-based NER fallback (capitalized multi-word phrases)
- If `all-MiniLM-L6-v2` model download fails -> try `all-MiniLM-L12-v2` (same 384D output)

**Script structure:**
```python
#!/usr/bin/env python3
"""Generate entity-RAG demo data for EdgeVec.

Produces data.json with 1000 SQuAD paragraphs, 384D embeddings,
and entity metadata from spaCy NER.

Dependencies: sentence-transformers, spacy, datasets, numpy
Run: python prepare_data.py

Output: data.json (~3-4MB)
"""

import json
import os
import numpy as np
from datasets import load_dataset
from sentence_transformers import SentenceTransformer
import spacy

def main():
    # 1. Load SQuAD v2 contexts (deduplicate, take first 1000)
    dataset = load_dataset("squad_v2", split="train")
    contexts = list(dict.fromkeys(dataset["context"]))[:1000]

    # 2. Embed with all-MiniLM-L6-v2 (384D)
    model = SentenceTransformer("all-MiniLM-L6-v2")
    embeddings = model.encode(contexts, show_progress_bar=True,
                              normalize_embeddings=True)

    # 3. Extract entities with spaCy
    nlp = spacy.load("en_core_web_sm")

    # 4. Also embed 10 sample queries for the demo dropdown
    sample_queries = [
        "What is the capital of France?",
        "Who invented the telephone?",
        "When was the Declaration of Independence signed?",
        "What causes earthquakes?",
        "Who won the 2014 FIFA World Cup?",
        "What is photosynthesis?",
        "Who painted the Mona Lisa?",
        "What is the speed of light?",
        "When did World War II end?",
        "What is the largest planet in our solar system?",
    ]
    query_embeddings = model.encode(sample_queries,
                                     normalize_embeddings=True)

    # 5. Build output
    documents = []
    for i, (ctx, emb) in enumerate(zip(contexts, embeddings)):
        doc = nlp(ctx)
        entities = list(set(ent.text for ent in doc.ents))
        entity_types = list(set(ent.label_ for ent in doc.ents))
        # Simple topic heuristic: most common entity type
        topic = entity_types[0] if entity_types else "general"

        documents.append({
            "id": i,
            "text": ctx[:500],  # Truncate for display
            "embedding": [round(float(x), 6) for x in emb.tolist()],  # Round to 6 decimals for size control
            "metadata": {
                "entities": entities[:10],     # Cap at 10
                "entity_types": entity_types,
                "topic": topic,
                "has_entities": len(entities) > 0,
            }
        })

    output = {
        "documents": documents,
        "sample_queries": [
            {"text": q, "embedding": [round(float(x), 6) for x in e.tolist()]}
            for q, e in zip(sample_queries, query_embeddings)
        ],
        "model": "all-MiniLM-L6-v2",
        "dimensions": 384,
    }

    with open("data.json", "w") as f:
        json.dump(output, f)

    # Verify
    size_mb = os.path.getsize("data.json") / (1024 * 1024)
    entity_pct = sum(1 for d in documents
                     if d["metadata"]["has_entities"]) / len(documents) * 100
    types = set()
    for d in documents:
        types.update(d["metadata"]["entity_types"])

    print(f"Documents: {len(documents)}")
    print(f"Dimensions: {384}")
    print(f"File size: {size_mb:.1f} MB")
    print(f"Docs with entities: {entity_pct:.0f}%")
    print(f"Entity types: {len(types)} — {sorted(types)[:10]}")

if __name__ == "__main__":
    main()
```

**Commands:**
```bash
cd demo/entity-rag
python prepare_data.py
```

**Expected Output:**
- `demo/entity-rag/prepare_data.py` — reproducible script
- `demo/entity-rag/data.json` — ~3-4MB
- Console output showing doc count, dimensions, file size, entity stats

**Acceptance:**
- [ ] Script runs to completion without errors
- [ ] `data.json` created in `demo/entity-rag/`
- [ ] Script committed to git, `data.json` in `.gitignore` (too large)

---

### W48.3b: Verify Data Quality (0.5h) — BENCHMARK_SCIENTIST

**Dependency:** W48.3a complete

**Commands:**
```bash
cd demo/entity-rag

# Verify structure and dimensions
python -c "
import json
d = json.load(open('data.json'))
docs = d['documents']
queries = d['sample_queries']
assert len(docs) == 1000, f'Expected 1000 docs, got {len(docs)}'
assert len(queries) == 10, f'Expected 10 queries, got {len(queries)}'
assert len(docs[0]['embedding']) == 384, f'Expected 384D, got {len(docs[0][\"embedding\"])}'
assert len(queries[0]['embedding']) == 384, f'Expected 384D query, got {len(queries[0][\"embedding\"])}'

# Check all embeddings finite
import math
for i, doc in enumerate(docs):
    for j, v in enumerate(doc['embedding']):
        assert math.isfinite(v), f'NaN/Inf at doc {i} dim {j}'

# Entity coverage
with_entities = sum(1 for doc in docs if doc['metadata']['has_entities'])
pct = with_entities / len(docs) * 100
assert pct >= 80, f'Entity coverage too low: {pct:.0f}%'

# Entity type diversity
types = set()
for doc in docs:
    types.update(doc['metadata']['entity_types'])
assert len(types) >= 3, f'Only {len(types)} entity types: {types}'

# File size
import os
size_mb = os.path.getsize('data.json') / (1024*1024)
assert size_mb < 5, f'File too large: {size_mb:.1f}MB'

print(f'PASS: {len(docs)} docs, 384D, {size_mb:.1f}MB, {pct:.0f}% with entities, {len(types)} types')
"
```

**Acceptance:**
- [ ] 1000 documents with 384D embeddings
- [ ] 10 sample queries with 384D embeddings
- [ ] All embeddings finite (no NaN/Inf)
- [ ] All embeddings rounded to 6 decimal places
- [ ] >= 80% documents have at least 1 entity
- [ ] >= 3 distinct entity types
- [ ] File size < 5MB

---

### W48.3c: Add data.json to .gitignore (0.1h) — BENCHMARK_SCIENTIST

**Dependency:** W48.3b complete

**Commands:**
```bash
echo "demo/entity-rag/data.json" >> .gitignore
git add demo/entity-rag/prepare_data.py .gitignore
```

**Acceptance:**
- [ ] `data.json` in `.gitignore`
- [ ] `prepare_data.py` staged for commit

---

## Day 3 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~3.6h |
| New files | 2 (prepare_data.py, data.json) |
| Python dependencies | sentence-transformers, spacy, datasets |
| Regressions allowed | 0 (no Rust changes today) |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.3a | 3h | | |
| W48.3b | 0.5h | | |
| W48.3c | 0.1h | | |
| **Total** | **3.6h** | | |

---

## Handoff to Day 4

**Codebase state at EOD:**
- `demo/entity-rag/data.json` exists and verified (1000 docs, 384D, < 5MB)
- `demo/entity-rag/prepare_data.py` committed
- MetadataBoost API hostile-reviewed (from Day 2)
- No Rust changes today — all existing tests still pass

**Day 4 prerequisites satisfied:**
- [ ] `data.json` exists (needed for demo HTML to load)
- [ ] MetadataBoost WASM export compiles (from Day 2)
- [ ] `wasm-pack build --release` needs to be run (Day 4 first step)

**Day 4 focus:** In-browser demo HTML + JS

---

**END OF DAY 3 TASKS**
