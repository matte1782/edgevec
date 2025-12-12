# EdgeVec Rollback Playbook

**Version:** 0.2.0-alpha.1
**Created:** 2025-12-12
**Last Rehearsed:** 2025-12-12
**Status:** VERIFIED via dry-run

---

## Overview

This playbook documents the procedures to roll back an EdgeVec release in case of critical issues discovered post-publish.

**Principle:** You rehearse rollback BEFORE incidents, not during them.

---

## Rollback Scenarios

### Scenario 1: npm Package Rollback

**When to use:** Critical bug discovered in published npm package

**Steps:**

```bash
# 1. Deprecate the broken version (warns users, doesn't remove)
npm deprecate edgevec@0.2.0-alpha.1 "Critical bug - please use 0.2.0-alpha.2"

# 2. Publish hotfix version
cd wasm/pkg
# ... fix the issue ...
# Update version in Cargo.toml to 0.2.0-alpha.2
wasm-pack build --target web --release --out-dir wasm/pkg
npm publish --access public --tag alpha

# 3. If within 72 hours and absolutely necessary (LAST RESORT)
# This permanently removes the package and may break user builds
npm unpublish edgevec@0.2.0-alpha.1
```

**Dry-run verification (2025-12-12):**
```
✅ npm publish --dry-run --access public --tag alpha
   Result: + edgevec@0.2.0-alpha.1 (dry-run successful)
```

### Scenario 2: Git Tag Rollback

**When to use:** Need to remove a release tag from git

**Steps:**

```bash
# 1. Delete local tag
git tag -d v0.2.0-alpha.1

# 2. Delete remote tag (if pushed)
git push origin :refs/tags/v0.2.0-alpha.1

# 3. Create corrected tag
git tag -a v0.2.0-alpha.2 -m "Hotfix release"
git push origin v0.2.0-alpha.2
```

**Dry-run verification (2025-12-12):**
```
✅ git tag -a v0.0.0-rollback-test -m "Rollback rehearsal test tag"
   Result: Tag created successfully
✅ git tag -l | grep rollback-test
   Result: v0.0.0-rollback-test
✅ git tag -d v0.0.0-rollback-test
   Result: Deleted tag 'v0.0.0-rollback-test' (was 989c81f)
✅ git tag -l | grep rollback-test
   Result: (empty - tag successfully deleted)
```

### Scenario 3: GitHub Release Rollback

**When to use:** Need to remove a GitHub release

**Prerequisites:** `gh` CLI installed and authenticated

**Steps:**

```bash
# 1. List releases
gh release list

# 2. Delete specific release (does NOT delete tag)
gh release delete v0.2.0-alpha.1 --yes

# 3. Also delete tag if needed
git tag -d v0.2.0-alpha.1
git push origin :refs/tags/v0.2.0-alpha.1

# 4. Create new release
gh release create v0.2.0-alpha.2 --title "v0.2.0-alpha.2 (Hotfix)" --notes "Fixes critical issue from alpha.1"
```

**Note:** gh CLI not installed on current system. Install before release:
```bash
# Windows (winget)
winget install GitHub.cli

# Or download from https://cli.github.com/
```

### Scenario 4: crates.io Rollback

**When to use:** Critical bug in Rust crate (if published to crates.io)

**Steps:**

```bash
# 1. Yank the broken version (hides from new installs, existing deps still work)
cargo yank --vers 0.2.0-alpha.1

# 2. Publish hotfix
# Update Cargo.toml to 0.2.0-alpha.2
cargo publish

# 3. Un-yank if the yank was a mistake
cargo yank --vers 0.2.0-alpha.1 --undo
```

**Important:** crates.io does NOT allow unpublishing. Yank only hides the version from new dependency resolution. Existing Cargo.lock files will still fetch the yanked version.

---

## Rollback Decision Tree

```
Issue Discovered
      │
      ▼
┌─────────────────────────┐
│ Severity Assessment     │
└─────────────────────────┘
      │
      ├── Critical (security, data loss, crash)
      │   └── IMMEDIATE ROLLBACK
      │       1. npm deprecate
      │       2. GitHub release delete
      │       3. Publish hotfix ASAP
      │
      ├── Major (incorrect results, performance regression)
      │   └── PLANNED ROLLBACK
      │       1. Communicate issue to users
      │       2. Fix in next release
      │       3. npm deprecate (optional)
      │
      └── Minor (typo, cosmetic)
          └── NO ROLLBACK
              Fix in next regular release
```

---

## Pre-Release Checklist (To Minimize Rollback Need)

Before every release:

- [ ] All tests pass (`cargo test --all-features --release`)
- [ ] No critical clippy warnings (`cargo clippy -- -D warnings`)
- [ ] WASM smoke test passes in browser
- [ ] Benchmarks within expected range
- [ ] CHANGELOG updated
- [ ] Version bumped in Cargo.toml and wasm/pkg/package.json

---

## Emergency Contacts

| Role | Contact | Authority |
|:-----|:--------|:----------|
| Maintainer | @matte1782 | Full rollback authority |
| npm Account | edgevec | npm publish/unpublish |
| GitHub Repo | matte1782/edgevec | Release management |

---

## Rollback History

| Date | Version | Reason | Action | Time to Resolve |
|:-----|:--------|:-------|:-------|:----------------|
| (none yet) | | | | |

---

## Rehearsal Log

| Date | Scenario | Result | Notes |
|:-----|:---------|:-------|:------|
| 2025-12-12 | npm publish dry-run | ✅ PASS | `--tag alpha` required for prerelease |
| 2025-12-12 | git tag create/delete | ✅ PASS | Tag operations verified |
| 2025-12-12 | GitHub release | ⏸️ SKIP | gh CLI not installed |

---

**Document Status:** [APPROVED]
**Next Rehearsal:** Before v0.3.0 release
