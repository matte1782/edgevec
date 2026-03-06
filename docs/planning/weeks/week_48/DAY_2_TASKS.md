# Week 48 — Day 2 Tasks (Tuesday, Apr 15)

**Date:** 2026-04-15
**Focus:** WASM Boost Export + Module Re-exports + MID-WEEK HOSTILE REVIEW
**Agents:** WASM_SPECIALIST, RUST_ENGINEER, HOSTILE_REVIEWER
**Status:** PENDING

---

## Day Objective

Export MetadataBoost search to WASM, wire up module re-exports, and pass mid-week hostile review.

**Success Criteria:**
- `search_with_boost()` WASM export compiles for `wasm32-unknown-unknown`
- `MetadataBoost` re-exported from `edgevec::filter::MetadataBoost`
- Mid-week hostile review: GO verdict (0 Critical, 0 Major)
- All regression tests pass

---

## Pre-Task Context Loading

Read these files before starting:
- [ ] `src/filter/boost.rs` — MetadataBoost struct from Day 1
- [ ] `src/filter/filtered_search.rs` — search_boosted() method from Day 1
- [ ] `src/wasm/mod.rs` — existing WASM export patterns (handle/opaque-pointer)
- [ ] `src/filter/mod.rs` — current re-exports (need to add boost)

---

## Tasks

### W48.2a: Add `search_with_boost()` WASM Export (3h) — WASM_SPECIALIST

**Dependency:** W48.1a, W48.1b complete (Day 1)

**Implementation:**
Add to `src/wasm/mod.rs` (or appropriate WASM submodule):

```rust
/// Search with metadata boosting for entity-enhanced RAG.
///
/// # Arguments
/// * `query` - Query vector (f32 array)
/// * `k` - Number of results
/// * `boosts_json` - JSON array of boost configs:
///   `[{"field": "entity_type", "value": "ORG", "weight": 0.3}]`
/// * `filter_str` - Optional filter expression string
#[wasm_bindgen(js_name = "searchWithBoost")]
pub fn search_with_boost(
    /* existing handle pattern */
    query: &[f32],
    k: u32,
    boosts_json: &str,
    filter_str: Option<String>,
) -> Result<JsValue, JsValue> { ... }
```

**JSON parsing for boosts:**
- Parse `boosts_json` as `Vec<BoostConfig>` where `BoostConfig { field: String, value: serde_json::Value, weight: f32 }`
- **IMPORTANT:** `BoostConfig` is a SEPARATE struct with its own serde deserialization. It does NOT reuse MetadataValue's adjacently-tagged format (`{"type":"string","value":"ORG"}`). The WASM JSON API accepts bare JSON values:
  ```json
  [{"field": "entity_type", "value": "ORG", "weight": 0.3}]
  ```
  Where `"value"` is a bare JSON value that gets manually converted to MetadataValue internally.
- Convert `serde_json::Value` to `MetadataValue`:
  - `Value::String(s)` -> `MetadataValue::String(s)`
  - `Value::Number(n)` -> `MetadataValue::Integer(n.as_i64())` or `MetadataValue::Float(n.as_f64())`
  - `Value::Bool(b)` -> `MetadataValue::Boolean(b)`
- Return error via `JsValue` for invalid JSON (no `unwrap()`)

**Commands:**
```bash
cargo build
cargo check --target wasm32-unknown-unknown
cargo clippy -- -D warnings
```

**Acceptance:**
- [ ] `search_with_boost()` function with `#[wasm_bindgen]` attribute
- [ ] Parses boosts from JSON string using `BoostConfig` struct (separate from `MetadataValue` serde)
- [ ] BoostConfig JSON format uses bare values, NOT adjacently-tagged MetadataValue format
- [ ] No `unwrap()` — all errors go through `map_err(|e| JsValue::from_str(...))`
- [ ] `cargo check --target wasm32-unknown-unknown` succeeds
- [ ] Follows existing handle/export pattern from `EdgeVecIndex`

---

### W48.2b: Add boost module re-export to `src/filter/mod.rs` (0.25h) — RUST_ENGINEER

**Dependency:** W48.1a complete (Day 1)

**Changes:**
```rust
// In src/filter/mod.rs — add:
pub mod boost;

// In re-exports section — add:
pub use boost::MetadataBoost;
```

**Commands:**
```bash
cargo build
```

**Acceptance:**
- [ ] `pub mod boost;` in `src/filter/mod.rs`
- [ ] `pub use boost::MetadataBoost;` in re-exports
- [ ] `cargo build` succeeds

---

### W48.2c: Update `src/lib.rs` with MetadataBoost accessibility (0.25h) — RUST_ENGINEER

**Dependency:** W48.2b complete

**Verify:** `MetadataBoost` is accessible as `edgevec::filter::MetadataBoost`. If `src/lib.rs` already re-exports the filter module, no changes needed. If not, add appropriate re-export.

**Commands:**
```bash
cargo build
cargo doc --no-deps    # Verify MetadataBoost appears in docs
```

**Acceptance:**
- [ ] `edgevec::filter::MetadataBoost` resolves
- [ ] `cargo build` succeeds

---

### W48.2d: MID-WEEK HOSTILE REVIEW (1.5h) — HOSTILE_REVIEWER

**Dependency:** W48.2a, W48.2b, W48.2c complete

**Scope:** `src/filter/boost.rs` + `search_boosted()` in `filtered_search.rs` + WASM export

**Attack vectors:**
1. **Size guard:** `boost.rs` < 200 lines (excluding test block)
2. **No panics:** Zero `unwrap()` in library code. All paths return Result or 0.0.
3. **Formula correctness:** MULTIPLICATIVE boosting: `final_distance = raw_distance * (1.0 - boost_factor)` where `boost_factor = sum(boosts).clamp(-1.0, 0.99)`. Scale-independent.
4. **NaN handling:** NaN/Inf weight rejected at construction (`MetadataBoost::new()` returns `Result<Self, BoostError>`). Defense-in-depth: `compute_boost()` also guards against NaN.
5. **Type mismatch:** `MetadataValue::String("ORG")` boost on `MetadataValue::Integer(42)` field returns 0.0 silently. StringArray fields match via `contains(value)`.
6. **WASM pattern:** Export follows existing handle pattern. JSON parsing has error handling. BoostConfig uses bare JSON values, NOT MetadataValue's adjacently-tagged format.
7. **Test coverage:** All 11 named tests present and passing (including `test_boost_string_array_contains`).
8. **Regression:** `cargo test --lib` passes 1024+ tests.

**Verdict options:**
- **GO:** 0 Critical, 0 Major. Day 3 proceeds.
- **CONDITIONAL GO:** 0 Critical, <= 2 Major. Fix before Day 3 demo data prep can use boost API.
- **NO-GO:** >= 1 Critical. Fix all criticals, resubmit.

**Acceptance:**
- [ ] Written review document at `docs/reviews/2026-04-15_W48_MIDWEEK_REVIEW.md`
- [ ] Verdict: GO or CONDITIONAL GO
- [ ] All Critical + Major findings listed with specific file:line references

---

## Day 2 Totals

| Metric | Target |
|:-------|:-------|
| Hours | ~5h |
| New WASM exports | 1 (search_with_boost) |
| Module changes | 2 (filter/mod.rs, lib.rs) |
| Hostile review | 1 (mid-week) |
| Regressions allowed | 0 |

---

## Time Tracking

| Task | Estimated | Actual | Notes |
|:-----|:----------|:-------|:------|
| W48.2a | 3h | | |
| W48.2b | 0.25h | | |
| W48.2c | 0.25h | | |
| W48.2d | 1.5h | | |
| **Total** | **5h** | | |

---

## Handoff to Day 3

**Codebase state at EOD:**
- MetadataBoost API complete (struct + search_boosted + WASM export)
- Mid-week hostile review complete with GO/CONDITIONAL GO verdict
- All tests passing, WASM builds

**Day 3 prerequisites satisfied:**
- [ ] MetadataBoost API hostile-reviewed (needed for confidence in demo data alignment)
- [ ] WASM export compiles (needed for Day 4 demo)

**Day 3 focus:** Python data prep (SQuAD + embeddings + NER metadata)

**HALT CONDITION:** If hostile review is NO-GO, fix findings immediately. Day 3 Track B (Python data prep) can start regardless — it is independent Python code that doesn't call Rust.

---

**END OF DAY 2 TASKS**
