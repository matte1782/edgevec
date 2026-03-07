#!/usr/bin/env python3
"""Prepare demo data for EdgeVec entity-enhanced RAG demo.

Loads 1000 SQuAD v2 paragraphs, embeds with all-MiniLM-L6-v2 (384D),
extracts entities with spaCy NER, and outputs data.json.

Dependencies:
    pip install sentence-transformers spacy datasets
    python -m spacy download en_core_web_sm

Usage:
    python prepare_data.py
    python prepare_data.py --num-docs 500  # smaller dataset
    python prepare_data.py --output custom.json
"""

import argparse
import json
import math
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Prepare EdgeVec entity-RAG demo data")
    parser.add_argument("--num-docs", type=int, default=1000, help="Number of documents")
    parser.add_argument("--output", type=str, default="data.json", help="Output file path")
    parser.add_argument("--model", type=str, default="all-MiniLM-L6-v2", help="Embedding model")
    parser.add_argument("--batch-size", type=int, default=64, help="Embedding batch size")
    parser.add_argument("--num-queries", type=int, default=10, help="Number of sample queries")
    args = parser.parse_args()

    print(f"Loading dependencies...")

    from datasets import load_dataset
    from sentence_transformers import SentenceTransformer
    import spacy

    # Step 1: Load SQuAD paragraphs
    print(f"Loading SQuAD v2 dataset...")
    dataset = load_dataset("rajpurkar/squad_v2", split="train")

    # Extract unique paragraphs (SQuAD has many questions per context)
    seen = set()
    paragraphs = []
    for item in dataset:
        ctx = item["context"].strip()
        if ctx not in seen and len(ctx) > 50:
            seen.add(ctx)
            paragraphs.append(ctx)
            if len(paragraphs) >= args.num_docs:
                break

    print(f"Selected {len(paragraphs)} unique paragraphs")

    if len(paragraphs) < args.num_docs:
        print(f"WARNING: Only found {len(paragraphs)} unique paragraphs (requested {args.num_docs})")

    # Step 2: Embed with sentence-transformers
    print(f"Loading embedding model: {args.model}...")
    model = SentenceTransformer(args.model)

    print(f"Encoding {len(paragraphs)} paragraphs (batch_size={args.batch_size})...")
    embeddings = model.encode(
        paragraphs,
        batch_size=args.batch_size,
        show_progress_bar=True,
        normalize_embeddings=True,
    )
    dim = embeddings.shape[1]
    print(f"Embeddings shape: {embeddings.shape} (dim={dim})")

    # Step 3: Extract entities with spaCy
    print(f"Loading spaCy model...")
    nlp = spacy.load("en_core_web_sm")

    print(f"Extracting entities...")
    docs_data = []
    entity_type_counts = {}

    for i, (text, emb) in enumerate(zip(paragraphs, embeddings)):
        doc = nlp(text)

        # Extract unique entities and their types
        entities = []
        entity_types = []
        seen_ents = set()
        for ent in doc.ents:
            ent_text = ent.text.strip()
            if ent_text and ent_text not in seen_ents:
                seen_ents.add(ent_text)
                entities.append(ent_text)
                entity_types.append(ent.label_)
                entity_type_counts[ent.label_] = entity_type_counts.get(ent.label_, 0) + 1

        # Round embeddings to 6 decimal places
        embedding_list = [round(float(v), 6) for v in emb]

        # Validate finiteness
        if any(not math.isfinite(v) for v in embedding_list):
            print(f"WARNING: Non-finite embedding at doc {i}, skipping")
            continue

        docs_data.append({
            "id": i,
            "text": text[:500],  # Truncate very long paragraphs for demo
            "embedding": embedding_list,
            "metadata": {
                "entities": entities,
                "entity_types": entity_types,
            },
        })

        if (i + 1) % 200 == 0:
            print(f"  Processed {i + 1}/{len(paragraphs)} documents")

    print(f"Processed {len(docs_data)} documents")

    # Step 4: Generate sample queries with embeddings
    print(f"Generating {args.num_queries} sample queries...")
    sample_queries = [
        "Who was the first person to walk on the moon?",
        "What is the capital of France?",
        "How does photosynthesis work in plants?",
        "When was the Declaration of Independence signed?",
        "What causes earthquakes and tectonic activity?",
        "Who invented the telephone?",
        "What is the theory of general relativity?",
        "How do vaccines protect against diseases?",
        "What is the largest ocean on Earth?",
        "When did World War II end?",
    ]
    sample_queries = sample_queries[: args.num_queries]

    query_embeddings = model.encode(
        sample_queries,
        batch_size=args.batch_size,
        normalize_embeddings=True,
    )

    queries_data = []
    for q_text, q_emb in zip(sample_queries, query_embeddings):
        q_list = [round(float(v), 6) for v in q_emb]
        if all(math.isfinite(v) for v in q_list):
            queries_data.append({
                "text": q_text,
                "embedding": q_list,
            })

    # Step 5: Build output
    output = {
        "documents": docs_data,
        "queries": queries_data,
        "metadata": {
            "model": args.model,
            "dimensions": dim,
            "num_documents": len(docs_data),
            "num_queries": len(queries_data),
            "entity_type_distribution": dict(
                sorted(entity_type_counts.items(), key=lambda x: -x[1])
            ),
        },
    }

    # Step 6: Write output
    output_path = Path(args.output)
    print(f"Writing to {output_path}...")
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False)

    file_size = output_path.stat().st_size
    print(f"\nDone!")
    print(f"  Documents: {len(docs_data)}")
    print(f"  Queries: {len(queries_data)}")
    print(f"  Dimensions: {dim}")
    print(f"  Entity types: {len(entity_type_counts)} ({', '.join(list(entity_type_counts.keys())[:5])}...)")
    print(f"  Docs with entities: {sum(1 for d in docs_data if d['metadata']['entities'])}/{len(docs_data)}")
    print(f"  File size: {file_size / 1024 / 1024:.1f} MB")

    if file_size > 5 * 1024 * 1024:
        print(f"WARNING: File exceeds 5MB limit ({file_size / 1024 / 1024:.1f} MB)")
        print(f"Consider reducing --num-docs or using fewer decimal places")

    return 0


if __name__ == "__main__":
    sys.exit(main())
