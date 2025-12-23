# EdgeVec Roadmap v3.1

**Date:** 2025-12-22
**Author:** PLANNER
**Status:** [REVISED] — Week 28 Update (Pre-Release Planning)
**Current Version:** v0.5.3 (released)
**Next Version:** v0.6.0 (planned — Week 26-29)

---

## Executive Summary

**Total Duration:** ~36 Weeks (Dec 2025 – Aug 2026)
**Current Status:** Week 25 COMPLETE — v0.6.0 Planning Ready
**Philosophy:** Test-First, WASM-Native, Memory-Constrained
**Critical Path:** v0.6.0 (Metadata + BQ) → v0.7.0 (Ecosystem) → v1.0 (Production)

---

## Phase 1: Foundation (Weeks 1-4) — COMPLETE

### Milestone 1: Core Scaffolding & Test Harness
**Status:** COMPLETE
**Gate:** `.claude/GATE_2_COMPLETE.md`

**Deliverables:**
- Repo setup with CI pipeline
- Test harness (proptest, cargo-fuzz, miri)
- Core types (VectorId, NodeId, VectorStorage)

---

## Phase 2: Persistence & Compression (Weeks 5-8) — COMPLETE

### Milestone 2: Storage Engine, Persistence, & Quantization
**Status:** COMPLETE
**Gate:** `.claude/GATE_WEEK8_COMPLETE.md`

**Deliverables:**
- WriteAheadLog, SnapshotManager
- Scalar Quantization (SQ8) — 4x memory reduction
- Binary file format (.evec)

---

## Phase 3: Intelligence (Weeks 9-15) — COMPLETE

### Milestone 3: HNSW Graph + SIMD + RFC-001
**Status:** COMPLETE
**Gates:**
- `.claude/GATE_9_COMPLETE.md` through `.claude/GATE_15_COMPLETE.md`

**Deliverables:**
- HNSW graph algorithms (Insert, Search)
- NeighborPool with VByte compression
- Runtime SIMD detection
- RFC-001 Soft Delete design (approved)

---

## Phase 4: Feature Development (Weeks 16-18) — COMPLETE

### Milestone 4.1: Soft Delete (Week 16)
**Status:** COMPLETE
**Gate:** `.claude/GATE_16_COMPLETE.md`
**Score:** 92/100

**Deliverables:**
- `soft_delete()`, `is_deleted()`, `deleted_count()`, `live_count()`
- `compact()`, `needs_compaction()`, `compaction_warning()`
- Persistence format v0.3 with tombstone support
- Zero memory overhead (reuses padding byte)

### Milestone 4.2: v0.3.0 Release (Week 17)
**Status:** COMPLETE
**Gate:** `.claude/GATE_17_COMPLETE.md`
**Release:** v0.3.0 on crates.io and npm

**Deliverables:**
- WASM soft delete bindings
- TypeScript definitions
- Browser demo (`wasm/examples/soft_delete.html`)
- Documentation update

### Milestone 4.3: Process Hardening & Batch Delete (Week 18)
**Status:** COMPLETE
**Gate:** `.claude/GATE_18_COMPLETE.md`

**Deliverables:**
- CI hardening with `cargo xtask ci-check`
- P99 latency tracking infrastructure
- `soft_delete_batch()` API
- WASM batch delete bindings with Safari fallback
- Dual-license (MIT OR Apache-2.0)

---

## Phase 5: v0.4.0 Release (Week 19) — COMPLETE

### Milestone 5: Documentation & Release Polish
**Status:** COMPLETE
**Gate:** `.claude/GATE_19_COMPLETE.md`

**Deliverables:**
- v0.4.0 released to crates.io and npm
- Documentation update
- CI hardening complete

---

## Phase 6: v0.5.x Filter Expression Language (Week 20-25) — COMPLETE

### Milestone 6: Filter System + RFC-002 Design
**Status:** COMPLETE
**Gate:** v0.5.3 released to crates.io + RFC-002 APPROVED

**Week 20-24 Deliverables:**
- Filter expression parser with AND/OR/NOT/comparison operators
- Error messages with contextual suggestions
- iOS Safari WASM compatibility research
- v0.5.3 crates.io publish (size optimization: 28 MB → 358 KB)

**Week 25 Deliverables:**
- RFC-002 Metadata Storage Design (4 documents) — APPROVED
- RFC-002 Implementation Plan (182 hours) — APPROVED
- Scale-Up Analysis with HOSTILE_REVIEWER verdict
- v0.6.0 roadmap finalized

---

## Phase 7: v0.6.0 Metadata Storage + Binary Quantization (Week 26-29) — PLANNED

### Milestone 7: RFC-002 + Scale-Up Analysis Implementation
**Status:** PLANNED (RFC-002 APPROVED)
**Target:** v0.6.0
**Estimated Duration:** 4.5 weeks (140 hours base + 30% contingency = ~182 hours)

**Week 26: Core Metadata (32 hours)**
- [ ] HnswIndex + insert_with_metadata()
- [ ] soft_delete cleanup + compact metadata
- [ ] search_filtered() with selectivity estimation
- [ ] Persistence format v0.4 with MetadataSectionHeader
- [ ] v0.3 → v0.4 migration

**Week 27: Binary Quantization (48 hours)**
- [ ] BinaryVector type + sign-based encoding
- [ ] SIMD popcount (x86 SSE/AVX + ARM NEON)
- [ ] Hamming distance + BinaryVectorStorage
- [ ] BQ search + rescoring layer
- [ ] Benchmarks (target: 3-5x speedup)

**Week 28: WASM & Integration (40 hours)**
- [ ] Metadata WASM bindings (insertWithMetadata, searchFiltered)
- [ ] BQ WASM bindings (searchBQ, searchHybrid)
- [ ] Memory pressure monitoring
- [ ] Integration tests + browser demo
- [ ] Documentation + CHANGELOG

**Week 29: Buffer & Release (22 hours contingency)**
- [ ] Performance tuning (7 hours)
- [ ] Unforeseen integration issues (15 hours)
- [ ] v0.6.0 release

**Week 29: Pre-Release Cleanup (CRITICAL REMINDER)**

> **INTERNAL FILES TO REMOVE BEFORE RELEASE:**
> These files contain internal development prompts and agent configurations
> that should NOT be public on GitHub.

| File/Folder | Action | Reason |
|:------------|:-------|:-------|
| `.claude/` | DELETE from repo | Internal agent prompts, gate files |
| `.cursor/` | DELETE from repo | Internal Cursor IDE commands |
| `.cursorrules` | DELETE from repo | Internal development rules |
| `CLAUDE.md` | DELETE from repo | Internal project instructions |

**Git Commands for Cleanup:**
```bash
# Remove from git tracking (keeps local copies)
git rm -r --cached .claude/
git rm -r --cached .cursor/
git rm --cached .cursorrules
git rm --cached CLAUDE.md

# Add to .gitignore
echo ".claude/" >> .gitignore
echo ".cursor/" >> .gitignore
echo ".cursorrules" >> .gitignore
echo "CLAUDE.md" >> .gitignore

# Commit cleanup
git commit -m "chore: remove internal development files before v0.6.0 release"
```

**Documentation Checklist for v0.6.0:**
- [ ] README.md updated with v0.6.0 features
- [ ] CHANGELOG.md complete
- [ ] API documentation generated (`cargo doc`)
- [ ] Browser demo deployed (GitHub Pages or similar)
- [ ] TypeScript types published to npm
- [ ] Internal files removed from public repo

**Success Metrics:**
| Metric | Target |
|:-------|:-------|
| BQ memory reduction | 32x vs F32 |
| BQ search speedup | 3-5x |
| BQ recall (with rescore) | >0.90 |
| Filter evaluation | <1μs/vector |
| Metadata overhead | <50 bytes (empty) |

---

## Phase 8: v0.6.1+ Documentation & Ecosystem (Future)

### Milestone 8: Documentation Sprint
**Status:** PLANNED
**Target:** v0.6.1

**Planned Features:**
- Embedding Integration Guide
- LangChain integration
- Example gallery (5+ use cases)
- Cloud sync MVP (S3/R2)

---

## Phase 9: v0.7.0+ Advanced Features (Future)

### Milestone 9: Production Hardening
**Status:** PLANNED
**Target:** v0.7.0+

**Planned Features:**
- ACORN in-algorithm filtering (if needed)
- Metadata indexing
- Hybrid SQ8+BQ two-stage search
- Mobile Safari testing suite

---

## Deferred Features (v1.0+)

Per SCALE_UP_ANALYSIS_2025-12-20.md HOSTILE_REVIEWER verdict:

| Feature | Condition to Revisit |
|:--------|:--------------------|
| P2P Sync (WebRTC) | 10k+ users + 100+ issues requesting |
| React Hooks | Community submits PR |
| Distributed Architecture | Memory64 ships in all browsers |

## Abandoned Features (Never)

| Feature | Reason |
|:--------|:-------|
| AT Protocol patterns | Mathematically incompatible (CRDT + HNSW unsolved) |
| Custom embedding model | Bundle size impossible |
| Own embedding system | Out of scope |

---

## Publication & Outreach Strategy

### v0.6.0 Launch Documentation

| Document | Status | Priority |
|:---------|:-------|:---------|
| README.md (viral hook) | To update | HIGH |
| CHANGELOG.md | To update | HIGH |
| API Reference (rustdoc) | To generate | MEDIUM |
| Browser Demo (GitHub Pages) | To deploy | HIGH |
| Embedding Integration Guide | To write | MEDIUM |

### Research Paper / Technical Write-up

**Objective:** Create a technical paper documenting EdgeVec's architecture and benchmarks for credibility and discoverability.

**Paper Topics:**
1. HNSW + Binary Quantization in WASM (novel combination)
2. 32x memory reduction with >0.90 recall preservation
3. Browser-native vector search without server dependencies
4. SIMD optimization strategies for WASM (popcount, distance)

**Target Venues (Research Before Posting):**

| Venue | Type | Audience | Timing |
|:------|:-----|:---------|:-------|
| **arXiv (cs.IR / cs.DB)** | Preprint | Academics, researchers | After v0.6.0 |
| **Hacker News** | Show HN | Developers, early adopters | Launch day |
| **Reddit r/rust** | Post | Rust community | Launch day |
| **Reddit r/MachineLearning** | Post | ML practitioners | Launch week |
| **Reddit r/LocalLLaMA** | Post | Local AI enthusiasts | Launch week |
| **Dev.to** | Article | Web developers | Launch week |
| **Lobste.rs** | Post | Technical audience | Launch day |
| **X/Twitter** | Thread | General tech | Launch day |
| **LinkedIn** | Article | Professional network | Launch week |

### Launch Checklist (Week 29+)

**Pre-Launch (Week 29):**
- [ ] Finalize v0.6.0 release
- [ ] Deploy browser demo to GitHub Pages
- [ ] Record 60-second demo video/GIF
- [ ] Prepare Show HN post draft
- [ ] Prepare Reddit posts draft
- [ ] Remove internal files (.claude, .cursor, .cursorrules)

**Launch Day:**
- [ ] Publish to crates.io
- [ ] Publish to npm
- [ ] Post to Hacker News (Show HN)
- [ ] Post to Reddit r/rust
- [ ] Tweet/post thread with demo GIF

**Launch Week:**
- [ ] Post to r/MachineLearning, r/LocalLLaMA
- [ ] Publish Dev.to article
- [ ] LinkedIn announcement
- [ ] Monitor feedback, respond to issues

**Post-Launch (v0.6.1+):**
- [ ] Write arXiv paper if traction warrants
- [ ] Create YouTube tutorial
- [ ] Prepare conference talk proposal (if applicable)

### Key Messaging Points

1. **"Vector search in your browser, no server required"**
2. **"32x memory reduction with Binary Quantization"**
3. **"Sub-10ms search on 100k vectors in WASM"**
4. **"MIT licensed, works offline, Safari/iOS compatible"**

---

## Version History

| Version | Date | Highlights |
|:--------|:-----|:-----------|
| v0.1.0 | 2025-12-05 | Initial alpha (HNSW, SQ8) |
| v0.2.0 | 2025-12-10 | Batch API, WASM bindings |
| v0.2.1 | 2025-12-14 | Safety hardening (bytemuck) |
| v0.3.0 | 2025-12-15 | Soft Delete API (RFC-001) |
| v0.4.0 | 2025-12-17 | Documentation, Dashboard, CI |
| v0.5.0 | 2025-12-18 | Filter Expression Language |
| v0.5.2 | 2025-12-19 | Error messages with suggestions |
| v0.5.3 | 2025-12-19 | Crate size optimization (358 KB) |
| v0.6.0 | TBD (W29) | Metadata Storage + Binary Quantization |

---

## Risk Register Summary

| ID | Risk | Status |
|:---|:-----|:-------|
| R1 | WASM Memory Limits (4GB) | MITIGATED |
| R2 | Browser IDB Variability | TESTED |
| R3 | Recall degradation | TESTED (>0.95) |
| R4 | SIMD portability | RUNTIME DETECTION |
| R5 | Memory usage >1GB | MITIGATED (SQ8) |

---

## Approval Status

| Reviewer | Verdict | Date |
|:---------|:--------|:-----|
| HOSTILE_REVIEWER | APPROVED | 2025-12-05 (v1.0) |
| HOSTILE_REVIEWER | APPROVED | 2025-12-14 (Week 16) |
| HOSTILE_REVIEWER | APPROVED | 2025-12-15 (Week 17) |
| HOSTILE_REVIEWER | APPROVED | 2025-12-16 (Week 19 Plan) |
| HOSTILE_REVIEWER | APPROVED | 2025-12-20 (RFC-002) |
| HOSTILE_REVIEWER | APPROVED | 2025-12-20 (Week 25 Day 6) |

---

## Revision History

| Version | Date | Change |
|:--------|:-----|:-------|
| v1.0 | 2025-12-05 | Initial roadmap |
| v1.3 | 2025-12-11 | Week 7 update |
| v2.0 | 2025-12-16 | Week 19 reconciliation — Weeks 16-18 complete |
| v3.0 | 2025-12-20 | Week 25 update — RFC-002 APPROVED, v0.6.0 planning complete |
| v3.1 | 2025-12-22 | Week 28 update — Added pre-release cleanup, publication strategy |

---

**END OF ROADMAP**
