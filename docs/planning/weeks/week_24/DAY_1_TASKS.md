# Week 24 Day 1: Release Foundation

**Date:** TBD
**Focus:** Commit v0.5.0 and start validation campaigns
**Estimated Duration:** 4 hours active + background fuzz runs

---

## Tasks

### W24.1.1: Commit All Week 23 Changes

**Objective:** Create a clean git commit with all Week 23 Filter API work.

**Acceptance Criteria:**
- [ ] All modified files staged (src/filter/*, tests/*, pkg/*)
- [ ] Commit message follows conventional format
- [ ] No untracked files left behind
- [ ] `cargo test` passes post-commit

**Deliverables:**
- Git commit with Week 23 changes

**Dependencies:** None

**Estimated Duration:** 30 minutes

**Agent:** RUST_ENGINEER

**Commit Message Template:**
```
feat(filter): Add complete Filter API for v0.5.0

- Filter parser with pest grammar (15 operators)
- Filter evaluator with metadata matching
- Strategy selector (prefilter/postfilter/hybrid)
- WASM bindings for filtered search
- 2,395 tests passing

Closes: Week 23 sprint
```

---

### W24.1.2: Create v0.5.0 Git Tag

**Objective:** Create annotated git tag for v0.5.0 release.

**Acceptance Criteria:**
- [ ] Tag `v0.5.0` created with annotation
- [ ] Tag message includes release summary
- [ ] Tag pushed to remote (after review)

**Deliverables:**
- Git tag `v0.5.0`

**Dependencies:** W24.1.1

**Estimated Duration:** 15 minutes

**Agent:** RUST_ENGINEER

**Tag Command:**
```bash
git tag -a v0.5.0 -m "v0.5.0: Filter API Release

Features:
- Metadata filtering with 15 operators
- AND/OR/NOT boolean logic
- Strategy selection (prefilter/postfilter/hybrid)
- WASM bindings for filtered search

Performance:
- Search P50: ~145µs (7x under target)
- Search P99: ~350µs
- WASM bundle: 206KB gzipped (58% under target)

Tests: 2,395 passing"
```

---

### W24.1.3: Start Fuzz Campaign - filter_simple

**Objective:** Run filter_simple fuzz target for 24+ hours.

**Acceptance Criteria:**
- [ ] Fuzz target compiles successfully
- [ ] Fuzz campaign started in background
- [ ] Logging enabled for crash detection
- [ ] Campaign duration: minimum 24 hours

**Deliverables:**
- Running fuzz campaign
- Log file: `fuzz/artifacts/filter_simple_run_w24.log`

**Dependencies:** W24.1.1

**Estimated Duration:** 30 minutes setup + 24h background

**Agent:** TEST_ENGINEER

**Commands:**
```bash
cd fuzz
cargo +nightly fuzz run filter_simple -- -max_total_time=86400 2>&1 | tee artifacts/filter_simple_run_w24.log &
```

---

### W24.1.4: Start Fuzz Campaign - filter_deep

**Objective:** Run filter_deep fuzz target for 24+ hours.

**Acceptance Criteria:**
- [ ] Fuzz target compiles successfully
- [ ] Fuzz campaign started in background
- [ ] Logging enabled for crash detection
- [ ] Campaign duration: minimum 24 hours

**Deliverables:**
- Running fuzz campaign
- Log file: `fuzz/artifacts/filter_deep_run_w24.log`

**Dependencies:** W24.1.1

**Estimated Duration:** 30 minutes setup + 24h background

**Agent:** TEST_ENGINEER

**Commands:**
```bash
cd fuzz
cargo +nightly fuzz run filter_deep -- -max_total_time=86400 2>&1 | tee artifacts/filter_deep_run_w24.log &
```

---

## Day 1 Checklist

- [ ] W24.1.1: Week 23 changes committed
- [ ] W24.1.2: v0.5.0 tag created
- [ ] W24.1.3: filter_simple fuzz running
- [ ] W24.1.4: filter_deep fuzz running

## Day 1 Exit Criteria

- All Week 23 code in git history
- v0.5.0 tag exists locally
- Two fuzz campaigns running in background
- No blocking issues discovered

## Notes

- Do NOT push tag to remote until Day 7 (after hostile review)
- Monitor fuzz campaigns periodically for early crashes
- If crashes found, stop and fix before Day 2
