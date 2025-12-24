/**
 * EdgeVec Cyberpunk UI Components
 * Reusable component library for the v0.6.0 demo
 * @version 0.6.0
 */

// =============================================================================
// Toast Notification System
// =============================================================================

export class ToastManager {
  constructor(containerId = 'toastContainer') {
    this.container = document.getElementById(containerId);
    if (!this.container) {
      console.warn('Toast container not found, creating one');
      this.container = document.createElement('div');
      this.container.id = containerId;
      this.container.className = 'toast-container';
      document.body.appendChild(this.container);
    }
  }

  show(message, type = 'info', duration = 3000) {
    const toast = document.createElement('div');
    toast.className = `toast toast--${type}`;
    toast.innerHTML = `
      <span class="toast__icon">${this.getIcon(type)}</span>
      <span class="toast__message">${this.escapeHtml(message)}</span>
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

    return toast;
  }

  success(message, duration = 3000) {
    return this.show(message, 'success', duration);
  }

  error(message, duration = 4000) {
    return this.show(message, 'error', duration);
  }

  warning(message, duration = 3500) {
    return this.show(message, 'warning', duration);
  }

  info(message, duration = 3000) {
    return this.show(message, 'info', duration);
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

  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

// =============================================================================
// Loading Skeleton
// =============================================================================

export class SkeletonLoader {
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

export class ResultCard {
  static escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = String(text);
    return div.innerHTML;
  }

  static create(result, index, mode = 'f32') {
    const card = document.createElement('div');
    card.className = 'result-card result-card--enter';
    card.style.animationDelay = `${index * 50}ms`;

    // F32 returns L2 distance, BQ returns Hamming, Hybrid returns similarity
    const distanceLabel = mode === 'bq' ? 'HAMMING' : (mode === 'hybrid' ? 'SIMILARITY' : 'L2_DIST');
    const distanceValue = mode === 'bq'
      ? Math.round(result.distance || 0)
      : (result.distance || result.score || 0).toFixed(4);

    const metadata = result.metadata || {};
    const metadataHtml = Object.entries(metadata)
      .slice(0, 4) // Limit to 4 tags
      .map(([k, v]) => `<span class="result-card__tag">${this.escapeHtml(k)}: ${this.escapeHtml(v)}</span>`)
      .join('');

    card.innerHTML = `
      <div class="result-card__header">
        <span class="result-card__rank neon-text">#${index + 1}</span>
        <span class="result-card__id">ID: ${this.escapeHtml(result.id)}</span>
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

    return card;
  }

  static renderResults(container, results, mode) {
    // Clear container
    container.innerHTML = '';

    if (!results || results.length === 0) {
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
  }

  static renderWithLoading(container, results, mode, delay = 300) {
    // Show skeleton
    SkeletonLoader.show(container);

    // Render actual results after delay
    setTimeout(() => {
      this.renderResults(container, results, mode);
    }, delay);
  }
}

// =============================================================================
// Benchmark Chart
// =============================================================================

export class BenchmarkChart {
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

  clear() {
    this.data = { f32: [], bq: [] };
    this.container.innerHTML = `
      <div class="results__placeholder">
        <p class="neon-text">NO_DATA</p>
        <p class="text-muted">Run benchmark to see performance comparison</p>
      </div>
    `;
  }

  render() {
    if (this.data.f32.length === 0) {
      this.clear();
      return;
    }

    const width = this.container.clientWidth || 600;
    const height = 200;
    const padding = 50;

    const maxValue = Math.max(
      ...this.data.f32,
      ...this.data.bq,
      0.1 // Prevent division by zero
    );

    const dataPoints = this.data.f32.length;
    const xScale = dataPoints > 1 ? (width - padding * 2) / (dataPoints - 1) : 0;
    const yScale = (height - padding * 2) / maxValue;

    const f32Points = this.data.f32
      .map((v, i) => `${padding + i * xScale},${height - padding - v * yScale}`)
      .join(' ');

    const bqPoints = this.data.bq
      .map((v, i) => `${padding + i * xScale},${height - padding - v * yScale}`)
      .join(' ');

    // Generate grid lines
    const gridLines = [0, 0.25, 0.5, 0.75, 1].map(v => {
      const y = height - padding - v * (height - padding * 2);
      return `
        <line
          x1="${padding}"
          y1="${y}"
          x2="${width - padding}"
          y2="${y}"
          stroke="var(--text-muted)"
          stroke-dasharray="4"
          stroke-opacity="0.3"
        />
        <text
          x="${padding - 8}"
          y="${y + 4}"
          fill="var(--text-secondary)"
          text-anchor="end"
          font-size="10"
          font-family="var(--font-mono)"
        >${(maxValue * v).toFixed(1)}</text>
      `;
    }).join('');

    // Generate dots
    const f32Dots = this.data.f32.map((v, i) => `
      <circle
        cx="${padding + i * xScale}"
        cy="${height - padding - v * yScale}"
        r="4"
        fill="var(--neon-cyan)"
      />
    `).join('');

    const bqDots = this.data.bq.map((v, i) => `
      <circle
        cx="${padding + i * xScale}"
        cy="${height - padding - v * yScale}"
        r="4"
        fill="var(--neon-magenta)"
      />
    `).join('');

    this.container.innerHTML = `
      <svg viewBox="0 0 ${width} ${height}" class="chart" preserveAspectRatio="xMidYMid meet">
        <!-- Grid -->
        <g class="chart__grid">${gridLines}</g>

        <!-- F32 Line -->
        ${dataPoints > 1 ? `
          <polyline
            points="${f32Points}"
            fill="none"
            stroke="var(--neon-cyan)"
            stroke-width="2"
            class="chart__line chart__line--f32"
          />
        ` : ''}
        <g class="chart__dots chart__dots--f32">${f32Dots}</g>

        <!-- BQ Line -->
        ${dataPoints > 1 ? `
          <polyline
            points="${bqPoints}"
            fill="none"
            stroke="var(--neon-magenta)"
            stroke-width="2"
            class="chart__line chart__line--bq"
          />
        ` : ''}
        <g class="chart__dots chart__dots--bq">${bqDots}</g>

        <!-- Legend -->
        <g class="chart__legend" transform="translate(${width - 100}, 20)">
          <rect x="0" y="0" width="12" height="12" fill="var(--neon-cyan)" rx="2"/>
          <text x="18" y="10" fill="var(--text-primary)" font-size="12" font-family="var(--font-mono)">F32</text>
          <rect x="0" y="20" width="12" height="12" fill="var(--neon-magenta)" rx="2"/>
          <text x="18" y="30" fill="var(--text-primary)" font-size="12" font-family="var(--font-mono)">BQ</text>
        </g>

        <!-- Y-axis label -->
        <text
          x="15"
          y="${height / 2}"
          fill="var(--text-muted)"
          font-size="10"
          font-family="var(--font-mono)"
          transform="rotate(-90, 15, ${height / 2})"
          text-anchor="middle"
        >LATENCY (ms)</text>
      </svg>
    `;
  }
}

// =============================================================================
// Memory Gauge Component
// =============================================================================

export class MemoryGauge {
  constructor(gaugeProgressId = 'gaugeProgress', gaugeTextId = 'gaugeText', statusId = 'memStatus') {
    this.progressEl = document.getElementById(gaugeProgressId);
    this.textEl = document.getElementById(gaugeTextId);
    this.statusEl = document.getElementById(statusId);
    this.circumference = 2 * Math.PI * 88; // radius = 88
    this.percentage = 0;
    this.status = 'NORMAL';
  }

  update(used, total) {
    if (total <= 0) {
      this.percentage = 0;
    } else {
      this.percentage = Math.min(100, (used / total) * 100);
    }

    // Determine status
    if (this.percentage < 70) {
      this.status = 'NORMAL';
    } else if (this.percentage < 90) {
      this.status = 'WARNING';
    } else {
      this.status = 'CRITICAL';
    }

    this.render();
  }

  render() {
    const offset = this.circumference - (this.percentage / 100) * this.circumference;

    const colors = {
      NORMAL: 'var(--neon-green)',
      WARNING: 'var(--neon-yellow)',
      CRITICAL: 'var(--neon-magenta)'
    };

    const color = colors[this.status];

    if (this.progressEl) {
      this.progressEl.style.strokeDashoffset = offset;
      this.progressEl.style.stroke = color;
      this.progressEl.style.filter = `drop-shadow(0 0 10px ${color})`;
    }

    if (this.textEl) {
      this.textEl.textContent = `${this.percentage.toFixed(1)}%`;
      this.textEl.style.fill = color;
    }

    if (this.statusEl) {
      this.statusEl.textContent = this.status;
      this.statusEl.style.color = color;
    }
  }
}

// =============================================================================
// Theme Manager
// =============================================================================

export class ThemeManager {
  constructor(toggleId = 'themeToggle', storageKey = 'edgevec-theme') {
    this.toggle = document.getElementById(toggleId);
    this.storageKey = storageKey;
    this.theme = this.loadTheme();

    this.applyTheme(this.theme);
    this.bindEvents();
  }

  loadTheme() {
    const stored = localStorage.getItem(this.storageKey);
    if (stored) return stored;

    // Check system preference
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches) {
      return 'light';
    }
    return 'dark';
  }

  saveTheme(theme) {
    localStorage.setItem(this.storageKey, theme);
  }

  applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    this.theme = theme;
    this.saveTheme(theme);
  }

  toggleTheme() {
    const newTheme = this.theme === 'dark' ? 'light' : 'dark';
    this.applyTheme(newTheme);
    return newTheme;
  }

  bindEvents() {
    if (this.toggle) {
      this.toggle.addEventListener('click', () => {
        this.toggleTheme();
      });
    }

    // Listen for system preference changes
    if (window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem(this.storageKey)) {
          this.applyTheme(e.matches ? 'dark' : 'light');
        }
      });
    }
  }
}

// =============================================================================
// Filter Tag Manager
// =============================================================================

export class FilterTagManager {
  constructor(containerId, inputId, onFilterChange) {
    this.container = document.getElementById(containerId);
    this.input = document.getElementById(inputId);
    this.onFilterChange = onFilterChange;
    this.bindEvents();
  }

  bindEvents() {
    if (this.container) {
      this.container.addEventListener('click', (e) => {
        const tag = e.target.closest('.filter-tag');
        if (tag) {
          const filter = tag.dataset.filter;
          if (this.input) {
            this.input.value = filter;
          }
          // Toggle active state
          this.container.querySelectorAll('.filter-tag').forEach(t => {
            t.classList.toggle('filter-tag--active', t === tag);
          });
          if (this.onFilterChange) {
            this.onFilterChange(filter);
          }
        }
      });
    }
  }

  clearActive() {
    if (this.container) {
      this.container.querySelectorAll('.filter-tag').forEach(t => {
        t.classList.remove('filter-tag--active');
      });
    }
  }
}

// =============================================================================
// Stats Updater
// =============================================================================

export class StatsUpdater {
  constructor(elements) {
    this.elements = elements;
  }

  update(stats) {
    for (const [key, value] of Object.entries(stats)) {
      const el = this.elements[key];
      if (el) {
        el.textContent = value;
        // Add animation class
        el.classList.remove('number-count');
        void el.offsetWidth; // Trigger reflow
        el.classList.add('number-count');
      }
    }
  }
}
