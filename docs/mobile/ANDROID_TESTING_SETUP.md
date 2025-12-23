# Android Testing Setup

**Document:** W25.4.2 — Android Testing Environment Setup
**Author:** WASM_SPECIALIST
**Date:** 2025-12-20
**Status:** [PROPOSED]

---

## Testing Strategy Overview

Given the constraints (no physical Android device available), we use a **tiered testing approach**:

| Priority | Method | What It Tests | Reliability |
|:---------|:-------|:--------------|:------------|
| 1 | Remote Friend Testing | Real device behavior | ⭐⭐⭐⭐⭐ |
| 2 | Chrome DevTools Device Mode | Layout, touch simulation | ⭐⭐⭐ |
| 3 | BrowserStack Free Tier | Real device, limited | ⭐⭐⭐⭐ |
| 4 | Android Emulator | Full WASM testing | ⭐⭐⭐⭐ |

---

## Method 1: Remote Friend Testing (Primary)

### Setup Requirements

1. **Deploy Demos to Public URL:**
   ```bash
   # Option A: Use local server with ngrok
   cd wasm/examples
   python -m http.server 8080
   # In another terminal:
   ngrok http 8080
   # Gives you: https://xxxx.ngrok.io

   # Option B: Deploy to GitHub Pages
   # Push to gh-pages branch
   # Access: https://matte1782.github.io/edgevec/wasm/examples/
   ```

2. **Share Test Checklist with Friend:**

### Test Checklist for Remote Tester

```markdown
## EdgeVec Android Testing Checklist

**Device Info (please fill):**
- Phone model: ________________
- Android version: ____________
- Chrome version: _____________

---

### Test 1: Filter Playground
URL: [provided URL]/filter-playground.html

- [ ] Page loads without errors
- [ ] "WASM module loaded" message appears
- [ ] Type: category = "electronics"
- [ ] Click "Parse" button
- [ ] AST output shows result
- [ ] Screenshot: [attach]

### Test 2: Benchmark Dashboard
URL: [provided URL]/benchmark-dashboard.html

- [ ] Page loads without errors
- [ ] "Run Benchmark" button appears
- [ ] Click "Run Benchmark"
- [ ] Results show (not 0ms, not NaN)
- [ ] Timing values are reasonable (> 0.01ms)
- [ ] Screenshot: [attach]

### Test 3: Touch Interactions
- [ ] Can tap all buttons
- [ ] Text input works
- [ ] Can scroll page smoothly
- [ ] No horizontal scroll issues
- [ ] Examples grid is tappable

### Test 4: Index Demo (if available)
URL: [provided URL]/index.html
- [ ] All demo cards visible
- [ ] Links are clickable
- [ ] Cards navigate correctly

---

**Overall Result:**
- [ ] PASS: All features work
- [ ] PARTIAL: Some issues (describe below)
- [ ] FAIL: Major features broken

**Issues Found:**
_________________
_________________
```

### Collecting Results

1. Ask friend to send:
   - Completed checklist
   - Screenshots of each page
   - Screen recording if issues found
   - Device/browser details

2. Document results in `docs/mobile/ANDROID_TEST_RESULTS.md`

---

## Method 2: Chrome DevTools Device Mode

### Setup

1. Open Chrome on desktop
2. Press `F12` to open DevTools
3. Click "Toggle Device Toolbar" (Ctrl+Shift+M)
4. Select device: "Pixel 7" or "Samsung Galaxy S20"

### What It Tests

| Aspect | Tested | Not Tested |
|:-------|:-------|:-----------|
| Layout/CSS | ✅ | |
| Touch simulation | ✅ | |
| Viewport sizes | ✅ | |
| **WASM execution** | | ❌ Desktop V8 |
| **Real touch latency** | | ❌ Simulated |
| **Memory limits** | | ❌ Desktop memory |
| **Performance** | | ❌ Desktop CPU |

### How to Use

```bash
# Start local server
cd "C:\Users\matte\Desktop\Desktop OLD\AI\Università AI\courses\personal_project\fortress_problem_driven\research_fortress\edgevec"
python -m http.server 8080

# Open in Chrome
# http://localhost:8080/wasm/examples/filter-playground.html
```

Then in DevTools:
1. Toggle device mode
2. Select "Pixel 7" or similar
3. Reload page
4. Test touch interactions by clicking

### Limitations

This method is **NOT reliable for:**
- WASM compatibility testing (uses desktop V8)
- Memory limit testing (uses desktop memory)
- Real performance testing (uses desktop CPU)
- Actual touch behavior (simulated only)

---

## Method 3: BrowserStack Free Tier

### Account Setup

1. Go to [browserstack.com](https://www.browserstack.com/)
2. Sign up for free trial (100 minutes)
3. Access Live Testing

### Testing Steps

1. Select "Live" testing
2. Choose Android device (e.g., Samsung Galaxy S21, Chrome)
3. Enter URL: Your demo URL (must be public)
4. Interact with the remote device

### Limitations

- Free tier: 100 minutes total
- Slow network may affect load times
- Session timeout after inactivity

---

## Method 4: Android Emulator

### Prerequisites

1. **Install Android Studio** (if not installed):
   - Download from [developer.android.com/studio](https://developer.android.com/studio)
   - Install (requires ~2GB disk space)

2. **Create Virtual Device (AVD):**
   - Open Android Studio
   - Tools → Device Manager
   - Create Device → Pixel 7 → Android 14 (API 34)
   - Download system image if needed

3. **Start Emulator:**
   ```bash
   # From command line
   emulator -avd Pixel_7_API_34
   ```

### Testing in Emulator

1. Open Chrome in emulator
2. Navigate to `http://10.0.2.2:8080/wasm/examples/filter-playground.html`
   - Note: `10.0.2.2` is the host machine's localhost
3. Test all demos

### Emulator Limitations

| Aspect | Accuracy |
|:-------|:---------|
| WASM execution | ⭐⭐⭐⭐ Good (x86 translation) |
| Touch behavior | ⭐⭐⭐ Simulated |
| Memory limits | ⭐⭐⭐ Configurable |
| Performance | ⭐⭐ Slower than real |
| GPU/rendering | ⭐⭐ Software rendering |

---

## Quick Start: Local Testing

### 1. Start Server

```bash
cd "C:\Users\matte\Desktop\Desktop OLD\AI\Università AI\courses\personal_project\fortress_problem_driven\research_fortress\edgevec"
python -m http.server 8080
```

### 2. Test URLs

| Demo | URL |
|:-----|:----|
| Index | http://localhost:8080/wasm/examples/index.html |
| Filter Playground | http://localhost:8080/wasm/examples/filter-playground.html |
| Benchmark Dashboard | http://localhost:8080/wasm/examples/benchmark-dashboard.html |
| Soft Delete | http://localhost:8080/wasm/examples/soft_delete.html |

### 3. DevTools Device Mode Testing

1. Open any URL above in Chrome
2. F12 → Toggle Device Mode (Ctrl+Shift+M)
3. Select "Pixel 7" or "Samsung Galaxy S21"
4. Reload and test

---

## Test Matrix Template

```markdown
| Test Case | DevTools | Emulator | Real Device | Notes |
|:----------|:---------|:---------|:------------|:------|
| Page loads | | | | |
| WASM init | | | | |
| Filter parse | | | | |
| Benchmark run | | | | |
| Touch buttons | | | | |
| Text input | | | | |
| Scroll | | | | |
| No horiz scroll | | | | |
```

---

## Environment Verification

### Verify WASM Works

Open browser console on the test page and run:

```javascript
// Check WASM loaded
console.log('WASM available:', typeof WebAssembly !== 'undefined');

// Check EdgeVec exports (if module loaded)
// Should see: parse_filter_js, validate_filter_js, EdgeVec, etc.
```

### Verify IndexedDB Works

```javascript
// Test IndexedDB
const request = indexedDB.open('test');
request.onsuccess = () => console.log('IndexedDB: OK');
request.onerror = () => console.log('IndexedDB: FAILED');
```

### Check Storage Quota

```javascript
if (navigator.storage?.estimate) {
  const { quota, usage } = await navigator.storage.estimate();
  console.log(`Quota: ${(quota / 1024 / 1024).toFixed(1)} MB`);
  console.log(`Used: ${(usage / 1024 / 1024).toFixed(1)} MB`);
}
```

---

## Troubleshooting

### "WASM module not found"

```bash
# Ensure pkg/ directory has compiled WASM
wasm-pack build --target web

# Check file exists
ls pkg/edgevec.js
ls pkg/edgevec_bg.wasm
```

### "CORS error" when loading WASM

- Must serve via HTTP server, not `file://` protocol
- Use `python -m http.server 8080`

### "Out of memory" in emulator

- Increase emulator RAM in AVD settings
- Reduce vector count in test

### Slow performance in emulator

- Enable hardware acceleration (HAXM on Intel)
- Use x86_64 system image, not ARM

---

## Current Setup Status

| Method | Status | Notes |
|:-------|:-------|:------|
| Remote Friend | ⏳ Pending contact | Need demo URLs deployed |
| Chrome DevTools | ✅ Ready | Use for layout testing |
| BrowserStack | ⏳ Account needed | Free tier available |
| Android Emulator | ⏳ Needs install | Optional, heavy setup |

---

**Document Status:** [PROPOSED]
**Next:** W25.4.3 Android Chrome Manual Testing
