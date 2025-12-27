# Week 31 Day 6: GitHub Pages Deployment

**Date:** 2026-01-01
**Focus:** Deploy all demos to GitHub Pages
**Estimated Duration:** 3 hours
**Priority:** P1 — Live demos for release announcement

---

## Objectives

1. Deploy filter playground to GitHub Pages
2. Deploy SIMD benchmark page
3. Deploy v0.7.0 demo
4. Update demo index page
5. Verify all demos work

---

## Tasks

### W31.6.1: Deploy Filter Playground

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Source:** `wasm/examples/filter-playground.html`
**Target:** `https://matte1782.github.io/edgevec/filter-playground.html`

**Deployment Steps:**

1. **Ensure gh-pages branch exists:**
```bash
git checkout gh-pages || git checkout -b gh-pages
```

2. **Copy demo files:**
```bash
# Copy HTML
cp wasm/examples/filter-playground.html docs/

# Copy WASM bundle
cp -r pkg/ docs/pkg/

# Copy any CSS/JS assets
cp wasm/examples/*.css docs/ 2>/dev/null || true
```

3. **Commit and push:**
```bash
git add docs/
git commit -m "deploy: filter playground v0.7.0"
git push origin gh-pages
```

4. **Verify deployment:**
   - Wait 2-5 minutes for GitHub Pages to update
   - Visit: `https://matte1782.github.io/edgevec/filter-playground.html`
   - Test filter parsing works
   - Test live sandbox works

**Acceptance Criteria:**
- [ ] Page accessible via GitHub Pages
- [ ] Filter parsing works
- [ ] Live sandbox loads

---

### W31.6.2: Deploy SIMD Benchmark Page

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Source:** `wasm/examples/simd_benchmark.html`
**Target:** `https://matte1782.github.io/edgevec/simd_benchmark.html`

**Deployment Steps:**

```bash
# On gh-pages branch
cp wasm/examples/simd_benchmark.html docs/
cp wasm/examples/simd_test.html docs/

git add docs/
git commit -m "deploy: SIMD benchmark pages v0.7.0"
git push origin gh-pages
```

**Verify:**
- Visit benchmark page
- Click "Run Benchmark"
- Verify SIMD detection shows ✅
- Verify benchmark completes

**Acceptance Criteria:**
- [ ] Benchmark page accessible
- [ ] SIMD detection works
- [ ] Benchmark runs successfully

---

### W31.6.3: Deploy v0.7.0 Demo

**Duration:** 0.5 hours
**Agent:** WASM_SPECIALIST

**Option A:** Create new v070 demo page
**Option B:** Update existing cyberpunk demo

**For Option B (recommended):**

1. Copy existing demo:
```bash
cp wasm/examples/v060_cyberpunk_demo.html wasm/examples/v070_demo.html
```

2. Update for v0.7.0:
   - Change version in header
   - Add SIMD status indicator
   - Add Hamming distance benchmark section
   - Update feature highlights

3. Deploy:
```bash
cp wasm/examples/v070_demo.html docs/
git add docs/v070_demo.html
git commit -m "deploy: v0.7.0 demo page"
git push origin gh-pages
```

**Acceptance Criteria:**
- [ ] Demo page accessible
- [ ] Shows v0.7.0 features
- [ ] SIMD acceleration visible

---

### W31.6.4: Update Demo Index Page

**Duration:** 0.5 hours
**Agent:** DOCWRITER

**File:** `docs/index.html` (on gh-pages branch)

**Content:**

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EdgeVec v0.7.0 — WASM Vector Database</title>
    <style>
        :root {
            --primary: #ff0066;
            --secondary: #00ff88;
            --bg: #0a0a0f;
            --surface: #1a1a2e;
            --text: #e0e0e0;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Segoe UI', system-ui, sans-serif;
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
            padding: 40px 20px;
        }
        .container { max-width: 1000px; margin: 0 auto; }
        h1 {
            color: var(--primary);
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
            text-shadow: 0 0 20px var(--primary);
        }
        .tagline {
            color: var(--secondary);
            font-size: 1.2rem;
            margin-bottom: 2rem;
        }
        .highlight {
            background: var(--surface);
            border-left: 4px solid var(--secondary);
            padding: 1rem;
            margin-bottom: 2rem;
            border-radius: 0 8px 8px 0;
        }
        .demo-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin: 2rem 0;
        }
        .demo-card {
            background: var(--surface);
            border-radius: 12px;
            padding: 24px;
            text-decoration: none;
            color: var(--text);
            transition: transform 0.2s, box-shadow 0.2s;
            border: 1px solid transparent;
        }
        .demo-card:hover {
            transform: translateY(-4px);
            box-shadow: 0 10px 30px rgba(255, 0, 102, 0.2);
            border-color: var(--primary);
        }
        .demo-card h3 {
            color: var(--primary);
            margin-bottom: 0.5rem;
            font-size: 1.3rem;
        }
        .demo-card p {
            color: #a0a0a0;
            line-height: 1.5;
        }
        .links {
            display: flex;
            gap: 20px;
            margin-top: 2rem;
            flex-wrap: wrap;
        }
        .links a {
            color: var(--secondary);
            text-decoration: none;
            padding: 10px 20px;
            border: 1px solid var(--secondary);
            border-radius: 6px;
            transition: background 0.2s;
        }
        .links a:hover {
            background: rgba(0, 255, 136, 0.1);
        }
        .badge {
            display: inline-block;
            background: var(--primary);
            color: white;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 0.8rem;
            margin-left: 8px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>EdgeVec v0.7.0</h1>
        <p class="tagline">The first WASM-native vector database</p>

        <div class="highlight">
            🎉 <strong>First External Contribution!</strong> v0.7.0 includes SIMD Hamming distance
            with <strong>8.75x speedup</strong> contributed by
            <a href="https://github.com/jsonMartin" style="color: var(--secondary)">@jsonMartin</a>!
        </div>

        <h2>Interactive Demos</h2>
        <div class="demo-grid">
            <a href="filter-playground.html" class="demo-card">
                <h3>🔍 Filter Playground <span class="badge">NEW</span></h3>
                <p>Build filter expressions visually, see AST representation,
                   and execute against real data in a live WASM sandbox.</p>
            </a>

            <a href="simd_benchmark.html" class="demo-card">
                <h3>⚡ SIMD Benchmark</h3>
                <p>Measure SIMD acceleration in your browser. See the 2-3x
                   speedup from WASM SIMD128 instructions.</p>
            </a>

            <a href="v070_demo.html" class="demo-card">
                <h3>🚀 v0.7.0 Demo</h3>
                <p>Full-featured demo showcasing search, filtering,
                   binary quantization, and SIMD performance.</p>
            </a>

            <a href="benchmark-dashboard.html" class="demo-card">
                <h3>📊 Benchmark Dashboard</h3>
                <p>Compare EdgeVec performance against alternatives
                   with interactive charts.</p>
            </a>
        </div>

        <h2>Quick Links</h2>
        <div class="links">
            <a href="https://github.com/matte1782/edgevec">GitHub</a>
            <a href="https://crates.io/crates/edgevec">crates.io</a>
            <a href="https://www.npmjs.com/package/edgevec">npm</a>
            <a href="https://docs.rs/edgevec">API Docs</a>
        </div>
    </div>
</body>
</html>
```

**Deploy:**
```bash
git add docs/index.html
git commit -m "deploy: update index page for v0.7.0"
git push origin gh-pages
```

**Acceptance Criteria:**
- [ ] Index page shows v0.7.0
- [ ] @jsonMartin contribution highlighted
- [ ] All demo links work
- [ ] Mobile responsive

---

### W31.6.5: Verify All Demos Work

**Duration:** 0.5 hours
**Agent:** TEST_ENGINEER

**Test Each Demo:**

| Demo | Test | Expected |
|:-----|:-----|:---------|
| Index | Page loads | ✅ |
| Index | All links work | ✅ |
| Filter Playground | Filter parsing | ✅ |
| Filter Playground | Live sandbox | ✅ |
| SIMD Benchmark | SIMD detection | ✅ |
| SIMD Benchmark | Benchmark runs | ✅ |
| v0.7.0 Demo | Search works | ✅ |
| Dashboard | Charts load | ✅ |

**Test URLs:**
- `https://matte1782.github.io/edgevec/`
- `https://matte1782.github.io/edgevec/filter-playground.html`
- `https://matte1782.github.io/edgevec/simd_benchmark.html`
- `https://matte1782.github.io/edgevec/v070_demo.html`

**Acceptance Criteria:**
- [ ] All pages load
- [ ] No console errors
- [ ] WASM initializes
- [ ] Features work

---

### W31.6.6: Test Mobile Responsiveness

**Duration:** 0.5 hours
**Agent:** TEST_ENGINEER

**Test Devices:**
- Desktop (1920x1080)
- Tablet (768x1024)
- Mobile (375x667)

**Test Method:**
1. Chrome DevTools → Toggle device toolbar
2. Test each demo at each resolution
3. Check for layout issues
4. Verify touch interactions work

**Common Issues:**
- Tables overflow on mobile
- Buttons too small
- Text too small
- Horizontal scroll

**Acceptance Criteria:**
- [ ] Desktop layout works
- [ ] Tablet layout works
- [ ] Mobile layout works
- [ ] No horizontal scroll

---

## Day 6 Exit Criteria

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Index page | Deployed and accessible | [ ] |
| Filter playground | Live and working | [ ] |
| SIMD benchmark | Live and working | [ ] |
| v0.7.0 demo | Live and working | [ ] |
| Dashboard | Live and working | [ ] |
| All links | Working | [ ] |
| Mobile | Responsive | [ ] |
| @jsonMartin | Highlighted on index | [ ] |

---

**Day 6 Total:** 3 hours
**Agent:** WASM_SPECIALIST, DOCWRITER, TEST_ENGINEER
