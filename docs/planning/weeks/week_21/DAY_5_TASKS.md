# Week 21, Day 5: BrowserStack CI & Schema Freeze

**Date:** 2026-01-03
**Sprint:** Week 21 (v0.5.0 Phase)
**Day Theme:** Automated Mobile Testing & Schema Finalization
**Status:** PLANNED

---

## Task W21.5: BrowserStack CI Integration & Metadata Schema Freeze

**Priority:** HIGH (P1)
**Estimated Effort:** 8 hours (3x rule: 2h optimistic × 3 = 6h + 2h buffer)
**Status:** PLANNED
**Depends On:** W21.4 complete (Manual mobile testing done)
**Blocks:** Week 22 (Filtering Architecture)

**⚠️ PREREQUISITE (per HR-2025-12-16-W21-PLAN [M2]):**
Before starting Day 5, verify GitHub Actions secrets exist:
- `BROWSERSTACK_USERNAME` — BrowserStack account username
- `BROWSERSTACK_ACCESS_KEY` — BrowserStack API key
If secrets are not configured, use manual device testing as fallback (see Rollback Plan).

---

### Context

Day 5 automates mobile testing via BrowserStack and formally freezes the metadata schema. This is the final gate before Week 22's Filtering Architecture sprint begins.

**Strategic Importance:**
- BrowserStack automation ensures ongoing mobile compatibility
- Schema freeze is a contractual commitment (breaking changes = major version)
- GATE_W21_COMPLETE.md unlocks Week 22

**Reference Documents:**
- `docs/planning/V0.5.0_STRATEGIC_ROADMAP.md` (binding timeline)
- `docs/testing/MOBILE_TEST_RESULTS.md` (Day 4 results)

---

### Objective

Complete Week 21 deliverables:
1. BrowserStack CI integration for automated mobile testing
2. Metadata schema documentation and FREEZE declaration
3. Week 21 gate completion and handoff to Week 22

---

### Technical Approach

#### 1. BrowserStack CI Integration

**File: `.github/workflows/browserstack.yml`**
```yaml
name: BrowserStack Mobile Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    # Run weekly to catch browser updates
    - cron: '0 6 * * 1'

env:
  BROWSERSTACK_USERNAME: ${{ secrets.BROWSERSTACK_USERNAME }}
  BROWSERSTACK_ACCESS_KEY: ${{ secrets.BROWSERSTACK_ACCESS_KEY }}

jobs:
  mobile-tests:
    runs-on: ubuntu-latest
    timeout-minutes: 30

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: npm ci

      - name: Build WASM
        run: wasm-pack build --target web --release

      - name: Setup BrowserStack Local
        uses: browserstack/github-actions/setup-local@master
        with:
          local-testing: start
          local-identifier: ${{ github.run_id }}

      - name: Run iOS Safari Tests
        run: |
          node tests/browserstack/run-tests.js \
            --platform ios \
            --browser safari \
            --os-version 16 \
            --device "iPhone 14"

      - name: Run Android Chrome Tests
        run: |
          node tests/browserstack/run-tests.js \
            --platform android \
            --browser chrome \
            --os-version 13 \
            --device "Google Pixel 7"

      - name: Run iOS Safari 15 Tests
        run: |
          node tests/browserstack/run-tests.js \
            --platform ios \
            --browser safari \
            --os-version 15 \
            --device "iPhone 13"

      - name: Run Android Chrome (older) Tests
        run: |
          node tests/browserstack/run-tests.js \
            --platform android \
            --browser chrome \
            --os-version 11 \
            --device "Samsung Galaxy S21"

      - name: Upload Test Results
        uses: actions/upload-artifact@v4
        with:
          name: browserstack-results
          path: tests/browserstack/results/

      - name: Stop BrowserStack Local
        uses: browserstack/github-actions/setup-local@master
        with:
          local-testing: stop
```

**File: `tests/browserstack/run-tests.js`**
```javascript
const { Builder, By, until } = require('selenium-webdriver');
const browserstack = require('browserstack-local');
const fs = require('fs');
const path = require('path');

const args = process.argv.slice(2);
const options = {};
for (let i = 0; i < args.length; i += 2) {
  options[args[i].replace('--', '')] = args[i + 1];
}

async function runTests() {
  const capabilities = {
    'bstack:options': {
      os: options.platform === 'ios' ? 'ios' : 'android',
      osVersion: options['os-version'],
      deviceName: options.device,
      browserName: options.browser,
      local: true,
      localIdentifier: process.env.GITHUB_RUN_ID || 'local',
      sessionName: `EdgeVec Mobile Test - ${options.platform} ${options.browser}`,
      buildName: `EdgeVec CI - ${process.env.GITHUB_SHA?.slice(0, 7) || 'local'}`,
    },
  };

  const driver = await new Builder()
    .usingServer('https://hub-cloud.browserstack.com/wd/hub')
    .withCapabilities(capabilities)
    .build();

  try {
    // Navigate to test page (served via BrowserStack Local)
    await driver.get('http://localhost:8080/tests/mobile/index.html');

    // Wait for tests to complete (max 60 seconds)
    await driver.wait(
      async () => {
        const log = await driver.findElement(By.id('log')).getText();
        return log.includes('All tests complete!') || log.includes('Unexpected Error');
      },
      60000
    );

    // Collect results
    const results = await driver.findElement(By.id('results')).getText();
    const log = await driver.findElement(By.id('log')).getText();

    // Parse results
    const passed = (results.match(/✅/g) || []).length;
    const failed = (results.match(/❌/g) || []).length;
    const total = passed + failed;

    // Save results
    const resultDir = path.join(__dirname, 'results');
    if (!fs.existsSync(resultDir)) {
      fs.mkdirSync(resultDir, { recursive: true });
    }

    const resultFile = path.join(
      resultDir,
      `${options.platform}_${options.browser}_${options['os-version']}.json`
    );

    fs.writeFileSync(resultFile, JSON.stringify({
      platform: options.platform,
      browser: options.browser,
      osVersion: options['os-version'],
      device: options.device,
      passed,
      failed,
      total,
      results,
      log,
      timestamp: new Date().toISOString(),
    }, null, 2));

    console.log(`Results: ${passed}/${total} passed`);

    if (failed > 0) {
      console.error(`FAILED: ${failed} tests failed`);
      process.exit(1);
    }

  } finally {
    await driver.quit();
  }
}

runTests().catch((err) => {
  console.error('Test runner error:', err);
  process.exit(1);
});
```

#### 2. Metadata Schema Documentation & Freeze

**File: `docs/schemas/METADATA_SCHEMA_V1.md`**
```markdown
# Metadata Schema v1.0 (FROZEN)

**Version:** 1.0.0
**Status:** FROZEN
**Frozen Date:** 2026-01-03
**Breaking Changes:** Require major version bump (v1.0.0)

---

## Schema Overview

EdgeVec metadata is a key-value store attached to vectors. Each vector
can have up to 64 metadata keys. This schema is FROZEN — any breaking
changes require a major version bump.

## Value Types

| Type | Rust Type | JSON Type | TypeScript Type | Constraints |
|:-----|:----------|:----------|:----------------|:------------|
| String | `String` | `string` | `string` | Max 65,536 bytes |
| Integer | `i64` | `number` | `number` | -2^63 to 2^63-1 |
| Float | `f64` | `number` | `number` | IEEE 754, no NaN/Inf |
| Boolean | `bool` | `boolean` | `boolean` | true/false |
| StringArray | `Vec<String>` | `string[]` | `string[]` | Max 1,024 elements |

## Key Constraints

| Constraint | Value | Rationale |
|:-----------|:------|:----------|
| Max keys per vector | 64 | Memory budget |
| Max key length | 256 bytes | Reasonable limit |
| Key format | `[a-zA-Z0-9_]+` | JSON/WASM compatibility |

## JSON Serialization

Values are serialized with type tags for unambiguous deserialization:

```json
{"type": "string", "value": "hello"}
{"type": "integer", "value": 42}
{"type": "float", "value": 3.14159}
{"type": "boolean", "value": true}
{"type": "string_array", "value": ["a", "b", "c"]}
```

## Persistence Format

Metadata is stored in snapshot v2 format:

```
[4 bytes] Magic: "EVEC"
[4 bytes] Version: 2
[N bytes] Header (bincode)
[N bytes] Vectors (bincode)
[N bytes] Graph (bincode)
[N bytes] Metadata (bincode) <-- NEW in v2
[4 bytes] CRC32 checksum
```

## API Surface (FROZEN)

### Rust API

```rust
impl MetadataStore {
    pub fn new() -> Self;
    pub fn insert(&mut self, vector_id: u32, key: &str, value: MetadataValue) -> Result<()>;
    pub fn get(&self, vector_id: u32, key: &str) -> Option<&MetadataValue>;
    pub fn get_all(&self, vector_id: u32) -> Option<&HashMap<String, MetadataValue>>;
    pub fn update(&mut self, vector_id: u32, key: &str, value: MetadataValue) -> Result<()>;
    pub fn delete(&mut self, vector_id: u32, key: &str) -> Result<bool>;
    pub fn delete_all(&mut self, vector_id: u32) -> Result<bool>;
    pub fn has_key(&self, vector_id: u32, key: &str) -> bool;
    pub fn keys(&self, vector_id: u32) -> Option<impl Iterator<Item = &String>>;
}
```

### WASM/TypeScript API

```typescript
class JsMetadataValue {
  static fromString(value: string): JsMetadataValue;
  static fromInteger(value: number): JsMetadataValue;
  static fromFloat(value: number): JsMetadataValue;
  static fromBoolean(value: boolean): JsMetadataValue;
  static fromStringArray(value: string[]): JsMetadataValue;
  getType(): 'string' | 'integer' | 'float' | 'boolean' | 'string_array';
  asString(): string | undefined;
  asInteger(): number | undefined;
  asFloat(): number | undefined;
  asBoolean(): boolean | undefined;
  asStringArray(): string[] | undefined;
  toJS(): string | number | boolean | string[];
}

interface HnswIndex {
  setMetadata(vectorId: number, key: string, value: JsMetadataValue): void;
  getMetadata(vectorId: number, key: string): JsMetadataValue | undefined;
  getAllMetadata(vectorId: number): Record<string, any> | undefined;
  deleteMetadata(vectorId: number, key: string): boolean;
  deleteAllMetadata(vectorId: number): boolean;
  hasMetadata(vectorId: number, key: string): boolean;
  metadataKeyCount(vectorId: number): number;
}
```

## Compatibility Guarantees

1. **Forward Compatibility:** v0.5.0+ can read v0.4.0 snapshots (empty metadata)
2. **Backward Compatibility:** v0.4.0 cannot read v0.5.0 snapshots (version check fails)
3. **API Stability:** All methods listed above are stable for v0.x lifetime

## Future Extensions (v2.0)

The following are candidates for schema v2.0 (requires major version bump):

- Additional value types (Date, Binary, Object)
- Nested metadata objects
- Type coercion/casting
- Computed/derived metadata

---

## Freeze Declaration

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│   METADATA SCHEMA v1.0 IS NOW FROZEN                                │
│                                                                     │
│   Date: 2026-01-03                                                  │
│   Authority: HOSTILE_REVIEWER                                       │
│                                                                     │
│   Any breaking changes to:                                          │
│   - Value types (adding/removing/modifying)                         │
│   - Key constraints                                                 │
│   - Serialization format                                            │
│   - API method signatures                                           │
│                                                                     │
│   REQUIRE A MAJOR VERSION BUMP (v0.x → v1.0)                        │
│                                                                     │
│   Non-breaking additions (new methods, relaxed constraints)         │
│   are permitted in minor versions.                                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```
```

#### 3. Gate Completion Document

**File: `.claude/GATE_W21_COMPLETE.md`**
```markdown
# GATE_W21_COMPLETE

**Week:** 21
**Date Completed:** 2026-01-03
**Status:** COMPLETE
**Reviewed By:** HOSTILE_REVIEWER

---

## Week 21 Deliverables

### Day 1: Core Types (W21.1)
- [x] `src/metadata/mod.rs` created
- [x] `MetadataValue` enum with 5 types
- [x] Validation constants and functions
- [x] Unit tests pass

### Day 2: Implementation (W21.2)
- [x] `MetadataStore` CRUD operations
- [x] Persistence integration (snapshot v2)
- [x] `HnswIndex` integration
- [x] Unit tests >90% coverage

### Day 3: WASM Bindings (W21.3)
- [x] `JsMetadataValue` class exported
- [x] `HnswIndex` metadata methods
- [x] TypeScript definitions
- [x] Bundle size verified (<500KB)

### Day 4: Mobile Testing (W21.4)
- [x] iOS Safari 15+ tested
- [x] Android Chrome 90+ tested
- [x] Mobile usage guide created
- [x] Limitations documented

### Day 5: CI & Freeze (W21.5)
- [x] BrowserStack CI workflow
- [x] Metadata schema documented
- [x] Schema FROZEN
- [x] GATE_W21_COMPLETE.md created

---

## Success Criteria Verification

| Criterion | Status | Evidence |
|:----------|:-------|:---------|
| MetadataStore API complete | ✅ PASS | All CRUD operations working |
| 5 value types supported | ✅ PASS | String, Integer, Float, Boolean, StringArray |
| Metadata persisted | ✅ PASS | Snapshot v2 format |
| WASM bindings exported | ✅ PASS | All methods in edgevec.d.ts |
| Bundle size <500KB | ✅ PASS | [actual size] bytes gzipped |
| iOS Safari tested | ✅ PASS | 12/12 tests pass |
| Android Chrome tested | ✅ PASS | 12/12 tests pass |
| BrowserStack CI operational | ✅ PASS | Workflow runs on PR/push |
| Unit test coverage >90% | ✅ PASS | [actual]% coverage |
| Schema FROZEN | ✅ PASS | METADATA_SCHEMA_V1.md |
| HOSTILE_REVIEWER approved | ✅ PASS | This document |

---

## Schema Freeze Acknowledgment

The Metadata Schema v1.0 is now FROZEN.

**Implications:**
- Week 22 Filtering Architecture will build on this frozen schema
- No breaking changes without major version bump
- Filtering queries will use metadata types as defined

---

## Handoff to Week 22

**Prerequisite Met:** GATE_W21_COMPLETE.md exists

**Week 22 Focus:** Filtering Architecture (Design Sprint)

**Required Reading:**
- `docs/schemas/METADATA_SCHEMA_V1.md`
- `docs/planning/V0.5.0_STRATEGIC_ROADMAP.md`

**Week 22 Deliverable:**
- `docs/architecture/FILTERING_API.md`
- Query syntax specification (EBNF)
- NO implementation code

---

## Approval

```
┌─────────────────────────────────────────────────────────────────────┐
│   HOSTILE_REVIEWER: WEEK 21 APPROVED                                │
│                                                                     │
│   Date: 2026-01-03                                                  │
│   Verdict: GO                                                       │
│                                                                     │
│   All deliverables complete.                                        │
│   All tests pass.                                                   │
│   Schema frozen.                                                    │
│   Week 22 unblocked.                                                │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

**GATE_W21_COMPLETE.md**
**Status:** APPROVED
**Next:** Week 22 - Filtering Architecture
```

---

### Acceptance Criteria

**CRITICAL (Must Pass):**
- [ ] BrowserStack workflow created (`.github/workflows/browserstack.yml`)
- [ ] BrowserStack tests execute successfully on PR
- [ ] iOS Safari tests pass in CI
- [ ] Android Chrome tests pass in CI
- [ ] Metadata schema documented (`METADATA_SCHEMA_V1.md`)
- [ ] Schema marked as FROZEN
- [ ] GATE_W21_COMPLETE.md created

**MAJOR (Should Pass):**
- [ ] Test results uploaded as artifacts
- [ ] Multiple browser versions tested
- [ ] Schema freeze declaration visible in docs
- [ ] Week 21 retrospective written

**MINOR (Nice to Have):**
- [ ] BrowserStack badge in README
- [ ] Slack/Discord notification on CI failure
- [ ] Performance regression detection

---

### Implementation Checklist

- [ ] Create `.github/workflows/browserstack.yml`
- [ ] Create `tests/browserstack/run-tests.js`
- [ ] Configure BrowserStack secrets in GitHub
- [ ] Run CI workflow manually to verify
- [ ] Create `docs/schemas/METADATA_SCHEMA_V1.md`
- [ ] Add FROZEN declaration to schema doc
- [ ] Create `.claude/GATE_W21_COMPLETE.md`
- [ ] Update README with mobile support badge
- [ ] Write Week 21 retrospective

---

### Dependencies

**Blocks:**
- Week 22 (Filtering Architecture requires GATE_W21_COMPLETE.md)

**Blocked By:**
- W21.4 complete (Manual tests must pass first)

**External Dependencies:**
- BrowserStack account with API access
- GitHub Actions secrets configured

---

### Verification Method

**Day 5 is COMPLETE when:**

1. BrowserStack CI runs successfully:
   ```
   GitHub Actions → browserstack.yml → All jobs pass
   ```

2. Schema freeze is documented:
   ```
   docs/schemas/METADATA_SCHEMA_V1.md contains FROZEN declaration
   ```

3. Gate file exists:
   ```
   .claude/GATE_W21_COMPLETE.md with HOSTILE_REVIEWER approval
   ```

4. All Week 21 checklist items complete

---

### Rollback Plan

If Day 5 encounters blocking issues:

1. **BrowserStack fails:** Manual testing sufficient (Day 4 results)
2. **CI configuration issues:** Defer BrowserStack to Week 22
3. **Schema questions:** Document concerns, do not freeze until resolved
4. **Gate approval delayed:** Continue to Week 22 prep, get approval async

---

### Estimated Timeline

| Phase | Time | Cumulative |
|:------|:-----|:-----------|
| BrowserStack workflow | 2h | 2h |
| Test runner script | 1.5h | 3.5h |
| CI verification | 1h | 4.5h |
| Schema documentation | 1.5h | 6h |
| Gate completion | 1h | 7h |
| Buffer | 1h | 8h |

---

### Hostile Review Checkpoint

**End of Day 5:** Submit for `/review` with:
- `.github/workflows/browserstack.yml`
- `tests/browserstack/run-tests.js`
- `docs/schemas/METADATA_SCHEMA_V1.md`
- `.claude/GATE_W21_COMPLETE.md`

**Expected Review Focus:**
- CI workflow correctness
- Schema completeness and freeze validity
- Gate checklist accuracy
- Week 22 readiness

---

### Week 21 Completion Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│   WEEK 21: METADATA API + MOBILE TESTING — COMPLETE                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   Deliverables:                                                     │
│   ├── src/metadata/           (Core types + CRUD)                   │
│   ├── src/wasm/metadata.rs    (WASM bindings)                       │
│   ├── tests/mobile/           (Mobile test page)                    │
│   ├── tests/browserstack/     (CI automation)                       │
│   ├── docs/schemas/           (Frozen schema)                       │
│   └── docs/guides/            (Mobile usage)                        │
│                                                                     │
│   Tests:                                                            │
│   ├── Unit tests: >90% coverage                                     │
│   ├── Property tests: 1000+ inputs                                  │
│   ├── WASM tests: Node.js                                           │
│   ├── Mobile tests: iOS Safari, Android Chrome                      │
│   └── CI tests: BrowserStack automated                              │
│                                                                     │
│   Status: GATE_W21_COMPLETE.md created                              │
│   Schema: FROZEN                                                    │
│   Next: Week 22 — Filtering Architecture                            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

**Task Owner:** TEST_ENGINEER
**Review Required:** HOSTILE_REVIEWER
**Next Week:** Week 22 (Filtering Architecture Design Sprint)

---

*"Automate what can be automated. Freeze what must be frozen. Ship with confidence."*
