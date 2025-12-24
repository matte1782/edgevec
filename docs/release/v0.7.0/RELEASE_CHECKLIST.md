# v0.7.0 Release Checklist

**Version:** 0.7.0
**Release Date:** Week 31 (Late December 2025)
**Focus:** SIMD Acceleration + Filter Playground

---

## Pre-Release Verification (COMPLETE)

### Quality Gates

- [x] All tests pass: `cargo test --all-features` — **667 passed**
- [x] Clippy clean: `cargo clippy -- -D warnings` — **0 warnings**
- [x] WASM builds: `wasm-pack build --target web` — **Success**
- [x] SIMD verified: `.cargo/config.toml` has `+simd128`
- [x] Code quality: Reddit script passed

### Documentation

- [x] CHANGELOG.md v0.7.0 entry complete
- [x] README.md updated (SIMD section, Filter Playground)
- [x] docs/api/FILTER_SYNTAX.md playground link added
- [x] All demo links verified

### Demos

- [x] Filter Playground: `docs/demo/index.html`
- [x] Demo Hub: `docs/demo/hub.html`
- [x] Cyberpunk Demo: `docs/demo/cyberpunk.html`
- [x] SIMD Benchmark: `wasm/examples/simd_benchmark.html`

### Reviews

- [x] Week 30 Gate Review: `docs/reviews/2025-12-24_W30_GATE_REVIEW.md` — **APPROVED**

---

## Version Numbers (VERIFIED)

| File | Version | Status |
|:-----|:--------|:-------|
| `Cargo.toml` | 0.7.0 | ✅ |
| `pkg/package.json` | 0.7.0 | Needs verification |
| `CHANGELOG.md` | 0.7.0 | ✅ |

---

## Release Commands (Week 31)

### 1. Final Verification

```bash
# Clean build
cargo clean && cargo build --release --all-features

# Run tests one more time
cargo test --all-features

# Verify WASM
wasm-pack build --target web
```

### 2. Git Tag

```bash
# Commit any final changes
git add -A
git commit -m "chore: prepare v0.7.0 release"

# Create annotated tag
git tag -a v0.7.0 -m "v0.7.0: SIMD Acceleration + Filter Playground

Features:
- WASM SIMD128 acceleration (2x+ speedup)
- Interactive Filter Playground demo
- enableBQ() API for binary quantization
- Bundle size optimization (477 KB)

Fixes:
- AVX2 popcount optimization (Reddit feedback)
- Code cleanup and professional comments
- Safety documentation improvements
"

# Push tag
git push origin v0.7.0
git push origin main
```

### 3. Publish to crates.io

```bash
# Dry run first
cargo publish --dry-run

# Publish
cargo publish
```

### 4. Publish to npm

```bash
# Build fresh WASM
wasm-pack build --target web

# Verify package.json version
cat pkg/package.json | grep version

# Publish
cd pkg && npm publish && cd ..
```

### 5. GitHub Release

Create release at: https://github.com/matte1782/edgevec/releases/new

**Title:** v0.7.0: SIMD Acceleration + Filter Playground

**Body:**
```markdown
## Highlights

### WASM SIMD Acceleration
- **2x+ faster** vector operations on modern browsers
- SIMD128 enabled by default
- Automatic scalar fallback for iOS Safari

### Interactive Filter Playground
- Visual filter expression builder
- 10 ready-to-use examples
- Live WASM execution sandbox
- Copy-paste code snippets

### Performance
| Dimension | Speedup |
|:----------|:--------|
| 128D | 2.3x |
| 768D | 2.1x |
| 1536D | 2.0x |

### Try It Now
- [Filter Playground](https://matte1782.github.io/edgevec/demo/)
- [Demo Hub](https://matte1782.github.io/edgevec/demo/hub.html)

### Installation
\`\`\`bash
npm install edgevec@0.7.0
\`\`\`

\`\`\`bash
cargo add edgevec@0.7.0
\`\`\`

See [CHANGELOG](CHANGELOG.md) for full details.
```

---

## Post-Release Tasks

### Reddit Response

Reply to chillfish8 with:

> Thanks for the detailed code review! We addressed your feedback in v0.7.0:
>
> 1. **AVX2 popcount** — Now uses native `popcnt` instruction as you suggested
> 2. **The "crisis" comments** — Cleaned up, embarrassing indeed!
> 3. **Code consolidation** — Audit complete, major refactor planned for v0.8.0
>
> The detailed feedback made EdgeVec better. Appreciate it!
>
> [Link to v0.7.0 release]

### Monitoring

- [ ] Check GitHub issues for bug reports
- [ ] Monitor npm download stats
- [ ] Check crates.io for any issues
- [ ] Respond to user feedback

### v0.8.0 Planning

- [ ] RFC-004 Query Caching
- [ ] Code consolidation refactor
- [ ] Bundle size optimization
- [ ] TypeScript SDK improvements

---

## Release Status

| Step | Status |
|:-----|:-------|
| Pre-release verification | ✅ COMPLETE |
| Version numbers verified | ✅ COMPLETE |
| Hostile review approved | ✅ APPROVED |
| Git tag | ⏳ Week 31 |
| crates.io publish | ⏳ Week 31 |
| npm publish | ⏳ Week 31 |
| GitHub release | ⏳ Week 31 |
| Reddit response | ⏳ After release |

---

**Prepared by:** EdgeVec Development Team
**Date:** 2025-12-24
**Gate Review:** APPROVED

