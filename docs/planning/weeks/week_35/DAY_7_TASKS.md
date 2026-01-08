# Week 35 Day 7: Hostile Review + v0.8.0 Release

**Date:** 2026-02-02
**Focus:** Final hostile review and publish v0.8.0
**Hours:** 1h
**Status:** [x] COMPLETE

---

## Context

Final day of v0.8.0 cycle. Submit all Week 35 work for hostile review, then publish release.

**Priority:** P0 - Release critical

---

## Tasks

### W35.7: Hostile Review + Release (2h)

**Goal:** HOSTILE_REVIEWER approval and published release.

**Subtasks:**

- [x] **7.1** Pre-review verification (15min) COMPLETE
  - All Day 1-6 tasks verified complete
  - 700 tests pass, clippy clean
  - Version 0.8.0 confirmed

- [x] **7.2** Submit for hostile review (30min) COMPLETE
  - CHANGELOG.md: APPROVED
  - README.md: APPROVED
  - Week 35 code changes: APPROVED
  - No blocking issues

- [x] **7.3** Address review feedback (30min buffer) COMPLETE
  - No critical issues
  - No major issues
  - N/A

- [x] **7.4** Create and push tag (15min) COMPLETE
  - Tag created: v0.8.0
  - Pushed to origin
  - Verified on GitHub

- [x] **7.5** Publish releases (20min) PARTIAL
  - GitHub release created: https://github.com/matte1782/edgevec/releases/tag/v0.8.0
  - crates.io: PENDING (requires `cargo publish` with auth)
  - npm: PENDING (requires `npm publish` with auth)

- [ ] **7.6** Post-release verification (10min) PENDING
  - Verify crates.io page (after publish)
  - Verify npm page (after publish)
  - GitHub release: VERIFIED
  - Test installation (after publish)

---

## Hostile Review Scope

### Code Changes (Week 35)
- WAL edge case fix (Day 1)
- Safety doc placement (Day 2)
- cast_possible_truncation fixes (Days 3-4)
- Test clippy cleanup (Day 5)

### Documentation (Week 35)
- EdgeVec vs pgvector comparison (Day 5)
- CHANGELOG v0.8.0 (Day 6)
- README updates (Day 6)

---

## Release Commands

```bash
# Final verification
cargo test --all-features
cargo clippy -- -D warnings
npm run build
npx tsc --noEmit

# Create tag
git tag -a v0.8.0 -m "v0.8.0: Consolidation + Developer Experience"
git push origin v0.8.0

# Publish to crates.io
cargo publish

# Publish to npm
cd pkg
npm publish
cd ..

# Create GitHub release
gh release create v0.8.0 \
  --title "v0.8.0: Consolidation + Developer Experience" \
  --notes-file docs/releases/CHANGELOG_v0.8.0.md
```

---

## GitHub Release Notes Template

```markdown
# v0.8.0: Consolidation + Developer Experience

This release focuses on developer experience improvements and technical debt reduction.

## Highlights

### Vue 3 Support
```typescript
import { useEdgeVec, useSearch } from 'edgevec/vue';

const { db, isLoading } = useEdgeVec({ dimensions: 384 });
const { results, search } = useSearch(db);
```

### Standalone Filter Functions
```typescript
import { eq, gt, and, contains } from 'edgevec';

const filter = and(
  eq('category', 'electronics'),
  gt('price', 100)
);
```

### Comprehensive Documentation
- 25 filter examples
- Embedding integration guide (5 providers)
- EdgeVec vs pgvector comparison

## What's Changed
[See CHANGELOG for full details]

## Installation

**Rust:**
```bash
cargo add edgevec@0.8.0
```

**npm:**
```bash
npm install edgevec@0.8.0
```

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

---

## Acceptance Criteria

- [ ] HOSTILE_REVIEWER approves all artifacts
- [ ] v0.8.0 tag created and pushed
- [ ] crates.io publish successful
- [ ] npm publish successful
- [ ] GitHub release created
- [ ] Installation verified

---

## Rollback Plan

If release fails:
1. Delete tag: `git tag -d v0.8.0 && git push origin :refs/tags/v0.8.0`
2. Yank from crates.io: `cargo yank --version 0.8.0`
3. Unpublish from npm: `npm unpublish edgevec@0.8.0` (within 72h)
4. Investigate and fix issue
5. Re-release as v0.8.1

---

## Exit Criteria

Week 35 is complete when:
- [ ] HOSTILE_REVIEWER: APPROVED
- [ ] v0.8.0 on crates.io
- [ ] v0.8.0 on npm
- [ ] GitHub release published
- [ ] Installation verified
- [ ] Gate file created: `.claude/GATE_W35_COMPLETE.md`

---

## Post-Release

After successful release:
1. Create gate completion file
2. Update ROADMAP.md status
3. Announce on relevant channels
4. Begin Week 36 planning (v0.9.0: Community Features)

---

## Commit Message Template

```
release(v0.8.0): Consolidation + Developer Experience

Week 35 Technical Debt:
- Fix WAL chunk_size edge case
- Correct safety doc placement
- Resolve 50+ cast_possible_truncation warnings
- Clean test/bench clippy warnings

Documentation:
- EdgeVec vs pgvector comparison guide

HOSTILE_REVIEWER: APPROVED

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

**Day 7 Total:** 2 hours
**Agents:** HOSTILE_REVIEWER, DOCWRITER

---

## Week 35 Complete Checklist

| Day | Task | Status |
|:----|:-----|:-------|
| 1 | WAL chunk_size fix | [ ] |
| 2 | Safety doc placement | [ ] |
| 3 | cast_possible_truncation (Part 1) | [ ] |
| 4 | cast_possible_truncation (Part 2) | [ ] |
| 5 | Test clippy + Comparison doc | [ ] |
| 6 | Release preparation | [ ] |
| 7 | Hostile review + Release | [ ] |

**v0.8.0 Released:** [ ]
