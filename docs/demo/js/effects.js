/**
 * EdgeVec Cyberpunk Visual Effects
 * Performance-optimized particle and matrix effects
 * @version 0.6.0
 */

// =============================================================================
// Particle System
// =============================================================================

export class ParticleSystem {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    if (!this.canvas) {
      console.warn(`ParticleSystem: Canvas "${canvasId}" not found`);
      return;
    }
    this.ctx = this.canvas.getContext('2d');
    this.particles = [];
    this.mouse = { x: null, y: null, radius: 150 };
    this.animationId = null;
    this.isRunning = false;

    // Store bound event handlers for cleanup
    this._boundHandlers = {
      resize: this._handleResize.bind(this),
      mousemove: this._handleMouseMove.bind(this),
      mouseout: this._handleMouseOut.bind(this),
      touchmove: this._handleTouchMove.bind(this),
      touchend: this._handleTouchEnd.bind(this),
      motionChange: this._handleMotionChange.bind(this)
    };

    this.resize();
    this.init();
    this.bindEvents();
  }

  _handleResize() {
    this.resize();
    this.init();
  }

  _handleMouseMove(e) {
    this.mouse.x = e.clientX;
    this.mouse.y = e.clientY;
  }

  _handleMouseOut() {
    this.mouse.x = null;
    this.mouse.y = null;
  }

  _handleTouchMove(e) {
    if (e.touches.length > 0) {
      this.mouse.x = e.touches[0].clientX;
      this.mouse.y = e.touches[0].clientY;
    }
  }

  _handleTouchEnd() {
    this.mouse.x = null;
    this.mouse.y = null;
  }

  _handleMotionChange(e) {
    if (e.matches) {
      this.stop();
    } else {
      this.start();
    }
  }

  resize() {
    if (!this.canvas) return;
    this.canvas.width = window.innerWidth;
    this.canvas.height = window.innerHeight;
  }

  init() {
    if (!this.canvas) return;
    // Limit particles based on screen size for performance
    const particleCount = Math.min(100, Math.floor((this.canvas.width * this.canvas.height) / 15000));

    this.particles = [];
    for (let i = 0; i < particleCount; i++) {
      this.particles.push(new Particle(this.canvas));
    }
  }

  bindEvents() {
    window.addEventListener('resize', this._boundHandlers.resize);
    window.addEventListener('mousemove', this._boundHandlers.mousemove);
    window.addEventListener('mouseout', this._boundHandlers.mouseout);
    window.addEventListener('touchmove', this._boundHandlers.touchmove);
    window.addEventListener('touchend', this._boundHandlers.touchend);

    // Respect reduced motion preference
    this._mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    if (this._mediaQuery.matches) {
      this.stop();
    }
    this._mediaQuery.addEventListener('change', this._boundHandlers.motionChange);
  }

  unbindEvents() {
    window.removeEventListener('resize', this._boundHandlers.resize);
    window.removeEventListener('mousemove', this._boundHandlers.mousemove);
    window.removeEventListener('mouseout', this._boundHandlers.mouseout);
    window.removeEventListener('touchmove', this._boundHandlers.touchmove);
    window.removeEventListener('touchend', this._boundHandlers.touchend);
    if (this._mediaQuery) {
      this._mediaQuery.removeEventListener('change', this._boundHandlers.motionChange);
    }
  }

  animate() {
    if (!this.ctx || !this.isRunning) return;

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
    if (!this.canvas) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }
    this.isRunning = true;
    this.animate();
  }

  stop() {
    this.isRunning = false;
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  destroy() {
    this.stop();
    this.unbindEvents();
    if (this.ctx) {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
    this.particles = [];
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

export class MatrixRain {
  constructor(canvasId) {
    this.canvas = document.getElementById(canvasId);
    if (!this.canvas) {
      console.warn(`MatrixRain: Canvas "${canvasId}" not found`);
      return;
    }
    this.ctx = this.canvas.getContext('2d');
    this.columns = [];
    this.fontSize = 14;
    this.animationId = null;
    this.isRunning = false;

    // Characters: mix of katakana, latin, and numbers
    this.chars = 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';

    // Store bound event handlers for cleanup
    this._boundHandlers = {
      resize: this._handleResize.bind(this),
      motionChange: this._handleMotionChange.bind(this)
    };

    this.resize();
    this.init();
    this.bindEvents();
  }

  _handleResize() {
    this.resize();
    this.init();
  }

  _handleMotionChange(e) {
    if (e.matches) {
      this.stop();
    } else {
      this.start();
    }
  }

  bindEvents() {
    window.addEventListener('resize', this._boundHandlers.resize);

    // Respect reduced motion preference
    this._mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    if (this._mediaQuery.matches) {
      this.stop();
    }
    this._mediaQuery.addEventListener('change', this._boundHandlers.motionChange);
  }

  unbindEvents() {
    window.removeEventListener('resize', this._boundHandlers.resize);
    if (this._mediaQuery) {
      this._mediaQuery.removeEventListener('change', this._boundHandlers.motionChange);
    }
  }

  resize() {
    if (!this.canvas) return;
    this.canvas.width = window.innerWidth;
    this.canvas.height = window.innerHeight;
  }

  init() {
    if (!this.canvas) return;
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
    if (!this.ctx || !this.isRunning) return;

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
    if (!this.canvas) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      return;
    }
    this.isRunning = true;
    this.animate();
  }

  stop() {
    this.isRunning = false;
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  destroy() {
    this.stop();
    this.unbindEvents();
    if (this.ctx) {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
    this.columns = [];
  }
}

// =============================================================================
// Effect Manager - Coordinates multiple effects
// =============================================================================

export class EffectManager {
  constructor() {
    this.effects = [];
    this.isEnabled = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    // Listen for preference changes
    window.matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change', (e) => {
      this.isEnabled = !e.matches;
      if (this.isEnabled) {
        this.startAll();
      } else {
        this.stopAll();
      }
    });
  }

  add(effect) {
    this.effects.push(effect);
    if (this.isEnabled) {
      effect.start();
    }
  }

  startAll() {
    this.effects.forEach(effect => effect.start());
  }

  stopAll() {
    this.effects.forEach(effect => effect.stop());
  }

  destroyAll() {
    this.effects.forEach(effect => effect.destroy());
    this.effects = [];
  }
}
