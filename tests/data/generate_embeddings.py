#!/usr/bin/env python3
"""Generate real embedding dataset for PQ recall validation.

Produces embeddings_768d_50k.bin with 50,000 real-world text embeddings
from sentence-transformers/all-mpnet-base-v2 (768D).

This data is used for W47 PQ recall@10 validation (G3 gate: >0.90 on real
embeddings). Uniform random data is meaningless at high D (lesson #64).

Dependencies:
    pip install sentence-transformers numpy datasets

Run:
    python tests/data/generate_embeddings.py

Output:
    tests/data/embeddings_768d_50k.bin  (153,600,000 bytes = 50000 * 768 * 4)

Reproducibility:
    - Model: all-mpnet-base-v2 (768D) from sentence-transformers
    - Source: ag_news dataset (train split, first 50K)
    - Deterministic ordering (dataset index 0..49999)
    - L2-normalized embeddings
"""

import os
import sys
import time
from pathlib import Path

import numpy as np

TARGET_COUNT = 50_000
MODEL_NAME = "all-mpnet-base-v2"
EXPECTED_DIM = 768
FALLBACK_MODEL = "all-MiniLM-L6-v2"
FALLBACK_DIM = 384
BATCH_SIZE = 512

# Output path relative to this script's location
SCRIPT_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = SCRIPT_DIR


def load_sentences(n: int) -> list[str]:
    """Load n diverse English sentences from HuggingFace datasets.

    Priority order:
    1. ag_news (train split) -- 120K news headlines, diverse topics
    2. squad (train split, context field) -- Wikipedia paragraphs
    3. Fallback: simple generated sentences (last resort)
    """
    try:
        from datasets import load_dataset

        print(f"Loading ag_news dataset (need {n} sentences)...")
        ds = load_dataset("ag_news", split="train", trust_remote_code=False)
        if len(ds) >= n:
            sentences = [ds[i]["text"] for i in range(n)]
            print(f"  Loaded {len(sentences)} sentences from ag_news")
            return sentences
        print(f"  ag_news has only {len(ds)} rows, trying squad...")

        ds = load_dataset("squad", split="train", trust_remote_code=False)
        # Use question + context for variety
        sentences = []
        for i, row in enumerate(ds):
            if len(sentences) >= n:
                break
            sentences.append(row["question"])
            if len(sentences) < n:
                # Take first sentence of context for variety
                ctx = row["context"].split(".")[0].strip()
                if ctx:
                    sentences.append(ctx)
        if len(sentences) >= n:
            print(f"  Loaded {n} sentences from squad")
            return sentences[:n]

        print(f"  squad produced only {len(sentences)} sentences")
        raise ValueError("Not enough sentences from available datasets")

    except Exception as e:
        print(f"  Dataset loading failed: {e}")
        print("  FATAL: Cannot proceed without real text data.")
        print("  Install datasets: pip install datasets")
        sys.exit(1)


def generate_embeddings(sentences: list[str], model_name: str, expected_dim: int) -> np.ndarray:
    """Encode sentences into embeddings using sentence-transformers.

    Returns L2-normalized f32 embeddings of shape (n, dim).
    """
    from sentence_transformers import SentenceTransformer

    print(f"Loading model: {model_name}...")
    model = SentenceTransformer(model_name)

    actual_dim = model.get_sentence_embedding_dimension()
    if actual_dim != expected_dim:
        print(f"  WARNING: Expected {expected_dim}D, got {actual_dim}D from {model_name}")
        if model_name == MODEL_NAME:
            print(f"  Falling back to {FALLBACK_MODEL}...")
            return generate_embeddings(sentences, FALLBACK_MODEL, FALLBACK_DIM)

    print(f"Encoding {len(sentences)} sentences (batch_size={BATCH_SIZE})...")
    t0 = time.time()
    embeddings = model.encode(
        sentences,
        batch_size=BATCH_SIZE,
        show_progress_bar=True,
        normalize_embeddings=True,  # L2 normalize
        convert_to_numpy=True,
    )
    elapsed = time.time() - t0
    print(f"  Encoding complete in {elapsed:.1f}s ({len(sentences)/elapsed:.0f} sentences/s)")

    embeddings = embeddings.astype(np.float32)
    return embeddings


def verify_embeddings(embeddings: np.ndarray, expected_dim: int) -> bool:
    """Validate embedding array meets all requirements."""
    ok = True

    # Shape check
    if embeddings.shape != (TARGET_COUNT, expected_dim):
        print(f"  FAIL: shape={embeddings.shape}, expected=({TARGET_COUNT}, {expected_dim})")
        ok = False
    else:
        print(f"  Shape: {embeddings.shape} OK")

    # Dtype check
    if embeddings.dtype != np.float32:
        print(f"  FAIL: dtype={embeddings.dtype}, expected=float32")
        ok = False
    else:
        print(f"  Dtype: {embeddings.dtype} OK")

    # Finiteness check (no NaN, no Inf)
    if not np.all(np.isfinite(embeddings)):
        nan_count = np.sum(np.isnan(embeddings))
        inf_count = np.sum(np.isinf(embeddings))
        print(f"  FAIL: {nan_count} NaN values, {inf_count} Inf values")
        ok = False
    else:
        print(f"  Finiteness: all values finite OK")

    # Value range
    vmin, vmax = embeddings.min(), embeddings.max()
    print(f"  Value range: [{vmin:.4f}, {vmax:.4f}]")

    # L2 norm check (should be ~1.0 since we normalized)
    norms = np.linalg.norm(embeddings, axis=1)
    norm_mean = norms.mean()
    norm_std = norms.std()
    print(f"  L2 norms: mean={norm_mean:.6f}, std={norm_std:.6f}")
    if abs(norm_mean - 1.0) > 0.01:
        print(f"  WARNING: Mean norm deviates from 1.0")

    # Uniqueness check (no duplicate rows)
    # Sample-based check to avoid O(n^2)
    sample_idx = np.random.default_rng(42).choice(len(embeddings), size=min(1000, len(embeddings)), replace=False)
    sample = embeddings[sample_idx]
    dists = np.linalg.norm(sample[:100, None] - sample[None, :100], axis=2)
    np.fill_diagonal(dists, 999.0)
    min_dist = dists.min()
    print(f"  Min pairwise distance (100-sample): {min_dist:.6f}")
    if min_dist < 1e-6:
        print(f"  WARNING: Near-duplicate embeddings detected")

    return ok


def save_embeddings(embeddings: np.ndarray, dim: int) -> Path:
    """Save embeddings as raw f32 binary file."""
    if dim == EXPECTED_DIM:
        filename = f"embeddings_{dim}d_50k.bin"
    else:
        filename = f"embeddings_{dim}d_50k.bin"

    output_path = OUTPUT_DIR / filename
    embeddings.tofile(str(output_path))

    file_size = output_path.stat().st_size
    expected_size = TARGET_COUNT * dim * 4
    print(f"  Saved to: {output_path}")
    print(f"  File size: {file_size:,} bytes (expected: {expected_size:,})")

    if file_size != expected_size:
        print(f"  FAIL: File size mismatch!")
        sys.exit(1)
    else:
        print(f"  File size: OK")

    return output_path


def main():
    print("=" * 60)
    print("EdgeVec Embedding Generator")
    print("PQ Recall Validation Dataset (W47 G3 Gate)")
    print("=" * 60)
    print()

    # Step 1: Load sentences
    sentences = load_sentences(TARGET_COUNT)
    print()

    # Step 2: Generate embeddings
    try:
        embeddings = generate_embeddings(sentences, MODEL_NAME, EXPECTED_DIM)
        dim = embeddings.shape[1]
    except Exception as e:
        print(f"Primary model failed: {e}")
        print(f"Trying fallback model: {FALLBACK_MODEL}...")
        embeddings = generate_embeddings(sentences, FALLBACK_MODEL, FALLBACK_DIM)
        dim = embeddings.shape[1]
    print()

    # Step 3: Verify
    print("Verification:")
    if not verify_embeddings(embeddings, dim):
        print("\nFATAL: Verification failed!")
        sys.exit(1)
    print()

    # Step 4: Save
    print("Saving:")
    output_path = save_embeddings(embeddings, dim)
    print()

    # Summary
    print("=" * 60)
    print("SUMMARY")
    print(f"  Model: {MODEL_NAME if dim == EXPECTED_DIM else FALLBACK_MODEL}")
    print(f"  Dimensions: {dim}")
    print(f"  Vectors: {TARGET_COUNT:,}")
    print(f"  File: {output_path}")
    print(f"  Size: {output_path.stat().st_size:,} bytes")
    print(f"  Source: ag_news (train split, first {TARGET_COUNT})")
    print("=" * 60)


if __name__ == "__main__":
    main()
