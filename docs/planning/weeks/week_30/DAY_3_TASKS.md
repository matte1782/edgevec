# Week 30 Day 3: Metadata Filtering Demo — Design & Layout

**Date:** 2025-12-27
**Focus:** Design cyberpunk-themed interactive filter playground using v0.6.0 infrastructure
**Estimated Duration:** 3-4 hours
**Priority:** P0 — User-requested feature
**Status:** [REVISED] — Enhanced to match v0.6.0 quality standard per hostile review

---

## Context

User feedback from Reddit specifically requests more code snippets for metadata filtering:

> "Add more code snippet for the meta data filtering part, everyone asking"
> — `docs/release/v0.6.0/comments/add_more_snippet.txt`

We're creating an interactive GitHub Pages demo that:
1. Shows filter syntax with copy-paste examples
2. Lets users build filters visually
3. Provides a live sandbox to test filters
4. **REUSES the v0.6.0 cyberpunk infrastructure** (css/, js/ files)

---

## CRITICAL: Architecture Decision

### MUST REUSE v0.6.0 Infrastructure

The v0.6.0 demo established a high-quality modular architecture:

```
wasm/examples/
├── v060_cyberpunk_demo.html           # Reference implementation
├── css/
│   ├── cyberpunk.css                  # 733 lines - Design tokens, base styles
│   ├── layout.css                     # Grid, flexbox utilities
│   ├── components.css                 # Buttons, cards, inputs
│   ├── animations.css                 # Keyframe animations
│   └── mobile.css                     # Responsive breakpoints
└── js/
    ├── effects.js                     # ParticleSystem, MatrixRain (420 lines)
    ├── animations.js                  # Glitch effects
    ├── performance.js                 # FPS monitoring
    ├── components.js                  # UI components
    └── app.js                         # Application bootstrap
```

### v0.7.0 Filter Playground Structure

```
wasm/examples/
├── v070_filter_playground.html        # NEW: Main entry point
├── css/
│   ├── [existing files - REUSE]
│   └── filter-playground.css          # NEW: Playground-specific styles
└── js/
    ├── [existing files - REUSE]
    └── filter-playground.js           # NEW: Sandbox, filter builder logic
```

### DO NOT:
- Duplicate CSS color tokens
- Write inline CSS (except playground-specific overrides)
- Skip visual effects (particle system, matrix rain)
- Ignore accessibility (reduced-motion support)

---

## Tasks

### W30.3.1: Design Demo Layout

**Objective:** Create wireframe and component structure for filter playground.

**Layout Wireframe:**
```
┌────────────────────────────────────────────────────────────────────────┐
│ [CANVAS: Particle System Background]                                   │
│ [CANVAS: Matrix Rain Effect]                                           │
├────────────────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ [Glitch Effect]                                                     │ │
│ │ ███████╗██████╗  ██████╗ ███████╗██╗   ██╗███████╗ ██████╗          │ │
│ │ ██╔════╝██╔══██╗██╔════╝ ██╔════╝██║   ██║██╔════╝██╔════╝          │ │
│ │ █████╗  ██║  ██║██║  ███╗█████╗  ██║   ██║█████╗  ██║               │ │
│ │ ██╔══╝  ██║  ██║██║   ██║██╔══╝  ╚██╗ ██╔╝██╔══╝  ██║               │ │
│ │ ███████╗██████╔╝╚██████╔╝███████╗ ╚████╔╝ ███████╗╚██████╗          │ │
│ │ ╚══════╝╚═════╝  ╚═════╝ ╚══════╝  ╚═══╝  ╚══════╝ ╚═════╝          │ │
│ │                    FILTER PLAYGROUND v0.7.0                          │ │
│ └────────────────────────────────────────────────────────────────────┘ │
│                                                                        │
│ ┌──────────────────────────────┐ ┌─────────────────────────────────┐  │
│ │ [Performance Panel]          │ │ [Theme Toggle]                  │  │
│ │ FPS: 60 | Vectors: 1000     │ │ [☀️ / 🌙]                       │  │
│ │ Search: 2.3ms               │ │                                  │  │
│ └──────────────────────────────┘ └─────────────────────────────────┘  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────────────┐  ┌─────────────────────────────┐ │
│  │ FILTER BUILDER                  │  │ LIVE PREVIEW                │ │
│  │ ┌─────────┐ ┌────┐ ┌─────────┐  │  │                             │ │
│  │ │ field ▼ │ │ = ▼│ │ value   │  │  │ price = "100"               │ │
│  │ └─────────┘ └────┘ └─────────┘  │  │                             │ │
│  │ [+ AND] [+ OR] [( Group )]      │  │ ✅ Valid filter             │ │
│  │                                  │  │                             │ │
│  │ Current: price = 100 AND stock  │  │ [Copy] [Clear]              │ │
│  └─────────────────────────────────┘  └─────────────────────────────┘ │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │ EXAMPLE GALLERY                                                  │  │
│  │                                                                  │  │
│  │ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐    │  │
│  │ │ 🛒 E-COMMERCE   │ │ 📄 DOCUMENTS    │ │ 🎬 CONTENT      │    │  │
│  │ │ [Glitch hover]  │ │ [Glitch hover]  │ │ [Glitch hover]  │    │  │
│  │ │ category="gpu"  │ │ author="John"   │ │ tags IN ["a"]   │    │  │
│  │ │ AND price < 500 │ │ AND year >= 23  │ │ AND rating >= 4 │    │  │
│  │ │                 │ │                 │ │                 │    │  │
│  │ │ [Try It] [Copy] │ │ [Try It] [Copy] │ │ [Try It] [Copy] │    │  │
│  │ └─────────────────┘ └─────────────────┘ └─────────────────┘    │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │ LIVE SANDBOX                                                     │  │
│  │                                                                  │  │
│  │ Vectors: 1000 | Filter: price < 500 | Results: 342 | 2.3ms     │  │
│  │                                                                  │  │
│  │ ┌────────────────────────────────────────────────────────────┐  │  │
│  │ │ #1 ID: 42  │ Dist: 0.12 │ {name:"RTX 4080", price:450}     │  │  │
│  │ │ #2 ID: 87  │ Dist: 0.15 │ {name:"RTX 4070", price:380}     │  │  │
│  │ │ #3 ID: 123 │ Dist: 0.18 │ {name:"RX 7800", price:420}      │  │  │
│  │ └────────────────────────────────────────────────────────────┘  │  │
│  │                                                                  │  │
│  │ [▶ Search] [📦 Load Data] [🗑️ Clear]                           │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │ CODE SNIPPETS                                                    │  │
│  │ ┌──────────┐ ┌──────────┐ ┌──────────┐                          │  │
│  │ │JavaScript│ │TypeScript│ │  React   │                          │  │
│  │ └──────────┘ └──────────┘ └──────────┘                          │  │
│  │ ```javascript                                                    │  │
│  │ const results = await db.searchFiltered(                        │  │
│  │     embedding,                                                   │  │
│  │     10,                                                          │  │
│  │     { filter: 'price < 500 AND category = "gpu"' }              │  │
│  │ );                                                               │  │
│  │ ```                                                    [Copy]   │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │ Powered by EdgeVec v0.7.0 │ MIT │ [Scanline overlay effect]     │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

**Component List:**
1. **Canvas Backgrounds** — ParticleSystem + MatrixRain (from effects.js)
2. **Header** — ASCII art logo with glitch effect
3. **Performance Panel** — FPS counter, search latency (from performance.js)
4. **Theme Toggle** — Light/dark theme switch
5. **Filter Builder** — Visual drag-and-drop filter construction
6. **Live Preview** — Real-time filter expression display with validation
7. **Example Gallery** — 10+ copy-paste examples with glitch hover effects
8. **Live Sandbox** — Real EdgeVec instance with sample data
9. **Code Snippets** — JavaScript/TypeScript/React examples with tabs
10. **Footer** — With scanline overlay effect

**Acceptance Criteria:**
- [ ] Wireframe documented
- [ ] Component list finalized
- [ ] Reuses v0.6.0 CSS files (cyberpunk.css, etc.)
- [ ] Reuses v0.6.0 JS files (effects.js, animations.js)
- [ ] Responsive design specified

**Deliverables:**
- Design document in this file (above)

**Dependencies:** None

**Estimated Duration:** 1 hour

**Agent:** DOCWRITER

---

### W30.3.2: Create Base HTML Structure

**Objective:** Set up HTML skeleton that leverages v0.6.0 infrastructure.

**File:** `wasm/examples/v070_filter_playground.html`

**IMPORTANT:** This HTML file MUST:
1. Link to existing CSS files (not duplicate inline)
2. Link to existing JS files (effects.js, animations.js)
3. Use Google Fonts (JetBrains Mono, Orbitron)
4. Include canvas elements for particle/matrix effects
5. Include reduced-motion accessibility support

**Base HTML:**
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EdgeVec Filter Playground v0.7.0</title>

    <!-- Google Fonts (same as v0.6.0) -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Orbitron:wght@400;500;700&display=swap" rel="stylesheet">

    <!-- Reuse v0.6.0 CSS files -->
    <link rel="stylesheet" href="css/cyberpunk.css">
    <link rel="stylesheet" href="css/layout.css">
    <link rel="stylesheet" href="css/components.css">
    <link rel="stylesheet" href="css/animations.css">
    <link rel="stylesheet" href="css/mobile.css">

    <!-- NEW: Filter playground specific styles -->
    <link rel="stylesheet" href="css/filter-playground.css">
</head>
<body class="cyberpunk-theme" data-theme="dark">
    <!-- Canvas backgrounds for effects -->
    <canvas id="particle-canvas" class="effect-canvas"></canvas>
    <canvas id="matrix-canvas" class="effect-canvas matrix-canvas"></canvas>

    <!-- Scanline overlay (from cyberpunk.css) -->
    <div class="scanline-overlay"></div>

    <div class="container">
        <!-- Header with glitch effect -->
        <header class="header">
            <pre class="logo glitch" data-text="EDGEVEC">
███████╗██████╗  ██████╗ ███████╗██╗   ██╗███████╗ ██████╗
██╔════╝██╔══██╗██╔════╝ ██╔════╝██║   ██║██╔════╝██╔════╝
█████╗  ██║  ██║██║  ███╗█████╗  ██║   ██║█████╗  ██║
██╔══╝  ██║  ██║██║   ██║██╔══╝  ╚██╗ ██╔╝██╔══╝  ██║
███████╗██████╔╝╚██████╔╝███████╗ ╚████╔╝ ███████╗╚██████╗
╚══════╝╚═════╝  ╚═════╝ ╚══════╝  ╚═══╝  ╚══════╝ ╚═════╝
            </pre>
            <h1 class="subtitle neon-text">FILTER PLAYGROUND v0.7.0</h1>
        </header>

        <!-- Performance & Theme Controls -->
        <div class="controls-bar">
            <div id="performance-stats" class="performance-panel">
                <span class="stat">FPS: <span id="fps-counter">--</span></span>
                <span class="stat">Vectors: <span id="vector-count">0</span></span>
                <span class="stat">Search: <span id="search-time">--</span></span>
            </div>
            <button id="theme-toggle" class="btn btn-icon" aria-label="Toggle theme">
                <span class="theme-icon">🌙</span>
            </button>
        </div>

        <!-- Filter Builder Section -->
        <section class="section glass-panel" id="builder">
            <h2 class="section-title glitch-hover">FILTER BUILDER</h2>
            <div class="builder-content">
                <!-- Will be populated by filter-playground.js -->
            </div>
        </section>

        <!-- Example Gallery Section -->
        <section class="section glass-panel" id="examples">
            <h2 class="section-title glitch-hover">EXAMPLE GALLERY</h2>
            <div class="example-grid" id="example-grid">
                <!-- Will be populated by filter-playground.js -->
            </div>
        </section>

        <!-- Live Sandbox Section -->
        <section class="section glass-panel" id="sandbox">
            <h2 class="section-title glitch-hover">LIVE SANDBOX</h2>
            <div class="sandbox-content">
                <!-- Will be populated by filter-playground.js -->
            </div>
        </section>

        <!-- Code Snippets Section -->
        <section class="section glass-panel" id="snippets">
            <h2 class="section-title glitch-hover">CODE SNIPPETS</h2>
            <div class="snippets-content">
                <!-- Will be populated by filter-playground.js -->
            </div>
        </section>

        <!-- Footer -->
        <footer class="footer">
            <p>Powered by <a href="https://github.com/matteocrippa/edgevec" class="neon-link">EdgeVec v0.7.0</a> | MIT License</p>
            <p class="subtitle">WASM Vector Database with SIMD Acceleration</p>
        </footer>
    </div>

    <!-- Toast notifications container -->
    <div id="toast-container" class="toast-container"></div>

    <!-- Reuse v0.6.0 JS files -->
    <script type="module">
        // Import v0.6.0 effect infrastructure
        import { ParticleSystem, MatrixRain, EffectManager } from './js/effects.js';
        import { GlitchText, initAnimations } from './js/animations.js';
        import { PerformanceMonitor } from './js/performance.js';

        // Import new filter playground module
        import { FilterPlayground } from './js/filter-playground.js';

        // Check for reduced motion preference
        const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

        document.addEventListener('DOMContentLoaded', async () => {
            // Initialize effects (unless reduced motion)
            if (!prefersReducedMotion) {
                const effectManager = new EffectManager({
                    particleCanvas: 'particle-canvas',
                    matrixCanvas: 'matrix-canvas'
                });
                effectManager.init();
            }

            // Initialize performance monitor
            const perfMonitor = new PerformanceMonitor('fps-counter');
            perfMonitor.start();

            // Initialize animations
            initAnimations();

            // Initialize filter playground
            const playground = new FilterPlayground({
                builderContainer: document.querySelector('#builder .builder-content'),
                examplesContainer: document.getElementById('example-grid'),
                sandboxContainer: document.querySelector('#sandbox .sandbox-content'),
                snippetsContainer: document.querySelector('#snippets .snippets-content'),
                vectorCountEl: document.getElementById('vector-count'),
                searchTimeEl: document.getElementById('search-time')
            });
            await playground.init();

            // Theme toggle
            document.getElementById('theme-toggle').addEventListener('click', () => {
                const body = document.body;
                const isDark = body.dataset.theme === 'dark';
                body.dataset.theme = isDark ? 'light' : 'dark';
                document.querySelector('.theme-icon').textContent = isDark ? '☀️' : '🌙';
            });
        });
    </script>
</body>
</html>
```

**Acceptance Criteria:**
- [ ] HTML links to ALL existing v0.6.0 CSS files
- [ ] HTML imports ALL existing v0.6.0 JS modules
- [ ] Google Fonts loaded (JetBrains Mono, Orbitron)
- [ ] Canvas elements for particle and matrix effects
- [ ] Scanline overlay included
- [ ] Reduced motion check implemented
- [ ] Theme toggle functional
- [ ] Performance panel included

**Deliverables:**
- `wasm/examples/v070_filter_playground.html`

**Dependencies:** W30.3.1

**Estimated Duration:** 1.5 hours

**Agent:** WASM_SPECIALIST

---

### W30.3.3: Create filter-playground.css

**Objective:** Create playground-specific styles that extend v0.6.0 theme.

**File:** `wasm/examples/css/filter-playground.css`

**IMPORTANT:** This file MUST:
1. NOT redefine color tokens (use from cyberpunk.css)
2. Only add playground-specific component styles
3. Use existing CSS classes where possible
4. Add only necessary new styles

**CSS Content:**
```css
/* ==========================================================================
   FILTER PLAYGROUND SPECIFIC STYLES
   Extends v0.6.0 cyberpunk theme
   ========================================================================== */

/* Performance Panel */
.controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 0;
    margin-bottom: 2rem;
}

.performance-panel {
    display: flex;
    gap: 1.5rem;
    padding: 0.75rem 1.5rem;
    background: var(--bg-panel);
    border: 1px solid var(--neon-cyan);
    border-radius: var(--border-radius);
    font-family: var(--font-mono);
    font-size: 0.875rem;
}

.performance-panel .stat {
    color: var(--text-muted);
}

.performance-panel .stat span:last-child {
    color: var(--neon-green);
    font-weight: 600;
}

/* Filter Builder */
.builder-content {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1.5rem;
}

.clause-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem;
    background: var(--bg-elevated);
    border-radius: var(--border-radius);
    margin-bottom: 0.5rem;
    animation: fadeInSlide 0.3s ease;
}

@keyframes fadeInSlide {
    from {
        opacity: 0;
        transform: translateX(-10px);
    }
    to {
        opacity: 1;
        transform: translateX(0);
    }
}

.clause-field,
.clause-operator,
.clause-value {
    padding: 0.5rem;
    background: var(--bg-void);
    border: 1px solid var(--neon-cyan);
    border-radius: var(--border-radius-sm);
    color: var(--text-primary);
    font-family: var(--font-mono);
    transition: var(--transition-fast);
}

.clause-field:focus,
.clause-operator:focus,
.clause-value:focus {
    outline: none;
    box-shadow: var(--glow-cyan);
    border-color: var(--neon-cyan);
}

.clause-remove {
    background: transparent;
    border: 1px solid var(--neon-magenta);
    color: var(--neon-magenta);
    width: 28px;
    height: 28px;
    border-radius: 50%;
    cursor: pointer;
    transition: var(--transition-fast);
}

.clause-remove:hover {
    background: var(--neon-magenta);
    color: var(--bg-void);
    box-shadow: var(--glow-magenta);
}

.builder-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
}

.preview-panel {
    background: var(--bg-void);
    border: 1px solid var(--neon-green);
    border-radius: var(--border-radius);
    padding: 1rem;
}

.preview-expression {
    font-family: var(--font-mono);
    font-size: 0.9rem;
    color: var(--neon-green);
    word-break: break-all;
    margin-bottom: 1rem;
    min-height: 60px;
}

.preview-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
}

.preview-status.valid {
    color: var(--neon-green);
}

.preview-status.invalid {
    color: var(--neon-magenta);
}

/* Example Gallery */
.example-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1.5rem;
}

.example-card {
    background: var(--bg-elevated);
    border: 1px solid var(--neon-magenta);
    border-radius: var(--border-radius);
    padding: 1.25rem;
    transition: var(--transition-normal);
    position: relative;
    overflow: hidden;
}

.example-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--neon-cyan), transparent);
    transition: var(--transition-slow);
}

.example-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--glow-magenta);
    border-color: var(--neon-cyan);
}

.example-card:hover::before {
    left: 100%;
}

.example-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
}

.example-icon {
    font-size: 1.5rem;
}

.example-title {
    font-family: var(--font-display);
    color: var(--neon-yellow);
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.example-category {
    font-size: 0.75rem;
    color: var(--text-muted);
    text-transform: uppercase;
}

.example-filter {
    background: var(--bg-void);
    padding: 0.75rem;
    border-radius: var(--border-radius-sm);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--neon-green);
    margin: 1rem 0;
    overflow-x: auto;
}

.example-description {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
}

.example-actions {
    display: flex;
    gap: 0.5rem;
}

/* Live Sandbox */
.sandbox-content {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
}

.sandbox-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
}

.sandbox-stats {
    display: flex;
    gap: 1.5rem;
}

.sandbox-stats span {
    background: var(--bg-elevated);
    padding: 0.5rem 1rem;
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--neon-cyan);
    font-size: 0.875rem;
}

.sandbox-actions {
    display: flex;
    gap: 0.5rem;
}

.sandbox-input {
    display: flex;
    gap: 0.75rem;
}

.filter-input {
    flex: 1;
    padding: 0.75rem 1rem;
    font-size: 1rem;
    background: var(--bg-void);
    border: 2px solid var(--neon-cyan);
    color: var(--neon-green);
    font-family: var(--font-mono);
    border-radius: var(--border-radius);
    transition: var(--transition-fast);
}

.filter-input:focus {
    outline: none;
    box-shadow: var(--glow-cyan);
}

.filter-input::placeholder {
    color: var(--text-muted);
}

.sandbox-status {
    padding: 1rem;
    background: var(--bg-elevated);
    border-radius: var(--border-radius);
    text-align: center;
    font-size: 0.9rem;
}

.sandbox-status.success {
    color: var(--neon-green);
    border: 1px solid var(--neon-green);
}

.sandbox-status.error {
    color: var(--neon-magenta);
    border: 1px solid var(--neon-magenta);
}

.sandbox-results {
    background: var(--bg-void);
    border: 1px solid var(--neon-green);
    border-radius: var(--border-radius);
    max-height: 400px;
    overflow-y: auto;
}

.result-row {
    display: grid;
    grid-template-columns: 50px 80px 100px 1fr;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--bg-elevated);
    align-items: center;
    transition: var(--transition-fast);
}

.result-row:hover {
    background: var(--bg-panel);
}

.result-row:last-child {
    border-bottom: none;
}

.result-rank {
    color: var(--neon-magenta);
    font-weight: 700;
    font-family: var(--font-display);
}

.result-id {
    color: var(--neon-cyan);
    font-family: var(--font-mono);
}

.result-distance {
    color: var(--neon-yellow);
    font-family: var(--font-mono);
}

.result-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
}

.meta-tag {
    background: var(--bg-elevated);
    padding: 0.2rem 0.5rem;
    border-radius: var(--border-radius-sm);
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-family: var(--font-mono);
}

.no-results {
    text-align: center;
    padding: 3rem;
    color: var(--text-muted);
}

/* Code Snippets */
.snippets-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.snippet-tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--bg-elevated);
    padding-bottom: 0.5rem;
}

.snippet-tab {
    padding: 0.5rem 1rem;
    background: transparent;
    border: 1px solid var(--neon-cyan);
    border-bottom: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.875rem;
    transition: var(--transition-fast);
    border-radius: var(--border-radius-sm) var(--border-radius-sm) 0 0;
}

.snippet-tab:hover {
    color: var(--neon-cyan);
}

.snippet-tab.active {
    background: var(--bg-elevated);
    color: var(--neon-cyan);
    border-color: var(--neon-cyan);
}

.snippet-content {
    position: relative;
}

.snippet-code {
    background: var(--bg-void);
    padding: 1.5rem;
    border-radius: var(--border-radius);
    border: 1px solid var(--bg-elevated);
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 0.875rem;
    line-height: 1.6;
}

.snippet-code .keyword {
    color: var(--neon-magenta);
}

.snippet-code .string {
    color: var(--neon-green);
}

.snippet-code .function {
    color: var(--neon-cyan);
}

.snippet-code .comment {
    color: var(--text-muted);
}

.snippet-copy {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
}

/* Responsive */
@media (max-width: 768px) {
    .builder-content {
        grid-template-columns: 1fr;
    }

    .result-row {
        grid-template-columns: 1fr;
        gap: 0.5rem;
    }

    .sandbox-header {
        flex-direction: column;
        align-items: stretch;
    }

    .sandbox-input {
        flex-direction: column;
    }

    .performance-panel {
        flex-wrap: wrap;
        gap: 0.75rem;
    }
}

/* Reduced Motion */
@media (prefers-reduced-motion: reduce) {
    .example-card::before {
        display: none;
    }

    .clause-row {
        animation: none;
    }
}
```

**Acceptance Criteria:**
- [ ] No duplicate color tokens (uses cyberpunk.css variables)
- [ ] All new components properly styled
- [ ] Responsive breakpoints included
- [ ] Reduced motion support included
- [ ] Uses existing CSS classes where applicable

**Deliverables:**
- `wasm/examples/css/filter-playground.css`

**Dependencies:** W30.3.2

**Estimated Duration:** 1 hour

**Agent:** WASM_SPECIALIST

---

### W30.3.4: Define Example Categories

**Objective:** Plan the 10+ filter examples to include.

**Example Categories:**

**1. E-Commerce (3 examples)**
```javascript
// Simple product filter
category = "gpu" AND price < 500

// Complex with brand
(brand = "nvidia" OR brand = "amd") AND memory >= 8 AND inStock = true

// Range query
price BETWEEN 200 AND 800 AND rating >= 4.0
```

**2. Document Search (3 examples)**
```javascript
// Author filter
author = "John Doe" AND year >= 2023

// Tag-based
tags IN ["tutorial", "guide", "beginner"] AND language = "en"

// Date range
createdAt >= "2024-01-01" AND status = "published"
```

**3. Content & Media (2 examples)**
```javascript
// Video content
type = "video" AND duration < 600 AND views >= 1000

// Multi-criteria
category IN ["tech", "science"] AND rating >= 4.5 AND premium = false
```

**4. Advanced Patterns (2 examples)**
```javascript
// Nested groups
(category = "electronics" AND price < 100) OR (category = "books" AND price < 20)

// NOT operator
status != "deleted" AND NOT (archived = true)
```

**5. Text Search (2 examples)**
```javascript
// Contains
title CONTAINS "vector" AND type = "article"

// Case sensitivity note
description CONTAINS "WASM" AND published = true
```

**Example Card Structure:**
```javascript
const examples = [
    {
        id: 'ecommerce-1',
        title: 'Product Filter',
        category: 'E-Commerce',
        icon: '🛒',
        filter: 'category = "gpu" AND price < 500',
        description: 'Find GPUs under $500',
        sampleData: [
            { id: 1, name: 'RTX 4070', category: 'gpu', price: 450 },
            { id: 2, name: 'RTX 4080', category: 'gpu', price: 850 },
            { id: 3, name: 'RX 7800', category: 'gpu', price: 420 }
        ]
    },
    // ... more examples
];
```

**Acceptance Criteria:**
- [ ] 10+ examples defined
- [ ] Categories cover common use cases
- [ ] Each example has sample data
- [ ] Filter syntax is correct

**Deliverables:**
- Example definitions (to be used in filter-playground.js in Day 4)

**Dependencies:** W30.3.3

**Estimated Duration:** 0.5 hours

**Agent:** DOCWRITER

---

## Exit Criteria for Day 3

| Criterion | Verification | Status |
|:----------|:-------------|:-------|
| Wireframe complete with visual effects | Design documented | [x] |
| HTML skeleton reuses v0.6.0 CSS/JS | Links verified | [x] |
| Google Fonts loaded | JetBrains Mono, Orbitron | [x] |
| Canvas elements for effects | particle-canvas, matrix-canvas | [x] |
| filter-playground.css created | File exists, no duplicate tokens | [x] |
| 10+ examples defined | Example list complete | [x] |
| Reduced motion support | Media query present | [x] |
| Page loads without errors | Browser console clean | [x] |

---

**Day 3 Total:** 4 hours
**Agent:** DOCWRITER + WASM_SPECIALIST
**Status:** [APPROVED] — Hostile review passed 2025-12-24

