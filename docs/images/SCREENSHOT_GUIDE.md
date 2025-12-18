# Screenshot Capture Guide

**Version:** 1.1.0
**Purpose:** Documentation for capturing demo screenshots

---

## Automated Capture (Recommended)

The fastest way to capture all screenshots is using the automated script:

```bash
# 1. Start the local server (from project root)
python -m http.server 8080

# 2. In another terminal, install dependencies and run
cd scripts
npm install
npm run screenshots

# Or with custom options
node capture-screenshots.js --port 8080 --output ../docs/images
```

**What the script captures:**
- `playground-dark.png` - Filter Playground with dark theme
- `playground-light.png` - Filter Playground with light theme
- `dashboard.png` - Benchmark Dashboard with charts
- `soft-delete.png` - Soft Delete Demo
- `demo-catalog.png` - Main Demo Catalog

**Requirements:**
- Node.js 18+
- Puppeteer (installed via npm)
- Local server running on port 8080

---

## Required Screenshots

### 1. Filter Playground (filter-playground.html)

**Filename:** `playground-dark.png`, `playground-light.png`

**Capture State:**
- Filter input: `category = "electronics" AND price < 500`
- Show successful parse result with AST visualization
- Status bar showing parse time

**Resolution:** 1200x800 (2x for retina: 2400x1600)

### 2. Benchmark Dashboard (benchmark-dashboard.html)

**Filename:** `dashboard.png`

**Capture State:**
- All charts rendered with data
- Filter Performance section visible
- Show EdgeVec vs competitors comparison

**Resolution:** 1200x800

### 3. Soft Delete Demo (soft_delete.html)

**Filename:** `soft-delete.png`

**Capture State:**
- Some vectors inserted and deleted
- Activity log showing operations
- Compaction available

**Resolution:** 1200x800

### 4. Main Demo / Index (index.html)

**Filename:** `demo-catalog.png`

**Capture State:**
- Hero section visible
- Demo grid cards visible
- Quick Try section with filter

**Resolution:** 1200x800

---

## Capture Instructions

### Using Chrome DevTools

1. Open demo in Chrome
2. Press F12 to open DevTools
3. Press Ctrl+Shift+P and type "screenshot"
4. Select "Capture full size screenshot" or "Capture node screenshot"

### Using Puppeteer (Automated)

```javascript
const puppeteer = require('puppeteer');

async function captureScreenshots() {
    const browser = await puppeteer.launch();
    const page = await browser.newPage();
    await page.setViewport({ width: 1200, height: 800, deviceScaleFactor: 2 });

    const demos = [
        { url: 'filter-playground.html', name: 'playground-dark.png' },
        { url: 'benchmark-dashboard.html', name: 'dashboard.png' },
        { url: 'soft_delete.html', name: 'soft-delete.png' },
        { url: 'index.html', name: 'demo-catalog.png' }
    ];

    for (const demo of demos) {
        await page.goto(`http://localhost:8080/wasm/examples/${demo.url}`);
        await page.waitForSelector('.loaded, .ready', { timeout: 5000 }).catch(() => {});
        await page.screenshot({ path: `docs/images/${demo.name}` });
    }

    await browser.close();
}

captureScreenshots();
```

---

## Image Optimization

After capture:

1. Compress with TinyPNG or ImageOptim
2. Target: <200KB per image
3. Format: PNG for demos, WebP if supported

### Compression Command

```bash
# Using pngquant
pngquant --quality=65-80 docs/images/*.png --ext .png --force

# Using ImageMagick
convert input.png -strip -quality 85 output.png
```

---

## Alt Text for Accessibility

| Image | Alt Text |
|:------|:---------|
| playground-dark.png | "EdgeVec Filter Playground showing SQL-like filter syntax with real-time AST visualization" |
| playground-light.png | "EdgeVec Filter Playground in light theme" |
| dashboard.png | "EdgeVec Benchmark Dashboard comparing search performance against hnswlib and voy" |
| soft-delete.png | "EdgeVec Soft Delete Demo showing tombstone-based deletion and compaction" |
| demo-catalog.png | "EdgeVec Demo Catalog with six interactive examples" |

---

## README Usage

```markdown
![Filter Playground](docs/images/playground-dark.png)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/playground-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="docs/images/playground-light.png">
  <img alt="Filter Playground" src="docs/images/playground-dark.png" width="800">
</picture>
```

---

*Screenshots should be recaptured for each major release.*
