# iOS Safari Testing Setup

**Version:** EdgeVec v0.5.3
**Date:** 2025-12-19
**Agent:** WASM_SPECIALIST
**Task:** W25.3.2

---

## Testing Options Overview

| Option | Platform | Cost | Accuracy | Recommended |
|:-------|:---------|:-----|:---------|:------------|
| **Real iOS Device** | Any | Device cost | Highest | ✅ Best |
| **BrowserStack** | Cross-platform | $39+/mo (30 min free) | High | ✅ For CI |
| **LambdaTest** | Cross-platform | Free tier available | High | ✅ Alternative |
| **Xcode Simulator** | macOS only | Free | Medium | ⚠️ macOS required |
| **Safari on Windows** | Windows | N/A | N/A | ❌ Not available |

---

## Option 1: Real iOS Device (Recommended)

### Requirements

- iPhone or iPad with iOS 14+ (recommended 17+)
- Local development server accessible on same network

### Setup Steps

1. **Start local server:**
   ```bash
   cd edgevec
   python -m http.server 8080
   ```

2. **Find your local IP:**
   ```bash
   # Windows
   ipconfig | findstr "IPv4"

   # macOS/Linux
   ifconfig | grep "inet "
   ```

3. **Access on iOS:**
   - Open Safari on iOS device
   - Navigate to `http://<your-ip>:8080/wasm/examples/index.html`

### Test URLs

| Demo | URL Path |
|:-----|:---------|
| Demo Catalog | `/wasm/examples/index.html` |
| Filter Playground | `/wasm/examples/filter-playground.html` |
| Benchmark Dashboard | `/wasm/examples/benchmark-dashboard.html` |
| Soft Delete | `/wasm/examples/soft_delete.html` |
| Batch Insert | `/wasm/examples/batch_insert.html` |

### Debugging

- Enable **Web Inspector** in iOS Settings > Safari > Advanced
- Connect device to Mac via USB
- Open Safari on Mac > Develop > [Device Name] > [Page]

---

## Option 2: BrowserStack (Cloud Testing)

### Features

- Real iOS devices (not simulators)
- Access to iOS 10-17+, Safari 4-18
- DevTools integration
- Screenshot and video recording

### Free Trial

- 30 minutes of live testing
- 100 screenshots
- No credit card required

### Setup

1. Sign up at [browserstack.com/users/sign_up](https://www.browserstack.com/users/sign_up)
2. Navigate to Live > iOS Safari
3. Select device (iPhone 15, iOS 17 recommended)
4. Enter EdgeVec demo URL

### For Open Source Projects

BrowserStack offers **free unlimited access** for open source projects:
- Apply at [browserstack.com/open-source](https://www.browserstack.com/open-source)
- Requires public GitHub repository
- EdgeVec may qualify once public

**Sources:**
- [BrowserStack Pricing](https://www.browserstack.com/pricing)
- [BrowserStack Free Trial](https://www.browserstack.com/support/faq/plans-pricing/plans/what-do-i-get-with-a-free-trial)
- [BrowserStack Safari Testing](https://www.browserstack.com/test-on-safari-browser)

---

## Option 3: LambdaTest (Alternative Cloud)

### Features

- Real iOS devices and browsers
- WebAssembly compatibility testing
- 10,000+ real browsers and devices
- Playwright integration for automation

### Free Tier

- 60 minutes of live testing per month
- Access to real devices
- Basic debugging tools

### Setup

1. Sign up at [lambdatest.com](https://www.lambdatest.com/)
2. Navigate to Real Time Testing > Real Device
3. Select iOS Safari configuration
4. Enter EdgeVec demo URL

### WebAssembly Compatibility Checks

LambdaTest provides specific WebAssembly compatibility pages:
- [WASM Safari Compatibility](https://www.lambdatest.com/web-technologies/wasm-safari)
- [WASM SIMD Safari Compatibility](https://www.lambdatest.com/web-technologies/wasm-simd-safari)

**Sources:**
- [LambdaTest Safari Browser Testing](https://www.lambdatest.com/test-on-safari-browsers)
- [LambdaTest Playwright on iOS](https://www.lambdatest.com/blog/playwright-testing-on-ios-real-devices/)

---

## Option 4: Xcode Simulator (macOS Only)

### Requirements

- macOS 12+ (Monterey or later)
- Xcode 14+ installed
- ~10 GB disk space for iOS simulators

### Setup

1. Install Xcode from App Store
2. Open Xcode > Preferences > Components
3. Download iOS 17 Simulator runtime
4. Open Simulator.app
5. File > Open Simulator > iOS 17 > iPhone 15

### Limitations

| Aspect | Simulator | Real Device |
|:-------|:----------|:------------|
| WASM Performance | Slower | Native |
| Memory Limits | Host memory | iOS limits |
| Touch | Mouse emulated | Native |
| IndexedDB | Works | Works |
| DevTools | Safari Develop menu | Safari Develop menu |

**Note:** Simulator does NOT accurately reproduce iOS Safari memory limits.

---

## Chosen Approach for W25.3

Given this is a Windows development environment:

### Primary Testing: LambdaTest Free Tier

**Why:**
1. 60 minutes free per month (sufficient for manual testing)
2. Real iOS devices (not simulators)
3. Cross-platform (works on Windows)
4. WebAssembly compatibility pre-verified

### Secondary: Real iOS Device (if available)

For ongoing development, a real iOS device provides:
- Accurate memory behavior
- Touch interaction testing
- Offline persistence testing
- Performance profiling

---

## Test Environment Configuration

### EdgeVec Demo Access

**Option A: GitHub Pages (when deployed)**
```
https://matte1782.github.io/edgevec/wasm/examples/
```

**Option B: Local Server (for development)**
```bash
cd edgevec
python -m http.server 8080
# Access via ngrok for external access
npx ngrok http 8080
```

**Option C: Temporary Hosting**
```bash
# Deploy to Vercel (free)
npx vercel --prod

# Deploy to Netlify (free)
npx netlify deploy --prod
```

### Recommended Test Configurations

| Test Case | Device | iOS Version |
|:----------|:-------|:------------|
| Baseline | iPhone 15 Pro | iOS 17.4 |
| Older iOS | iPhone 12 | iOS 16.x |
| iPad | iPad Pro | iOS 17.x |
| Low Memory | iPhone SE | iOS 17.x |

---

## Testing Checklist

### Setup Complete

- [ ] Testing platform account created (BrowserStack or LambdaTest)
- [ ] EdgeVec demos accessible via URL
- [ ] Device configurations documented

### Ready to Test

- [ ] Filter Playground URL accessible
- [ ] Benchmark Dashboard URL accessible
- [ ] Soft Delete demo URL accessible
- [ ] Console access available for error logging

---

## Next Steps

1. **W25.3.3:** Execute manual testing on iOS Safari
2. Document test results in `docs/mobile/IOS_TEST_RESULTS.md`
3. File any bugs found as GitHub issues

---

**Agent:** WASM_SPECIALIST
**Status:** W25.3.2 COMPLETE
**Next:** W25.3.3 (iOS Safari Manual Testing)
