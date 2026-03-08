#!/usr/bin/env python3
"""Patch data.json with new sample queries that match the actual SQuAD dataset topics.

The original queries ("capital of France", "moon landing") don't match the 1000 SQuAD
paragraphs which cover: Beyoncé, Chopin, Tibet, iPod, Zelda, NYC, Solar Energy,
Kanye West, Buddhism, American Idol, To Kill a Mockingbird, Wenchuan earthquake.

Usage:
    python patch_queries.py
"""

import json
import math
import os
import sys
from pathlib import Path

# Force UTF-8 output on Windows
if sys.platform == "win32":
    os.environ["PYTHONIOENCODING"] = "utf-8"
    sys.stdout.reconfigure(encoding="utf-8")


def main():
    print("Loading sentence-transformers...")
    from sentence_transformers import SentenceTransformer

    data_path = Path("data.json")
    print(f"Loading {data_path}...")
    with open(data_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # New queries matched to actual SQuAD topics in the dataset
    new_queries = [
        "Who is Beyoncé and what are her biggest achievements?",
        "When did Chopin move to Paris and what was his life like?",
        "How does solar energy work and what are its applications?",
        "What is the history of New York City?",
        "What are the core beliefs and practices of Buddhism?",
        "What caused the Wenchuan earthquake and its aftermath?",
        "Who is Kanye West and how did he start his career?",
        "What is the plot of To Kill a Mockingbird?",
        "When was the iPod released and how did it evolve?",
        "How did American Idol impact the music industry?",
    ]

    model_name = data.get("metadata", {}).get("model", "all-MiniLM-L6-v2")
    print(f"Loading model: {model_name}...")
    model = SentenceTransformer(model_name)

    print(f"Encoding {len(new_queries)} queries...")
    query_embeddings = model.encode(
        new_queries,
        batch_size=16,
        normalize_embeddings=True,
    )

    # Validate
    for i, emb in enumerate(query_embeddings):
        norm = sum(v**2 for v in emb) ** 0.5
        print(f"  Q{i}: norm={norm:.4f} | {new_queries[i][:50]}")

    # Compute brute-force top-3 for each query to verify quality
    print("\nBrute-force verification:")
    for qi, (q_text, q_emb) in enumerate(zip(new_queries, query_embeddings)):
        sims = []
        for doc in data["documents"]:
            dot = sum(a * b for a, b in zip(q_emb, doc["embedding"]))
            sims.append((doc["id"], dot, doc["text"][:80]))
        sims.sort(key=lambda x: -x[1])
        top = sims[0]
        print(f"  Q{qi} max_sim={top[1]:.4f} | {q_text[:40]}...")
        print(f"       -> id={top[0]}: {top[2]}")

    # Patch queries
    queries_data = []
    for q_text, q_emb in zip(new_queries, query_embeddings):
        q_list = [round(float(v), 6) for v in q_emb]
        if all(math.isfinite(v) for v in q_list):
            queries_data.append({"text": q_text, "embedding": q_list})

    data["queries"] = queries_data
    data["metadata"]["num_queries"] = len(queries_data)

    # Write back
    print(f"\nWriting patched {data_path}...")
    with open(data_path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False)

    file_size = data_path.stat().st_size
    print(f"Done! File size: {file_size / 1024 / 1024:.1f} MB")


if __name__ == "__main__":
    main()
