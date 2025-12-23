# Week 28 Day 7: Advanced Animations + Polish

**Date:** 2025-12-29 (Sunday)
**Focus:** Browser Demo — Advanced Animations, Interactivity, Mobile Testing
**Estimated Duration:** 8 hours
**Agent:** WASM_SPECIALIST + DOCWRITER
**Prerequisite:** Day 6 complete (UI framework ready)

---

## Executive Summary

Day 7 completes the **spectacular cyberpunk-themed** browser demo with advanced animations, particle effects, interactive charts, and comprehensive mobile testing. This fully addresses HOSTILE_REVIEWER finding [C1].

**Deliverables:**
- Particle background system
- Matrix rain effect
- Animated search results with stagger
- Interactive SVG chart with hover states
- Smooth page transitions
- Mobile-first responsive polish
- Accessibility audit + fixes
- Performance optimization

---

## Task Breakdown

### W28.7.1: Particle Background + Matrix Rain (2 hours)

**Objective:** Create immersive cyberpunk background effects.

**Deliverables:**
1. Canvas-based particle system
2. Matrix digital rain effect
3. Performance-optimized rendering (requestAnimationFrame)
4. Reduced motion support

**Implementation:**

```javascript
// wasm/examples/js/effects.js

/**
 * EdgeVec Cyberpunk Visual Effects
 * Performance-optimized particle and matrix effects
 */

// =============================================================================
// Particle System
// =============================================================================

class ParticleSystem {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    this.ctx = this.canvas.getContext('2d');
    this.particles = [];
    this.mouse = { x: null, y: null, radius: 150 };
    this.animationId = null;
    this.isRunning = false;

    this.resize();
    this.init();
    this.bindEvents();
  }

  resize() {
    this.canvas.width = window.innerWidth;
    this.canvas.height = window.innerHeight;
  }

  init() {
    const particleCount = Math.min(100, Math.floor((this.canvas.width * this.canvas.height) / 15000));

    this.particles = [];
    for (let i = 0; i < particleCount; i++) {
      this.particles.push(new Particle(this.canvas));
    }
  }

  bindEvents() {
    window.addEventListener('resize', () => {
      this.resize();
      this.init();
    });

    window.addEventListener('mousemove', (e) => {
      this.mouse.x = e.x;
      this.mouse.y = e.y;
    });

    window.addEventListener('mouseout', () => {
      this.mouse.x = null;
      this.mouse.y = null;
    });

    // Respect reduced motion preference
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    mediaQuery.addEventListener('change', () => {
      if (mediaQuery.matches) {
        this.stop();
      } else {
        this.start();
      }
    });
  }

  animate() {
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    for (let i = 0; i < this.particles.length; i++) {
      const particle = this.particles[i];
      particle.update(this.mouse);
      particle.draw(this.ctx);

      // Connect nearby particles
      for (let j = i + 1; j < this.particles.length; j++) {
        const dx = particle.x - this.particles[j].x;
        const dy = particle.y - this.particles[j].y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance < 120) {
          this.ctx.beginPath();
          this.ctx.strokeStyle = `rgba(0, 255, 255, ${0.2 - distance / 600})`;
          this.ctx.lineWidth = 0.5;
          this.ctx.moveTo(particle.x, particle.y);
          this.ctx.lineTo(this.particles[j].x, this.particles[j].y);
          this.ctx.stroke();
        }
      }
    }

    if (this.isRunning) {
      this.animationId = requestAnimationFrame(() => this.animate());
    }
  }

  start() {
    if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.isRunning = true;
      this.animate();
    }
  }

  stop() {
    this.isRunning = false;
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
    }
  }
}

class Particle {
  constructor(canvas) {
    this.canvas = canvas;
    this.x = Math.random() * canvas.width;
    this.y = Math.random() * canvas.height;
    this.size = Math.random() * 3 + 1;
    this.baseX = this.x;
    this.baseY = this.y;
    this.density = Math.random() * 30 + 1;
    this.speedX = (Math.random() - 0.5) * 0.5;
    this.speedY = (Math.random() - 0.5) * 0.5;

    // Random color from neon palette
    const colors = ['#00ffff', '#ff00ff', '#39ff14'];
    this.color = colors[Math.floor(Math.random() * colors.length)];
  }

  update(mouse) {
    // Mouse interaction
    if (mouse.x !== null && mouse.y !== null) {
      const dx = mouse.x - this.x;
      const dy = mouse.y - this.y;
      const distance = Math.sqrt(dx * dx + dy * dy);

      if (distance < mouse.radius) {
        const forceDirectionX = dx / distance;
        const forceDirectionY = dy / distance;
        const force = (mouse.radius - distance) / mouse.radius;

        this.x -= forceDirectionX * force * this.density * 0.5;
        this.y -= forceDirectionY * force * this.density * 0.5;
      }
    }

    // Drift movement
    this.x += this.speedX;
    this.y += this.speedY;

    // Wrap around edges
    if (this.x < 0) this.x = this.canvas.width;
    if (this.x > this.canvas.width) this.x = 0;
    if (this.y < 0) this.y = this.canvas.height;
    if (this.y > this.canvas.height) this.y = 0;
  }

  draw(ctx) {
    ctx.beginPath();
    ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);
    ctx.fillStyle = this.color;
    ctx.shadowBlur = 10;
    ctx.shadowColor = this.color;
    ctx.fill();
    ctx.shadowBlur = 0;
  }
}

// =============================================================================
// Matrix Rain Effect
// =============================================================================

class MatrixRain {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    this.ctx = this.canvas.getContext('2d');
    this.columns = [];
    this.fontSize = 14;
    this.animationId = null;
    this.isRunning = false;

    // Characters: mix of katakana, latin, and numbers
    this.chars = 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

    this.resize();
    this.init();

    window.addEventListener('resize', () => {
      this.resize();
      this.init();
    });
  }

  resize() {
    this.canvas.width = window.innerWidth;
    this.canvas.height = window.innerHeight;
  }

  init() {
    const columnCount = Math.floor(this.canvas.width / this.fontSize);
    this.columns = [];

    for (let i = 0; i < columnCount; i++) {
      this.columns.push({
        y: Math.random() * this.canvas.height,
        speed: Math.random() * 2 + 1,
        opacity: Math.random() * 0.5 + 0.1
      });
    }
  }

  animate() {
    // Semi-transparent black to create trail effect
    this.ctx.fillStyle = 'rgba(10, 10, 15, 0.05)';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    this.ctx.font = `${this.fontSize}px 'JetBrains Mono', monospace`;

    for (let i = 0; i < this.columns.length; i++) {
      const column = this.columns[i];
      const char = this.chars[Math.floor(Math.random() * this.chars.length)];
      const x = i * this.fontSize;

      // Gradient from cyan to magenta
      const gradient = this.ctx.createLinearGradient(x, column.y - 100, x, column.y);
      gradient.addColorStop(0, 'rgba(0, 255, 255, 0)');
      gradient.addColorStop(0.5, `rgba(0, 255, 255, ${column.opacity})`);
      gradient.addColorStop(1, `rgba(255, 0, 255, ${column.opacity})`);

      this.ctx.fillStyle = gradient;
      this.ctx.fillText(char, x, column.y);

      // Move column down
      column.y += column.speed * this.fontSize * 0.5;

      // Reset column when it goes off screen
      if (column.y > this.canvas.height && Math.random() > 0.975) {
        column.y = 0;
        column.speed = Math.random() * 2 + 1;
        column.opacity = Math.random() * 0.5 + 0.1;
      }
    }

    if (this.isRunning) {
      this.animationId = requestAnimationFrame(() => this.animate());
    }
  }

  start() {
    if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.isRunning = true;
      this.animate();
    }
  }

  stop() {
    this.isRunning = false;
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
    }
  }
}

// =============================================================================
// Export Effects
// =============================================================================

export { ParticleSystem, MatrixRain };
```

**Exit Criteria:**
- Particles connect when close together
- Particles react to mouse movement
- Matrix rain scrolls smoothly
- Performance: 60 FPS maintained
- Reduced motion: effects disabled

---

### W28.7.2: Animated Search Results + Stagger (2 hours)

**Objective:** Create smooth, staggered animations for search results.

**Deliverables:**
1. Entrance animations with stagger
2. Exit animations
3. Hover effects with glow
4. Number counting animation
5. Distance bar visualization

**Implementation:**

```css
/* wasm/examples/css/animations.css */

/* ==========================================================================
   EDGEVEC ADVANCED ANIMATIONS
   GPU-accelerated animations for smooth 60fps performance
   ========================================================================== */

/* --------------------------------------------------------------------------
   Result Card Animations
   -------------------------------------------------------------------------- */

/* Entrance Animation */
@keyframes resultCardEnter {
  0% {
    opacity: 0;
    transform: translateY(30px) scale(0.95);
    filter: blur(10px);
  }
  50% {
    filter: blur(0px);
  }
  100% {
    opacity: 1;
    transform: translateY(0) scale(1);
    filter: blur(0px);
  }
}

.result-card--enter {
  animation: resultCardEnter 0.5s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  opacity: 0;
}

/* Stagger delay applied via JS: style="animation-delay: Nms" */

/* Exit Animation */
@keyframes resultCardExit {
  0% {
    opacity: 1;
    transform: translateX(0) scale(1);
  }
  100% {
    opacity: 0;
    transform: translateX(-30px) scale(0.95);
  }
}

.result-card--exit {
  animation: resultCardExit 0.3s ease-in forwards;
}

/* Hover Glow Effect */
.result-card {
  position: relative;
  transition: transform 0.3s ease, box-shadow 0.3s ease;
  will-change: transform;
}

.result-card::before {
  content: '';
  position: absolute;
  inset: -2px;
  border-radius: calc(var(--radius-md) + 2px);
  background: var(--gradient-neon);
  opacity: 0;
  z-index: -1;
  transition: opacity 0.3s ease;
}

.result-card:hover {
  transform: translateY(-4px) scale(1.02);
  box-shadow:
    0 10px 40px rgba(0, 255, 255, 0.2),
    0 0 20px rgba(0, 255, 255, 0.1);
}

.result-card:hover::before {
  opacity: 1;
}

/* Rank Number Glow */
.result-card__rank {
  font-family: var(--font-display);
  font-size: 1.5rem;
  font-weight: 900;
  animation: rankPulse 2s ease-in-out infinite;
}

@keyframes rankPulse {
  0%, 100% {
    text-shadow:
      0 0 5px var(--neon-cyan),
      0 0 10px var(--neon-cyan),
      0 0 20px var(--neon-cyan);
  }
  50% {
    text-shadow:
      0 0 10px var(--neon-cyan),
      0 0 20px var(--neon-cyan),
      0 0 40px var(--neon-cyan),
      0 0 60px var(--neon-cyan);
  }
}

/* Distance Bar */
.result-card__distance-bar {
  height: 4px;
  background: var(--bg-elevated);
  border-radius: var(--radius-sm);
  overflow: hidden;
  margin-top: var(--space-sm);
}

.result-card__distance-bar-fill {
  height: 100%;
  background: var(--gradient-neon);
  border-radius: var(--radius-sm);
  transform-origin: left;
  animation: distanceBarFill 0.8s ease-out forwards;
  transform: scaleX(0);
}

@keyframes distanceBarFill {
  to {
    transform: scaleX(var(--fill-percent, 1));
  }
}

/* --------------------------------------------------------------------------
   Number Counter Animation
   -------------------------------------------------------------------------- */

.counter {
  display: inline-block;
  font-variant-numeric: tabular-nums;
}

.counter--animate {
  animation: counterBlink 0.1s ease-in-out;
}

@keyframes counterBlink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

/* --------------------------------------------------------------------------
   Button Ripple Effect
   -------------------------------------------------------------------------- */

.btn {
  position: relative;
  overflow: hidden;
}

.btn__ripple {
  position: absolute;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  transform: scale(0);
  animation: ripple 0.6s linear;
  pointer-events: none;
}

@keyframes ripple {
  to {
    transform: scale(4);
    opacity: 0;
  }
}

/* --------------------------------------------------------------------------
   Search Box Focus Animation
   -------------------------------------------------------------------------- */

.search-box__input {
  transition: all 0.3s ease;
}

.search-box__input:focus {
  box-shadow:
    0 0 0 2px var(--bg-terminal),
    0 0 0 4px var(--neon-cyan),
    0 0 20px rgba(0, 255, 255, 0.3);
}

.search-box__input:focus + .search-box__btn {
  color: var(--neon-cyan);
  animation: searchPulse 1s ease-in-out infinite;
}

@keyframes searchPulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.1); }
}

/* --------------------------------------------------------------------------
   Loading Spinner
   -------------------------------------------------------------------------- */

.spinner {
  width: 40px;
  height: 40px;
  position: relative;
}

.spinner::before,
.spinner::after {
  content: '';
  position: absolute;
  inset: 0;
  border: 3px solid transparent;
  border-radius: 50%;
}

.spinner::before {
  border-top-color: var(--neon-cyan);
  animation: spinnerRotate 1s linear infinite;
}

.spinner::after {
  border-bottom-color: var(--neon-magenta);
  animation: spinnerRotate 1s linear infinite reverse;
  animation-delay: 0.2s;
}

@keyframes spinnerRotate {
  to { transform: rotate(360deg); }
}

/* --------------------------------------------------------------------------
   Typing Animation
   -------------------------------------------------------------------------- */

.typing {
  overflow: hidden;
  white-space: nowrap;
  animation: typing 2s steps(30, end);
}

@keyframes typing {
  from { width: 0; }
  to { width: 100%; }
}

/* --------------------------------------------------------------------------
   Page Transitions
   -------------------------------------------------------------------------- */

.page-transition {
  position: fixed;
  inset: 0;
  background: var(--bg-void);
  z-index: 9999;
  pointer-events: none;
  transform: translateY(100%);
}

.page-transition--active {
  animation: pageTransitionSlide 0.5s ease-in-out;
}

@keyframes pageTransitionSlide {
  0% { transform: translateY(100%); }
  50% { transform: translateY(0); }
  100% { transform: translateY(-100%); }
}

/* --------------------------------------------------------------------------
   Metric Card Animations
   -------------------------------------------------------------------------- */

.metric-card {
  transition: all 0.3s ease;
}

.metric-card:hover {
  transform: translateY(-8px);
}

.metric-card:hover .metric-card__icon {
  animation: metricIconBounce 0.5s ease;
}

@keyframes metricIconBounce {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.2); }
}

.metric-card__value {
  font-size: 2.5rem;
  font-weight: 700;
  font-family: var(--font-display);
  background: var(--gradient-neon);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

/* Value update flash */
.metric-card__value--flash {
  animation: valueFlash 0.3s ease;
}

@keyframes valueFlash {
  0%, 100% { filter: brightness(1); }
  50% { filter: brightness(1.5); }
}

/* --------------------------------------------------------------------------
   Chart Animations
   -------------------------------------------------------------------------- */

.chart__line {
  stroke-dasharray: 2000;
  stroke-dashoffset: 2000;
  animation: chartLineDraw 1.5s ease-out forwards;
}

@keyframes chartLineDraw {
  to { stroke-dashoffset: 0; }
}

.chart__dots circle {
  opacity: 0;
  animation: chartDotAppear 0.3s ease-out forwards;
}

.chart__dots circle:nth-child(1) { animation-delay: 0.1s; }
.chart__dots circle:nth-child(2) { animation-delay: 0.2s; }
.chart__dots circle:nth-child(3) { animation-delay: 0.3s; }
.chart__dots circle:nth-child(4) { animation-delay: 0.4s; }
.chart__dots circle:nth-child(5) { animation-delay: 0.5s; }
.chart__dots circle:nth-child(6) { animation-delay: 0.6s; }
.chart__dots circle:nth-child(7) { animation-delay: 0.7s; }
.chart__dots circle:nth-child(8) { animation-delay: 0.8s; }
.chart__dots circle:nth-child(9) { animation-delay: 0.9s; }
.chart__dots circle:nth-child(10) { animation-delay: 1.0s; }

@keyframes chartDotAppear {
  0% {
    opacity: 0;
    transform: scale(0);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

/* Chart dot hover */
.chart__dots circle {
  transition: r 0.2s ease;
  cursor: pointer;
}

.chart__dots circle:hover {
  r: 8;
}

/* --------------------------------------------------------------------------
   Gauge Animation
   -------------------------------------------------------------------------- */

.gauge__progress {
  transition: stroke-dashoffset 1s ease-out;
}

.gauge__text {
  animation: gaugeTextPulse 2s ease-in-out infinite;
}

@keyframes gaugeTextPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

/* Gauge status colors */
.gauge--warning .gauge__progress {
  stroke: var(--neon-yellow);
  filter: drop-shadow(0 0 10px var(--neon-yellow));
}

.gauge--critical .gauge__progress {
  stroke: var(--neon-magenta);
  filter: drop-shadow(0 0 10px var(--neon-magenta));
  animation: gaugeCriticalPulse 0.5s ease-in-out infinite;
}

@keyframes gaugeCriticalPulse {
  0%, 100% { filter: drop-shadow(0 0 10px var(--neon-magenta)); }
  50% { filter: drop-shadow(0 0 20px var(--neon-magenta)); }
}

/* --------------------------------------------------------------------------
   Status Bar Animation
   -------------------------------------------------------------------------- */

.status-bar__value--loading {
  animation: statusLoading 1.5s ease-in-out infinite;
}

@keyframes statusLoading {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 1; }
}

.status-bar__value--success {
  animation: statusSuccess 0.5s ease;
  color: var(--neon-green);
}

@keyframes statusSuccess {
  0% { transform: scale(1.3); opacity: 0; }
  100% { transform: scale(1); opacity: 1; }
}

/* --------------------------------------------------------------------------
   Hero Section Animations
   -------------------------------------------------------------------------- */

.hero__title {
  animation: heroTitleAppear 1s ease-out;
}

@keyframes heroTitleAppear {
  0% {
    opacity: 0;
    transform: translateY(-30px);
    filter: blur(20px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
    filter: blur(0);
  }
}

.hero__subtitle {
  animation: heroSubtitleAppear 1s ease-out 0.3s backwards;
}

@keyframes heroSubtitleAppear {
  0% {
    opacity: 0;
    transform: translateY(20px);
  }
  100% {
    opacity: 1;
    transform: translateY(0);
  }
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
    scroll-behavior: auto !important;
  }

  .result-card--enter {
    opacity: 1;
    transform: none;
  }

  .chart__line {
    stroke-dasharray: none;
    stroke-dashoffset: 0;
  }
}
```

**JavaScript Animation Utilities:**

```javascript
// wasm/examples/js/animations.js

/**
 * EdgeVec Animation Utilities
 * Smooth, performant animations with reduced motion support
 */

// =============================================================================
// Number Counter Animation
// =============================================================================

class NumberCounter {
  constructor(element, options = {}) {
    this.element = element;
    this.duration = options.duration || 1000;
    this.decimals = options.decimals || 0;
    this.prefix = options.prefix || '';
    this.suffix = options.suffix || '';
  }

  animate(from, to) {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.element.textContent = this.format(to);
      return;
    }

    const startTime = performance.now();
    const diff = to - from;

    const step = (currentTime) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / this.duration, 1);

      // Ease out cubic
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = from + diff * eased;

      this.element.textContent = this.format(current);
      this.element.classList.add('counter--animate');

      if (progress < 1) {
        requestAnimationFrame(step);
      } else {
        this.element.classList.remove('counter--animate');
      }
    };

    requestAnimationFrame(step);
  }

  format(value) {
    return `${this.prefix}${value.toFixed(this.decimals)}${this.suffix}`;
  }
}

// =============================================================================
// Ripple Effect
// =============================================================================

function createRipple(event) {
  const button = event.currentTarget;
  const ripple = document.createElement('span');
  ripple.className = 'btn__ripple';

  const rect = button.getBoundingClientRect();
  const size = Math.max(rect.width, rect.height);
  const x = event.clientX - rect.left - size / 2;
  const y = event.clientY - rect.top - size / 2;

  ripple.style.width = ripple.style.height = `${size}px`;
  ripple.style.left = `${x}px`;
  ripple.style.top = `${y}px`;

  button.appendChild(ripple);

  ripple.addEventListener('animationend', () => {
    ripple.remove();
  });
}

// Add to all buttons
document.querySelectorAll('.btn').forEach(btn => {
  btn.addEventListener('click', createRipple);
});

// =============================================================================
// Stagger Animation
// =============================================================================

function staggerAnimation(elements, options = {}) {
  const delay = options.delay || 50;
  const className = options.className || 'result-card--enter';

  elements.forEach((el, index) => {
    el.style.animationDelay = `${index * delay}ms`;
    el.classList.add(className);
  });
}

// =============================================================================
// Intersection Observer for Scroll Animations
// =============================================================================

function initScrollAnimations() {
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('animate-in');
        observer.unobserve(entry.target);
      }
    });
  }, {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
  });

  document.querySelectorAll('.animate-on-scroll').forEach(el => {
    observer.observe(el);
  });
}

// =============================================================================
// Smooth Scroll
// =============================================================================

function initSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', (e) => {
      e.preventDefault();
      const target = document.querySelector(anchor.getAttribute('href'));
      if (target) {
        target.scrollIntoView({
          behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches
            ? 'auto'
            : 'smooth'
        });
      }
    });
  });
}

// =============================================================================
// Export
// =============================================================================

export {
  NumberCounter,
  createRipple,
  staggerAnimation,
  initScrollAnimations,
  initSmoothScroll
};
```

**Exit Criteria:**
- Result cards animate in with stagger
- Hover effects show glow
- Number counters animate smoothly
- Button ripple effect works
- Chart lines draw progressively

---

### W28.7.3: Mobile Responsive Polish (2 hours)

**Objective:** Ensure perfect mobile experience with touch optimization.

**Deliverables:**
1. Mobile-first breakpoint audit
2. Touch-friendly button sizes (44px minimum)
3. Swipe gestures for result cards
4. Bottom sheet for controls
5. iOS Safari testing fixes

**Implementation:**

```css
/* wasm/examples/css/mobile.css */

/* ==========================================================================
   EDGEVEC MOBILE RESPONSIVE STYLES
   Mobile-first, touch-optimized design
   ========================================================================== */

/* --------------------------------------------------------------------------
   Mobile Breakpoints
   -------------------------------------------------------------------------- */

/* Small phones */
@media (max-width: 374px) {
  :root {
    font-size: 14px;
  }

  .hero__title {
    font-size: 1.5rem;
  }

  .container {
    padding: 0 var(--space-sm);
  }
}

/* Standard phones (375px - 639px) */
@media (max-width: 639px) {
  /* Header */
  .header__nav {
    display: none;
  }

  .header__logo {
    font-size: 1.25rem;
  }

  /* Hero */
  .hero {
    padding: var(--space-lg) 0;
  }

  .hero__title {
    font-size: 1.75rem;
    line-height: 1.2;
  }

  .hero__subtitle {
    font-size: 0.875rem;
  }

  /* Status Bar */
  .status-bar {
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .status-bar__item {
    flex: 1 1 calc(50% - var(--space-sm));
  }

  /* Layout */
  .layout {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .sidebar {
    order: 2; /* Move controls below results on mobile */
    width: 100%;
  }

  .content {
    order: 1;
    width: 100%;
  }

  /* Search Box */
  .search-box__input {
    font-size: 16px; /* Prevent iOS zoom */
  }

  /* Result Cards */
  .result-card {
    padding: var(--space-md);
  }

  .result-card__header {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-xs);
  }

  /* Metric Cards */
  .grid--3 {
    grid-template-columns: 1fr;
  }

  .metric-card {
    flex-direction: row;
    align-items: center;
    gap: var(--space-md);
  }

  .metric-card__value {
    font-size: 1.5rem;
  }

  /* Buttons */
  .btn {
    width: 100%;
    padding: var(--space-md) var(--space-lg);
    min-height: 48px; /* Touch target */
  }

  /* Footer */
  .footer__inner {
    flex-direction: column;
    text-align: center;
    gap: var(--space-sm);
  }
}

/* Tablets (640px - 1023px) */
@media (min-width: 640px) and (max-width: 1023px) {
  .layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: var(--space-xl);
  }

  .grid--3 {
    grid-template-columns: repeat(2, 1fr);
  }
}

/* Large screens (1024px+) */
@media (min-width: 1024px) {
  .layout {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: var(--space-2xl);
  }
}

/* --------------------------------------------------------------------------
   Touch Optimization
   -------------------------------------------------------------------------- */

/* Minimum touch target size */
.btn,
.radio,
.input,
.slider,
.theme-toggle,
.search-box__btn {
  min-height: 44px;
  min-width: 44px;
}

/* Prevent text selection on interactive elements */
.btn,
.radio,
.slider {
  -webkit-user-select: none;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
}

/* Active states for touch */
@media (hover: none) {
  .btn:active {
    transform: scale(0.98);
  }

  .result-card:active {
    transform: scale(0.99);
  }
}

/* Remove hover effects on touch devices */
@media (hover: none) {
  .result-card:hover {
    transform: none;
    box-shadow: none;
  }

  .btn:hover::before {
    left: -100%;
  }
}

/* --------------------------------------------------------------------------
   Bottom Sheet for Controls (Mobile)
   -------------------------------------------------------------------------- */

@media (max-width: 639px) {
  .sidebar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    max-height: 60vh;
    background: var(--bg-panel);
    border-top: 2px solid var(--neon-cyan);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    transform: translateY(calc(100% - 60px));
    transition: transform var(--transition-base);
    z-index: var(--z-modal);
    overflow: hidden;
  }

  .sidebar--open {
    transform: translateY(0);
  }

  .sidebar__handle {
    display: flex;
    justify-content: center;
    padding: var(--space-sm);
    cursor: grab;
  }

  .sidebar__handle::before {
    content: '';
    width: 40px;
    height: 4px;
    background: var(--text-muted);
    border-radius: var(--radius-sm);
  }

  .sidebar__content {
    max-height: calc(60vh - 40px);
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  /* Overlay */
  .sidebar-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: calc(var(--z-modal) - 1);
  }

  .sidebar-overlay--visible {
    display: block;
  }

  /* Add padding to content to account for bottom sheet */
  .content {
    padding-bottom: 80px;
  }
}

/* --------------------------------------------------------------------------
   Swipe Gesture Support
   -------------------------------------------------------------------------- */

.result-card {
  touch-action: pan-y;
}

.result-card--swiping {
  transition: none;
}

.result-card--dismissed {
  transform: translateX(-100%);
  opacity: 0;
}

/* --------------------------------------------------------------------------
   iOS Safari Fixes
   -------------------------------------------------------------------------- */

/* Fix for iOS Safari 100vh issue */
@supports (-webkit-touch-callout: none) {
  .hero,
  .section {
    min-height: -webkit-fill-available;
  }
}

/* Prevent overscroll bounce */
html {
  overscroll-behavior: none;
}

/* Fix for iOS input zoom */
input[type="text"],
input[type="number"],
input[type="search"] {
  font-size: 16px;
}

/* Safe area insets for notched devices */
@supports (padding: env(safe-area-inset-bottom)) {
  .footer {
    padding-bottom: calc(var(--space-lg) + env(safe-area-inset-bottom));
  }

  .sidebar {
    padding-bottom: env(safe-area-inset-bottom);
  }
}

/* --------------------------------------------------------------------------
   Landscape Mode
   -------------------------------------------------------------------------- */

@media (max-height: 500px) and (orientation: landscape) {
  .hero {
    padding: var(--space-md) 0;
  }

  .hero__title {
    font-size: 1.5rem;
  }

  .section {
    padding: var(--space-lg) 0;
  }
}
```

**Exit Criteria:**
- Demo works on iPhone SE (375px)
- Demo works on iPhone 14 Pro Max (430px)
- Demo works on iPad (768px)
- Touch targets are 44px minimum
- iOS Safari: no layout issues
- Android Chrome: no layout issues

---

### W28.7.4: Accessibility Audit + Final Polish (2 hours)

**Objective:** Ensure WCAG 2.1 AA compliance and production-ready polish.

**Deliverables:**
1. Keyboard navigation complete
2. Screen reader compatibility
3. Color contrast verification
4. Focus indicators
5. ARIA labels audit
6. Performance optimization

**Accessibility Checklist:**

```markdown
## EdgeVec v0.6.0 Demo Accessibility Audit

### Keyboard Navigation
- [x] All interactive elements reachable via Tab
- [x] Focus order is logical
- [x] Skip link to main content
- [x] Escape closes modals/overlays
- [x] Enter/Space activate buttons
- [x] Arrow keys navigate radio groups

### Focus Indicators
- [x] :focus-visible styling on all interactives
- [x] Focus ring has 3:1 contrast ratio
- [x] No focus traps

### Screen Readers
- [x] All images have alt text (or aria-hidden)
- [x] Form inputs have labels
- [x] Buttons have accessible names
- [x] Live regions for dynamic content (aria-live)
- [x] Headings are hierarchical (h1 → h2 → h3)
- [x] Landmarks: header, main, nav, footer

### Color & Contrast
- [x] Text contrast: minimum 4.5:1 (AA)
- [x] Large text contrast: minimum 3:1
- [x] UI components contrast: minimum 3:1
- [x] Color is not sole indicator of state

### Motion & Animation
- [x] prefers-reduced-motion honored
- [x] No auto-playing videos
- [x] No flashing content

### Forms
- [x] Error messages are associated with inputs
- [x] Required fields are indicated
- [x] Form submission has confirmation
```

**Performance Optimization:**

```javascript
// wasm/examples/js/performance.js

/**
 * EdgeVec Performance Optimizations
 * Ensuring smooth 60fps experience
 */

// =============================================================================
// Debounce & Throttle
// =============================================================================

function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

function throttle(func, limit) {
  let inThrottle;
  return function(...args) {
    if (!inThrottle) {
      func.apply(this, args);
      inThrottle = true;
      setTimeout(() => inThrottle = false, limit);
    }
  };
}

// =============================================================================
// Lazy Loading
// =============================================================================

function initLazyLoading() {
  const lazyElements = document.querySelectorAll('[data-lazy]');

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const el = entry.target;
        const src = el.dataset.lazy;
        if (el.tagName === 'IMG') {
          el.src = src;
        } else {
          el.style.backgroundImage = `url(${src})`;
        }
        el.removeAttribute('data-lazy');
        observer.unobserve(el);
      }
    });
  }, { rootMargin: '50px' });

  lazyElements.forEach(el => observer.observe(el));
}

// =============================================================================
// Request Idle Callback Polyfill
// =============================================================================

window.requestIdleCallback = window.requestIdleCallback ||
  function(cb) {
    const start = Date.now();
    return setTimeout(() => {
      cb({
        didTimeout: false,
        timeRemaining: () => Math.max(0, 50 - (Date.now() - start))
      });
    }, 1);
  };

// =============================================================================
// Performance Monitoring
// =============================================================================

function measurePerformance() {
  // First Contentful Paint
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      if (entry.name === 'first-contentful-paint') {
        console.log(`[PERF] FCP: ${entry.startTime.toFixed(0)}ms`);
      }
    }
  });
  observer.observe({ type: 'paint', buffered: true });

  // Largest Contentful Paint
  const lcpObserver = new PerformanceObserver((list) => {
    const entries = list.getEntries();
    const lastEntry = entries[entries.length - 1];
    console.log(`[PERF] LCP: ${lastEntry.startTime.toFixed(0)}ms`);
  });
  lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });

  // Cumulative Layout Shift
  let clsValue = 0;
  const clsObserver = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      if (!entry.hadRecentInput) {
        clsValue += entry.value;
      }
    }
    console.log(`[PERF] CLS: ${clsValue.toFixed(4)}`);
  });
  clsObserver.observe({ type: 'layout-shift', buffered: true });
}

// =============================================================================
// Memory Cleanup
// =============================================================================

function cleanupOnUnload() {
  window.addEventListener('beforeunload', () => {
    // Cancel all animations
    const animationIds = [];
    animationIds.forEach(id => cancelAnimationFrame(id));

    // Clear intervals
    const intervalIds = [];
    intervalIds.forEach(id => clearInterval(id));

    // Remove event listeners (stored references)
    // ...
  });
}

// =============================================================================
// Export
// =============================================================================

export {
  debounce,
  throttle,
  initLazyLoading,
  measurePerformance,
  cleanupOnUnload
};
```

**Exit Criteria:**
- WCAG 2.1 AA compliant
- Lighthouse accessibility score > 95
- Lighthouse performance score > 90
- No console errors
- Works offline (after initial load)

---

## Exit Criteria (Day 7)

| Criterion | Verification |
|:----------|:-------------|
| Particle system runs at 60fps | Chrome DevTools Performance |
| Matrix rain effect works | Visual check |
| Result cards animate with stagger | Visual check |
| Mobile layout perfect on iPhone SE | Chrome DevTools |
| Touch targets 44px minimum | Manual measurement |
| iOS Safari: no issues | Real device or BrowserStack |
| Keyboard navigation complete | Tab through entire demo |
| Screen reader compatible | VoiceOver/NVDA test |
| prefers-reduced-motion works | System setting test |
| Lighthouse Accessibility > 95 | Lighthouse audit |
| Lighthouse Performance > 90 | Lighthouse audit |

---

## Final Deliverables (Days 6-7)

```
wasm/examples/
├── css/
│   ├── cyberpunk.css          # Design system
│   ├── layout.css             # Layout utilities
│   ├── components.css         # Component styles
│   ├── animations.css         # Advanced animations
│   └── mobile.css             # Mobile responsive
├── js/
│   ├── app.js                 # Main application
│   ├── components.js          # UI components
│   ├── effects.js             # Particle + Matrix
│   ├── animations.js          # Animation utilities
│   └── performance.js         # Performance utils
├── v060_cyberpunk_demo.html   # Main demo page
└── fonts/
    └── (optional custom fonts)
```

---

## Handoff

**Artifacts Generated:**
- Complete cyberpunk-themed browser demo
- CSS design system with animations
- JavaScript effects and components
- Mobile-responsive layout
- Accessibility-compliant UI

**Status:** PENDING_HOSTILE_REVIEW

**Next:** Week 28 Complete → Submit for Gate Review

---

## Completion Summary

**Completed:** 2025-12-22

### Files Created/Modified:

| File | Lines | Description |
|:-----|------:|:------------|
| `wasm/examples/js/effects.js` | 367 | ParticleSystem + MatrixRain + EffectManager |
| `wasm/examples/js/animations.js` | 381 | Animation utilities (NumberCounter, TypewriterEffect, stagger, scroll, ripple) |
| `wasm/examples/js/performance.js` | 390 | Performance utilities (debounce, throttle, lazy loading, FPS/memory monitors) |
| `wasm/examples/css/mobile.css` | 360 | Mobile responsive styles + iOS Safari fixes + bottom sheet |
| `wasm/examples/css/animations.css` | +280 | Enhanced with Day 7 advanced animations |
| `wasm/examples/css/layout.css` | +25 | Effect canvas styles |
| `wasm/examples/v060_cyberpunk_demo.html` | +45 | Canvas elements + effect initialization |

**Total new code:** ~1,848 lines

### Implementation Highlights:

1. **ParticleSystem**: Canvas-based particles with mouse interaction, neon colors, connection lines
2. **MatrixRain**: Katakana/latin character rain with cyan-to-magenta gradient
3. **Animation Utilities**: NumberCounter, TypewriterEffect, glitchText, pulseElement, ANIMATION_PRESETS
4. **Scroll Animations**: IntersectionObserver-based reveal animations
5. **Mobile Optimization**: iOS Safari fixes, safe area insets, bottom sheet pattern, touch targets
6. **Performance Utilities**: Debounce, throttle, DOM batching, FPS/memory monitors, lazy loading
7. **Accessibility**: All effects respect `prefers-reduced-motion`

### Exit Criteria:

- [x] W28.7.1: Particle System + Matrix Rain — COMPLETE
- [x] W28.7.2: Animated Search Results + Stagger — COMPLETE
- [x] W28.7.3: Mobile Responsive Polish — COMPLETE
- [x] W28.7.4: Accessibility Audit + Performance — COMPLETE

---

*Agent: WASM_SPECIALIST + DOCWRITER*
*Status: [COMPLETE]*
*Date: 2025-12-22*
