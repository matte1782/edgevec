# Week 19 Day 6: Release Blocker Resolution

**Date:** 2025-12-16
**Sprint:** Week 19 (Extended)
**Status:** IN_PROGRESS

---

## Context

W19.6 was added following HOSTILE_REVIEWER feedback on W19.5 release preparation.
The review identified 3 critical blockers that must be resolved before v0.4.0 release.

**Source:** `docs/reviews/2025-12-16_W19.5_release_prep.md`

---

## W19.6 Tasks

### Task W19.6.1: Version Bump (Critical C1)

**Description:** Update Cargo.toml version from 0.3.0 to 0.4.0

**File:** `Cargo.toml` line 15

**Change:**
```toml
# Before:
version = "0.3.0"

# After:
version = "0.4.0"
```

**Acceptance Criteria:**
- [ ] Cargo.toml shows version = "0.4.0"
- [ ] `cargo build` succeeds

---

### Task W19.6.2: Clippy Warnings (Critical C2)

**Description:** Fix 12 clippy warnings that block release

**Location:** `src/wasm/mod.rs`

**Warnings (12 total):**

1. **Cast truncation warnings (5x)** — Lines 852-856:
   - `result.deleted as u32`
   - `result.already_deleted as u32`
   - `result.invalid_ids as u32`
   - `result.total as u32`
   - `result.unique_count as u32`

   **Fix:** Add `#[allow(clippy::cast_possible_truncation)]` to function

2. **Missing #[must_use] warnings (7x)** — Lines 955, 961, 967, 973, 979, 987, 995:
   - `deleted()`
   - `already_deleted()`
   - `invalid_ids()`
   - `total()`
   - `unique_count()`
   - `all_valid()`
   - `any_deleted()`

   **Fix:** Add `#[must_use]` attribute to each getter method

**Acceptance Criteria:**
- [ ] `cargo clippy -- -D warnings` passes with zero warnings/errors

---

### Task W19.6.3: README Update (Critical C3)

**Description:** Update README.md version references from v0.3.0 to v0.4.0

**Locations:**
- Line 14: Section header "What's New in v0.3.0" → "What's New in v0.4.0"
- Line 349: npm package reference "edgevec@0.3.0" → "edgevec@0.4.0"
- Line 407: Bundle size reference "edgevec@0.3.0" → "edgevec@0.4.0"

**Acceptance Criteria:**
- [ ] No references to v0.3.0 remain for current version
- [ ] "What's New in v0.4.0" section reflects v0.4.0 features

---

### Task W19.6.4: Verification

**Description:** Run full test suite and verification

**Commands:**
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --lib
cargo test --test '*'
```

**Acceptance Criteria:**
- [ ] All formatting checks pass
- [ ] All clippy checks pass (zero warnings)
- [ ] All tests pass

---

### Task W19.6.5: Professional Announcement Post

**Description:** Create professional v0.4.0 release announcement for community engagement

**Output:** `docs/announcements/v0.4.0_release.md`

**Requirements:**
- Professional tone suitable for Reddit, Hacker News, Twitter
- Key features and improvements highlighted
- Performance metrics included
- Call to action (try it, contribute, feedback)

---

### Task W19.6.6: Hostile Review Iteration

**Description:** Final hostile review before commit/push

**Acceptance Criteria:**
- [ ] All critical issues resolved
- [ ] `cargo clippy -- -D warnings` passes
- [ ] README.md updated
- [ ] Cargo.toml version bumped
- [ ] HOSTILE_REVIEWER approval

---

## Exit Criteria

W19.6 is complete when:
1. All 3 critical issues (C1, C2, C3) are resolved
2. Full verification passes
3. Announcement post created
4. HOSTILE_REVIEWER grants GO for commit

---

## Dependencies

- W19.5 documentation artifacts (completed)
- HOSTILE_REVIEWER approval (pending)

---

**END OF W19.6 TASK PLAN**
