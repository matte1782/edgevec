# Week 10 Risk Register

**Version:** v3.0 [APPROVED]
**Date:** 2025-12-13
**Total Risks:** 7
**Status:** All risks have mitigation strategies

---

## Risk Summary

| ID | Risk | Probability | Impact | Score | Mitigation Status |
|:---|:-----|:------------|:-------|:------|:------------------|
| R10.1 | Fuzz refactor exceeds estimate | MEDIUM (40%) | HIGH | MEDIUM-HIGH | Mitigation defined |
| R10.2 | Corpus generation complexity | LOW (25%) | MEDIUM | LOW-MEDIUM | Mitigation defined |
| R10.3 | Property tests reveal bugs | MEDIUM (50%) | HIGH | HIGH | Mitigation defined |
| R10.4 | Property tests are flaky | LOW (20%) | MEDIUM | LOW | Mitigation defined |
| R10.5 | CI integration failures | MEDIUM (35%) | MEDIUM | MEDIUM | Mitigation defined |
| R10.6 | Batch implementation complexity | MEDIUM (40%) | HIGH | MEDIUM-HIGH | Mitigation defined |
| R10.8 | Benchmark suite bugs | LOW (20%) | MEDIUM | LOW | Mitigation defined |

---

## Risk Details

### R10.1: Fuzz Refactor Takes Longer Than Expected

**Category:** Schedule / Technical
**Probability:** MEDIUM (40%)
**Impact:** HIGH (blocks W10.2-W10.4)
**Risk Score:** MEDIUM-HIGH

**Description:**
Restructuring fuzz targets (W10.1) may uncover unexpected issues:
- Cargo.toml path updates break other dependencies
- Corpus seed file generation is more complex than anticipated
- Migration breaks existing fuzz infrastructure

**Triggers:**
- Time spent on W10.1 exceeds 20h (with 3x multiplier)
- Compilation errors persist after restructuring
- Corpus generation requires custom tooling

**Impact if Realized:**
- W10.2a-W10.2d delayed by cascade effect
- Critical path extends from 60h to 72h+
- Possible deferral of W10.4 (property tests)

**Mitigation (REVISED v3.0 — Made Proactive):**
- **Time-box W10.1 to 20h absolute maximum** (not 18h)
- **If >20h:** Implement minimal restructure (top-level fuzz/ directory only, defer corpus/ subdirectories)
- **Start W10.5 in parallel** regardless of W10.1 status (W10.5 has no dependencies)
- **Fallback plan:** Keep flat structure, add corpus/ directory to each target without moving files

**Monitoring:**
- Track time spent on W10.1 daily
- Set alarm at 15h mark to evaluate progress
- Decision point at 20h: continue or fallback

**Probability Justification:** 40% because fuzz infrastructure is well-documented, but we haven't attempted this specific restructure before.

---

### R10.2: Corpus Generation Complexity

**Category:** Technical
**Probability:** LOW (25%)
**Impact:** MEDIUM (delays W10.3)
**Risk Score:** LOW-MEDIUM

**Description:**
Generating diverse, high-quality corpus seed files for fuzz targets may be more difficult than expected:
- Need domain expertise to create meaningful seeds
- Automated generation produces low-coverage inputs
- Manual seed creation is time-consuming

**Triggers:**
- Automated corpus generation fails to achieve sufficient coverage
- Manual seed creation exceeds 6h
- Corpus seeds don't trigger interesting code paths

**Impact if Realized:**
- W10.3 delayed by 6-12h
- Fuzzing effectiveness reduced (low code coverage)
- May need to defer comprehensive corpus to Week 11

**Mitigation (REVISED v2.0 — Quantified):**
- **Accept partial corpus if generation is complex**
- **Minimum acceptable coverage:** 80% line coverage across hamming, encoder, quantizer targets
- **Time-box corpus generation:** Max 8h total
- **Fallback:** Use minimal seeds (1-2 per target) and rely on fuzzer's mutation engine

**Monitoring:**
- Measure line coverage with `cargo tarpaulin` after corpus is added
- Verify fuzzer finds crashes/hangs within first 60s of runtime
- Review seed diversity (different vector dimensions, edge cases)

---

### R10.3: Property Tests Reveal Bugs

**Category:** Quality / Schedule
**Probability:** MEDIUM (50%)
**Impact:** HIGH (blocks merge)
**Risk Score:** HIGH

**Description:**
Property tests (W10.4) may reveal bugs in HNSW implementation that were not caught by unit tests:
- Connectivity property fails (orphaned vectors)
- Level distribution is biased
- Neighbor relationships violate distance constraints

**Triggers:**
- Any property test fails with reproducible counterexample
- Bug is in core HNSW algorithm (not test code)
- Fix requires algorithm redesign

**Impact if Realized:**
- W10.4 cannot be marked "done" until bugs are fixed
- May require emergency bug fix task (8-16h)
- Week 10 timeline slips by 1-2 days

**Mitigation:**
- **Expectation:** Property tests WILL find bugs (this is the goal)
- **Budget 12h for bug fixes** (not in current plan, but accepted as "expected cost of quality")
- **Fallback:** If bug is complex (>12h fix), disable that specific property test and create W11.X task
- **Document all counterexamples** in `docs/bugs/` for future reference

**Monitoring:**
- Run property tests frequently during W10.4 development
- Isolate each property test (run independently) to identify which invariant is broken
- Use `cargo test --test hnsw_properties -- --nocapture` for detailed failure logs

---

### R10.4: Property Tests Are Flaky

**Category:** Quality / CI
**Probability:** LOW (20%)
**Impact:** MEDIUM (blocks CI)
**Risk Score:** LOW

**Description:**
Property tests may be non-deterministic due to:
- Random seed variation
- Timing-dependent behavior
- Floating-point precision differences across platforms

**Triggers:**
- Property tests pass locally but fail in CI
- Tests fail intermittently (<100% reproducibility)
- Different results on Linux vs. Windows

**Impact if Realized:**
- Cannot merge PR if CI is red
- Developer time wasted debugging flaky tests
- May need to disable property tests temporarily

**Mitigation:**
- **Use fixed seed for proptest:** `ProptestConfig::with_seed(0x1234)`
- **Increase test cases:** Run 10,000 cases in CI (vs. 1,000 locally) to surface flakiness
- **Deterministic HNSW:** Ensure RNG is seeded deterministically in test code
- **Platform-specific tolerances:** Allow for floating-point epsilon differences

**Monitoring:**
- Run CI 5 times on same commit to detect flakiness
- Check test failure logs for seed values
- Document any platform-specific differences

---

### R10.5: CI Integration Failures

**Category:** Infrastructure
**Probability:** MEDIUM (35%)
**Impact:** MEDIUM (delays merge)
**Risk Score:** MEDIUM

**Description:**
Integrating fuzz tests and benchmarks into GitHub Actions CI may fail due to:
- Insufficient memory/CPU resources for fuzzing
- Timeout limits (fuzz runs take >10 minutes)
- Dependencies not available in CI environment

**Triggers:**
- CI job times out (>30min)
- Out-of-memory errors
- `cargo +nightly` not available in workflow

**Impact if Realized:**
- Cannot enable CI gating for fuzz/benchmarks
- Must run fuzz tests manually (reduces safety)
- Week 10 deliverables cannot be verified automatically

**Mitigation:**
- **Reduce fuzz time in CI:** Run for 30s in CI, 300s locally
- **Use `cargo check` instead of `cargo fuzz build`** if full fuzz build fails
- **Add resource limits:** Use GitHub Actions `timeout-minutes: 15` per job
- **Fallback:** Document manual fuzz verification process if CI is not feasible

**Monitoring:**
- Test CI workflow on feature branch before merging
- Monitor CI job duration and memory usage
- Review GitHub Actions documentation for resource limits

---

### R10.6: Batch Insert Implementation Exceeds 16h Estimate

**Category:** Schedule / Technical
**Probability:** MEDIUM (40%)
**Impact:** HIGH (Week 11 timeline)
**Risk Score:** MEDIUM-HIGH

**Description:**
During W10.5 design phase, we may discover that batch insert implementation (deferred to W11.1) is more complex than the current 16h estimate:
- Error recovery requires transaction rollback logic (complex)
- WASM memory constraints require sophisticated chunking
- Progress reporting adds significant overhead
- Edge cases (e.g., duplicate vectors, memory exhaustion) multiply implementation paths

**Triggers:**
- W10.5 RFC reveals >4 distinct error scenarios
- Memory budget analysis shows batches must be <1000 vectors (smaller than expected)
- API design requires >3 trait methods (indicates complexity)

**Impact if Realized:**
- W11.1 exceeds 16h raw estimate (violates planning standard)
- Week 11 timeline slips
- May cascade into Week 12

**Mitigation (REVISED v3.0 — Fixed N2 Logical Impossibility):**
- **Decomposition during W11.1 kickoff** (before implementation starts)
- **Time-box W11.1 first 2h to complexity analysis:**
  - Review W10.5 RFC for hidden complexity signals
  - Identify subtasks (e.g., "implement error recovery", "implement progress reporting")
  - If analysis reveals >16h implementation: split into W11.1a/b/c
- **Before W11.1 starts:** Review W10.5 API design for batch insert hints
- **Fallback:** If complexity is extreme (>24h even after decomposition), defer advanced features (progress reporting, partial success) to Week 12

**Monitoring:**
- During W10.5: Track number of error cases, trait methods, and edge cases documented
- **Complexity threshold:** If RFC has >5 error scenarios OR >4 trait methods, flag for decomposition
- At end of Week 10: Refine W11.1 estimate based on W10.5 findings

**Probability Justification:** 40% because batch operations often have hidden complexity (atomicity, error handling, progress reporting).

---

### R10.8: Benchmark Suite Implementation Bugs

**Category:** Technical / Quality
**Probability:** LOW (20%)
**Impact:** MEDIUM (false positives delay release)
**Risk Score:** LOW

**Description:**
W10.8 benchmark validation suite may have bugs that:
- Report false performance regressions (false positives)
- Miss actual regressions (false negatives)
- Have flawed baseline comparison methodology
- Produce non-reproducible results (variance too high)

**Triggers:**
- Benchmark reports regression when no code changed
- Known performance degradation is not detected
- Baseline values vary by >20% across runs

**Impact if Realized:**
- Cannot trust benchmark CI results
- Developers ignore benchmark failures (boy-who-cried-wolf)
- Actual performance regressions slip through
- Time wasted debugging false alarms

**Mitigation:**
- **Peer review of benchmark code** before execution (have second engineer review methodology)
- **Manual spot-check of 3 regression claims** before accepting as real
- **Run benchmarks 3 times, use median values** (not single run)
- **Document measurement methodology** in code comments (explain what is being measured)
- **Establish baseline with statistical rigor:** Run 10 times, compute mean ± stddev
- **Set regression threshold with margin:** Only fail if >15% slower (not 10%) to account for variance

**Monitoring:**
- Track false positive rate: How many CI failures are later found to be false alarms?
- Review baseline values periodically (monthly) to ensure they're still valid
- Cross-validate benchmarks against external tools (e.g., `perf` on Linux)

**Probability Justification:** 20% because benchmarking is hard to get right, but we have prior experience with `criterion`.

---

## Worst-Case Scenario Analysis

**Scenario:** R10.1 (fuzz refactor overruns) + R10.2 (corpus complexity) both trigger

**Impact Calculation:**
- **R10.1 worst-case:** W10.1 takes 24h (not 18h) → +6h slip
- **R10.2 worst-case:** Corpus generation takes 12h (not 6h, part of W10.3) → +6h slip
- **Cascade effect:** W10.3 delayed by 12h total (starts late AND takes longer)
- **W10.4 impact:** Delayed start, but no additional time added
- **Total slip:** +12h on critical path

**Revised Critical Path:**
- **Original:** 60h raw → 180h with 3x
- **Worst-case:** 72h raw → 216h with 3x
- **Buffer remaining:** 228h - 216h = 12h (5% margin)

**Acceptable:** YES, still within week capacity if we allow for 228h total budget.

---

## Mitigation Summary

| Risk | Mitigation Type | Effectiveness |
|:-----|:----------------|:--------------|
| R10.1 | Time-boxing + Parallel W10.5 | HIGH (prevents cascade) |
| R10.2 | Quantified partial success | MEDIUM (accepts compromise) |
| R10.3 | Budgeted bug fixes + Fallback | HIGH (expects failures) |
| R10.4 | Fixed seed + Determinism | HIGH (prevents flakiness) |
| R10.5 | Resource limits + Fallback | MEDIUM (manual verification option) |
| R10.6 | Decomposition during W11.1 kickoff | HIGH (proactive splitting) |
| R10.8 | Peer review + Statistical rigor | HIGH (validates methodology) |

---

## Monitoring Schedule

### Daily (During Week 10 Execution)

- Check time spent on W10.1 (alarm at 15h)
- Review property test failures
- Track CI job durations

### End of Week 10

- Review all 7 risks: Which triggered? Which didn't?
- Update probability estimates based on actual outcomes
- Document lessons learned for future weeks

---

## Revision History

### v1.0 (2025-12-13)
- Initial 5 risks identified
- HOSTILE_REVIEWER identified missing R10.6

### v2.0 (2025-12-13)
- Added R10.6 (batch implementation complexity)
- Quantified R10.2 mitigation (80% coverage)

### v3.0 (2025-12-13)
- **Fixed N2 (R10.6 mitigation logical impossibility):** Moved decomposition from "during W10.5 design phase" to "during W11.1 kickoff (before implementation starts)"
- **Fixed N3 (missing W10.8 risk):** Added R10.8 (benchmark suite bugs) with full risk structure
- **Fixed N1 (worst-case scenario):** Updated critical path from 72h to 60h baseline

**Status:** ✅ APPROVED by HOSTILE_REVIEWER

---

**Version:** v3.0
**Last Updated:** 2025-12-13
**Next Review:** End of Week 10
