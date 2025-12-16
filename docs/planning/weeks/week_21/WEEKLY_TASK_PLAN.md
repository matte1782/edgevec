# Week 21: Metadata Storage API & Mobile Testing Sprint

**Version:** 0.5.0-alpha
**Date Range:** 2025-12-30 to 2026-01-03 (spans year boundary)
**Theme:** Metadata Storage Foundation & Mobile Platform Verification
**Status:** APPROVED (HR-2025-12-16-W21-PLAN)
**Revision:** 1.0

---

## Executive Summary

Week 21 delivers the **P0 CRITICAL** Metadata Storage API — the #1 user complaint from the v0.4.0 external review. This feature transforms EdgeVec from a pure vector store into a production-ready database by enabling key-value metadata attachment to vectors.

**Strategic Context (per V0.5.0_STRATEGIC_ROADMAP.md):**
- Metadata API is the FOUNDATION for Week 22-23 Filtering implementation
- Schema will be FROZEN after this week (breaking changes require major version bump)
- Unlocks production RAG use cases

**Week 20 Completion Status:**
- ✅ Day 1: ARM CI verification complete
- ✅ Day 2: neon.rs module & dispatcher integration complete
- ✅ Day 3: NEON Hamming distance implementation complete
- ✅ Day 4: NEON Dot Product & Euclidean distance complete
- ✅ Day 5: Correctness testing & bundle analysis complete
- ✅ GATE_W20_PLANNING_COMPLETE.md exists

**Week 21 Mandatory Scope (40 hours):**
1. **Metadata Storage API (24h)** — P0 CRITICAL
2. **Mobile Browser Testing (8h)** — P1 HIGH
3. **BrowserStack CI Integration (8h)** — P2 MEDIUM

---

## Week Overview

| Day | Task ID | Title | Hours | Priority | Deliverable |
|:----|:--------|:------|:------|:---------|:------------|
| 1 | W21.1 | Metadata API Architecture & Core Types | 8 | **CRITICAL** | `src/metadata/mod.rs`, types defined |
| 2 | W21.2 | Metadata Storage Implementation | 8 | **CRITICAL** | CRUD operations, persistence integration |
| 3 | W21.3 | Metadata WASM Bindings & TypeScript Types | 8 | **CRITICAL** | `wasm/`, TypeScript definitions |
| 4 | W21.4 | Mobile Browser Testing (iOS Safari, Android Chrome) | 8 | HIGH | Test reports, limitation docs |
| 5 | W21.5 | BrowserStack CI & Schema Freeze | 8 | HIGH | CI integration, GATE_W21_COMPLETE.md |

**Total Estimated Hours:** 40 hours (full budget)
**Critical Path:** W21.1 → W21.2 → W21.3 → W21.4 → W21.5

---

## Strategic Alignment

### External Review Response (v0.4.0)

| Criticism | Response | Week 21 Deliverable |
|:----------|:---------|:-------------------|
| "Missing metadata storage" | METADATA_API | Days 1-3 |
| "Database lie" | Metadata makes it a real database | Full CRUD + persistence |
| "Loading bottleneck" | Web Worker docs | Day 4 documentation |
| "No mobile testing" | BrowserStack | Day 4-5 |

### v0.5.0 Timeline Position

```
Week 20: ARM/NEON SIMD ✅ COMPLETE
    │
    ▼
Week 21: METADATA_API + Mobile Testing ◄── YOU ARE HERE
    │    └── Deliverable: GATE_W21_COMPLETE.md + schema FROZEN
    ▼
Week 22: FILTERING_ARCHITECTURE (design sprint)
    │    └── Prerequisite: GATE_W21_COMPLETE.md exists
    ▼
Week 23: FILTERING_IMPLEMENTATION
    │
    ▼
Week 24: v0.5.0 RELEASE
```

---

## Technical Specifications

### Metadata API Design

**Data Model:**
```rust
/// Metadata storage for vectors
/// Maps VectorId → HashMap<String, MetadataValue>
pub struct MetadataStore {
    data: HashMap<u32, HashMap<String, MetadataValue>>,
}

/// Supported metadata value types
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MetadataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    StringArray(Vec<String>),
}
```

**Public API Surface:**
```rust
impl MetadataStore {
    pub fn new() -> Self;
    pub fn insert(&mut self, vector_id: u32, key: &str, value: MetadataValue) -> Result<()>;
    pub fn get(&self, vector_id: u32, key: &str) -> Option<&MetadataValue>;
    pub fn get_all(&self, vector_id: u32) -> Option<&HashMap<String, MetadataValue>>;
    pub fn update(&mut self, vector_id: u32, key: &str, value: MetadataValue) -> Result<()>;
    pub fn delete(&mut self, vector_id: u32, key: &str) -> Result<bool>;
    pub fn delete_all(&mut self, vector_id: u32) -> Result<bool>;
    pub fn has_key(&self, vector_id: u32, key: &str) -> bool;
    pub fn keys(&self, vector_id: u32) -> Option<impl Iterator<Item = &String>>;
}
```

**WASM API (TypeScript):**
```typescript
interface MetadataValue {
  type: 'string' | 'integer' | 'float' | 'boolean' | 'string_array';
  value: string | number | boolean | string[];
}

class MetadataStore {
  set(vectorId: number, key: string, value: MetadataValue): void;
  get(vectorId: number, key: string): MetadataValue | undefined;
  getAll(vectorId: number): Record<string, MetadataValue> | undefined;
  delete(vectorId: number, key: string): boolean;
  deleteAll(vectorId: number): boolean;
}
```

### Persistence Integration

Metadata will be persisted alongside vectors using the existing snapshot mechanism:

```rust
// Extended snapshot format
pub struct SnapshotV2 {
    pub magic: [u8; 4],       // "EVEC"
    pub version: u32,         // 2 (bumped from 1)
    pub header: SnapshotHeader,
    pub vectors: Vec<VectorData>,
    pub graph: HnswGraph,
    pub metadata: MetadataStore,  // NEW
    pub checksum: u32,
}
```

### Constraints

| Constraint | Value | Rationale |
|:-----------|:------|:----------|
| Max keys per vector | 64 | Memory budget |
| Max key length | 256 bytes | Reasonable limit |
| Max string value length | 65,536 bytes | 64KB limit |
| Max array elements | 1,024 | Reasonable limit |
| Supported types | 5 | String, Integer, Float, Boolean, StringArray |

---

## Daily Breakdown

### Day 1: Metadata API Architecture & Core Types (W21.1)
- Define `MetadataValue` enum with all supported types
- Define `MetadataStore` struct with `HashMap<u32, HashMap<String, MetadataValue>>`
- Implement `Serialize`/`Deserialize` for persistence
- Create `src/metadata/mod.rs` module structure
- Write unit tests for type serialization

### Day 2: Metadata Storage Implementation (W21.2)
- Implement all CRUD operations (insert, get, update, delete)
- Implement validation (key length, value size limits)
- Integrate with existing error handling (`EdgeVecError`)
- Add persistence hooks (extend snapshot format)
- Write comprehensive unit tests (>90% coverage)

### Day 3: Metadata WASM Bindings & TypeScript (W21.3)
- Create `wasm-bindgen` exports for all operations
- Generate TypeScript type definitions
- Test WASM bindings in Node.js environment
- Document WASM API usage
- Verify no bundle size regression (target: <500KB gzipped)

### Day 4: Mobile Browser Testing (W21.4)
- Test on iOS Safari (manual or Simulator)
- Test on Android Chrome (manual or Emulator)
- Document any platform-specific limitations
- Create mobile usage guide
- Verify WASM loads and executes correctly

### Day 5: BrowserStack CI & Schema Freeze (W21.5)
- Integrate BrowserStack into GitHub Actions
- Run automated tests on real mobile devices
- Document schema as FROZEN
- Create GATE_W21_COMPLETE.md
- Prepare handoff for Week 22 (Filtering Architecture)

---

## Success Criteria

Week 21 is **COMPLETE** when:

1. [ ] `MetadataStore` API fully implemented with all CRUD operations
2. [ ] All 5 metadata value types supported (String, Integer, Float, Boolean, StringArray)
3. [ ] Metadata persisted in snapshots (v2 format)
4. [ ] WASM bindings exported and TypeScript types generated
5. [ ] Bundle size < 500KB gzipped (no regression)
6. [ ] iOS Safari test passes (WASM loads and executes)
7. [ ] Android Chrome test passes (WASM loads and executes)
8. [ ] BrowserStack CI integration operational
9. [ ] Unit test coverage > 90% for metadata module
10. [ ] Property tests for metadata operations (1000+ inputs)
11. [ ] Metadata schema documented and marked FROZEN
12. [ ] HOSTILE_REVIEWER grants final GO verdict
13. [ ] GATE_W21_COMPLETE.md created

---

## Risk Register

| Risk | Probability | Impact | Mitigation | Fallback |
|:-----|:------------|:-------|:-----------|:---------|
| R1: Snapshot format migration | Medium | HIGH | Version field in header | Support v1 read, v2 write |
| R2: WASM bundle size regression | Low | MEDIUM | Monitor with wasm-opt | Lazy-load metadata module |
| R3: Mobile WASM compatibility | Medium | HIGH | Test early (Day 4) | Document limitations |
| R4: BrowserStack API issues | Medium | MEDIUM | Local testing backup | Manual device testing |
| R5: Type serialization bugs | Low | HIGH | Property tests | Conservative type set |

---

## Estimation Methodology

**3x Rule Applied:**

| Day | Task | Optimistic | 3x Applied | Buffer | Final |
|:----|:-----|:-----------|:-----------|:-------|:------|
| 1 | Core Types | 2h | 6h | 2h | 8h |
| 2 | Implementation | 2h | 6h | 2h | 8h |
| 3 | WASM Bindings | 2h | 6h | 2h | 8h |
| 4 | Mobile Testing | 2h | 6h | 2h | 8h |
| 5 | CI + Freeze | 2h | 6h | 2h | 8h |

**Total:** 40h with buffer included

---

## Quality Gates

### Gate 21.1: Core Types Complete
- [ ] `MetadataValue` enum defined with 5 types
- [ ] `MetadataStore` struct defined
- [ ] Serialization tests pass
- [ ] `/review` approved

### Gate 21.2: Implementation Complete
- [ ] All CRUD operations working
- [ ] Validation enforced
- [ ] Persistence integrated
- [ ] Unit tests pass (>90% coverage)
- [ ] `/review` approved

### Gate 21.3: WASM Bindings Complete
- [ ] All operations exported
- [ ] TypeScript types generated
- [ ] Bundle size verified
- [ ] Node.js tests pass
- [ ] `/review` approved

### Gate 21.4: Mobile Testing Complete
- [ ] iOS Safari verified
- [ ] Android Chrome verified
- [ ] Limitations documented
- [ ] `/review` approved

### Gate 21.5: Week 21 Complete
- [ ] BrowserStack CI operational
- [ ] Schema FROZEN documented
- [ ] All tests pass
- [ ] HOSTILE_REVIEWER final approval
- [ ] GATE_W21_COMPLETE.md created

---

## Dependency Graph

```
W21.1 (Core Types) ─────────────────────┐
    │                                    │
    │ [BLOCKS]                           │
    ▼                                    │
W21.2 (Implementation) ─────────────────┤
    │                                    │
    │ [BLOCKS]                           │
    ▼                                    │
W21.3 (WASM Bindings) ─────────────────┤
    │                                    │
    │ [ENABLES parallel work]            │
    ├───────────────────────────┐        │
    ▼                           ▼        │
W21.4 (Mobile Testing)    (Docs)         │
    │                           │        │
    │ [BLOCKS]                  │        │
    ▼                           ▼        │
W21.5 (BrowserStack + Freeze) ◄─────────┘
```

**Critical Path:** W21.1 → W21.2 → W21.3 → W21.5
**Parallel Track:** W21.4 can start after W21.3 completes

---

## Constraints

- **Time:** Maximum 8 hours per day, 40 hours total
- **Technical:** MSRV 1.70, no new dependencies without approval
- **Quality:** All deliverables must pass `/review`
- **Process:** Days 1-3 are sequential (API builds on types)
- **Schema:** FROZEN after Day 5 (no changes without major version)
- **WASM:** Bundle must stay < 500KB gzipped

---

## Handoff Protocol

After each day:
1. Complete all deliverables listed in DAY_X_TASKS.md
2. Run `/review [artifact]` for each deliverable
3. Update this document with completion status
4. **ONLY proceed to next day after review approval**
5. If blocked, document blocker and apply risk mitigation

---

## Post-Week 21 Handoff

**For Week 22 PLANNER:**

```markdown
Prerequisite: GATE_W21_COMPLETE.md must exist

Week 22 Theme: FILTERING_ARCHITECTURE (Design Sprint)

Mandatory Scope:
1. Run /architect-design filtering_api (16h)
2. Define query syntax (EBNF grammar)
3. Design filter evaluator
4. Decide pre-filter vs post-filter strategy
5. Validate performance budget (<10ms with filtering)
6. Specify WASM boundary
7. Define test strategy

Critical Output:
- docs/architecture/FILTERING_API.md (HOSTILE approved)
- NO implementation code (design only)
```

---

**PLANNER:** Week 21 Planning Complete (Revision 1.0)
**Status:** PROPOSED
**Next:** `/review docs/planning/weeks/week_21/WEEKLY_TASK_PLAN.md`

---

*"Metadata first. Filtering second. Quality always."*
