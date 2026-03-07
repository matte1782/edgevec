# W49 Mandatory Add-ons

## Status: [DRAFT]

These tasks are **mandatory carry-forward items** that MUST be completed in W49, in addition to whatever the weekly plan defines.

---

## M1: Demo Hub Update (P0 — Day 1)

### Context
The Demo Hub (`docs/demo/hub.html`) has not been updated since **v0.7.0**. The current version is **v0.9.0**. Three full feature weeks (W46 PQ, W47 PQ Validation, W48 Growth/MetadataBoost) are missing from the hub.

**Live URL:** https://matte1782.github.io/edgevec/demo/hub.html

### Requirements

**Version Updates:**
- [ ] All version badges: v0.7.0 -> v0.9.0
- [ ] Footer brand: `EDGEVEC v0.7.0` -> `EDGEVEC v0.9.0`
- [ ] Remove stale "NEW" badge from SIMD Benchmark card (shipped in v0.7.0, no longer new)

**New Featured Card: Entity-RAG Demo**
- [ ] Add Entity-RAG demo as the new **FEATURED** card (replaces Filter Playground as top card)
- [ ] Link: `entity-rag/index.html`
- [ ] Version badge: v0.9.0
- [ ] Badges: LATEST, NEW, FEATURED
- [ ] Features: Entity NER, MetadataBoost, SQuAD Dataset, Live WASM
- [ ] Description highlighting entity-enhanced RAG in the browser
- [ ] Date: March 2026

**Demote Filter Playground:**
- [ ] Move Filter Playground from featured section to Core Features grid
- [ ] Remove LATEST/FEATURED badges, keep v0.7.0 version badge
- [ ] Keep all existing content

**Hero Stats Update:**
- [ ] Consider adding PQ compression stat (e.g., "12x PQ Compress" or "8B/vec PQ")
- [ ] Verify existing stats still accurate (32x Memory, 2x+ SIMD, <1ms Latency)

**Optional — New Category: AI/RAG**
- [ ] Consider adding an "AI / RAG" category section for Entity-RAG
- [ ] Or place Entity-RAG in a renamed "Featured Demos" category

### Quality Constraints
- **UI quality MUST match existing cyberpunk aesthetic** — same design language, animations, card styles
- Consistent with Orbitron + JetBrains Mono font stack
- Responsive: must look good on mobile (test at 375px width)
- Matrix rain background, gradient overlay, hover effects — all preserved
- Card hover animations (translateY, glow, top-line reveal) — maintained
- New cards must have appropriate SVG icons matching the style

### Acceptance Criteria
- [ ] Hub loads with no console errors
- [ ] Entity-RAG card links correctly to `entity-rag/index.html`
- [ ] All version references say v0.9.0
- [ ] No "NEW" badge on anything older than v0.9.0
- [ ] Mobile responsive (375px - 1200px+)
- [ ] Visual consistency with existing cards (hostile reviewer will check)
- [ ] Passes `/review docs/demo/hub.html`

### Estimated Effort
4-6 hours (HTML/CSS only, no JS changes needed beyond existing matrix rain)
