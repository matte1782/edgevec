/**
 * EdgeVec Animation Utilities
 * Provides reusable animation helpers for the cyberpunk UI
 * @version 0.6.0
 */

// =============================================================================
// Number Counter Animation
// =============================================================================

export class NumberCounter {
  /**
   * Animates a number from start to end value
   * @param {HTMLElement} element - Element to update
   * @param {number} endValue - Target value
   * @param {Object} options - Animation options
   */
  constructor(element, endValue, options = {}) {
    this.element = element;
    this.endValue = endValue;
    this.startValue = options.startValue ?? 0;
    this.duration = options.duration ?? 1000;
    this.decimals = options.decimals ?? 0;
    this.prefix = options.prefix ?? '';
    this.suffix = options.suffix ?? '';
    this.easing = options.easing ?? this.easeOutExpo;
    this.onComplete = options.onComplete ?? null;

    this.startTime = null;
    this.animationId = null;
  }

  easeOutExpo(t) {
    return t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
  }

  easeOutCubic(t) {
    return 1 - Math.pow(1 - t, 3);
  }

  start() {
    // Respect reduced motion preference
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.element.textContent = this.format(this.endValue);
      if (this.onComplete) this.onComplete();
      return;
    }

    this.startTime = performance.now();
    this.animate();
  }

  animate() {
    const currentTime = performance.now();
    const elapsed = currentTime - this.startTime;
    const progress = Math.min(elapsed / this.duration, 1);
    const easedProgress = this.easing(progress);

    const currentValue = this.startValue + (this.endValue - this.startValue) * easedProgress;
    this.element.textContent = this.format(currentValue);

    if (progress < 1) {
      this.animationId = requestAnimationFrame(() => this.animate());
    } else {
      this.element.textContent = this.format(this.endValue);
      if (this.onComplete) this.onComplete();
    }
  }

  format(value) {
    const formatted = this.decimals > 0
      ? value.toFixed(this.decimals)
      : Math.round(value).toString();
    return `${this.prefix}${formatted}${this.suffix}`;
  }

  stop() {
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }
}

// =============================================================================
// Ripple Effect
// =============================================================================

/**
 * Creates a ripple effect on click
 * @param {MouseEvent} event - Click event
 * @param {HTMLElement} container - Container for the ripple
 * @param {Object} options - Ripple options
 */
export function createRipple(event, container, options = {}) {
  // Respect reduced motion preference
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    return null;
  }

  const color = options.color ?? 'rgba(0, 255, 255, 0.4)';
  const duration = options.duration ?? 600;
  const size = options.size ?? Math.max(container.offsetWidth, container.offsetHeight) * 2;

  const ripple = document.createElement('span');
  ripple.className = 'ripple-effect';

  // Calculate position
  const rect = container.getBoundingClientRect();
  const x = event.clientX - rect.left - size / 2;
  const y = event.clientY - rect.top - size / 2;

  // Apply styles
  Object.assign(ripple.style, {
    position: 'absolute',
    width: `${size}px`,
    height: `${size}px`,
    left: `${x}px`,
    top: `${y}px`,
    background: `radial-gradient(circle, ${color} 0%, transparent 70%)`,
    borderRadius: '50%',
    transform: 'scale(0)',
    opacity: '1',
    pointerEvents: 'none',
    zIndex: '1000'
  });

  // Ensure container has position for absolute positioning
  const containerPosition = getComputedStyle(container).position;
  if (containerPosition === 'static') {
    container.style.position = 'relative';
  }
  container.style.overflow = 'hidden';

  container.appendChild(ripple);

  // Animate
  ripple.animate([
    { transform: 'scale(0)', opacity: 1 },
    { transform: 'scale(1)', opacity: 0 }
  ], {
    duration: duration,
    easing: 'cubic-bezier(0.4, 0, 0.2, 1)'
  }).onfinish = () => {
    ripple.remove();
  };

  return ripple;
}

// =============================================================================
// Stagger Animation
// =============================================================================

/**
 * Applies staggered animation to a list of elements
 * @param {NodeList|HTMLElement[]} elements - Elements to animate
 * @param {Object} options - Animation options
 */
export function staggerAnimation(elements, options = {}) {
  // Respect reduced motion preference
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    elements.forEach(el => {
      el.style.opacity = '1';
      el.style.transform = 'none';
    });
    return;
  }

  const delay = options.delay ?? 100;
  const duration = options.duration ?? 500;
  const stagger = options.stagger ?? 50;
  const from = options.from ?? { opacity: 0, transform: 'translateY(20px)' };
  const to = options.to ?? { opacity: 1, transform: 'translateY(0)' };
  const easing = options.easing ?? 'cubic-bezier(0.34, 1.56, 0.64, 1)';

  elements.forEach((element, index) => {
    // Set initial state
    Object.assign(element.style, from);

    // Create animation after staggered delay
    setTimeout(() => {
      element.animate([from, to], {
        duration: duration,
        easing: easing,
        fill: 'forwards'
      });
    }, delay + index * stagger);
  });
}

// =============================================================================
// Scroll-triggered Animations
// =============================================================================

/**
 * Initializes scroll-triggered animations using IntersectionObserver
 * @param {Object} options - Observer options
 * @returns {IntersectionObserver}
 */
export function initScrollAnimations(options = {}) {
  // Respect reduced motion preference
  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  const threshold = options.threshold ?? 0.1;
  const rootMargin = options.rootMargin ?? '0px 0px -50px 0px';
  const once = options.once ?? true;
  const selector = options.selector ?? '[data-scroll-animate]';

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const element = entry.target;

        if (reduceMotion) {
          // Skip animation, just show element
          element.classList.add('scroll-visible');
          element.classList.remove('scroll-hidden');
        } else {
          // Trigger animation
          const animation = element.dataset.scrollAnimate || 'fade-in';
          element.classList.add('scroll-visible', `animate-${animation}`);
          element.classList.remove('scroll-hidden');
        }

        if (once) {
          observer.unobserve(element);
        }
      } else if (!once) {
        entry.target.classList.remove('scroll-visible');
        entry.target.classList.add('scroll-hidden');
      }
    });
  }, {
    threshold,
    rootMargin
  });

  // Observe all elements with the selector
  const elements = document.querySelectorAll(selector);
  elements.forEach(el => {
    el.classList.add('scroll-hidden');
    observer.observe(el);
  });

  return observer;
}

// =============================================================================
// Smooth Scroll
// =============================================================================

/**
 * Initializes smooth scrolling for anchor links
 * @param {Object} options - Scroll options
 * @param {number} [options.offset=80] - Offset from top in pixels
 * @param {string} [options.selector='a[href^="#"]'] - CSS selector for anchor links
 */
export function initSmoothScroll(options = {}) {
  const offset = options.offset ?? 80;
  const selector = options.selector ?? 'a[href^="#"]';

  // Respect reduced motion preference - use instant scroll
  const behavior = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    ? 'auto'
    : 'smooth';

  document.querySelectorAll(selector).forEach(anchor => {
    anchor.addEventListener('click', (e) => {
      const href = anchor.getAttribute('href');
      if (!href || href === '#') return;

      const target = document.querySelector(href);
      if (!target) return;

      e.preventDefault();

      const targetPosition = target.getBoundingClientRect().top + window.scrollY - offset;

      window.scrollTo({
        top: targetPosition,
        behavior: behavior
      });

      // Update URL without jumping
      history.pushState(null, '', href);
    });
  });
}

// =============================================================================
// Typewriter Effect
// =============================================================================

export class TypewriterEffect {
  /**
   * Creates a typewriter text animation
   * @param {HTMLElement} element - Element to type into
   * @param {string} text - Text to type
   * @param {Object} options - Animation options
   */
  constructor(element, text, options = {}) {
    this.element = element;
    this.text = text;
    this.speed = options.speed ?? 50;
    this.delay = options.delay ?? 0;
    this.cursor = options.cursor ?? true;
    this.cursorChar = options.cursorChar ?? '▋';
    this.onComplete = options.onComplete ?? null;

    this.currentIndex = 0;
    this.timeoutId = null;
  }

  start() {
    // Respect reduced motion preference
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.element.textContent = this.text;
      if (this.onComplete) this.onComplete();
      return;
    }

    this.element.textContent = '';

    setTimeout(() => {
      this.type();
    }, this.delay);
  }

  type() {
    if (this.currentIndex < this.text.length) {
      const currentText = this.text.substring(0, this.currentIndex + 1);
      this.element.textContent = this.cursor
        ? currentText + this.cursorChar
        : currentText;

      this.currentIndex++;
      this.timeoutId = setTimeout(() => this.type(), this.speed);
    } else {
      // Typing complete
      if (this.cursor) {
        // Blinking cursor at the end
        this.element.classList.add('typewriter-complete');
      }
      if (this.onComplete) this.onComplete();
    }
  }

  stop() {
    if (this.timeoutId) {
      clearTimeout(this.timeoutId);
      this.timeoutId = null;
    }
  }
}

// =============================================================================
// Glitch Text Effect
// =============================================================================

/**
 * Applies glitch effect to text on hover/interval
 * @param {HTMLElement} element - Element with text
 * @param {Object} options - Effect options
 */
export function glitchText(element, options = {}) {
  // Respect reduced motion preference
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    return null;
  }

  const originalText = element.textContent;
  const duration = options.duration ?? 100;
  const iterations = options.iterations ?? 3;
  const chars = options.chars ?? '!@#$%^&*()_+-=[]{}|;:,.<>?/~`';

  let iteration = 0;

  const interval = setInterval(() => {
    element.textContent = originalText
      .split('')
      .map((char, index) => {
        if (index < iteration) {
          return originalText[index];
        }
        return chars[Math.floor(Math.random() * chars.length)];
      })
      .join('');

    if (iteration >= originalText.length) {
      clearInterval(interval);
      element.textContent = originalText;
    }

    iteration += 1 / iterations;
  }, duration / originalText.length);

  return interval;
}

// =============================================================================
// Pulse Animation
// =============================================================================

/**
 * Creates a pulse animation on an element
 * @param {HTMLElement} element - Element to pulse
 * @param {Object} options - Pulse options
 */
export function pulseElement(element, options = {}) {
  // Respect reduced motion preference
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    return null;
  }

  const color = options.color ?? 'var(--neon-cyan)';
  const duration = options.duration ?? 600;
  const scale = options.scale ?? 1.05;

  return element.animate([
    {
      transform: 'scale(1)',
      boxShadow: `0 0 0 0 ${color}`
    },
    {
      transform: `scale(${scale})`,
      boxShadow: `0 0 20px 10px transparent`
    },
    {
      transform: 'scale(1)',
      boxShadow: `0 0 0 0 ${color}`
    }
  ], {
    duration: duration,
    easing: 'cubic-bezier(0.4, 0, 0.6, 1)'
  });
}

// =============================================================================
// Animation Presets
// =============================================================================

export const ANIMATION_PRESETS = {
  fadeIn: {
    from: { opacity: 0 },
    to: { opacity: 1 },
    duration: 300,
    easing: 'ease-out'
  },

  fadeInUp: {
    from: { opacity: 0, transform: 'translateY(20px)' },
    to: { opacity: 1, transform: 'translateY(0)' },
    duration: 400,
    easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)'
  },

  fadeInDown: {
    from: { opacity: 0, transform: 'translateY(-20px)' },
    to: { opacity: 1, transform: 'translateY(0)' },
    duration: 400,
    easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)'
  },

  fadeInLeft: {
    from: { opacity: 0, transform: 'translateX(-20px)' },
    to: { opacity: 1, transform: 'translateX(0)' },
    duration: 400,
    easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)'
  },

  fadeInRight: {
    from: { opacity: 0, transform: 'translateX(20px)' },
    to: { opacity: 1, transform: 'translateX(0)' },
    duration: 400,
    easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)'
  },

  scaleIn: {
    from: { opacity: 0, transform: 'scale(0.9)' },
    to: { opacity: 1, transform: 'scale(1)' },
    duration: 300,
    easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)'
  },

  slideInUp: {
    from: { transform: 'translateY(100%)' },
    to: { transform: 'translateY(0)' },
    duration: 400,
    easing: 'cubic-bezier(0.16, 1, 0.3, 1)'
  },

  bounce: {
    keyframes: [
      { transform: 'translateY(0)' },
      { transform: 'translateY(-10px)' },
      { transform: 'translateY(0)' },
      { transform: 'translateY(-5px)' },
      { transform: 'translateY(0)' }
    ],
    duration: 600,
    easing: 'ease-out'
  }
};

/**
 * Applies an animation preset to an element
 * @param {HTMLElement} element - Element to animate
 * @param {string} presetName - Name of the preset
 * @param {Object} overrides - Override preset options
 */
export function applyPreset(element, presetName, overrides = {}) {
  // Respect reduced motion preference
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    if (overrides.to) {
      Object.assign(element.style, overrides.to);
    }
    return null;
  }

  const preset = ANIMATION_PRESETS[presetName];
  if (!preset) {
    console.warn(`Animation preset "${presetName}" not found`);
    return null;
  }

  const options = { ...preset, ...overrides };

  if (options.keyframes) {
    return element.animate(options.keyframes, {
      duration: options.duration,
      easing: options.easing,
      fill: options.fill ?? 'forwards'
    });
  }

  return element.animate([options.from, options.to], {
    duration: options.duration,
    easing: options.easing,
    fill: options.fill ?? 'forwards'
  });
}
