# Week 13 Risk Register

**Sprint:** Dec 16-22, 2025 (Week 13)
**Author:** PLANNER
**Status:** APPROVED
**Last Updated:** 2025-12-13

---

## Risk Summary

| ID | Risk | Probability | Impact | Score | Status |
|:---|:-----|:------------|:-------|:------|:-------|
| R13.1 | bytemuck incompatible with HnswNode | LOW | HIGH | 6 | OPEN |
| R13.2 | Competitor libraries fail in harness | MEDIUM | MEDIUM | 6 | OPEN |
| R13.3 | EdgeVec underperforms competitors | MEDIUM | MEDIUM | 6 | OPEN |
| R13.4 | Community finds additional UB | MEDIUM | HIGH | 9 | OPEN |
| R13.5 | Schedule slips | LOW | MEDIUM | 4 | OPEN |
| R13.6 | Miri still incompatible | HIGH | LOW | 4 | ACCEPTED |

**Scoring:** Probability (L=1, M=2, H=3) × Impact (L=1, M=2, H=3)

---

## Detailed Risk Analysis

### R13.1: bytemuck Incompatible with HnswNode

**Description:** HnswNode struct layout may not satisfy Pod trait requirements (e.g., implicit padding, non-primitive fields).

**Probability:** LOW
- HnswNode uses only primitive types (u32, u8, u16)
- `#[repr(C)]` already applied
- Similar structs work in other projects

**Impact:** HIGH
- Blocks entire safety hardening effort
- Would require struct redesign

**Mitigation:**
1. Add explicit `_padding` fields if needed
2. Property test verifies roundtrip before merge
3. Fallback: manual safe transmute implementation

**Detection:** Compilation failure on `#[derive(Pod, Zeroable)]`

**Owner:** RUST_ENGINEER

---

### R13.2: Competitor Libraries Fail in Benchmark Harness

**Description:** One or more competitor WASM libraries may fail to install, initialize, or run correctly.

**Probability:** MEDIUM
- WASM libraries often have complex build dependencies
- Version conflicts possible
- Some may be unmaintained

**Impact:** MEDIUM
- Incomplete competitive analysis
- Reduced credibility of benchmark

**Mitigation:**
1. Document any library failures
2. Continue with subset that works
3. Note limitations in report
4. Link to library issues if relevant

**Detection:** npm install failures, runtime errors

**Owner:** BENCHMARK_SCIENTIST

---

### R13.3: EdgeVec Underperforms Competitors

**Description:** EdgeVec may show worse latency, memory, or recall compared to established competitors.

**Probability:** MEDIUM
- hnswlib is mature C++ implementation
- usearch uses SIMD optimizations
- EdgeVec is newer

**Impact:** MEDIUM
- Marketing positioning challenge
- May need to emphasize other strengths

**Mitigation:**
1. Focus on unique strengths (safety, TypeScript types, memory efficiency)
2. Identify specific use cases where EdgeVec wins
3. Honest positioning: "Best for X, not for Y"
4. Use results to prioritize future optimizations

**Detection:** Benchmark results

**Owner:** BENCHMARK_SCIENTIST

---

### R13.4: Community Finds Additional UB Issues

**Description:** Community code review may identify additional undefined behavior beyond the known issue.

**Probability:** MEDIUM
- Codebase has multiple unsafe blocks (SIMD, persistence)
- Community is actively reviewing after Reddit post
- Complex WASM boundary code

**Impact:** HIGH
- Reputation damage
- Additional emergency fixes required
- Possible release delay

**Mitigation:**
1. Comprehensive audit in W13.1a/W13.1b
2. Document ALL unsafe blocks proactively
3. Classify each with clear SAFETY comments
4. Engage positively with community feedback

**Detection:** GitHub issues, Reddit comments, HN discussion

**Owner:** RUST_ENGINEER

---

### R13.5: Schedule Slips

**Description:** Week 13 tasks may take longer than estimated, impacting delivery.

**Probability:** LOW
- 3x multiplier applied to all estimates
- Weekend buffer available
- W13.4 (docs) can be deprioritized if needed

**Impact:** MEDIUM
- May delay GATE_13
- Could impact Week 14 planning

**Mitigation:**
1. Weekend buffer (Dec 21-22) for overflow
2. W13.4 is lowest priority (can slip to Week 14)
3. Daily progress validation
4. Early escalation if blocked

**Detection:** Daily task completion tracking

**Owner:** PLANNER

---

### R13.6: Miri Remains Incompatible

**Description:** Miri may continue to be incompatible with web-sys/WASM dependencies, preventing automated UB verification.

**Probability:** HIGH
- Already confirmed incompatible in Week 13 prep
- web-sys uses nightly features Miri doesn't support
- No known workaround

**Impact:** LOW (after mitigation)
- Cannot use Miri for automated verification
- Must rely on manual audit + bytemuck

**Mitigation:**
1. **ACCEPTED:** Manual audit replaces Miri
2. bytemuck provides compile-time + runtime verification
3. Property tests verify correctness
4. Document Miri limitation in audit report

**Detection:** N/A (already detected)

**Owner:** RUST_ENGINEER

**Status:** ACCEPTED - Miri verification replaced with bytemuck + manual audit

---

## Risk Response Matrix

| Risk | Primary Response | Trigger | Escalation |
|:-----|:-----------------|:--------|:-----------|
| R13.1 | Mitigate (explicit padding) | Compile failure | Redesign struct |
| R13.2 | Accept (document failures) | npm/runtime error | Skip library |
| R13.3 | Mitigate (honest positioning) | Benchmark results | Prioritize optimizations |
| R13.4 | Mitigate (proactive audit) | Community report | Emergency fix |
| R13.5 | Mitigate (buffer time) | Task overrun | Deprioritize W13.4 |
| R13.6 | Accept (manual audit) | N/A | N/A |

---

## Contingency Plans

### If bytemuck Integration Fails (R13.1 triggers)

1. Identify failing constraint (padding, field type, etc.)
2. Option A: Modify HnswNode to satisfy Pod
3. Option B: Implement `TryFrom<&[u8]>` with manual alignment check
4. Option C: Keep unsafe with enhanced SAFETY documentation
5. Decision requires HOSTILE_REVIEWER approval

### If Multiple Competitors Fail (R13.2 triggers)

1. Minimum viable: EdgeVec + 2 competitors
2. If <3 libraries work: Document why, cite library issues
3. If only EdgeVec works: Benchmark against itself (configurations)
4. Note: Any result is still valuable data

### If EdgeVec Significantly Underperforms (R13.3 triggers)

1. Verify measurement methodology
2. Check EdgeVec optimizations enabled (release build, SIMD)
3. If valid: Focus on memory efficiency / safety story
4. Create "optimization roadmap" for future improvements
5. Honest positioning in README

---

## Week 13 Risk Status

| Metric | Value |
|:-------|:------|
| Total Risks | 6 |
| Open | 5 |
| Accepted | 1 |
| Mitigated | 0 |
| Closed | 0 |
| Average Score | 5.8 |
| Max Score | 9 (R13.4) |

**Highest Risk:** R13.4 (Community finds additional UB) - Score 9
**Mitigation Focus:** Comprehensive proactive audit in W13.1a/W13.1b

---

## Approval

| Role | Name | Date | Signature |
|:-----|:-----|:-----|:----------|
| PLANNER | PLANNER | 2025-12-13 | ✓ |
| HOSTILE_REVIEWER | | | PENDING |

---

**Next Review:** End of Day 2 (after audits complete)
