#!/usr/bin/env node
/**
 * EdgeVec Demo Screenshot Automation
 *
 * Captures screenshots of all demo pages for documentation.
 *
 * Usage:
 *   node scripts/capture-screenshots.js [--port 8080] [--output docs/images]
 *
 * Prerequisites:
 *   npm install puppeteer
 *   python -m http.server 8080  (in project root)
 */

const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');

// Configuration
const CONFIG = {
    port: parseInt(process.argv.find((_, i, arr) => arr[i - 1] === '--port') || '8080'),
    outputDir: process.argv.find((_, i, arr) => arr[i - 1] === '--output') || 'docs/images',
    baseUrl: null, // Set dynamically
    viewport: { width: 1200, height: 800, deviceScaleFactor: 2 },
    timeout: 30000,
};

CONFIG.baseUrl = `http://localhost:${CONFIG.port}`;

// Demo pages to capture
const DEMOS = [
    {
        name: 'filter-playground',
        url: '/wasm/examples/filter-playground.html',
        filename: 'playground-dark.png',
        setup: async (page) => {
            // Wait for WASM to load
            await page.waitForFunction(
                () => document.querySelector('#loading-placeholder')?.style.display === 'none' ||
                      !document.querySelector('#loading-placeholder'),
                { timeout: CONFIG.timeout }
            ).catch(() => console.log('  Note: Loading indicator not found, continuing...'));

            // Type a filter expression
            const filterInput = await page.$('#filter-expression, #filterInput, input[type="text"]');
            if (filterInput) {
                await filterInput.click({ clickCount: 3 }); // Select all
                await filterInput.type('category = "electronics" AND price < 500');
                await page.keyboard.press('Enter');
                await page.waitForTimeout(500); // Wait for parse
            }
        }
    },
    {
        name: 'filter-playground-light',
        url: '/wasm/examples/filter-playground.html',
        filename: 'playground-light.png',
        setup: async (page) => {
            // Wait for WASM
            await page.waitForFunction(
                () => document.querySelector('#loading-placeholder')?.style.display === 'none' ||
                      !document.querySelector('#loading-placeholder'),
                { timeout: CONFIG.timeout }
            ).catch(() => {});

            // Toggle to light theme
            const themeToggle = await page.$('#themeToggle, .theme-toggle, [aria-label*="theme"]');
            if (themeToggle) {
                await themeToggle.click();
                await page.waitForTimeout(300);
            }

            // Type filter
            const filterInput = await page.$('#filter-expression, #filterInput, input[type="text"]');
            if (filterInput) {
                await filterInput.click({ clickCount: 3 });
                await filterInput.type('category = "books" AND rating >= 4.5');
                await page.keyboard.press('Enter');
                await page.waitForTimeout(500);
            }
        }
    },
    {
        name: 'benchmark-dashboard',
        url: '/wasm/examples/benchmark-dashboard.html',
        filename: 'dashboard.png',
        setup: async (page) => {
            // Wait for charts to render
            await page.waitForFunction(
                () => document.querySelectorAll('canvas').length > 0,
                { timeout: CONFIG.timeout }
            ).catch(() => console.log('  Note: Charts not found, continuing...'));

            await page.waitForTimeout(1000); // Extra time for chart animations
        }
    },
    {
        name: 'soft-delete',
        url: '/wasm/examples/soft_delete.html',
        filename: 'soft-delete.png',
        setup: async (page) => {
            // Wait for WASM status
            await page.waitForFunction(
                () => {
                    const status = document.querySelector('#wasmStatus');
                    return status && status.textContent.includes('Ready');
                },
                { timeout: CONFIG.timeout }
            ).catch(() => console.log('  Note: WASM status not found, continuing...'));

            // Click "Insert 100" button if exists
            const insertBtn = await page.$('button:has-text("Insert"), .btn-insert');
            if (insertBtn) {
                await insertBtn.click();
                await page.waitForTimeout(500);
            }

            await page.waitForTimeout(500);
        }
    },
    {
        name: 'demo-catalog',
        url: '/wasm/examples/index.html',
        filename: 'demo-catalog.png',
        setup: async (page) => {
            // Wait for page to load
            await page.waitForFunction(
                () => document.querySelector('.demo-card, .demo-grid, .hero'),
                { timeout: CONFIG.timeout }
            ).catch(() => console.log('  Note: Demo cards not found, continuing...'));

            await page.waitForTimeout(500);
        }
    }
];

async function ensureOutputDir() {
    const outputPath = path.resolve(process.cwd(), CONFIG.outputDir);
    if (!fs.existsSync(outputPath)) {
        fs.mkdirSync(outputPath, { recursive: true });
        console.log(`Created output directory: ${outputPath}`);
    }
    return outputPath;
}

async function checkServer() {
    const http = require('http');
    return new Promise((resolve) => {
        const req = http.get(CONFIG.baseUrl, (res) => {
            resolve(res.statusCode === 200);
        });
        req.on('error', () => resolve(false));
        req.setTimeout(3000, () => {
            req.destroy();
            resolve(false);
        });
    });
}

async function captureScreenshot(browser, demo, outputDir) {
    console.log(`\nCapturing: ${demo.name}`);
    console.log(`  URL: ${CONFIG.baseUrl}${demo.url}`);

    const page = await browser.newPage();

    try {
        await page.setViewport(CONFIG.viewport);

        // Navigate to page
        await page.goto(`${CONFIG.baseUrl}${demo.url}`, {
            waitUntil: 'networkidle2',
            timeout: CONFIG.timeout
        });

        // Run demo-specific setup
        if (demo.setup) {
            console.log('  Running setup...');
            await demo.setup(page);
        }

        // Capture screenshot
        const outputPath = path.join(outputDir, demo.filename);
        await page.screenshot({
            path: outputPath,
            fullPage: false,
            type: 'png'
        });

        console.log(`  Saved: ${outputPath}`);

        // Get file size
        const stats = fs.statSync(outputPath);
        const sizeKB = (stats.size / 1024).toFixed(1);
        console.log(`  Size: ${sizeKB} KB`);

        if (stats.size > 200 * 1024) {
            console.log('  Warning: File exceeds 200KB target');
        }

        return { success: true, path: outputPath, size: stats.size };
    } catch (error) {
        console.error(`  Error: ${error.message}`);
        return { success: false, error: error.message };
    } finally {
        await page.close();
    }
}

async function main() {
    console.log('EdgeVec Screenshot Capture');
    console.log('==========================');
    console.log(`Server: ${CONFIG.baseUrl}`);
    console.log(`Output: ${CONFIG.outputDir}`);
    console.log(`Viewport: ${CONFIG.viewport.width}x${CONFIG.viewport.height} @${CONFIG.viewport.deviceScaleFactor}x`);

    // Check if server is running
    const serverRunning = await checkServer();
    if (!serverRunning) {
        console.error(`\nError: Server not running at ${CONFIG.baseUrl}`);
        console.error('Start it with: python -m http.server 8080');
        process.exit(1);
    }
    console.log('Server: OK');

    // Ensure output directory exists
    const outputDir = await ensureOutputDir();

    // Launch browser
    console.log('\nLaunching browser...');
    const browser = await puppeteer.launch({
        headless: 'new',
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });

    const results = [];

    try {
        for (const demo of DEMOS) {
            const result = await captureScreenshot(browser, demo, outputDir);
            results.push({ demo: demo.name, ...result });
        }
    } finally {
        await browser.close();
    }

    // Summary
    console.log('\n==========================');
    console.log('Summary');
    console.log('==========================');

    const successful = results.filter(r => r.success);
    const failed = results.filter(r => !r.success);

    console.log(`Captured: ${successful.length}/${results.length}`);

    if (successful.length > 0) {
        console.log('\nScreenshots:');
        successful.forEach(r => {
            const sizeKB = (r.size / 1024).toFixed(1);
            console.log(`  ${r.demo}: ${sizeKB} KB`);
        });
    }

    if (failed.length > 0) {
        console.log('\nFailed:');
        failed.forEach(r => {
            console.log(`  ${r.demo}: ${r.error}`);
        });
    }

    // Total size
    const totalSize = successful.reduce((sum, r) => sum + r.size, 0);
    console.log(`\nTotal size: ${(totalSize / 1024).toFixed(1)} KB`);

    if (failed.length > 0) {
        process.exit(1);
    }
}

main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
});
