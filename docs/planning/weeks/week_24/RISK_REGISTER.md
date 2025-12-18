# Week 24 Risk Register

**Sprint:** v0.5.0 Production Launch
**Risk Assessment Date:** TBD

---

## Risk Summary

| ID | Risk | Probability | Impact | Mitigation | Owner |
|:---|:-----|:------------|:-------|:-----------|:------|
| R1 | Fuzz testing finds crashes | Low | Critical | Fix before publish | TEST_ENGINEER |
| R2 | Competitive benchmark shows regression | Medium | High | Profile and document | BENCHMARK_SCIENTIST |
| R3 | Demo accessibility failures | Low | Medium | Day 6 audit buffer | DOCWRITER |
| R4 | npm publish fails (auth/network) | Low | Medium | Human backup | RUST_ENGINEER |
| R5 | hnswlib-wasm unavailable | Medium | Low | Skip with documentation | BENCHMARK_SCIENTIST |
| R6 | Mobile responsiveness issues | Medium | Medium | Day 5 dedicated pass | WASM_SPECIALIST |
| R7 | Marketing claims disputed | Low | High | All claims verifiable | HOSTILE_REVIEWER |

---

## Detailed Risk Analysis

### R1: Fuzz Testing Finds Crashes

**Description:** Fuzz campaigns discover crashes in filter parser or evaluator.

**Probability:** Low (already 2,395 tests passing)

**Impact:** Critical — Cannot release with known crashes

**Indicators:**
- Crash files in `fuzz/artifacts/`
- Non-zero exit code from fuzz process

**Mitigation:**
1. Start fuzz campaigns Day 1 (48h runway)
2. Monitor periodically for early crashes
3. If crash found: stop, fix, restart campaign
4. Buffer day (Day 7) allows fix time

**Contingency:** Delay release if crash found Day 6+

---

### R2: Competitive Benchmark Shows Regression

**Description:** EdgeVec slower than competitors on some metric.

**Probability:** Medium (new filter code adds overhead)

**Impact:** High — Marketing claims compromised

**Indicators:**
- EdgeVec >20% slower on pure search
- Memory usage significantly higher

**Mitigation:**
1. Accept honest results (document, don't hide)
2. Profile and optimize if significant
3. Emphasize filter advantage (competitors can't match)
4. Document tradeoffs transparently

**Contingency:** Reframe positioning if needed

---

### R3: Demo Accessibility Failures

**Description:** Demos fail WCAG 2.1 AA audit.

**Probability:** Low (following established patterns)

**Impact:** Medium — Professional image affected

**Indicators:**
- Color contrast failures
- Keyboard navigation broken
- Screen reader issues

**Mitigation:**
1. Day 6 dedicated audit time
2. Use existing color palette (tested)
3. Test with keyboard throughout development
4. Lighthouse accessibility score target: 90+

**Contingency:** Fix high-priority issues, document known limitations

---

### R4: npm Publish Fails

**Description:** npm publish blocked by auth, network, or package issues.

**Probability:** Low

**Impact:** Medium — Delays release

**Indicators:**
- npm login fails
- OTP issues (2FA)
- Package validation errors

**Mitigation:**
1. Verify npm auth Day 1
2. `npm pack --dry-run` before publish
3. Human handles OTP entry
4. Stable network connection

**Contingency:** Retry with different network, contact npm support

---

### R5: hnswlib-wasm Unavailable

**Description:** hnswlib-wasm npm package unmaintained or broken.

**Probability:** Medium (older package)

**Impact:** Low — Benchmark less comprehensive

**Indicators:**
- npm install fails
- API breaking changes
- No recent updates

**Mitigation:**
1. Check package status Day 2 start
2. If unavailable, document and skip
3. Focus on voy comparison instead
4. Feature matrix still valuable

**Contingency:** Compare feature matrix only, note in report

---

### R6: Mobile Responsiveness Issues

**Description:** Demos unusable on mobile devices.

**Probability:** Medium (complex UI)

**Impact:** Medium — Limits demo reach

**Indicators:**
- Horizontal scroll on mobile
- Touch targets too small
- Text unreadable

**Mitigation:**
1. Mobile-first CSS approach
2. Day 5 dedicated mobile pass
3. Test on real devices (not just emulator)
4. Minimum width: 375px (iPhone SE)

**Contingency:** Accept graceful degradation on smallest screens

---

### R7: Marketing Claims Disputed

**Description:** Competitor or user challenges claims in README.

**Probability:** Low (following honesty principles)

**Impact:** High — Reputation damage

**Indicators:**
- GitHub issues challenging claims
- Social media criticism
- Competitor counter-claims

**Mitigation:**
1. Every claim has linked proof
2. Methodology fully disclosed
3. Limitations documented
4. Hostile reviewer validates all claims

**Contingency:** Update claims, acknowledge errors publicly

---

## Risk Response Matrix

| Risk Level | Response |
|:-----------|:---------|
| Critical | Stop sprint, address immediately |
| High | Allocate dedicated time, may delay |
| Medium | Address within sprint, document if deferred |
| Low | Monitor, address if time permits |

---

## Monitoring Schedule

| Day | Risks to Monitor |
|:----|:-----------------|
| 1 | R4 (npm auth), R1 (fuzz start) |
| 2 | R1 (fuzz progress), R2 (benchmark results), R5 (hnswlib) |
| 3 | R1 (fuzz midpoint) |
| 4 | R6 (early mobile check) |
| 5 | R6 (mobile pass), R1 (fuzz final) |
| 6 | R3 (accessibility audit), R7 (claim review) |
| 7 | R4 (publish), R7 (final review) |

---

## Escalation Path

1. **Day-level:** Task owner resolves
2. **Sprint-level:** Hostile reviewer consulted
3. **Critical:** Human decision on delay/proceed

---

## Lessons from Previous Weeks

**v0.4.0 Issue (snippets missing):**
- Root cause: `files` array incomplete in package.json
- Prevention: `npm pack --dry-run` before publish
- Status: Fixed in v0.4.1

**Week 23 Day 6 (test count discrepancy):**
- Root cause: Counting methodology (passed vs passed+ignored)
- Prevention: Document counting methodology
- Status: Resolved

---

## Risk Register Status

**Last Updated:** TBD
**Next Review:** Day 3 midpoint
**Owner:** HOSTILE_REVIEWER
