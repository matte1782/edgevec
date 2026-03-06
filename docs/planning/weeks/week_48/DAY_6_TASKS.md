# Week 48 — Day 6 Tasks (Saturday/Monday, Apr 19 or Apr 21)

**Date:** 2026-04-19 (or 2026-04-21 if weekend gap)
**Focus:** Fix Hostile Review Findings + ROADMAP v7.3 + Commit + Gate
**Agents:** RUST_ENGINEER, PLANNER
**Status:** PENDING

---

## Day Objective

Resolve all hostile review findings from both rounds, update ROADMAP to v7.3 reflecting the growth pivot, commit all W48 work, and create the week gate file.

**Success Criteria:**
- All Critical + Major findings from both hostile reviews resolved
- ROADMAP.md updated to v7.3 with Growth phase
- All regression tests pass
- Single conventional commit: `feat(w48): MetadataBoost API + entity-RAG demo + blog post`
- `.claude/GATE_W48_COMPLETE.md` created

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `docs/reviews/2026-04-15_W48_MIDWEEK_REVIEW.md` — mid-week hostile review findings
- [ ] `docs/reviews/2026-04-18_W48_ENDWEEK_REVIEW.md` — end-of-week hostile review findings
- [ ] `docs/planning/ROADMAP.md` — current v7.2, needs update to v7.3
- [ ] `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` — W48 plan for completion reference

**Weekend gap recovery:** If starting on Monday after a weekend gap:
1. `git status` — check for uncommitted changes
2. `git log --oneline -5` — verify last commits
3. `cargo test --lib` — verify tests still pass
4. Re-read both hostile review documents completely

---

## Tasks

### W48.6a: Fix Hostile Review Findings (2h) — RUST_ENGINEER

**Dependency:** Both hostile reviews complete (Day 2 + Day 5)

**Process:**
1. Read mid-week review (`docs/reviews/2026-04-15_W48_MIDWEEK_REVIEW.md`)
2. Read end-of-week review (`docs/reviews/2026-04-18_W48_ENDWEEK_REVIEW.md`)
3. Fix ALL Critical issues first
4. Fix ALL Major issues
5. Fix minor issues if time permits (document as accepted-as-is otherwise)

**Commands after each fix:**
```bash
cargo test --lib
cargo clippy -- -D warnings
```

**Acceptance:**
- [ ] 0 Critical issues remaining
- [ ] 0 Major issues remaining
- [ ] Minor issues either fixed or documented as accepted-as-is with justification

---

### W48.6b: Update ROADMAP.md to v7.3 (1h) — PLANNER

**Dependency:** W48.6a complete

**Changes:**
1. Header: `v7.2` -> `v7.3`
2. Date: update to current date
3. Add Growth phase (new section or update existing milestone):

```markdown
## Phase 10.5: Growth — Entity-Enhanced RAG (Week 48)

### Strategic Context
Pivot from technical depth (PQ validation) to community growth.
Riding the GraphRAG/entity-enhanced RAG trend.
Baseline: 83 GitHub stars. Target: 10x visibility.

### Milestone 10.5.1: MetadataBoost API
**Status:** COMPLETE
**Definition of Done:** MetadataBoost struct + search_boosted() + WASM export + 11 tests
**Deliverables:**
- `src/filter/boost.rs` — MetadataBoost struct (< 200 lines)
- `search_boosted()` on FilteredSearcher
- `searchWithBoost()` WASM export

### Milestone 10.5.2: In-Browser Entity-RAG Demo
**Status:** COMPLETE
**Definition of Done:** Demo loads, search < 100ms, boost toggle works
**Deliverables:**
- `demo/entity-rag/index.html` — single-page demo
- `demo/entity-rag/data.json` — 1000 SQuAD paragraphs + 384D embeddings + NER metadata

### Milestone 10.5.3: Positioning Blog Post
**Status:** COMPLETE
**Definition of Done:** 1500-2000 words, hostile-reviewed, published
**Deliverables:**
- `docs/blog/entity-enhanced-rag.md`
```

4. Update v0.10.0 target to include MetadataBoost
5. Verify all week numbers and dates are consistent

**Commands:**
```bash
# Verify no broken references
grep -n "v7.2" docs/planning/ROADMAP.md    # Should be 0 after update
```

**Acceptance:**
- [ ] Header says `v7.3`
- [ ] Growth phase documented with 3 milestones
- [ ] All W48 actuals reflected
- [ ] Dates and week numbers consistent

---

### W48.6c: Full Regression (0.5h) — TEST_ENGINEER

**Dependency:** W48.6a complete

**Commands:**
```bash
cargo test --lib                                  # 1024+ tests
cargo clippy -- -D warnings                       # 0 warnings
cargo check --target wasm32-unknown-unknown        # WASM build
```

**Acceptance:**
- [ ] `cargo test --lib` — 1024+ passed (1013 existing + 11 new boost), 0 failed
- [ ] `cargo clippy -- -D warnings` — 0 warnings
- [ ] `cargo check --target wasm32-unknown-unknown` — success

---

### W48.6d: Commit All W48 Work (0.5h) — PLANNER

**Dependency:** W48.6c complete

**Commands:**
```bash
# Stage all W48 files
git add src/filter/boost.rs
git add src/filter/filtered_search.rs    # search_boosted() addition
git add src/filter/mod.rs                # boost module re-export
git add src/wasm/mod.rs                  # search_with_boost export
git add demo/entity-rag/prepare_data.py
git add demo/entity-rag/index.html
git add docs/blog/entity-enhanced-rag.md
git add docs/planning/ROADMAP.md
git add docs/planning/weeks/week_48/
git add docs/reviews/2026-04-15_W48_MIDWEEK_REVIEW.md
git add docs/reviews/2026-04-18_W48_ENDWEEK_REVIEW.md
git add README.md
git add CHANGELOG.md
git add .gitignore

# Verify data.json is gitignored (m7 fix)
grep "data.json" .gitignore || echo "ERROR: data.json not in .gitignore!"

# Verify nothing unexpected
git status
git diff --cached --stat

# Commit
git commit -m "feat(w48): MetadataBoost API + entity-RAG demo + blog post

- Add MetadataBoost struct for entity-enhanced search (src/filter/boost.rs)
- Add search_boosted() method to FilteredSearcher
- Add searchWithBoost() WASM export
- Add in-browser entity-RAG demo (demo/entity-rag/)
- Add positioning blog post (docs/blog/)
- Update README with MetadataBoost section
- Update ROADMAP to v7.3 (Growth phase)
- 11 new unit tests (test_boost_*)
- Two hostile reviews: both GO"
```

**Acceptance:**
- [ ] Commit message follows conventional commit format
- [ ] All W48 files included
- [ ] No untracked files left behind (except data.json which is gitignored)

---

### W48.6e: Create `.claude/GATE_W48_COMPLETE.md` (0.5h) — PLANNER

**Dependency:** W48.6d complete

**Content:**
```markdown
# GATE W48 COMPLETE

**Date:** 2026-04-19
**Sprint:** Week 48 — Growth Pivot (MetadataBoost + Demo + Blog)
**Status:** [COMPLETE]

## Hostile Review Verdicts

| Review | Date | Verdict | Findings |
|:-------|:-----|:--------|:---------|
| Mid-week (MetadataBoost API) | 2026-04-15 | [verdict] | [C/M/m counts] |
| End-of-week (Demo + Blog) | 2026-04-18 | [verdict] | [C/M/m counts] |

## Deliverables

| Deliverable | Status | Commit |
|:------------|:-------|:-------|
| src/filter/boost.rs | COMPLETE | [hash] |
| search_boosted() | COMPLETE | [hash] |
| WASM searchWithBoost() | COMPLETE | [hash] |
| demo/entity-rag/ | COMPLETE | [hash] |
| docs/blog/entity-enhanced-rag.md | COMPLETE | [hash] |
| README.md update | COMPLETE | [hash] |
| ROADMAP v7.3 | COMPLETE | [hash] |
| 11 boost tests | COMPLETE | [hash] |

## Regression Results

- cargo test --lib: [count] passed, 0 failed
- cargo clippy: 0 warnings
- WASM build: success
- WASM bundle: [size] KB

## Carry-Forward Items

| Item | Priority | Target |
|:-----|:---------|:-------|
| Publish blog to dev.to | MEDIUM | W49 |
| npm publish edgevec-langchain@0.2.0 | LOW | User handles OTP |
| [Any hostile review minors accepted-as-is] | LOW | Document |

## Accepted-As-Is

[List any minor findings not fixed, with justification]
```

**Acceptance:**
- [ ] Gate file created
- [ ] Both hostile review verdicts documented
- [ ] All deliverables listed with status
- [ ] Carry-forward items explicitly listed (no silent drops)

---

## Day 6 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~4.5h |
| Fixes | All Critical + Major from both reviews |
| ROADMAP version | v7.2 -> v7.3 |
| Commits | 1 conventional commit |
| Gate file | GATE_W48_COMPLETE.md |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.6a | 2h | | |
| W48.6b | 1h | | |
| W48.6c | 0.5h | | |
| W48.6d | 0.5h | | |
| W48.6e | 0.5h | | |
| **Total** | **4.5h** | | |

---

## Handoff to Week 49

**Codebase state at EOD:**
- All W48 work committed
- ROADMAP v7.3 reflects growth pivot
- Gate file created

**W49 candidates:**
1. Publish blog to dev.to (needs account + formatting)
2. Deploy demo to GitHub Pages (needs gh-pages branch or Actions)
3. Social media push (Twitter/X thread about entity-enhanced RAG)
4. Community engagement (respond to issues, Discord)
5. v0.10.0 release planning (finalize feature list)

---

**END OF DAY 6 TASKS**
