# EdgeVec Release Checklist

**Version:** 1.0.0
**Created:** 2025-12-12
**Last Updated:** 2025-12-12
**Status:** [ACTIVE]

---

## Overview

This checklist MUST be followed for every npm publish. It was created after incident INC-2025-12-12-001 where v0.2.0-alpha.1 was published without the `snippets/` directory.

**Rule:** No step may be skipped. If a step fails, STOP and fix before continuing.

---

## Pre-Release Checklist

### 1. Code Quality Gates

- [ ] All tests pass: `cargo test --all-features`
- [ ] No clippy warnings: `cargo clippy -- -D warnings` (or documented exceptions)
- [ ] Code formatted: `cargo fmt --check`
- [ ] WASM builds successfully: `wasm-pack build --target web --release`

### 2. Version Consistency

- [ ] `Cargo.toml` version matches release version
- [ ] `wasm/pkg/package.json` version matches release version
- [ ] `pkg/package.json` version matches release version (if exists)
- [ ] `CHANGELOG.md` has entry for this version with correct date
- [ ] Git tag name matches version: `v{VERSION}`

### 3. Documentation

- [ ] `README.md` is up to date
- [ ] `CHANGELOG.md` entry is complete
- [ ] `docs/KNOWN_LIMITATIONS.md` is current
- [ ] All links in documentation work

### 4. Package Content Verification [CRITICAL]

**This section added after INC-2025-12-12-001**

```bash
# Step 4.1: Generate dry-run package listing
cd wasm/pkg
npm pack --dry-run 2>&1 | tee /tmp/pack-output.txt

# Step 4.2: Verify ALL required files are listed
# Check output contains:
grep -q "edgevec_bg.wasm" /tmp/pack-output.txt && echo "✅ WASM binary" || echo "❌ MISSING: WASM binary"
grep -q "edgevec.js" /tmp/pack-output.txt && echo "✅ JS entry" || echo "❌ MISSING: JS entry"
grep -q "edgevec.d.ts" /tmp/pack-output.txt && echo "✅ TypeScript types" || echo "❌ MISSING: TypeScript types"
grep -q "snippets" /tmp/pack-output.txt && echo "✅ Snippets directory" || echo "❌ MISSING: Snippets directory"
grep -q "README.md" /tmp/pack-output.txt && echo "✅ README" || echo "❌ MISSING: README"
grep -q "package.json" /tmp/pack-output.txt && echo "✅ package.json" || echo "❌ MISSING: package.json"
```

**Checklist:**
- [ ] `edgevec_bg.wasm` present
- [ ] `edgevec_bg.wasm.d.ts` present
- [ ] `edgevec.js` present
- [ ] `edgevec.d.ts` present
- [ ] `snippets/` directory present (CRITICAL - see INC-2025-12-12-001)
- [ ] `README.md` present
- [ ] `package.json` present
- [ ] Package size is reasonable (<500KB gzipped)

### 5. Fresh Install Smoke Test [CRITICAL]

**This section added after INC-2025-12-12-001**

```bash
# Step 5.1: Create isolated test environment
mkdir -p /tmp/edgevec-release-test
cd /tmp/edgevec-release-test
npm init -y

# Step 5.2: Pack and install locally (simulates npm install)
cd /path/to/edgevec/wasm/pkg
npm pack
mv edgevec-*.tgz /tmp/edgevec-release-test/
cd /tmp/edgevec-release-test
npm install ./edgevec-*.tgz

# Step 5.3: Verify installation
ls node_modules/edgevec/snippets/
# Must show: edgevec-XXXXXXXX/ directory

# Step 5.4: Test import
node -e "import('edgevec').then(m => console.log('✅ Import SUCCESS:', Object.keys(m).length, 'exports')).catch(e => console.log('❌ Import FAILED:', e.message))"

# Step 5.5: Cleanup
cd /tmp && rm -rf edgevec-release-test
```

**Checklist:**
- [ ] Local tarball install succeeds
- [ ] `snippets/` directory exists in node_modules
- [ ] Import succeeds without errors
- [ ] Correct number of exports

### 6. Service Health Check

```bash
# Check npm registry
curl -I https://registry.npmjs.org/ 2>&1 | head -1
# Expected: HTTP/1.1 200 OK or HTTP/2 200

# Check GitHub API
curl -I https://api.github.com/ 2>&1 | head -1
# Expected: HTTP/1.1 200 OK or HTTP/2 200

# Check GitHub status
curl -s https://www.githubstatus.com/api/v2/status.json | grep -o '"indicator":"[^"]*"'
# Expected: "indicator":"none"
```

**Checklist:**
- [ ] npm registry responds 200
- [ ] GitHub API responds 200
- [ ] GitHub status indicator is "none" (no incidents)
- [ ] Git remote is accessible

**STOP if any service is unhealthy. Reschedule release.**

---

## Release Execution

### 7. Git Tag Creation

```bash
# Verify clean working directory
git status
# Must show: nothing to commit, working tree clean

# Create annotated tag
git tag -a v{VERSION} -m "Release v{VERSION}

[Release description]"

# Push tag
git push origin v{VERSION}
```

**Checklist:**
- [ ] Working directory is clean
- [ ] Tag created with annotation
- [ ] Tag pushed to origin

### 8. npm Publish

```bash
# Final pre-publish verification
cd wasm/pkg
npm pack --dry-run

# Publish (requires auth)
npm publish --access public --tag alpha  # For alpha releases
npm publish --access public              # For stable releases

# If 2FA enabled:
npm publish --access public --otp=XXXXXX
```

**Checklist:**
- [ ] `npm pack --dry-run` shows all expected files
- [ ] `npm publish` exits with code 0
- [ ] No error messages

### 9. Post-Publish Verification [CRITICAL]

```bash
# Step 9.1: Verify package on registry
npm view edgevec@{VERSION}
# Must show package info

# Step 9.2: Fresh install from registry
mkdir -p /tmp/edgevec-verify
cd /tmp/edgevec-verify
npm init -y
npm install edgevec@{VERSION}

# Step 9.3: Verify snippets exist
ls node_modules/edgevec/snippets/
# Must show directory contents

# Step 9.4: Test import
node -e "import('edgevec').then(m => console.log('✅ Registry install SUCCESS')).catch(e => console.log('❌ FAILED:', e.message))"

# Step 9.5: Cleanup
cd /tmp && rm -rf edgevec-verify
```

**Checklist:**
- [ ] `npm view` shows correct version
- [ ] Fresh install from registry succeeds
- [ ] `snippets/` directory present
- [ ] Import test passes

### 10. GitHub Release (Manual)

1. Go to: `https://github.com/{USER}/edgevec/releases/new`
2. Select tag: `v{VERSION}`
3. Title: `v{VERSION} — [Description]`
4. Add release notes from CHANGELOG
5. Check "Pre-release" for alpha/beta versions
6. Click "Publish release"

**Checklist:**
- [ ] GitHub release created
- [ ] Pre-release checkbox correct
- [ ] Release notes accurate

---

## Rollback Procedures

### If npm publish fails:

1. Check error message
2. Fix issue (auth, network, version conflict)
3. Retry publish
4. If 3 retries fail, postpone 24h

### If critical bug discovered post-publish:

**Within 72 hours:**
```bash
npm unpublish edgevec@{VERSION}
# Then publish fixed version
```

**After 72 hours:**
```bash
npm deprecate edgevec@{VERSION} "Critical bug - use {NEW_VERSION}"
# Publish fixed version
```

### If git tag needs correction:

```bash
# Delete local tag
git tag -d v{VERSION}

# Delete remote tag
git push origin :refs/tags/v{VERSION}

# Create corrected tag
git tag -a v{VERSION} -m "..."
git push origin v{VERSION}
```

---

## Emergency Hotfix Protocol

If a critical bug requires immediate hotfix:

1. **DO NOT PANIC** — Alpha users expect some issues
2. **Assess severity** — Is it blocking all users or edge case?
3. **Fix forward** — Publish new version, don't try to patch existing
4. **Version bump** — alpha.1 → alpha.2
5. **Deprecate broken** — `npm deprecate edgevec@{BROKEN} "Use {FIXED}"`
6. **Create incident report** — `docs/incidents/YYYY-MM-DD_description.md`
7. **Update this checklist** — Add prevention measures

---

## Checklist Summary

| Section | Items | Critical |
|:--------|:------|:---------|
| Code Quality | 4 | No |
| Version Consistency | 5 | No |
| Documentation | 4 | No |
| Package Content | 8 | **YES** |
| Fresh Install Test | 5 | **YES** |
| Service Health | 4 | No |
| Git Tag | 3 | No |
| npm Publish | 3 | No |
| Post-Publish Verify | 5 | **YES** |
| GitHub Release | 3 | No |

**Total Items:** 44
**Critical Sections:** 3 (Package Content, Fresh Install, Post-Publish)

---

## Version History

| Version | Date | Changes |
|:--------|:-----|:--------|
| 1.0.0 | 2025-12-12 | Initial version after INC-2025-12-12-001 |

---

**Document Status:** [ACTIVE]
**Review Schedule:** Before each release
