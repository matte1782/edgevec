# EdgeVec v0.5.2 Release Commands

**Date:** 2025-12-19
**Status:** APPROVED by HOSTILE_REVIEWER

---

## Pre-Release Verification

Both dry-runs passed:
- `cargo publish --dry-run` — Ready (after commit)
- `npm pack --dry-run` — 31 files, 273.1 KB

---

## Step 1: Stage All Changes

```bash
cd "C:\Users\matte\Desktop\Desktop OLD\AI\Università AI\courses\personal_project\fortress_problem_driven\research_fortress\edgevec"

git add .gitignore
git add pkg/package.json
git add pkg/tsconfig.json
git add pkg/index.ts
git add pkg/index.js
git add pkg/index.d.ts
git add pkg/index.js.map
git add pkg/index.d.ts.map
git add pkg/filter.js
git add pkg/filter.d.ts
git add pkg/filter.js.map
git add pkg/filter.d.ts.map
git add pkg/filter-builder.js
git add pkg/filter-builder.d.ts
git add pkg/filter-builder.js.map
git add pkg/filter-builder.d.ts.map
git add pkg/edgevec-wrapper.js
git add pkg/edgevec-wrapper.d.ts
git add pkg/edgevec-wrapper.js.map
git add pkg/edgevec-wrapper.d.ts.map
git add pkg/README.md
git add docs/reviews/
git add docs/metrics/
```

---

## Step 2: Commit

```bash
git commit -m "fix(pkg): v0.5.2 hotfix - compile TypeScript to JavaScript

npm package changes:
- Fixed ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING in Node.js
- Package now exports compiled .js files instead of raw .ts
- Added subpath exports: ./filter, ./filter-builder, ./wrapper, ./core
- Re-exported core WASM bindings for backwards compatibility
- Updated .gitignore to exclude node_modules and test directories

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Step 3: Tag Release

```bash
git tag v0.5.2
```

---

## Step 4: Push to GitHub

```bash
git push origin main --tags
```

---

## Step 5: Publish to crates.io (Rust v0.5.0)

```bash
cargo publish
```

**Note:** The Rust crate version is 0.5.0 (unchanged). This is the first crates.io release since v0.4.0.

---

## Step 6: Publish to npm (v0.5.2)

```bash
cd pkg
npm publish
```

---

## Step 7: Create GitHub Release

Go to: https://github.com/matte1782/edgevec/releases/new

**Tag:** v0.5.2 (select existing tag)

**Title:** v0.5.2 - Filter API + TypeScript Export Fix

**Body:**
```
## v0.5.2 Release

### Rust/WASM (crates.io v0.5.0)
First crates.io release since v0.4.0!

- **Filter API:** 15 SQL-like operators for metadata filtering
- **Parser + Evaluator:** Pure Rust implementation
- **Fuzz tested:** 14.4 billion executions, 0 crashes
- **24h deep fuzzing:** Both filter_simple and filter_deep targets

### npm Package (v0.5.2)

#### Fixed
- **P0:** Package now exports compiled JavaScript instead of raw TypeScript
- **P0:** Node.js `ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING` error resolved

#### Added
- Subpath exports: `edgevec/filter`, `edgevec/filter-builder`, `edgevec/wrapper`, `edgevec/core`
- Re-exported core WASM bindings (`EdgeVec`, `EdgeVecConfig`, `init`) for backwards compatibility

### Installation

**Rust:**
```toml
[dependencies]
edgevec = "0.5.0"
```

**npm:**
```bash
npm install edgevec@0.5.2
```

### Notes
EdgeVec is designed for **browser-first** usage with bundlers (Vite, Webpack, Rollup).
```

---

## Step 8: Verify Releases

### Verify crates.io
```bash
cargo search edgevec
```

Expected output should show `edgevec = "0.5.0"`

### Verify npm
```bash
npm view edgevec version
```

Expected output: `0.5.2`

### Full npm test
```bash
mkdir /tmp/test-edgevec-052
cd /tmp/test-edgevec-052
npm init -y
npm install edgevec@0.5.2
node -e "import('edgevec').then(m => console.log('SUCCESS:', Object.keys(m).length, 'exports'))"
```

Expected output: `SUCCESS: 11 exports`

---

## All-in-One Command Block (Copy & Paste)

```bash
# Navigate to project
cd "C:\Users\matte\Desktop\Desktop OLD\AI\Università AI\courses\personal_project\fortress_problem_driven\research_fortress\edgevec"

# Stage all changes
git add .gitignore pkg/ docs/reviews/ docs/metrics/

# Commit
git commit -m "fix(pkg): v0.5.2 hotfix - compile TypeScript to JavaScript

npm package changes:
- Fixed ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING in Node.js
- Package now exports compiled .js files instead of raw .ts
- Added subpath exports: ./filter, ./filter-builder, ./wrapper, ./core
- Re-exported core WASM bindings for backwards compatibility

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

# Tag and push
git tag v0.5.2
git push origin main --tags

# Publish to crates.io
cargo publish

# Publish to npm
cd pkg
npm publish

# Return to root
cd ..

echo "✅ Release complete! Create GitHub release at:"
echo "https://github.com/matte1782/edgevec/releases/new"
```

---

## Rollback (If Needed)

### npm
```bash
npm unpublish edgevec@0.5.2
```

### crates.io
Cannot unpublish from crates.io. You can only yank:
```bash
cargo yank --version 0.5.0
```

### Git
```bash
git tag -d v0.5.2
git push origin :refs/tags/v0.5.2
git reset --hard HEAD~1
git push --force
```

---

*Generated: 2025-12-19*
*Approved by: HOSTILE_REVIEWER*
