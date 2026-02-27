# EdgeVec v0.4.0 Release Checklist

**Target Release Date:** 2025-12-20
**Current Version:** v0.3.0
**Target Version:** v0.4.0
**Release Manager:** [Your Name]

---

## Pre-Release Verification

### 1. Code Quality (8 items)

- [ ] All unit tests pass: `cargo test --lib`
- [ ] All integration tests pass: `cargo test --test '*'`
- [ ] All doc tests pass: `cargo test --doc`
- [ ] Chaos tests pass: `cargo test --test chaos_hnsw`
- [ ] Quick sanity load test passes: `cargo test --test load_test load_quick_sanity`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] MSRV verified: `cargo +1.70 build`

### 2. WASM Build (6 items)

- [ ] WASM builds successfully: `wasm-pack build --release --target web`
- [ ] Bundle size < 500KB gzipped: Check `pkg/edgevec_bg.wasm`
- [ ] TypeScript definitions valid: Check `pkg/edgevec.d.ts`
- [ ] Browser demo works: `wasm/examples/soft_delete.html`
- [ ] Benchmark dashboard works: `wasm/examples/benchmark-dashboard.html`
- [ ] npm package.json version updated to "0.4.0"

### 3. Benchmarks (5 items)

- [ ] Search latency < 1ms at 100k vectors (768d, SQ8)
- [ ] Insert latency < 2ms per vector (SQ8)
- [ ] P99 latency benchmark runs: `cargo bench --bench p99_bench`
- [ ] No performance regression from v0.3.0 (within 10%)
- [ ] Benchmark baselines documented

### 4. Documentation (8 items)

- [ ] README.md up to date with v0.4.0 features
- [ ] CHANGELOG.md complete with v0.4.0 section
- [ ] docs/TUTORIAL.md tested end-to-end
- [ ] docs/PERFORMANCE_TUNING.md complete
- [ ] docs/TROUBLESHOOTING.md covers top 10 errors
- [ ] docs/INTEGRATION_GUIDE.md examples verified
- [ ] docs/API_REFERENCE.md current
- [ ] All documentation links working

### 5. Legal (4 items)

- [ ] LICENSE-MIT present and correct
- [ ] LICENSE-APACHE present and correct
- [ ] Cargo.toml has `license = "MIT OR Apache-2.0"`
- [ ] Third-party attributions reviewed

---

## Release Process

### Step 1: Version Bump (4 items)

- [ ] Update `Cargo.toml` version to "0.4.0"
- [ ] Update `pkg/package.json` version to "0.4.0"
- [ ] Update README.md version references
- [ ] Ensure CHANGELOG.md has v0.4.0 section at top

### Step 2: Final Testing (6 items)

- [ ] Full test suite: `cargo test --all --release`
- [ ] Property tests: `cargo test --test proptest_hnsw_delete --release`
- [ ] Load tests (optional): `cargo test --release --test load_test -- --ignored`
- [ ] WASM tests: Manual browser testing in Chrome/Firefox/Safari
- [ ] CI pipeline passes on main branch
- [ ] Regression workflow passes

### Step 3: Build Artifacts (5 items)

- [ ] Native build: `cargo build --release`
- [ ] WASM build: `wasm-pack build --release --target web`
- [ ] Verify package contents: `cargo package --list`
- [ ] npm pack: `cd pkg && npm pack` (inspect tarball)
- [ ] Verify no sensitive files included

### Step 4: Git Tagging (3 items)

- [ ] Create annotated tag: `git tag -a v0.4.0 -m "Release v0.4.0"`
- [ ] Push tag: `git push origin v0.4.0`
- [ ] Verify tag on GitHub

### Step 5: Publish (4 items)

- [ ] Publish to crates.io: `cargo publish`
- [ ] Publish to npm: `cd pkg && npm publish`
- [ ] Create GitHub release with changelog
- [ ] Attach WASM bundle to GitHub release

### Step 6: Post-Release Verification (4 items)

- [ ] Verify crates.io page: https://crates.io/crates/edgevec
- [ ] Verify npm package: https://www.npmjs.com/package/edgevec
- [ ] Test install: `cargo add edgevec@0.4.0`
- [ ] Test npm install: `npm install edgevec@0.4.0`

---

## Communication

### Announcements (Optional)

- [ ] Update project documentation site (if any)
- [ ] Post release notes to discussions/issues
- [ ] Social media announcement (optional)

---

## Rollback Plan

If critical issues discovered post-release:

### Severity Levels

| Severity | Action | Timeline |
|:---------|:-------|:---------|
| **Minor** | Patch release (v0.4.1) | Next business day |
| **Major** | Yank + patch release | Same day |
| **Critical/Security** | Immediate yank + advisory | Immediate |

### Rollback Steps

1. **Identify Issue**
   - Confirm severity
   - Document reproduction steps
   - Identify affected versions

2. **Yank if Critical**
   ```bash
   cargo yank edgevec --version 0.4.0
   npm deprecate edgevec@0.4.0 "Critical bug, use 0.4.1"
   ```

3. **Prepare Fix**
   - Create hotfix branch
   - Implement fix with tests
   - Fast-track review

4. **Publish Patch**
   - Bump to v0.4.1
   - Publish with fix
   - Update release notes

---

## Quality Gate Sign-Off

### Pre-Merge (HOSTILE_REVIEWER)

| Check | Status | Notes |
|:------|:-------|:------|
| Code quality | [ ] | All tests pass |
| Documentation | [ ] | Complete and accurate |
| Performance | [ ] | No regressions |
| WASM build | [ ] | Bundle size OK |
| Legal | [ ] | Licenses correct |

### Release Approval

| Role | Name | Date | Signature |
|:-----|:-----|:-----|:----------|
| Developer | | | |
| HOSTILE_REVIEWER | | | |
| Release Manager | | | |

---

## Checklist Summary

| Category | Items | Status |
|:---------|:------|:-------|
| Code Quality | 8 | [ ] |
| WASM Build | 6 | [ ] |
| Benchmarks | 5 | [ ] |
| Documentation | 8 | [ ] |
| Legal | 4 | [ ] |
| Version Bump | 4 | [ ] |
| Final Testing | 6 | [ ] |
| Build Artifacts | 5 | [ ] |
| Git Tagging | 3 | [ ] |
| Publish | 4 | [ ] |
| Post-Release | 4 | [ ] |
| **Total** | **57** | |

---

## Notes

- This checklist follows the EdgeVec military-grade development protocol
- All items must be checked before proceeding to next phase
- HOSTILE_REVIEWER has final approval authority
- See `docs/ROLLBACK_PROCEDURES.md` for detailed rollback instructions

---

**Checklist Version:** 1.0.0
**Last Updated:** 2025-12-16
