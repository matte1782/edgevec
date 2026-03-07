# Week 49: Launch & Polish — Demo Hub v0.9.0 + Distribution + v1.0 Security Audit Kickoff

**Status:** [APPROVED] — R1 REJECT (1C/2M/5m) -> all fixed. R2 GO (0C/0M/1m accepted-as-is)
**Sprint Goal:** Deliver a world-class Demo Hub at v0.9.0, complete the W48 distribution pipeline, and kick off v1.0 security audit (Phase 11, M11.1)
**Dates:** 2026-04-21 to 2026-04-26 (5 core days + 1 overflow)
**Prerequisites:** W48 [COMPLETE], GATE_W48_COMPLETE.md exists, all 4 carry-forward items identified
**Milestone:** Phase 11 M11.1 Security Audit (first half of W49-W50). NOTE: ROADMAP v7.4 says M11.1 is "Week 48-49" — this is STALE. Day 6 updates ROADMAP to "Week 49-50".

---

## Strategic Context

W48 completed the Growth Pivot: MetadataBoost API, in-browser entity-RAG demo, and positioning blog post. Four carry-forward items remain: blog publish, GitHub Pages deploy, social push, and npm pre-publish prep. These are the distribution tail of the growth effort.

The Demo Hub (`docs/demo/hub.html`) is frozen at v0.7.0 — three feature releases behind (W46 PQ, W47 PQ Validation, W48 MetadataBoost). This is EdgeVec's public face and the first thing a developer sees. It must reflect v0.9.0 with the Entity-RAG demo as the new featured card.

Simultaneously, Phase 11 (v1.0 Production Release) begins with M11.1: Security Audit. This week delivers the first half (unsafe inventory, dependency audit, fuzzing campaign plan, Miri regression). No modifications to `src/` — read-only analysis only.

### Key References (READ BEFORE ANY TASK)

| Document | Path | Purpose |
|:---------|:-----|:--------|
| Demo Hub (current) | `docs/demo/hub.html` | Current state: v0.7.0, stale. Line 5 has `user-scalable=no` (WCAG violation) |
| Entity-RAG demo | `docs/demo/entity-rag/index.html` | Design reference: cyberpunk aesthetic, card styling, animation patterns |
| W49 Mandatory Add-ons | `docs/planning/weeks/week_49/W49_MANDATORY_ADDONS.md` | P0 Demo Hub spec |
| W49 Planner Prompt | `docs/planning/weeks/week_49/W49_PLANNER_PROMPT_DRAFT.md` | APPROVED prompt with all constraints |
| GATE_W48_COMPLETE.md | `.claude/GATE_W48_COMPLETE.md` | 4 carry-forward items (ALL must appear) |
| ROADMAP v7.4 | `docs/planning/ROADMAP.md` | Phase 11 context, M11.1 stale week range |
| Blog post | `docs/blog/entity-enhanced-rag.md` | Source for dev.to publish (Track B) |
| W48 Plan (format reference) | `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` | Format gold standard |

### W47/W48 Items NOT Re-Opened

- PQ core implementation, benchmarks, GO/NO-GO decision (W46-W47)
- MetadataBoost API, entity-RAG demo, blog post (W48 — all delivered)
- All items accepted-as-is from GATE_W48_COMPLETE.md

---

## Estimation Notes

**Optimistic total:** 20h
**3x ceiling:** 60h (disaster scenario boundary — not a target)
**Planned:** ~33.4h across 6 days (~5.6h/day)

**3x Rule Exceptions (documented):**
1. **Track A — Demo Hub (1.3x):** HTML/CSS editing in a single file with an established design system. All card patterns, badge styles, and animations already exist — this is pattern-matching, not greenfield.
2. **Track B — Distribution (1.5x):** Blog publish is copy-paste from existing hostile-reviewed source. Social/Reddit drafts are constrained content writing. GitHub Pages deploy may hit unexpected issues — 1.5x applied across all Track B tasks.
3. **Track C — Security Audit (1.5x):** Analysis/documentation tasks, not implementation. The codebase is well-known. Miri regression is a re-run of an established process (W42 baseline: 392+ pass, 0 UB). 1.5x is appropriate for potential nightly toolchain issues or unexpected findings.

**Buffer strategy:** Day 6 is overflow. Per-task estimates include ~30% padding. Total planned is within ~35h budget.

---

## Critical Path

```
Track A (Demo Hub — Days 1-2):
  W49.1a (Entity-RAG featured card + AI/RAG category)
  W49.1b (Filter Playground demotion + badge cleanup)
  W49.1c (Version bump all v0.9.0 + WCAG fix)
  W49.1d (Hero stats update + optional PQ stat)
  W49.1e (Mobile responsive testing + fixes)
         |
  W49.2a (Final polish + orphan CSS cleanup)
  W49.2b (Semantic HTML audit + focus indicators)
         |
  HOSTILE REVIEW #1 (Mid-Week, Day 2)
  [Scope: docs/demo/hub.html updated]
  [0C, 0M required for GO]
                                          |
Track B (Distribution — Days 3-4):       |
  W49.3a (dev.to blog publish)           |
  W49.3b (GitHub Pages deploy)───────────┤
         |                               |
  W49.4a (Social thread draft)           |
  W49.4b (Reddit drafts)                 |
  W49.4c (npm pre-publish prep)          |
  W49.4e (unsafe inventory — Track C     |
          early start, parallel)         |
                                          |
Track C (Security Audit — Days 4-5):     |
  W49.4e (unsafe code inventory) Day 4   |
  W49.5a (dependency audit) Day 5        |
  W49.5b (fuzzing campaign plan) Day 5   |
  W49.5c (Miri regression) Day 5         |
         |                               |
  HOSTILE REVIEW #2 (End-of-Week, Day 5) |
  [Scope: Security audit artifacts + Miri]
  [0C, 0M required for GO]
         |
  Day 6 (Fix findings + ROADMAP v7.5 + GATE_W49 + Commit)
```

**Track Independence Note:** Track A (Demo Hub HTML/CSS) is independent of Track C (security audit). Track B (distribution) depends on Track A completion for the GitHub Pages deploy (hub must be at v0.9.0 before deploying). Track C is fully independent of Tracks A and B.

**DOUBLE HOSTILE REVIEW GATES:**
1. **Mid-week (end of Day 2):** Review Demo Hub update
2. **End-of-week (end of Day 5):** Review security audit artifacts + Miri results

**Key decision points:**
- End of Day 1: Does Demo Hub have Entity-RAG featured card + version bumps? (Hub gate)
- End of Day 2: Does hostile reviewer approve hub.html? (Quality gate)
- End of Day 3: Is blog live on dev.to? Is GitHub Pages accessible? (Distribution gate)
- End of Day 5: Are all 4 security audit documents complete and hostile-reviewed? (Audit gate)

---

## Day-by-Day Summary

### Day 1 (2026-04-21): Demo Hub Overhaul — Structure + Content

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.1a | Add Entity-RAG featured card + new "AI / RAG" category section to `docs/demo/hub.html` | DOCWRITER | 2.5h | Browser | New featured card at top of page with: (1) link `href="entity-rag/index.html"`, (2) badges: `v0.9.0` + `LATEST` + `NEW` + `FEATURED`, (3) features: Entity NER, MetadataBoost, SQuAD Dataset, Live WASM, (4) description mentioning entity-enhanced RAG in browser, (5) date: March 2026, (6) inline SVG icon: stroke-only, 18x18 viewBox, stroke-width 1.5 (brain/nodes concept), (7) `.card--featured` class applied. New "AI / RAG" category section with: category-level SVG 14x14 viewBox stroke-width 2, category title "AI / RAG" |
| W49.1b | Demote Filter Playground from featured to Core Features grid | DOCWRITER | 1.3h | Browser | Filter Playground moved from top featured section to Core Features grid. LATEST + FEATURED badges removed. Version badge kept as `v0.7.0` (this is accurate — Filter Playground shipped in v0.7.0). `href="index.html"` is CORRECT and must be PRESERVED — `docs/demo/index.html` IS the Filter Playground page (confirmed: `<title>EdgeVec v0.7.0 // FILTER_PLAYGROUND</title>`). Do NOT change this href. Verify: clicking Filter Playground card navigates to the Filter Playground page. Card uses standard `.card` class (not `.card--featured`) |
| W49.1c | Version bump all references to v0.9.0 + WCAG viewport fix | DOCWRITER | 0.7h | Browser + grep | (1) All version badges on v0.9.0 items updated. Legitimate v0.7.0 references PRESERVED: Filter Playground badge (shipped v0.7.0), Soft Delete badge, SIMD Benchmark badge, Benchmark Dashboard badge — these are historically accurate version badges. (2) Footer brand reads `EDGEVEC v0.9.0`. (3) SIMD Benchmark card: "NEW" badge REMOVED (shipped v0.7.0, no longer new). (4) Viewport meta tag: `user-scalable=no` REMOVED, `maximum-scale=1.0` REMOVED. Final viewport: `<meta name="viewport" content="width=device-width, initial-scale=1.0">`. Verify: `grep "user-scalable" docs/demo/hub.html` returns 0 matches. (5) Entity-RAG card has `v0.9.0` badge |
| W49.1d | Hero stats update — consider 4th PQ stat | DOCWRITER | 0.7h | Browser | Existing 3 stats verified still accurate (32x Memory, 2x+ SIMD, <1ms Latency). If 4th PQ stat added: must use memory angle "8B/vec" (not compression ratio), must label "PQ (exp.)" to denote experimental status, must verify 4-stat flex-wrap on mobile (375px). If 4 stats cause layout break at 375px, keep 3 stats |
| W49.1e | Mobile responsive testing at 375px, 768px, 1024px, 1440px | DOCWRITER | 1.3h | Browser | Cards stack properly at 375px (single column). Grid works at 768px (2 columns). No horizontal overflow at any breakpoint. Featured card text readable at 375px. Entity-RAG card hover animations work on desktop. All touch targets >= 44x44px on mobile |

**Day 1 exit criterion:** Demo Hub has Entity-RAG featured card, Filter Playground demoted, all versions v0.9.0, WCAG viewport fixed, mobile responsive at all 4 breakpoints.

### Day 2 (2026-04-22): Demo Hub Polish + MID-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.2a | Final polish — orphan CSS cleanup + link verification | DOCWRITER | 1h | Browser | (1) No orphaned CSS rules (styles for deleted/moved elements removed), (2) ALL links in hub.html resolve — click each card, verify navigation, (3) Entity-RAG card links to `entity-rag/index.html` (relative, verified working), (4) Zero console errors in Chrome DevTools (no 404s for fonts, images, scripts), (5) Badge consistency: "NEW" ONLY on v0.9.0 items, "FEATURED" ONLY on Entity-RAG card |
| W49.2b | Semantic HTML audit + accessibility polish | DOCWRITER | 1h | Browser + Lighthouse | (1) Heading hierarchy: single `<h1>` (hero title), `<h2>` for card titles, no skipped levels, (2) Landmark elements present: `<nav>`, `<main>`, `<section>`, `<footer>`, (3) `prefers-reduced-motion` disables matrix rain AND any new animations (verify: set `prefers-reduced-motion: reduce` in DevTools, confirm no animation), (4) Decorative SVG icons marked `aria-hidden="true"`, (5) Lighthouse Accessibility score >= 90 |
| **W49.2c** | **MID-WEEK HOSTILE REVIEW: Demo Hub update** | **HOSTILE_REVIEWER** | **1.5h** | **Review** | **GO verdict on `docs/demo/hub.html`. 0 Critical, 0 Major. Attack vectors: (1) version consistency — footer reads `EDGEVEC v0.9.0`, Entity-RAG card has v0.9.0 badge, older cards retain accurate historical version badges, (2) badge logic — NEW only on v0.9.0 items, FEATURED only on Entity-RAG, (3) mobile responsive — 375px verified cards stack, (4) Entity-RAG card links to `entity-rag/index.html` correctly, (5) Filter Playground properly demoted to Core Features grid, href `index.html` preserved (it IS the Filter Playground), (6) SIMD Benchmark "NEW" badge removed, (7) zero console errors in Chrome DevTools, (8) no orphaned CSS rules, (9) semantic HTML — heading hierarchy `<h1>` > `<h2>`, landmarks present, (10) WCAG — `user-scalable=no` removed from viewport, (11) SVG icon is stroke-only 18x18 viewBox stroke-width 1.5, visually harmonious with existing icons** |

**Day 2 exit criterion:** Demo Hub hostile-reviewed and approved. All version references v0.9.0. WCAG compliant. Mobile responsive.

**HALT CONDITION:** If mid-week hostile review returns NO-GO, Day 3 Track B tasks that do NOT depend on hub (dev.to blog publish, social drafts) can proceed. GitHub Pages deploy is BLOCKED until hub passes review.

### Day 3 (2026-04-23): Distribution — Blog Publish + GitHub Pages Deploy

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.3a | Publish blog to dev.to | DOCWRITER | 2.3h | Browser | (1) Blog live on dev.to with correct Markdown formatting, (2) all code blocks render with syntax highlighting (Rust, TypeScript, JSON), (3) demo link `https://matte1782.github.io/edgevec/demo/entity-rag/index.html` included and works (after W49.3b completes), (4) Tags: `#webdev #rust #wasm #vectordatabase`, (5) Source: adapted from `docs/blog/entity-enhanced-rag.md` (hostile-reviewed W48). If demo link not yet live, add placeholder text "[demo link — deploying]" and update after W49.3b |
| W49.3b | Deploy demo to GitHub Pages | WASM_SPECIALIST | 3h | Browser | (1) `https://matte1782.github.io/edgevec/demo/hub.html` loads the updated v0.9.0 hub, (2) Entity-RAG demo accessible from hub card link at `https://matte1782.github.io/edgevec/demo/entity-rag/index.html`, (3) All hub card links resolve (no 404s), (4) Method: GitHub Actions workflow OR manual `gh-pages` branch push, (5) Matrix rain animation works, (6) Mobile responsive on actual mobile device or Chrome DevTools device mode |

**Day 3 exit criterion:** Blog live on dev.to. Demo Hub and Entity-RAG demo accessible on GitHub Pages.

**W48 Carry-Forward Items #1 and #2 resolved by Day 3.**

### Day 4 (2026-04-24): Distribution — Social + npm Prep + Security Audit Start

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.4a | Draft Twitter/X thread | DOCWRITER | 1.5h | Document | 6-tweet thread saved in `docs/planning/weeks/week_49/SOCIAL_THREAD_DRAFT.md`: (1) hook tweet, (2) problem statement, (3) solution with code snippet, (4) demo link + screenshot reference, (5) comparison table (EdgeVec vs Qdrant vs Weaviate on deployment model), (6) CTA: star repo + `npm install edgevec`. User posts manually |
| W49.4b | Draft Reddit posts | DOCWRITER | 1.5h | Document | Two posts saved in `docs/planning/weeks/week_49/REDDIT_DRAFT.md`: (1) r/rust — technical angle: WASM + SIMD + entity-enhanced RAG in Rust, 200-400 words, (2) r/webdev — demo angle: in-browser vector search, zero API calls, 200-400 words. User posts manually |
| W49.4c | npm pre-publish prep for edgevec-langchain@0.2.0 | PLANNER | 1.5h | Command | (1) `edgevec-langchain/CHANGELOG.md` updated with v0.2.0 section listing FilterExpression support, (2) `npm pack` in `edgevec-langchain/` produces tarball, (3) `tar tzf edgevec-langchain-0.2.0.tgz` shows only expected files (no extraneous), (4) `npm publish --dry-run` succeeds. If user not available for OTP, carry forward to W50 with explicit note in GATE_W49_COMPLETE.md |
| W49.4d | Update dev.to blog with live demo link (if W49.3b completed) | DOCWRITER | 0.3h | Browser | If demo link was placeholder in W49.3a, update blog post with live `https://matte1782.github.io/edgevec/demo/entity-rag/index.html` link. Verify link works in published post |
| W49.4e | `unsafe` code inventory (Track C early start) | RUST_ENGINEER | 3h | Document + grep | `docs/audits/UNSAFE_INVENTORY.md` created. Contains: (1) every `unsafe` block in `src/` with file path, line number, justification, and proof of soundness (invariant + why it holds), (2) cross-validated: `grep -rn "unsafe" src/` output included as appendix, (3) total count of unsafe blocks documented, (4) zero missed blocks — inventory count matches grep count. If zero unsafe blocks exist, document "0 unsafe blocks — pure safe Rust" |

**Day 4 exit criterion:** Social thread draft complete. Reddit drafts complete. npm pre-publish checklist done. Unsafe inventory started or complete.

**W48 Carry-Forward Items #3 and #4 resolved by Day 4.**

### Day 5 (2026-04-25): Security Audit Completion + END-OF-WEEK HOSTILE REVIEW

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.5a | Dependency audit | RUST_ENGINEER | 2.3h | Document + command | `docs/audits/DEPENDENCY_AUDIT.md` created. Contains: (1) `cargo audit` output (full), (2) `cargo deny check` output if cargo-deny installed (or note "cargo-deny not installed — skipped"), (3) list of all direct + transitive dependencies (from `cargo tree`), (4) any dependencies with known CVEs flagged with severity + remediation plan, (5) audit date and tool versions documented. If zero vulnerabilities: "0 known vulnerabilities as of YYYY-MM-DD" |
| W49.5b | Fuzzing campaign plan | TEST_ENGINEER | 3h | Document | `docs/audits/FUZZ_CAMPAIGN_PLAN.md` created. Contains: (1) inventory of existing fuzz targets in `fuzz/` directory (list each target file with description), (2) gap analysis: which parser/input boundaries in `src/` lack fuzz coverage, (3) 48h continuous fuzzing run plan with infrastructure needs (local machine specs or CI runner), (4) success criteria: "0 crashes after 48h continuous = PASS", (5) estimated corpus size and memory requirements |
| W49.5c | Miri regression run | TEST_ENGINEER | 3h | Document + command | `docs/audits/MIRI_REGRESSION_W49.md` created. Contains: (1) `cargo +nightly miri test` full output (or summary if large), (2) total tests run (must be >= 392, the W42 baseline), (3) any new UB findings (must be 0 for PASS), (4) any tests that no longer pass under Miri with root cause analysis, (5) comparison table: W42 baseline vs W49 results. If nightly toolchain install needed, document install command and version |
| **W49.5d** | **END-OF-WEEK HOSTILE REVIEW: Security audit artifacts + Miri** | **HOSTILE_REVIEWER** | **1.5h** | **Review** | **GO verdict on all 4 audit documents (UNSAFE_INVENTORY from Day 4, plus 3 from Day 5). 0 Critical, 0 Major. Attack vectors: (1) completeness — `grep -rn "unsafe" src/` cross-validated against inventory (zero missed blocks), (2) each unsafe block has specific invariant in soundness proof, not handwaving like "this is safe because we check", (3) fuzz campaign covers all parser/input boundaries identified in crate, (4) Miri test count >= 392 (W42 baseline), (5) dependency audit is dated and lists `cargo audit` version, (6) no "TODO" or "TBD" in any audit document** |

**Day 5 exit criterion:** All 4 security audit documents created and hostile-reviewed. Miri passes with 0 UB.

**Day 5 planned hours:** 2.3h + 3h + 3h + 1.5h = **9.8h** (with unsafe inventory moved to Day 4, down from 12.8h).

### Day 6 (2026-04-26): Overflow + Fix Findings + ROADMAP v7.5 + Commit

| ID | Task | Owner | Est. Hours | Verification | Acceptance Criteria |
|:---|:-----|:------|:-----------|:-------------|:--------------------|
| W49.6a | Fix all hostile review findings (both rounds) | Per finding | 2h | Review fixes | All Critical + Major issues from both hostile reviews (mid-week + end-of-week) resolved. Minor issues fixed or documented as accepted-as-is with justification |
| W49.6b | Update ROADMAP.md v7.4 to v7.5 | PLANNER | 1h | Document | (1) ROADMAP header updated to v7.5, (2) M11.1 week range corrected from "Week 48-49" to "Week 49-50", (3) Phase 10 fully marked COMPLETE, (4) W49 actuals added to revision history, (5) v0.10.0 status line updated if needed |
| W49.6c | Full regression | TEST_ENGINEER | 0.5h | Full suite | `cargo test --lib` passes (1048+ tests), `cargo clippy -- -D warnings` clean (0 warnings), `cargo check --target wasm32-unknown-unknown` succeeds |
| W49.6d | Create `.claude/GATE_W49_COMPLETE.md` | PLANNER | 0.5h | Gate | Gate file documents: (1) both hostile review verdicts with dates, (2) all deliverables with status, (3) regression results (test count, clippy, WASM check), (4) all 4 W48 carry-forward items resolved or explicitly carried to W50 with priority, (5) any new carry-forward items for W50 |
| W49.6e | Commit all W49 work | PLANNER | 0.5h | Git | Conventional commit: `chore(w49): Demo Hub v0.9.0 + distribution + security audit kickoff` |

**If surplus time available**, prioritize:
1. Optional polish from prompt: entrance animation for hub cards (staggered fadeIn, `prefers-reduced-motion` respected)
2. Open Graph meta tags for hub.html social sharing
3. "Last updated: March 2026" timestamp in hub footer

---

## Daily Execution Files

Each day has a dedicated execution file. These are atomic — an engineer can pick up any `DAY_N_TASKS.md` and execute without re-reading this weekly plan.

| File | Content | Created |
|:-----|:--------|:--------|
| `docs/planning/weeks/week_49/DAY_1_TASKS.md` | Demo Hub: Entity-RAG card + demotion + version bump + WCAG + mobile | Before Day 1 |
| `docs/planning/weeks/week_49/DAY_2_TASKS.md` | Demo Hub polish + semantic audit + mid-week hostile review | Before Day 2 |
| `docs/planning/weeks/week_49/DAY_3_TASKS.md` | dev.to blog publish + GitHub Pages deploy | Before Day 3 |
| `docs/planning/weeks/week_49/DAY_4_TASKS.md` | Social thread + Reddit drafts + npm prep + unsafe inventory (Track C early start) | Before Day 4 |
| `docs/planning/weeks/week_49/DAY_5_TASKS.md` | Dependency audit + fuzz plan + Miri + end-of-week review | Before Day 5 |
| `docs/planning/weeks/week_49/DAY_6_TASKS.md` | Overflow + fix findings + ROADMAP v7.5 + GATE_W49 + commit | Before Day 6 |

Each daily file contains:
1. **Day objective** and success criteria
2. **Pre-task context loading checklist** (which files to read)
3. **Task sequence** with dependencies clearly marked
4. **Specific command sequences** (cargo, npm, browser commands)
5. **Expected output format** for each task
6. **Decision trees** for conditional paths
7. **Handoff notes** (codebase state at EOD, next day prereqs)
8. **Time tracking template** (estimated vs actual per task)

---

## Deliverables

| Deliverable | Target Day | Owner |
|:------------|:-----------|:------|
| `DAY_1_TASKS.md` through `DAY_6_TASKS.md` — Daily execution files | Pre-sprint | PLANNER |
| `docs/demo/hub.html` — Updated to v0.9.0 with Entity-RAG featured card | Day 2 | DOCWRITER |
| Mid-week hostile review report | Day 2 | HOSTILE_REVIEWER |
| Blog live on dev.to | Day 3 | DOCWRITER |
| GitHub Pages deploy (hub + entity-rag accessible) | Day 3 | WASM_SPECIALIST |
| `docs/planning/weeks/week_49/SOCIAL_THREAD_DRAFT.md` — 6-tweet thread | Day 4 | DOCWRITER |
| `docs/planning/weeks/week_49/REDDIT_DRAFT.md` — r/rust + r/webdev posts | Day 4 | DOCWRITER |
| npm pre-publish checklist for edgevec-langchain@0.2.0 | Day 4 | PLANNER |
| `docs/audits/UNSAFE_INVENTORY.md` — Every unsafe block with soundness proof | Day 4 | RUST_ENGINEER |
| `docs/audits/DEPENDENCY_AUDIT.md` — cargo audit + cargo deny + dep tree | Day 5 | RUST_ENGINEER |
| `docs/audits/FUZZ_CAMPAIGN_PLAN.md` — Gap analysis + 48h run plan | Day 5 | TEST_ENGINEER |
| `docs/audits/MIRI_REGRESSION_W49.md` — Miri results vs W42 baseline | Day 5 | TEST_ENGINEER |
| End-of-week hostile review report | Day 5 | HOSTILE_REVIEWER |
| Updated `docs/planning/ROADMAP.md` v7.5 | Day 6 | PLANNER |
| `.claude/GATE_W49_COMPLETE.md` | Day 6 | PLANNER |

---

## Carry-Forward Traceability from W48

**ALL 4 items from GATE_W48_COMPLETE.md accounted for:**

| W48 Carry-Forward Item | Priority | W49 Task ID | Day | Status |
|:-----------------------|:---------|:------------|:----|:-------|
| Publish blog to dev.to | MEDIUM | W49.3a | Day 3 | PLANNED |
| Deploy demo to GitHub Pages | MEDIUM | W49.3b | Day 3 | PLANNED |
| Social media push (Twitter/X thread) | LOW | W49.4a | Day 4 | PLANNED |
| npm publish edgevec-langchain@0.2.0 | LOW | W49.4c | Day 4 | PLANNED (pre-publish prep; actual publish requires user OTP) |

**Zero items silently dropped.** All 4 explicitly scheduled with task IDs and acceptance criteria.

---

## Double Hostile Review Protocol

### Review #1: Mid-Week Demo Hub Review (Day 2)

**Scope:** `docs/demo/hub.html` (updated)
**Attack vectors:**
1. **Version consistency:** Footer reads `EDGEVEC v0.9.0`. Entity-RAG card has `v0.9.0` badge. Older cards retain historically accurate version badges (v0.6.0, v0.7.0)
2. **Badge logic:** "NEW" ONLY on v0.9.0 items, "FEATURED" ONLY on Entity-RAG card
3. **Mobile responsive:** Test at 375px width — cards stack single column, no horizontal overflow
4. **Entity-RAG card:** Links correctly to `entity-rag/index.html` (relative path from hub)
5. **Filter Playground:** Properly demoted to Core Features grid, `href="index.html"` PRESERVED (this is correct — `index.html` IS the Filter Playground page)
6. **SIMD Benchmark:** "NEW" badge removed (shipped v0.7.0)
7. **Console errors:** Zero errors in Chrome DevTools (no 404s for fonts, images, scripts)
8. **Orphaned CSS:** No styles for deleted/moved elements
9. **Semantic HTML:** Heading hierarchy `<h1>` > `<h2>`, landmark elements (`<nav>`, `<main>`, `<section>`, `<footer>`)
10. **WCAG:** `user-scalable=no` removed from viewport meta, `maximum-scale=1.0` removed
11. **SVG icon:** Stroke-only, 18x18 viewBox, stroke-width 1.5, visually harmonious with existing icons

**Verdict required: 0 Critical, 0 Major for GO.**

### Review #2: End-of-Week Security Audit Review (Day 5)

**Scope:** `docs/audits/UNSAFE_INVENTORY.md`, `docs/audits/DEPENDENCY_AUDIT.md`, `docs/audits/FUZZ_CAMPAIGN_PLAN.md`, `docs/audits/MIRI_REGRESSION_W49.md`
**Attack vectors:**
1. **Completeness:** Cross-validate unsafe inventory against `grep -rn "unsafe" src/` — zero missed blocks
2. **Soundness proofs:** Each unsafe block has a specific invariant (not handwaving like "this is safe")
3. **Fuzz coverage:** Campaign covers all parser/input boundaries identified in crate
4. **Miri baseline:** Test count >= 392 (W42 baseline), 0 new UB findings
5. **Dependency audit:** Dated, lists `cargo audit` version and `cargo deny` version (or notes it was skipped)
6. **No placeholders:** Zero instances of "TODO" or "TBD" in any audit document

**Verdict required: 0 Critical, 0 Major for GO.**

---

## Sprint-Level Acceptance Criteria

| Criterion | Pass/Fail |
|:----------|:----------|
| `docs/demo/hub.html` updated: footer `EDGEVEC v0.9.0`, Entity-RAG card `v0.9.0` badge, older cards retain accurate version badges | [ ] |
| Entity-RAG card is FEATURED with link to `entity-rag/index.html` | [ ] |
| Filter Playground demoted to Core Features grid with working href | [ ] |
| SIMD Benchmark "NEW" badge removed | [ ] |
| WCAG: `user-scalable=no` removed from viewport meta tag | [ ] |
| Footer brand reads `EDGEVEC v0.9.0` | [ ] |
| Mobile responsive at 375px, 768px, 1024px, 1440px (all tested) | [ ] |
| Zero console errors in Chrome DevTools | [ ] |
| Semantic HTML: `<h1>` > `<h2>` hierarchy, `<nav>` + `<main>` + `<footer>` landmarks | [ ] |
| Blog live on dev.to with syntax-highlighted code blocks | [ ] |
| GitHub Pages: `https://matte1782.github.io/edgevec/demo/hub.html` loads v0.9.0 hub | [ ] |
| GitHub Pages: entity-rag demo accessible from hub card link | [ ] |
| Social thread draft: 6 tweets in `SOCIAL_THREAD_DRAFT.md` | [ ] |
| Reddit drafts: 2 posts in `REDDIT_DRAFT.md` (r/rust + r/webdev) | [ ] |
| npm pre-publish: `npm publish --dry-run` succeeds for edgevec-langchain@0.2.0 | [ ] |
| `docs/audits/UNSAFE_INVENTORY.md` exists, cross-validated via grep (0 missed blocks) | [ ] |
| `docs/audits/DEPENDENCY_AUDIT.md` exists with `cargo audit` output and date | [ ] |
| `docs/audits/FUZZ_CAMPAIGN_PLAN.md` exists with gap analysis and 48h run plan | [ ] |
| `docs/audits/MIRI_REGRESSION_W49.md` exists, tests >= 392, 0 UB | [ ] |
| No "TODO" or "TBD" in any audit document | [ ] |
| Mid-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| End-of-week hostile review: GO (0 Critical, 0 Major) | [ ] |
| ROADMAP updated to v7.5 with M11.1 week range corrected to "W49-W50" | [ ] |
| No existing tests broken (1048+ lib pass) | [ ] |
| Clippy clean (`cargo clippy -- -D warnings` = 0 warnings) | [ ] |
| WASM build: `cargo check --target wasm32-unknown-unknown` succeeds | [ ] |
| W48 carry-forwards: all 4 resolved or explicitly carried to W50 | [ ] |
| GATE_W49_COMPLETE.md created with all required sections | [ ] |

---

## Risk Register

| Risk | Prob | Impact | Mitigation |
|:-----|:-----|:-------|:-----------|
| GitHub Pages deploy fails (branch config, Actions permissions) | MEDIUM | MEDIUM | Try manual `gh-pages` branch push first. If Actions needed, use standard GitHub Pages Action. Budget 3h for this task specifically |
| dev.to Markdown rendering differs from GitHub Markdown | MEDIUM | LOW | Preview before publishing. Adjust code fence syntax if needed. dev.to supports `rust` and `typescript` code fences |
| Miri nightly toolchain install fails or is incompatible | LOW | MEDIUM | Pin to known-good nightly. If Miri cannot run, document the failure in MIRI_REGRESSION_W49.md and carry forward to W50 with HIGH priority |
| Hub responsive layout breaks with 4 hero stats | MEDIUM | LOW | Test at 375px with 4 stats. If flex-wrap causes misalignment, keep 3 stats (original). PQ stat is optional |
| Hub Filter Playground href verification | LOW | LOW | RESOLVED: `docs/demo/index.html` IS the Filter Playground (confirmed by title tag). `href="index.html"` is correct and must be preserved during demotion |
| `cargo audit` finds CVEs in dependencies | LOW | HIGH | Document all findings in DEPENDENCY_AUDIT.md. If any are HIGH/CRITICAL severity, create immediate remediation plan for W50. Do NOT modify `Cargo.toml` this week (no src/ changes policy) |
| Mid-week hostile review returns NO-GO | LOW | MEDIUM | Fix all findings Day 2 evening. Track B blog publish can proceed regardless (independent). GitHub Pages deploy is BLOCKED until hub passes |
| npm pre-publish fails (missing files, version mismatch) | LOW | LOW | Follow `npm pack` + `tar tzf` verification. If issues found, fix in edgevec-langchain directory (not src/). Actual publish requires user OTP — if user unavailable, carry forward |

---

## Dependencies

| This Week Depends On | What |
|:---------------------|:-----|
| GATE_W48_COMPLETE.md [EXISTS] | Carry-forward items identified |
| `docs/demo/hub.html` [STABLE since v0.7.0] | Base file for Demo Hub update |
| `docs/demo/entity-rag/index.html` [STABLE from W48] | Design reference + link target |
| `docs/blog/entity-enhanced-rag.md` [HOSTILE-REVIEWED W48] | Source for dev.to publish |
| `cargo +nightly miri test` [REQUIRES nightly toolchain] | Miri regression — may need install |
| `cargo audit` [REQUIRES cargo-audit] | Dependency audit tool |
| GitHub Pages access [REQUIRES repo settings] | Deploy needs Pages enabled in repo settings |

| This Week Blocks | What |
|:-----------------|:-----|
| M11.1 second half (W50) | Security audit findings feed into W50 remediation |
| M11.2 Performance Guarantees (W50) | W49 Miri baseline establishes regression comparison |
| v1.0 release (W52) | Security audit is a v1.0 release criterion |
| Community growth | Blog + social + GitHub Pages are distribution prerequisites |

---

## NOT IN SCOPE THIS WEEK

| Task | Why Deferred |
|:-----|:-------------|
| Any modifications to `src/` | Read-only analysis only this week. Remediation in W50 |
| PQ modifications or improvements | PQ is CONDITIONAL GO (W47). No changes until v1.0 stabilization |
| New WASM exports or API changes | Stability phase — no API surface changes |
| BM25 engine | Not in v1.0 scope |
| Fuzzing execution (48h run) | W49 creates the PLAN. Actual fuzzing runs in W50 |
| v0.10.0 release | Pending all M11 milestones before version bump |
| React/Vue integration demos | Out of scope for Demo Hub — vanilla HTML/CSS only |
| New features or new tests in `src/` | Security audit is analysis, not implementation |
| Actual npm publish (requires OTP) | User handles OTP manually. W49 does pre-publish prep only |

---

## Anti-Error Checklist (Self-Validation)

- [x] No task > 16 hours (largest: W49.5a at 3h)
- [x] All acceptance criteria are binary pass/fail with specific numbers or commands
- [x] Dependencies reference specific files/functions, not vague descriptions
- [x] All 4 W48 carry-forward items explicitly mapped to W49 task IDs (W49.3a, W49.3b, W49.4a, W49.4c)
- [x] No modifications to `src/` — all Track C tasks are read-only analysis (grep, miri, cargo audit)
- [x] WCAG fix explicit: remove `user-scalable=no` and `maximum-scale=1.0` from viewport meta (line 5 of current hub.html)
- [x] Filter Playground href `index.html` is CORRECT (index.html IS the Filter Playground) — do NOT change, only verify it works after demotion
- [x] SIMD Benchmark "NEW" badge removal explicit
- [x] SVG icon spec: card-level 18x18 viewBox stroke-width 1.5, category-level 14x14 viewBox stroke-width 2
- [x] PQ hero stat uses "8B/vec" memory angle (not compression ratio) and labels "experimental"
- [x] WASM canonical export name: `searchBoosted` (camelCase js_name) — documented for reference, no code changes
- [x] Two hostile review gates with specific scopes and attack vectors
- [x] GATE_W49_COMPLETE.md listed as explicit Day 6 deliverable
- [x] ROADMAP correction (M11.1 "Week 48-49" -> "Week 49-50") listed as Day 6 task
- [x] DAY_1_TASKS.md through DAY_6_TASKS.md listed as deliverables
- [x] Track independence documented (A: HTML/CSS, B: distribution, C: security analysis)
- [x] 3x rule exceptions documented for all 3 tracks with rationale
- [x] Total planned hours: 33.4h across 6 days (~5.6h/day) — within ~35h budget
- [x] Commit convention: `chore(w49):` style specified
- [x] No PQ code modifications planned
- [x] Badge rules explicit: "NEW" only on v0.9.0 items, "FEATURED" only on Entity-RAG

---

## Total Estimation Summary

| Track | Optimistic | Planned | 3x Ceiling |
|:------|:-----------|:--------|:-----------|
| A: Demo Hub (Days 1-2) | 6h | 7.8h | 18h |
| B: Distribution (Days 3-4) | 6.5h | 10.1h | 19.5h |
| C: Security Audit (Days 4-5) | 7.5h | 11.3h | 22.5h |
| Day 6: Overflow | -- | 4.5h | -- |
| **Total** | **20h** | **33.4h** | **60h** |

---

## HOSTILE REVIEW REQUIRED

**Before distribution begins (end of Day 2):**
- [ ] HOSTILE_REVIEWER has approved Demo Hub update (hub.html at v0.9.0)

**Before committing (end of Day 5):**
- [ ] HOSTILE_REVIEWER has approved all 4 security audit documents + Miri results

---

## APPROVALS

| Role | Name | Signature | Date |
|:-----|:-----|:----------|:-----|
| PLANNER | | [APPROVED] | 2026-03-07 |
| HOSTILE_REVIEWER | | R1: REJECTED (1C, 2M, 5m) — all fixed. R2: GO (0C, 0M, 1m accepted-as-is: Track A estimation 7.8h vs 8.5h arithmetic discrepancy — non-blocking) | 2026-03-07 |

---

**END OF WEEKLY TASK PLAN**
