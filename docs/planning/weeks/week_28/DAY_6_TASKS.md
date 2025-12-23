# Week 28 Day 6: Cyberpunk UI Framework

**Date:** 2025-12-28 (Saturday)
**Focus:** Browser Demo — Cyberpunk Design System + Core UI
**Estimated Duration:** 8 hours
**Agent:** WASM_SPECIALIST + DOCWRITER
**Prerequisite:** Days 1-5 complete (WASM bindings functional)

---

## Executive Summary

Day 6 builds the **spectacular cyberpunk-themed** browser demo UI framework. This addresses HOSTILE_REVIEWER finding [C1] requiring 2 full days for "spectacular" UI/UX with advanced animations.

**Design Philosophy:**
- Cyberpunk aesthetic: Neon colors, glitch effects, dark terminals
- High-contrast for accessibility
- Performant CSS animations (GPU-accelerated)
- Mobile-first responsive design

---

## Task Breakdown

### W28.6.1: Cyberpunk CSS Design System (3 hours)

**Objective:** Create a complete CSS design system with cyberpunk theme.

**Deliverables:**
1. CSS custom properties for theme
2. Neon glow utilities
3. Glitch text animation
4. Scanline overlay effect
5. Terminal-style components

**Implementation:**

```css
/* wasm/examples/css/cyberpunk.css */

/* ==========================================================================
   EDGEVEC CYBERPUNK DESIGN SYSTEM
   A high-performance, accessibility-first cyberpunk theme
   ========================================================================== */

/* --------------------------------------------------------------------------
   CSS Custom Properties (Design Tokens)
   -------------------------------------------------------------------------- */
:root {
  /* Neon Color Palette */
  --neon-cyan: #00ffff;
  --neon-magenta: #ff00ff;
  --neon-yellow: #ffff00;
  --neon-green: #39ff14;
  --neon-orange: #ff6600;
  --neon-pink: #ff1493;

  /* Backgrounds */
  --bg-void: #0a0a0f;
  --bg-terminal: #0d1117;
  --bg-panel: #161b22;
  --bg-elevated: #21262d;

  /* Text */
  --text-primary: #e6edf3;
  --text-secondary: #8b949e;
  --text-muted: #484f58;

  /* Gradients */
  --gradient-neon: linear-gradient(135deg, var(--neon-cyan), var(--neon-magenta));
  --gradient-terminal: linear-gradient(180deg, var(--bg-terminal) 0%, var(--bg-void) 100%);

  /* Shadows */
  --glow-cyan: 0 0 10px var(--neon-cyan), 0 0 20px var(--neon-cyan), 0 0 40px var(--neon-cyan);
  --glow-magenta: 0 0 10px var(--neon-magenta), 0 0 20px var(--neon-magenta), 0 0 40px var(--neon-magenta);
  --glow-green: 0 0 10px var(--neon-green), 0 0 20px var(--neon-green);

  /* Typography */
  --font-mono: 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace;
  --font-display: 'Orbitron', 'Rajdhani', sans-serif;

  /* Spacing Scale */
  --space-xs: 0.25rem;
  --space-sm: 0.5rem;
  --space-md: 1rem;
  --space-lg: 1.5rem;
  --space-xl: 2rem;
  --space-2xl: 3rem;

  /* Border Radius */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;

  /* Transitions */
  --transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-base: 300ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-slow: 500ms cubic-bezier(0.4, 0, 0.2, 1);

  /* Z-Index Scale */
  --z-base: 0;
  --z-overlay: 100;
  --z-modal: 200;
  --z-toast: 300;
}

/* --------------------------------------------------------------------------
   Base Reset & Typography
   -------------------------------------------------------------------------- */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html {
  font-size: 16px;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  font-family: var(--font-mono);
  background: var(--gradient-terminal);
  color: var(--text-primary);
  line-height: 1.6;
  min-height: 100vh;
  overflow-x: hidden;
}

/* Scanline Overlay Effect */
body::before {
  content: '';
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 255, 255, 0.03) 2px,
    rgba(0, 255, 255, 0.03) 4px
  );
  z-index: var(--z-overlay);
}

/* --------------------------------------------------------------------------
   Neon Text Utilities
   -------------------------------------------------------------------------- */
.neon-text {
  text-shadow: var(--glow-cyan);
  color: var(--neon-cyan);
}

.neon-text--magenta {
  text-shadow: var(--glow-magenta);
  color: var(--neon-magenta);
}

.neon-text--green {
  text-shadow: var(--glow-green);
  color: var(--neon-green);
}

/* Animated Neon Pulse */
.neon-pulse {
  animation: neonPulse 2s ease-in-out infinite;
}

@keyframes neonPulse {
  0%, 100% {
    text-shadow:
      0 0 5px var(--neon-cyan),
      0 0 10px var(--neon-cyan),
      0 0 20px var(--neon-cyan);
    opacity: 1;
  }
  50% {
    text-shadow:
      0 0 2px var(--neon-cyan),
      0 0 5px var(--neon-cyan),
      0 0 10px var(--neon-cyan);
    opacity: 0.8;
  }
}

/* --------------------------------------------------------------------------
   Glitch Effect
   -------------------------------------------------------------------------- */
.glitch {
  position: relative;
  animation: glitchSkew 1s infinite linear alternate-reverse;
}

.glitch::before,
.glitch::after {
  content: attr(data-text);
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
}

.glitch::before {
  left: 2px;
  text-shadow: -2px 0 var(--neon-magenta);
  clip: rect(44px, 450px, 56px, 0);
  animation: glitchAnim 5s infinite linear alternate-reverse;
}

.glitch::after {
  left: -2px;
  text-shadow: -2px 0 var(--neon-cyan);
  clip: rect(44px, 450px, 56px, 0);
  animation: glitchAnim2 5s infinite linear alternate-reverse;
}

@keyframes glitchSkew {
  0% { transform: skew(0deg); }
  20% { transform: skew(0deg); }
  21% { transform: skew(1deg); }
  22% { transform: skew(-1deg); }
  23% { transform: skew(0deg); }
  100% { transform: skew(0deg); }
}

@keyframes glitchAnim {
  0% { clip: rect(31px, 9999px, 94px, 0); transform: skew(0.5deg); }
  5% { clip: rect(70px, 9999px, 71px, 0); transform: skew(0.4deg); }
  10% { clip: rect(29px, 9999px, 24px, 0); transform: skew(0.6deg); }
  15% { clip: rect(3px, 9999px, 42px, 0); transform: skew(0.3deg); }
  20% { clip: rect(51px, 9999px, 7px, 0); transform: skew(0.5deg); }
  25% { clip: rect(13px, 9999px, 89px, 0); transform: skew(0.4deg); }
  30% { clip: rect(64px, 9999px, 26px, 0); transform: skew(0.6deg); }
  35% { clip: rect(41px, 9999px, 58px, 0); transform: skew(0.3deg); }
  40% { clip: rect(87px, 9999px, 75px, 0); transform: skew(0.5deg); }
  45% { clip: rect(22px, 9999px, 33px, 0); transform: skew(0.4deg); }
  50% { clip: rect(56px, 9999px, 12px, 0); transform: skew(0.6deg); }
  55% { clip: rect(8px, 9999px, 67px, 0); transform: skew(0.3deg); }
  60% { clip: rect(95px, 9999px, 44px, 0); transform: skew(0.5deg); }
  65% { clip: rect(18px, 9999px, 82px, 0); transform: skew(0.4deg); }
  70% { clip: rect(39px, 9999px, 55px, 0); transform: skew(0.6deg); }
  75% { clip: rect(73px, 9999px, 28px, 0); transform: skew(0.3deg); }
  80% { clip: rect(5px, 9999px, 91px, 0); transform: skew(0.5deg); }
  85% { clip: rect(48px, 9999px, 36px, 0); transform: skew(0.4deg); }
  90% { clip: rect(62px, 9999px, 19px, 0); transform: skew(0.6deg); }
  95% { clip: rect(84px, 9999px, 79px, 0); transform: skew(0.3deg); }
  100% { clip: rect(27px, 9999px, 63px, 0); transform: skew(0.5deg); }
}

@keyframes glitchAnim2 {
  0% { clip: rect(65px, 9999px, 99px, 0); transform: skew(0.5deg); }
  5% { clip: rect(12px, 9999px, 77px, 0); transform: skew(-0.4deg); }
  10% { clip: rect(45px, 9999px, 23px, 0); transform: skew(0.3deg); }
  15% { clip: rect(88px, 9999px, 54px, 0); transform: skew(-0.6deg); }
  20% { clip: rect(33px, 9999px, 8px, 0); transform: skew(0.4deg); }
  25% { clip: rect(76px, 9999px, 41px, 0); transform: skew(-0.3deg); }
  30% { clip: rect(19px, 9999px, 86px, 0); transform: skew(0.5deg); }
  35% { clip: rect(57px, 9999px, 15px, 0); transform: skew(-0.4deg); }
  40% { clip: rect(4px, 9999px, 69px, 0); transform: skew(0.3deg); }
  45% { clip: rect(92px, 9999px, 38px, 0); transform: skew(-0.6deg); }
  50% { clip: rect(26px, 9999px, 81px, 0); transform: skew(0.4deg); }
  55% { clip: rect(61px, 9999px, 47px, 0); transform: skew(-0.3deg); }
  60% { clip: rect(11px, 9999px, 94px, 0); transform: skew(0.5deg); }
  65% { clip: rect(79px, 9999px, 22px, 0); transform: skew(-0.4deg); }
  70% { clip: rect(34px, 9999px, 66px, 0); transform: skew(0.3deg); }
  75% { clip: rect(97px, 9999px, 3px, 0); transform: skew(-0.6deg); }
  80% { clip: rect(43px, 9999px, 58px, 0); transform: skew(0.4deg); }
  85% { clip: rect(7px, 9999px, 85px, 0); transform: skew(-0.3deg); }
  90% { clip: rect(71px, 9999px, 31px, 0); transform: skew(0.5deg); }
  95% { clip: rect(52px, 9999px, 74px, 0); transform: skew(-0.4deg); }
  100% { clip: rect(16px, 9999px, 49px, 0); transform: skew(0.3deg); }
}

/* --------------------------------------------------------------------------
   Terminal Components
   -------------------------------------------------------------------------- */
.terminal {
  background: var(--bg-terminal);
  border: 1px solid var(--neon-cyan);
  border-radius: var(--radius-md);
  box-shadow:
    0 0 10px rgba(0, 255, 255, 0.2),
    inset 0 0 20px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.terminal__header {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--neon-cyan);
}

.terminal__dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--neon-magenta);
}

.terminal__dot--yellow { background: var(--neon-yellow); }
.terminal__dot--green { background: var(--neon-green); }

.terminal__title {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-left: auto;
}

.terminal__body {
  padding: var(--space-md);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  line-height: 1.8;
}

.terminal__prompt {
  color: var(--neon-green);
}

.terminal__prompt::before {
  content: '> ';
  color: var(--neon-cyan);
}

/* Typing Cursor Animation */
.terminal__cursor {
  display: inline-block;
  width: 8px;
  height: 1.2em;
  background: var(--neon-cyan);
  margin-left: 2px;
  animation: blink 1s step-end infinite;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

/* --------------------------------------------------------------------------
   Cyberpunk Buttons
   -------------------------------------------------------------------------- */
.btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--neon-cyan);
  background: transparent;
  border: 2px solid var(--neon-cyan);
  cursor: pointer;
  overflow: hidden;
  transition: all var(--transition-fast);
}

.btn::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(0, 255, 255, 0.4),
    transparent
  );
  transition: left var(--transition-base);
}

.btn:hover::before {
  left: 100%;
}

.btn:hover {
  box-shadow: var(--glow-cyan);
  transform: translateY(-2px);
}

.btn:active {
  transform: translateY(0);
}

.btn--magenta {
  color: var(--neon-magenta);
  border-color: var(--neon-magenta);
}

.btn--magenta:hover {
  box-shadow: var(--glow-magenta);
}

.btn--filled {
  background: var(--neon-cyan);
  color: var(--bg-void);
}

.btn--filled:hover {
  background: var(--neon-magenta);
  border-color: var(--neon-magenta);
}

/* --------------------------------------------------------------------------
   Input Fields
   -------------------------------------------------------------------------- */
.input {
  width: 100%;
  padding: var(--space-sm) var(--space-md);
  font-family: var(--font-mono);
  font-size: 1rem;
  color: var(--text-primary);
  background: var(--bg-terminal);
  border: 1px solid var(--text-muted);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.input:focus {
  outline: none;
  border-color: var(--neon-cyan);
  box-shadow: 0 0 10px rgba(0, 255, 255, 0.3);
}

.input::placeholder {
  color: var(--text-muted);
}

/* --------------------------------------------------------------------------
   Cards
   -------------------------------------------------------------------------- */
.card {
  position: relative;
  background: var(--bg-panel);
  border: 1px solid var(--text-muted);
  border-radius: var(--radius-md);
  padding: var(--space-lg);
  transition: all var(--transition-base);
}

.card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--gradient-neon);
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.card:hover {
  border-color: var(--neon-cyan);
  transform: translateY(-4px);
}

.card:hover::before {
  opacity: 1;
}

.card__title {
  font-family: var(--font-display);
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--neon-cyan);
  margin-bottom: var(--space-sm);
}

.card__content {
  color: var(--text-secondary);
}

/* --------------------------------------------------------------------------
   Progress Bar
   -------------------------------------------------------------------------- */
.progress {
  height: 8px;
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.progress__bar {
  height: 100%;
  background: var(--gradient-neon);
  border-radius: var(--radius-sm);
  transition: width var(--transition-slow);
  position: relative;
}

.progress__bar::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%
  );
  animation: progressShine 2s infinite;
}

@keyframes progressShine {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}

/* --------------------------------------------------------------------------
   Loading Skeleton
   -------------------------------------------------------------------------- */
.skeleton {
  background: linear-gradient(
    90deg,
    var(--bg-elevated) 0%,
    var(--bg-panel) 50%,
    var(--bg-elevated) 100%
  );
  background-size: 200% 100%;
  animation: skeletonShimmer 1.5s infinite;
  border-radius: var(--radius-sm);
}

@keyframes skeletonShimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.skeleton--text {
  height: 1rem;
  margin-bottom: var(--space-sm);
}

.skeleton--title {
  height: 1.5rem;
  width: 60%;
  margin-bottom: var(--space-md);
}

.skeleton--avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
}

/* --------------------------------------------------------------------------
   Toast Notifications
   -------------------------------------------------------------------------- */
.toast {
  position: fixed;
  bottom: var(--space-lg);
  right: var(--space-lg);
  padding: var(--space-md) var(--space-lg);
  background: var(--bg-panel);
  border-left: 4px solid var(--neon-cyan);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  transform: translateX(120%);
  transition: transform var(--transition-base);
  z-index: var(--z-toast);
}

.toast--visible {
  transform: translateX(0);
}

.toast--success { border-left-color: var(--neon-green); }
.toast--warning { border-left-color: var(--neon-yellow); }
.toast--error { border-left-color: var(--neon-magenta); }

/* --------------------------------------------------------------------------
   Responsive Grid
   -------------------------------------------------------------------------- */
.grid {
  display: grid;
  gap: var(--space-lg);
}

.grid--2 { grid-template-columns: repeat(2, 1fr); }
.grid--3 { grid-template-columns: repeat(3, 1fr); }
.grid--4 { grid-template-columns: repeat(4, 1fr); }

@media (max-width: 1024px) {
  .grid--4 { grid-template-columns: repeat(2, 1fr); }
  .grid--3 { grid-template-columns: repeat(2, 1fr); }
}

@media (max-width: 640px) {
  .grid--4,
  .grid--3,
  .grid--2 { grid-template-columns: 1fr; }
}

/* --------------------------------------------------------------------------
   Layout Containers
   -------------------------------------------------------------------------- */
.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 var(--space-lg);
}

.section {
  padding: var(--space-2xl) 0;
}

/* --------------------------------------------------------------------------
   Accessibility: Reduced Motion
   -------------------------------------------------------------------------- */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}

/* Focus Visible for Keyboard Navigation */
:focus-visible {
  outline: 2px solid var(--neon-cyan);
  outline-offset: 2px;
}
```

**Exit Criteria:**
- CSS file created with all design tokens
- Neon text utilities work
- Glitch effect animates correctly
- Terminal component renders
- Buttons have hover effects
- Accessibility: prefers-reduced-motion respected

---

### W28.6.2: Responsive Layout + Dark/Light Mode (2 hours)

**Objective:** Create the main demo page layout with theme toggle.

**Deliverables:**
1. Header with logo and theme toggle
2. Main content grid
3. Sidebar for controls
4. Footer with stats
5. Theme persistence in localStorage

**Implementation:**

```html
<!-- wasm/examples/v060_cyberpunk_demo.html -->
<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="EdgeVec v0.6.0 Demo - Cyberpunk Vector Search">
  <title>EdgeVec v0.6.0 // NEURAL_SEARCH_INTERFACE</title>

  <!-- Fonts -->
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;700&family=Orbitron:wght@700;900&display=swap" rel="stylesheet">

  <!-- Styles -->
  <link rel="stylesheet" href="css/cyberpunk.css">
  <link rel="stylesheet" href="css/layout.css">
  <link rel="stylesheet" href="css/components.css">
  <link rel="stylesheet" href="css/animations.css">
</head>
<body>
  <!-- Noise Overlay -->
  <div class="noise-overlay" aria-hidden="true"></div>

  <!-- Header -->
  <header class="header">
    <div class="container header__inner">
      <div class="header__logo">
        <span class="glitch neon-text" data-text="EDGEVEC">EDGEVEC</span>
        <span class="header__version">v0.6.0</span>
      </div>

      <nav class="header__nav" role="navigation" aria-label="Main navigation">
        <a href="#search" class="header__link">SEARCH</a>
        <a href="#benchmark" class="header__link">BENCHMARK</a>
        <a href="#memory" class="header__link">MEMORY</a>
      </nav>

      <div class="header__actions">
        <button
          class="theme-toggle"
          id="themeToggle"
          aria-label="Toggle dark/light mode"
        >
          <svg class="theme-toggle__icon theme-toggle__icon--dark" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"/>
          </svg>
          <svg class="theme-toggle__icon theme-toggle__icon--light" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"/>
          </svg>
        </button>

        <a href="https://github.com/user/edgevec" class="btn btn--small" target="_blank" rel="noopener">
          GITHUB
        </a>
      </div>
    </div>
  </header>

  <!-- Hero Section -->
  <section class="hero">
    <div class="container">
      <h1 class="hero__title glitch neon-pulse" data-text="NEURAL_SEARCH_INTERFACE">
        NEURAL_SEARCH_INTERFACE
      </h1>
      <p class="hero__subtitle">
        Binary Quantization + Metadata Filtering // <span class="neon-text--green">32x MEMORY REDUCTION</span>
      </p>

      <!-- Status Bar -->
      <div class="status-bar" id="statusBar" role="status" aria-live="polite">
        <div class="status-bar__item">
          <span class="status-bar__label">WASM</span>
          <span class="status-bar__value status-bar__value--loading" id="wasmStatus">LOADING...</span>
        </div>
        <div class="status-bar__item">
          <span class="status-bar__label">VECTORS</span>
          <span class="status-bar__value" id="vectorCount">0</span>
        </div>
        <div class="status-bar__item">
          <span class="status-bar__label">MEMORY</span>
          <span class="status-bar__value" id="memoryUsage">0 MB</span>
        </div>
      </div>
    </div>
  </section>

  <!-- Main Content -->
  <main class="main">
    <div class="container">
      <div class="layout">
        <!-- Sidebar: Controls -->
        <aside class="sidebar" role="complementary">
          <div class="terminal">
            <div class="terminal__header">
              <span class="terminal__dot"></span>
              <span class="terminal__dot terminal__dot--yellow"></span>
              <span class="terminal__dot terminal__dot--green"></span>
              <span class="terminal__title">CONTROL_PANEL</span>
            </div>
            <div class="terminal__body">
              <!-- Data Generation -->
              <div class="control-group">
                <label class="control-label">VECTOR_COUNT</label>
                <input
                  type="range"
                  id="vectorSlider"
                  class="slider"
                  min="100"
                  max="10000"
                  value="1000"
                  step="100"
                >
                <output id="vectorSliderValue" class="control-output">1000</output>
              </div>

              <button class="btn btn--filled" id="generateBtn">
                GENERATE_DATA
              </button>

              <hr class="divider">

              <!-- Search Mode -->
              <div class="control-group">
                <label class="control-label">SEARCH_MODE</label>
                <div class="radio-group" role="radiogroup" aria-label="Search mode">
                  <label class="radio">
                    <input type="radio" name="searchMode" value="f32" checked>
                    <span class="radio__label">F32 (ACCURATE)</span>
                  </label>
                  <label class="radio">
                    <input type="radio" name="searchMode" value="bq">
                    <span class="radio__label">BQ (FAST)</span>
                  </label>
                  <label class="radio">
                    <input type="radio" name="searchMode" value="hybrid">
                    <span class="radio__label">HYBRID (BALANCED)</span>
                  </label>
                </div>
              </div>

              <!-- Filter Expression -->
              <div class="control-group">
                <label class="control-label" for="filterInput">FILTER_EXPRESSION</label>
                <input
                  type="text"
                  id="filterInput"
                  class="input"
                  placeholder='category == "tech"'
                >
              </div>

              <!-- K Value -->
              <div class="control-group">
                <label class="control-label" for="kValue">TOP_K_RESULTS</label>
                <input
                  type="number"
                  id="kValue"
                  class="input"
                  min="1"
                  max="100"
                  value="10"
                >
              </div>
            </div>
          </div>
        </aside>

        <!-- Main Panel: Results -->
        <section class="content" aria-label="Search results">
          <!-- Search Input -->
          <div class="search-box" id="searchSection">
            <div class="search-box__input-wrapper">
              <input
                type="text"
                id="searchInput"
                class="search-box__input"
                placeholder="ENTER_SEARCH_QUERY..."
                aria-label="Search query"
              >
              <button class="search-box__btn" id="searchBtn" aria-label="Execute search">
                <svg viewBox="0 0 24 24" class="search-box__icon" aria-hidden="true">
                  <path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
                </svg>
              </button>
            </div>
          </div>

          <!-- Results Grid -->
          <div class="results" id="resultsContainer">
            <!-- Placeholder for skeleton loading -->
            <div class="results__placeholder">
              <p class="neon-text">AWAITING_INPUT...</p>
              <p class="text-muted">Generate data and run a search to see results</p>
            </div>
          </div>
        </section>
      </div>
    </div>
  </main>

  <!-- Benchmark Section -->
  <section class="section" id="benchmark">
    <div class="container">
      <h2 class="section__title glitch" data-text="PERFORMANCE_METRICS">PERFORMANCE_METRICS</h2>

      <div class="grid grid--3">
        <div class="metric-card">
          <div class="metric-card__icon metric-card__icon--cyan">
            <svg viewBox="0 0 24 24"><path d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
          </div>
          <div class="metric-card__value" id="f32Latency">--</div>
          <div class="metric-card__label">F32 LATENCY (ms)</div>
        </div>

        <div class="metric-card">
          <div class="metric-card__icon metric-card__icon--magenta">
            <svg viewBox="0 0 24 24"><path d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
          </div>
          <div class="metric-card__value" id="bqLatency">--</div>
          <div class="metric-card__label">BQ LATENCY (ms)</div>
        </div>

        <div class="metric-card">
          <div class="metric-card__icon metric-card__icon--green">
            <svg viewBox="0 0 24 24"><path d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/></svg>
          </div>
          <div class="metric-card__value" id="speedup">--x</div>
          <div class="metric-card__label">SPEEDUP</div>
        </div>
      </div>

      <!-- Benchmark Chart -->
      <div class="chart-container" id="benchmarkChart" aria-label="Performance comparison chart">
        <!-- SVG chart will be rendered here -->
      </div>

      <button class="btn btn--magenta" id="runBenchmarkBtn">
        RUN_BENCHMARK
      </button>
    </div>
  </section>

  <!-- Memory Section -->
  <section class="section section--dark" id="memory">
    <div class="container">
      <h2 class="section__title glitch" data-text="MEMORY_PRESSURE">MEMORY_PRESSURE</h2>

      <div class="memory-gauge" id="memoryGauge" role="meter" aria-valuemin="0" aria-valuemax="100">
        <!-- SVG gauge will be rendered here -->
      </div>

      <div class="memory-stats" id="memoryStats">
        <div class="memory-stats__item">
          <span class="memory-stats__label">USED</span>
          <span class="memory-stats__value" id="memUsed">0 MB</span>
        </div>
        <div class="memory-stats__item">
          <span class="memory-stats__label">TOTAL</span>
          <span class="memory-stats__value" id="memTotal">0 MB</span>
        </div>
        <div class="memory-stats__item">
          <span class="memory-stats__label">STATUS</span>
          <span class="memory-stats__value memory-stats__value--status" id="memStatus">NORMAL</span>
        </div>
      </div>
    </div>
  </section>

  <!-- Footer -->
  <footer class="footer">
    <div class="container footer__inner">
      <p class="footer__text">
        <span class="neon-text">EDGEVEC</span> v0.6.0 // MIT License //
        <a href="https://crates.io/crates/edgevec" target="_blank" rel="noopener">crates.io</a> //
        <a href="https://www.npmjs.com/package/edgevec" target="_blank" rel="noopener">npm</a>
      </p>
      <p class="footer__subtext">
        Built with Rust + WebAssembly // Binary Quantization + Metadata Filtering
      </p>
    </div>
  </footer>

  <!-- Toast Container -->
  <div class="toast-container" id="toastContainer" aria-live="polite"></div>

  <!-- Scripts -->
  <script type="module" src="js/app.js"></script>
</body>
</html>
```

**Exit Criteria:**
- Layout renders correctly on desktop
- Layout responds to mobile (< 640px)
- Theme toggle works
- Theme persists in localStorage
- All ARIA labels present

---

### W28.6.3: Component Library + Interactions (3 hours)

**Objective:** Create reusable JavaScript components with interactions.

**Deliverables:**
1. Toast notification system
2. Loading skeleton manager
3. Result card component
4. Chart rendering utilities
5. Memory gauge component

**Implementation:**

```javascript
// wasm/examples/js/components.js

/**
 * EdgeVec Cyberpunk UI Components
 * Reusable component library for the v0.6.0 demo
 */

// =============================================================================
// Toast Notification System
// =============================================================================

class ToastManager {
  constructor(containerId = 'toastContainer') {
    this.container = document.getElementById(containerId);
    this.queue = [];
    this.isShowing = false;
  }

  show(message, type = 'info', duration = 3000) {
    const toast = document.createElement('div');
    toast.className = `toast toast--${type}`;
    toast.innerHTML = `
      <span class="toast__icon">${this.getIcon(type)}</span>
      <span class="toast__message">${message}</span>
    `;

    this.container.appendChild(toast);

    // Trigger animation
    requestAnimationFrame(() => {
      toast.classList.add('toast--visible');
    });

    // Auto dismiss
    setTimeout(() => {
      toast.classList.remove('toast--visible');
      setTimeout(() => toast.remove(), 300);
    }, duration);
  }

  getIcon(type) {
    const icons = {
      success: '<svg viewBox="0 0 24 24"><path d="M5 13l4 4L19 7"/></svg>',
      error: '<svg viewBox="0 0 24 24"><path d="M6 18L18 6M6 6l12 12"/></svg>',
      warning: '<svg viewBox="0 0 24 24"><path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>',
      info: '<svg viewBox="0 0 24 24"><path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>'
    };
    return icons[type] || icons.info;
  }
}

// =============================================================================
// Loading Skeleton
// =============================================================================

class SkeletonLoader {
  static createResultSkeleton(count = 5) {
    const skeletons = [];
    for (let i = 0; i < count; i++) {
      const skeleton = document.createElement('div');
      skeleton.className = 'result-card result-card--skeleton';
      skeleton.innerHTML = `
        <div class="skeleton skeleton--title"></div>
        <div class="skeleton skeleton--text"></div>
        <div class="skeleton skeleton--text" style="width: 80%"></div>
        <div class="skeleton skeleton--text" style="width: 60%"></div>
      `;
      skeletons.push(skeleton);
    }
    return skeletons;
  }

  static show(container) {
    container.innerHTML = '';
    const skeletons = this.createResultSkeleton(5);
    skeletons.forEach(s => container.appendChild(s));
  }

  static hide(container) {
    const skeletons = container.querySelectorAll('.result-card--skeleton');
    skeletons.forEach(s => {
      s.classList.add('result-card--fade-out');
      setTimeout(() => s.remove(), 300);
    });
  }
}

// =============================================================================
// Result Card Component
// =============================================================================

class ResultCard {
  static create(result, index, mode = 'f32') {
    const card = document.createElement('div');
    card.className = 'result-card';
    card.style.animationDelay = `${index * 50}ms`;

    const distanceLabel = mode === 'bq' ? 'HAMMING' : 'COSINE';
    const distanceValue = mode === 'bq'
      ? result.distance.toFixed(0)
      : result.distance.toFixed(4);

    const metadata = result.metadata || {};
    const metadataHtml = Object.entries(metadata)
      .map(([k, v]) => `<span class="result-card__tag">${k}: ${v}</span>`)
      .join('');

    card.innerHTML = `
      <div class="result-card__header">
        <span class="result-card__rank neon-text">#${index + 1}</span>
        <span class="result-card__id">ID: ${result.id}</span>
      </div>
      <div class="result-card__body">
        <div class="result-card__distance">
          <span class="result-card__distance-label">${distanceLabel}</span>
          <span class="result-card__distance-value">${distanceValue}</span>
        </div>
        ${metadataHtml ? `<div class="result-card__metadata">${metadataHtml}</div>` : ''}
      </div>
      <div class="result-card__glow" aria-hidden="true"></div>
    `;

    // Add entrance animation class
    card.classList.add('result-card--enter');

    return card;
  }

  static renderResults(container, results, mode) {
    // Clear and show skeleton
    SkeletonLoader.show(container);

    // Simulate network delay for effect
    setTimeout(() => {
      container.innerHTML = '';

      if (results.length === 0) {
        container.innerHTML = `
          <div class="results__empty">
            <p class="neon-text--magenta">NO_RESULTS_FOUND</p>
            <p class="text-muted">Try adjusting your filter or search query</p>
          </div>
        `;
        return;
      }

      results.forEach((result, index) => {
        const card = ResultCard.create(result, index, mode);
        container.appendChild(card);
      });
    }, 300);
  }
}

// =============================================================================
// Benchmark Chart
// =============================================================================

class BenchmarkChart {
  constructor(containerId) {
    this.container = document.getElementById(containerId);
    this.data = { f32: [], bq: [] };
  }

  update(f32Time, bqTime) {
    this.data.f32.push(f32Time);
    this.data.bq.push(bqTime);

    // Keep last 10 measurements
    if (this.data.f32.length > 10) {
      this.data.f32.shift();
      this.data.bq.shift();
    }

    this.render();
  }

  render() {
    const width = this.container.clientWidth;
    const height = 200;
    const padding = 40;

    const maxValue = Math.max(
      ...this.data.f32,
      ...this.data.bq,
      1
    );

    const xScale = (width - padding * 2) / (this.data.f32.length - 1 || 1);
    const yScale = (height - padding * 2) / maxValue;

    const f32Points = this.data.f32
      .map((v, i) => `${padding + i * xScale},${height - padding - v * yScale}`)
      .join(' ');

    const bqPoints = this.data.bq
      .map((v, i) => `${padding + i * xScale},${height - padding - v * yScale}`)
      .join(' ');

    this.container.innerHTML = `
      <svg viewBox="0 0 ${width} ${height}" class="chart">
        <!-- Grid -->
        <g class="chart__grid">
          ${[0, 0.25, 0.5, 0.75, 1].map(v => `
            <line
              x1="${padding}"
              y1="${height - padding - v * (height - padding * 2)}"
              x2="${width - padding}"
              y2="${height - padding - v * (height - padding * 2)}"
              stroke="var(--text-muted)"
              stroke-dasharray="4"
              stroke-opacity="0.3"
            />
            <text
              x="${padding - 5}"
              y="${height - padding - v * (height - padding * 2)}"
              fill="var(--text-secondary)"
              text-anchor="end"
              font-size="10"
            >${(maxValue * v).toFixed(1)}</text>
          `).join('')}
        </g>

        <!-- F32 Line -->
        <polyline
          points="${f32Points}"
          fill="none"
          stroke="var(--neon-cyan)"
          stroke-width="2"
          class="chart__line chart__line--f32"
        />
        <g class="chart__dots chart__dots--f32">
          ${this.data.f32.map((v, i) => `
            <circle
              cx="${padding + i * xScale}"
              cy="${height - padding - v * yScale}"
              r="4"
              fill="var(--neon-cyan)"
            />
          `).join('')}
        </g>

        <!-- BQ Line -->
        <polyline
          points="${bqPoints}"
          fill="none"
          stroke="var(--neon-magenta)"
          stroke-width="2"
          class="chart__line chart__line--bq"
        />
        <g class="chart__dots chart__dots--bq">
          ${this.data.bq.map((v, i) => `
            <circle
              cx="${padding + i * xScale}"
              cy="${height - padding - v * yScale}"
              r="4"
              fill="var(--neon-magenta)"
            />
          `).join('')}
        </g>

        <!-- Legend -->
        <g class="chart__legend" transform="translate(${width - 100}, 20)">
          <rect x="0" y="0" width="12" height="12" fill="var(--neon-cyan)"/>
          <text x="18" y="10" fill="var(--text-primary)" font-size="12">F32</text>
          <rect x="0" y="20" width="12" height="12" fill="var(--neon-magenta)"/>
          <text x="18" y="30" fill="var(--text-primary)" font-size="12">BQ</text>
        </g>
      </svg>
    `;
  }
}

// =============================================================================
// Memory Gauge Component
// =============================================================================

class MemoryGauge {
  constructor(containerId) {
    this.container = document.getElementById(containerId);
    this.percentage = 0;
    this.status = 'normal';
  }

  update(used, total) {
    this.percentage = (used / total) * 100;
    this.status = this.percentage < 80 ? 'normal'
                : this.percentage < 95 ? 'warning'
                : 'critical';
    this.render();
  }

  render() {
    const size = 200;
    const strokeWidth = 12;
    const radius = (size - strokeWidth) / 2;
    const circumference = 2 * Math.PI * radius;
    const offset = circumference - (this.percentage / 100) * circumference;

    const colors = {
      normal: 'var(--neon-green)',
      warning: 'var(--neon-yellow)',
      critical: 'var(--neon-magenta)'
    };

    const color = colors[this.status];

    this.container.innerHTML = `
      <svg viewBox="0 0 ${size} ${size}" class="gauge">
        <!-- Background Circle -->
        <circle
          cx="${size / 2}"
          cy="${size / 2}"
          r="${radius}"
          fill="none"
          stroke="var(--bg-elevated)"
          stroke-width="${strokeWidth}"
        />

        <!-- Progress Circle -->
        <circle
          cx="${size / 2}"
          cy="${size / 2}"
          r="${radius}"
          fill="none"
          stroke="${color}"
          stroke-width="${strokeWidth}"
          stroke-linecap="round"
          stroke-dasharray="${circumference}"
          stroke-dashoffset="${offset}"
          transform="rotate(-90 ${size / 2} ${size / 2})"
          class="gauge__progress"
          style="filter: drop-shadow(0 0 10px ${color})"
        />

        <!-- Center Text -->
        <text
          x="${size / 2}"
          y="${size / 2}"
          text-anchor="middle"
          dominant-baseline="middle"
          fill="${color}"
          font-size="36"
          font-weight="bold"
          font-family="var(--font-mono)"
          class="gauge__text"
        >${this.percentage.toFixed(1)}%</text>

        <text
          x="${size / 2}"
          y="${size / 2 + 30}"
          text-anchor="middle"
          fill="var(--text-secondary)"
          font-size="12"
          font-family="var(--font-mono)"
        >MEMORY_USAGE</text>
      </svg>
    `;

    // Update aria attributes
    this.container.setAttribute('aria-valuenow', this.percentage.toFixed(1));
  }
}

// =============================================================================
// Export Components
// =============================================================================

export { ToastManager, SkeletonLoader, ResultCard, BenchmarkChart, MemoryGauge };
```

**Exit Criteria:**
- Toast notifications appear and dismiss
- Skeleton loading shows during data fetch
- Result cards render with animations
- Chart updates with benchmark data
- Memory gauge reflects current usage

---

## Exit Criteria (Day 6)

| Criterion | Verification |
|:----------|:-------------|
| CSS design system complete | Manual visual check |
| Neon glow effects work | Visual test in Chrome |
| Glitch animation renders | Visual test |
| Theme toggle persists | localStorage check |
| Mobile layout works | Chrome DevTools responsive mode |
| Components render correctly | Manual testing |
| Accessibility: focus visible | Keyboard navigation test |
| Accessibility: reduced motion | prefers-reduced-motion test |

---

## Day 6 Summary

**Completed:** 2025-12-22

**Artifacts Generated (7 files, 3,776 lines):**
- `wasm/examples/css/cyberpunk.css` (733 lines) — Design system with dark/light theme
- `wasm/examples/css/layout.css` (570 lines) — Page structure and responsive layout
- `wasm/examples/css/components.css` (617 lines) — Result cards and interactive elements
- `wasm/examples/css/animations.css` (459 lines) — GPU-accelerated animation library
- `wasm/examples/v060_cyberpunk_demo.html` (359 lines) — Main demo with ARIA labels
- `wasm/examples/js/components.js` (533 lines) — 8 reusable UI components
- `wasm/examples/js/app.js` (505 lines) — Application controller with mock fallback

**HOSTILE_REVIEWER Gate:**
- Round 1: APPROVED (0 Critical, 0 Major, 2 Minor issues)
- Review document: `docs/reviews/2025-12-22_W28_DAY6_APPROVED.md`

**Minor Issues Resolved:**
| Issue | Description | Resolution |
|:------|:------------|:-----------|
| m1 | GitHub link hardcoded | Updated to anthropics/edgevec |
| m2 | Gauge comment misleading | Updated comment to clarify update pattern |

**Exit Criteria Met:**
- [x] CSS design system complete with light/dark theme
- [x] Neon glow effects work
- [x] Glitch animation renders
- [x] Theme toggle persists in localStorage
- [x] Mobile layout works (640px/768px/1024px breakpoints)
- [x] All 8 components render correctly
- [x] Accessibility: focus-visible for keyboard navigation
- [x] Accessibility: prefers-reduced-motion respected
- [x] XSS prevention via escapeHtml()
- [x] Mock database fallback for demo mode

---

## Handoff

**Status:** COMPLETE

**Next:** Day 7 — Advanced Animations + Polish

---

*Agent: WASM_SPECIALIST + DOCWRITER*
*Status: [APPROVED]*
*Date: 2025-12-22*
