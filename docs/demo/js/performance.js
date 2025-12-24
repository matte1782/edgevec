/**
 * EdgeVec Performance Utilities
 * Debouncing, throttling, lazy loading, and performance monitoring
 * @version 0.6.0
 */

// =============================================================================
// Debounce & Throttle
// =============================================================================

/**
 * Debounces a function - delays execution until after wait milliseconds
 * have elapsed since the last time the debounced function was invoked.
 * @param {Function} func - Function to debounce
 * @param {number} wait - Milliseconds to wait
 * @param {Object} options - Options
 * @returns {Function} Debounced function
 */
export function debounce(func, wait = 250, options = {}) {
  let timeoutId = null;
  let lastArgs = null;
  let lastThis = null;
  let result = null;
  let lastCallTime = null;
  const leading = options.leading ?? false;
  const trailing = options.trailing ?? true;
  const maxWait = options.maxWait ?? null;

  function invokeFunc(time) {
    const args = lastArgs;
    const thisArg = lastThis;
    lastArgs = null;
    lastThis = null;
    lastCallTime = time;
    result = func.apply(thisArg, args);
    return result;
  }

  function shouldInvoke(time) {
    const timeSinceLastCall = lastCallTime === null ? 0 : time - lastCallTime;
    return (
      lastCallTime === null ||
      timeSinceLastCall >= wait ||
      timeSinceLastCall < 0 ||
      (maxWait !== null && timeSinceLastCall >= maxWait)
    );
  }

  function timerExpired() {
    const time = Date.now();
    if (shouldInvoke(time)) {
      return trailingEdge(time);
    }
    const remainingWait = wait - (time - lastCallTime);
    const maxRemaining = maxWait !== null ? maxWait - (time - lastCallTime) : remainingWait;
    timeoutId = setTimeout(timerExpired, Math.min(remainingWait, maxRemaining));
  }

  function trailingEdge(time) {
    timeoutId = null;
    if (trailing && lastArgs) {
      return invokeFunc(time);
    }
    lastArgs = null;
    lastThis = null;
    return result;
  }

  function leadingEdge(time) {
    lastCallTime = time;
    timeoutId = setTimeout(timerExpired, wait);
    return leading ? invokeFunc(time) : result;
  }

  function debounced(...args) {
    const time = Date.now();
    const isInvoking = shouldInvoke(time);

    lastArgs = args;
    lastThis = this;

    if (isInvoking) {
      if (timeoutId === null) {
        return leadingEdge(time);
      }
      if (maxWait !== null) {
        timeoutId = setTimeout(timerExpired, wait);
        return invokeFunc(time);
      }
    }

    if (timeoutId === null) {
      timeoutId = setTimeout(timerExpired, wait);
    }

    return result;
  }

  debounced.cancel = function() {
    if (timeoutId !== null) {
      clearTimeout(timeoutId);
    }
    lastCallTime = null;
    lastArgs = null;
    lastThis = null;
    timeoutId = null;
  };

  debounced.flush = function() {
    return timeoutId === null ? result : trailingEdge(Date.now());
  };

  debounced.pending = function() {
    return timeoutId !== null;
  };

  return debounced;
}

/**
 * Throttles a function - ensures it's called at most once per wait period.
 * @param {Function} func - Function to throttle
 * @param {number} wait - Milliseconds between allowed calls
 * @param {Object} options - Options
 * @returns {Function} Throttled function
 */
export function throttle(func, wait = 100, options = {}) {
  const leading = options.leading ?? true;
  const trailing = options.trailing ?? true;

  return debounce(func, wait, {
    leading,
    trailing,
    maxWait: wait
  });
}

// =============================================================================
// Lazy Loading
// =============================================================================

/**
 * Creates a lazy loader for images and iframes using IntersectionObserver.
 * @param {Object} options - Lazy loading options
 * @returns {Object} Lazy loader instance
 */
export function createLazyLoader(options = {}) {
  const rootMargin = options.rootMargin ?? '100px';
  const threshold = options.threshold ?? 0;
  const selector = options.selector ?? '[data-lazy]';
  const loadedClass = options.loadedClass ?? 'lazy-loaded';
  const errorClass = options.errorClass ?? 'lazy-error';
  const onLoad = options.onLoad ?? null;
  const onError = options.onError ?? null;

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const element = entry.target;
        loadElement(element);
        observer.unobserve(element);
      }
    });
  }, {
    rootMargin,
    threshold
  });

  function loadElement(element) {
    const src = element.dataset.src || element.dataset.lazy;
    const srcset = element.dataset.srcset;
    const bgImage = element.dataset.bgImage;

    if (src) {
      if (element.tagName === 'IMG') {
        element.onload = () => {
          element.classList.add(loadedClass);
          if (onLoad) onLoad(element);
        };
        element.onerror = () => {
          element.classList.add(errorClass);
          if (onError) onError(element);
        };
        element.src = src;
        if (srcset) {
          element.srcset = srcset;
        }
      } else if (element.tagName === 'IFRAME') {
        element.onload = () => {
          element.classList.add(loadedClass);
          if (onLoad) onLoad(element);
        };
        element.src = src;
      }
    }

    if (bgImage) {
      const img = new Image();
      img.onload = () => {
        element.style.backgroundImage = `url(${bgImage})`;
        element.classList.add(loadedClass);
        if (onLoad) onLoad(element);
      };
      img.onerror = () => {
        element.classList.add(errorClass);
        if (onError) onError(element);
      };
      img.src = bgImage;
    }

    // Remove data attributes
    delete element.dataset.src;
    delete element.dataset.srcset;
    delete element.dataset.bgImage;
    delete element.dataset.lazy;
  }

  function observe(element) {
    observer.observe(element);
  }

  function observeAll() {
    document.querySelectorAll(selector).forEach(el => {
      observer.observe(el);
    });
  }

  function disconnect() {
    observer.disconnect();
  }

  return {
    observe,
    observeAll,
    disconnect,
    loadElement
  };
}

// =============================================================================
// Performance Monitoring
// =============================================================================

/**
 * Performance monitor for tracking operation timing.
 */
export class PerformanceMonitor {
  constructor(options = {}) {
    this.metrics = new Map();
    this.enabled = options.enabled ?? true;
    this.maxSamples = options.maxSamples ?? 100;
    this.onMetric = options.onMetric ?? null;
  }

  /**
   * Start timing an operation.
   * @param {string} name - Operation name
   * @returns {Function} End function to call when operation completes
   */
  start(name) {
    if (!this.enabled) {
      return () => 0;
    }

    const startTime = performance.now();

    return () => {
      const duration = performance.now() - startTime;
      this.record(name, duration);
      return duration;
    };
  }

  /**
   * Record a metric value.
   * @param {string} name - Metric name
   * @param {number} value - Metric value
   */
  record(name, value) {
    if (!this.enabled) return;

    if (!this.metrics.has(name)) {
      this.metrics.set(name, []);
    }

    const samples = this.metrics.get(name);
    samples.push(value);

    // Keep only the last maxSamples
    if (samples.length > this.maxSamples) {
      samples.shift();
    }

    if (this.onMetric) {
      this.onMetric(name, value, this.getStats(name));
    }
  }

  /**
   * Get statistics for a metric.
   * @param {string} name - Metric name
   * @returns {Object} Statistics
   */
  getStats(name) {
    const samples = this.metrics.get(name);
    if (!samples || samples.length === 0) {
      return null;
    }

    const sorted = [...samples].sort((a, b) => a - b);
    const sum = samples.reduce((a, b) => a + b, 0);
    const count = samples.length;

    return {
      count,
      min: sorted[0],
      max: sorted[count - 1],
      mean: sum / count,
      median: sorted[Math.floor(count / 2)],
      p95: sorted[Math.floor(count * 0.95)],
      p99: sorted[Math.floor(count * 0.99)],
      sum,
      samples: [...samples]
    };
  }

  /**
   * Get all metrics.
   * @returns {Object} All metrics with statistics
   */
  getAll() {
    const result = {};
    for (const [name] of this.metrics) {
      result[name] = this.getStats(name);
    }
    return result;
  }

  /**
   * Clear all metrics.
   */
  clear() {
    this.metrics.clear();
  }

  /**
   * Clear a specific metric.
   * @param {string} name - Metric name
   */
  clearMetric(name) {
    this.metrics.delete(name);
  }
}

// =============================================================================
// Frame Rate Monitor
// =============================================================================

/**
 * Monitors frame rate (FPS) for performance debugging.
 */
export class FPSMonitor {
  constructor(options = {}) {
    this.samples = [];
    this.maxSamples = options.maxSamples ?? 60;
    this.lastFrameTime = 0;
    this.animationId = null;
    this.isRunning = false;
    this.onUpdate = options.onUpdate ?? null;
    this.updateInterval = options.updateInterval ?? 500;
    this.lastUpdateTime = 0;
  }

  start() {
    if (this.isRunning) return;
    this.isRunning = true;
    this.lastFrameTime = performance.now();
    this.lastUpdateTime = performance.now();
    this.tick();
  }

  tick() {
    if (!this.isRunning) return;

    const now = performance.now();
    const delta = now - this.lastFrameTime;
    this.lastFrameTime = now;

    // Calculate FPS for this frame
    const fps = 1000 / delta;
    this.samples.push(fps);

    // Keep only last N samples
    if (this.samples.length > this.maxSamples) {
      this.samples.shift();
    }

    // Call onUpdate at specified interval
    if (this.onUpdate && now - this.lastUpdateTime >= this.updateInterval) {
      this.lastUpdateTime = now;
      this.onUpdate(this.getStats());
    }

    this.animationId = requestAnimationFrame(() => this.tick());
  }

  stop() {
    this.isRunning = false;
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  getStats() {
    if (this.samples.length === 0) {
      return { current: 0, average: 0, min: 0, max: 0 };
    }

    const current = this.samples[this.samples.length - 1];
    const sum = this.samples.reduce((a, b) => a + b, 0);
    const average = sum / this.samples.length;
    const min = Math.min(...this.samples);
    const max = Math.max(...this.samples);

    return {
      current: Math.round(current),
      average: Math.round(average),
      min: Math.round(min),
      max: Math.round(max)
    };
  }
}

// =============================================================================
// Memory Monitor (WASM Specific)
// =============================================================================

/**
 * Monitors WASM memory usage.
 */
export class MemoryMonitor {
  constructor(options = {}) {
    this.wasmMemory = options.wasmMemory ?? null;
    this.intervalId = null;
    this.interval = options.interval ?? 1000;
    this.onUpdate = options.onUpdate ?? null;
    this.history = [];
    this.maxHistory = options.maxHistory ?? 60;
  }

  setWasmMemory(memory) {
    this.wasmMemory = memory;
  }

  start() {
    if (this.intervalId) return;

    this.intervalId = setInterval(() => {
      const stats = this.getStats();
      this.history.push({
        timestamp: Date.now(),
        ...stats
      });

      if (this.history.length > this.maxHistory) {
        this.history.shift();
      }

      if (this.onUpdate) {
        this.onUpdate(stats);
      }
    }, this.interval);
  }

  stop() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  getStats() {
    const result = {
      wasmBytes: 0,
      wasmMB: 0,
      jsHeapBytes: 0,
      jsHeapMB: 0,
      jsHeapLimit: 0,
      jsHeapUsedPercent: 0
    };

    // WASM memory
    if (this.wasmMemory) {
      result.wasmBytes = this.wasmMemory.buffer.byteLength;
      result.wasmMB = result.wasmBytes / (1024 * 1024);
    }

    // JS heap (if available)
    if (performance.memory) {
      result.jsHeapBytes = performance.memory.usedJSHeapSize;
      result.jsHeapMB = result.jsHeapBytes / (1024 * 1024);
      result.jsHeapLimit = performance.memory.jsHeapSizeLimit / (1024 * 1024);
      result.jsHeapUsedPercent = (result.jsHeapBytes / performance.memory.jsHeapSizeLimit) * 100;
    }

    return result;
  }

  getHistory() {
    return [...this.history];
  }
}

// =============================================================================
// Request Idle Callback Polyfill
// =============================================================================

/**
 * Schedules work during browser idle periods.
 * Falls back to setTimeout if requestIdleCallback is not available.
 * @param {Function} callback - Work to perform
 * @param {Object} options - Options
 * @returns {number} Handle for cancellation
 */
export function scheduleIdleWork(callback, options = {}) {
  const timeout = options.timeout ?? 1000;

  if (typeof requestIdleCallback !== 'undefined') {
    return requestIdleCallback(callback, { timeout });
  }

  // Fallback for Safari
  return setTimeout(() => {
    callback({
      didTimeout: false,
      timeRemaining: () => 50
    });
  }, 1);
}

/**
 * Cancels scheduled idle work.
 * @param {number} handle - Handle from scheduleIdleWork
 */
export function cancelIdleWork(handle) {
  if (typeof cancelIdleCallback !== 'undefined') {
    cancelIdleCallback(handle);
  } else {
    clearTimeout(handle);
  }
}

// =============================================================================
// Batch DOM Updates
// =============================================================================

/**
 * Batches DOM updates to avoid layout thrashing.
 */
export class DOMBatcher {
  constructor() {
    this.reads = [];
    this.writes = [];
    this.scheduled = false;
  }

  read(fn) {
    this.reads.push(fn);
    this.scheduleFlush();
    return this;
  }

  write(fn) {
    this.writes.push(fn);
    this.scheduleFlush();
    return this;
  }

  scheduleFlush() {
    if (this.scheduled) return;
    this.scheduled = true;

    requestAnimationFrame(() => {
      this.flush();
    });
  }

  flush() {
    // Execute all reads first
    const reads = this.reads;
    this.reads = [];
    reads.forEach(fn => fn());

    // Then execute all writes
    const writes = this.writes;
    this.writes = [];
    writes.forEach(fn => fn());

    this.scheduled = false;

    // If new work was added during flush, schedule again
    if (this.reads.length > 0 || this.writes.length > 0) {
      this.scheduleFlush();
    }
  }
}

// Global instance for convenience
export const domBatcher = new DOMBatcher();

// =============================================================================
// Resource Timing
// =============================================================================

/**
 * Gets timing information for loaded resources.
 * @param {Object} options - Filter options
 * @returns {Array} Resource timing entries
 */
export function getResourceTimings(options = {}) {
  const type = options.type ?? null;
  const minDuration = options.minDuration ?? 0;

  let entries = performance.getEntriesByType('resource');

  if (type) {
    entries = entries.filter(e => e.initiatorType === type);
  }

  if (minDuration > 0) {
    entries = entries.filter(e => e.duration >= minDuration);
  }

  return entries.map(e => ({
    name: e.name,
    type: e.initiatorType,
    duration: Math.round(e.duration),
    size: e.transferSize,
    cached: e.transferSize === 0 && e.decodedBodySize > 0
  }));
}

/**
 * Gets navigation timing metrics.
 * @returns {Object} Navigation timing
 */
export function getNavigationTiming() {
  const nav = performance.getEntriesByType('navigation')[0];
  if (!nav) return null;

  return {
    dns: Math.round(nav.domainLookupEnd - nav.domainLookupStart),
    tcp: Math.round(nav.connectEnd - nav.connectStart),
    ttfb: Math.round(nav.responseStart - nav.requestStart),
    download: Math.round(nav.responseEnd - nav.responseStart),
    domParse: Math.round(nav.domInteractive - nav.responseEnd),
    domContentLoaded: Math.round(nav.domContentLoadedEventEnd - nav.fetchStart),
    load: Math.round(nav.loadEventEnd - nav.fetchStart)
  };
}
