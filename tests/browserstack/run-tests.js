/**
 * EdgeVec BrowserStack Test Runner
 *
 * Runs the mobile test suite on BrowserStack real devices.
 *
 * Usage:
 *   node run-tests.js --platform ios --browser safari --os-version 16 --device "iPhone 14"
 *   node run-tests.js --platform android --browser chrome --os-version 13 --device "Google Pixel 7"
 */

const { Builder, By, until } = require('selenium-webdriver');
const fs = require('fs');
const path = require('path');

// Parse command line arguments
const args = process.argv.slice(2);
const options = {};
for (let i = 0; i < args.length; i += 2) {
  if (args[i].startsWith('--')) {
    options[args[i].replace('--', '')] = args[i + 1];
  }
}

// Validate required options
const requiredOptions = ['platform', 'browser', 'os-version', 'device'];
for (const opt of requiredOptions) {
  if (!options[opt]) {
    console.error(`Missing required option: --${opt}`);
    process.exit(1);
  }
}

// Validate environment variables
if (!process.env.BROWSERSTACK_USERNAME || !process.env.BROWSERSTACK_ACCESS_KEY) {
  console.error('Missing BrowserStack credentials. Set BROWSERSTACK_USERNAME and BROWSERSTACK_ACCESS_KEY.');
  process.exit(1);
}

async function runTests() {
  console.log(`\n========================================`);
  console.log(`EdgeVec BrowserStack Test Runner`);
  console.log(`========================================`);
  console.log(`Platform: ${options.platform}`);
  console.log(`Browser: ${options.browser}`);
  console.log(`OS Version: ${options['os-version']}`);
  console.log(`Device: ${options.device}`);
  console.log(`========================================\n`);

  // Build BrowserStack capabilities
  const capabilities = {
    browserName: options.browser,
    'bstack:options': {
      userName: process.env.BROWSERSTACK_USERNAME,
      accessKey: process.env.BROWSERSTACK_ACCESS_KEY,
      realMobile: true,
      osVersion: options['os-version'],
      deviceName: options.device,
      local: true,
      localIdentifier: process.env.GITHUB_RUN_ID || 'local-dev',
      sessionName: `EdgeVec Mobile Test - ${options.platform} ${options.browser} ${options['os-version']}`,
      buildName: `EdgeVec CI - ${process.env.GITHUB_SHA?.slice(0, 7) || 'local'}`,
      debug: true,
      consoleLogs: 'verbose',
      networkLogs: true,
    },
  };

  let driver;

  try {
    console.log('Connecting to BrowserStack...');

    driver = await new Builder()
      .usingServer('https://hub-cloud.browserstack.com/wd/hub')
      .withCapabilities(capabilities)
      .build();

    console.log('Connected. Navigating to test page...');

    // Navigate to the local test page (tunneled via BrowserStack Local)
    await driver.get('http://localhost:8080/tests/mobile/index.html');

    console.log('Waiting for page to load...');

    // Wait for the page to fully initialize
    await driver.wait(until.elementLocated(By.id('btnRun')), 30000);

    console.log('Page loaded. Starting tests...');

    // Click "Run All Tests" button
    const runButton = await driver.findElement(By.id('btnRun'));
    await runButton.click();

    console.log('Tests running...');

    // Wait for tests to complete (look for completion message in log)
    // Max wait: 120 seconds (stress test + benchmark can take time)
    const startTime = Date.now();
    const maxWait = 120000;

    let testComplete = false;
    while (!testComplete && (Date.now() - startTime) < maxWait) {
      try {
        const logContent = await driver.findElement(By.id('logContent')).getText();
        if (logContent.includes('Test suite complete') || logContent.includes('Unexpected Error')) {
          testComplete = true;
        } else {
          await new Promise(r => setTimeout(r, 2000)); // Poll every 2 seconds
        }
      } catch {
        await new Promise(r => setTimeout(r, 2000));
      }
    }

    if (!testComplete) {
      throw new Error('Test timeout: Tests did not complete within 120 seconds');
    }

    console.log('Tests complete. Collecting results...');

    // Collect results
    const resultsList = await driver.findElement(By.id('resultsList'));
    const resultsHtml = await resultsList.getAttribute('innerHTML');
    const logContent = await driver.findElement(By.id('logContent')).getText();

    // Count pass/fail from the results
    const passCount = (resultsHtml.match(/class="test-result pass"/g) || []).length;
    const failCount = (resultsHtml.match(/class="test-result fail"/g) || []).length;
    const totalCount = passCount + failCount;

    // Get stats from the UI
    const statPassed = await driver.findElement(By.id('statPassed')).getText();
    const statFailed = await driver.findElement(By.id('statFailed')).getText();
    const statTime = await driver.findElement(By.id('statTime')).getText();

    // Extract individual test results
    const testResults = await driver.findElements(By.css('.test-result'));
    const detailedResults = [];

    for (const result of testResults) {
      const className = await result.getAttribute('class');
      const isPassed = className.includes('pass');
      const testName = await result.findElement(By.css('.test-name')).getText();

      let testTime = 'N/A';
      try {
        testTime = await result.findElement(By.css('.test-time')).getText();
      } catch {
        // Time element may not exist for running tests
      }

      let errorMsg = null;
      try {
        const errorEl = await result.findElement(By.css('.test-error'));
        errorMsg = await errorEl.getText();
      } catch {
        // No error element
      }

      detailedResults.push({
        name: testName,
        passed: isPassed,
        time: testTime,
        error: errorMsg,
      });
    }

    // Build result object
    const result = {
      platform: options.platform,
      browser: options.browser,
      osVersion: options['os-version'],
      device: options.device,
      passed: parseInt(statPassed) || passCount,
      failed: parseInt(statFailed) || failCount,
      total: totalCount,
      duration: statTime,
      tests: detailedResults,
      log: logContent,
      timestamp: new Date().toISOString(),
      commit: process.env.GITHUB_SHA || 'local',
      runId: process.env.GITHUB_RUN_ID || 'local',
    };

    // Save results to file
    const resultDir = path.join(__dirname, 'results');
    if (!fs.existsSync(resultDir)) {
      fs.mkdirSync(resultDir, { recursive: true });
    }

    const fileName = `${options.platform}_${options.browser}_${options['os-version'].replace('.', '_')}.json`;
    const resultFile = path.join(resultDir, fileName);
    fs.writeFileSync(resultFile, JSON.stringify(result, null, 2));

    console.log(`\n========================================`);
    console.log(`RESULTS: ${result.passed}/${result.total} tests passed`);
    console.log(`Duration: ${result.duration}`);
    console.log(`Results saved to: ${resultFile}`);
    console.log(`========================================\n`);

    // Print detailed results
    for (const test of detailedResults) {
      const icon = test.passed ? '\u2713' : '\u2717';
      console.log(`  ${icon} ${test.name} (${test.time})`);
      if (test.error) {
        console.log(`    Error: ${test.error}`);
      }
    }

    // Exit with error if any tests failed
    if (result.failed > 0) {
      console.error(`\nFAILED: ${result.failed} test(s) failed`);
      process.exit(1);
    }

    console.log('\nAll tests passed!');

  } catch (err) {
    console.error('Test runner error:', err);
    process.exit(1);
  } finally {
    if (driver) {
      await driver.quit();
    }
  }
}

// Run the tests
runTests();
