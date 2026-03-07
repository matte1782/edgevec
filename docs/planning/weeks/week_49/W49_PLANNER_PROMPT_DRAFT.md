# W49 PLANNER PROMPT — DRAFT v2

**Status:** [APPROVED] — R1: 2C+5M+7m fixed. R2: GO (0C, 0M, 1m fixed). Ready for /planner-weekly 49.

---

## HOSTILE REVIEW R1 FIX LOG

| Finding | Severity | Fix |
|:--------|:---------|:----|
| C1: ROADMAP says M11.1 is "Week 48-49" but prompt said "W49-W50" | Critical | Fixed: M11.1 is W49-W50 per actual schedule (W48 was Growth, not security). Prompt now instructs planner to update ROADMAP week range from "Week 48-49" to "Week 49-50" |
| C2: "no code in src/" ambiguous — Miri/grep read src/ | Critical | Fixed: Reworded to "No modifications to src/ this week. Read-only analysis (grep, miri, cargo audit) is in scope." |
| M1: npm publish acceptance criteria vague | Major | Fixed: Specified edgevec-langchain CHANGELOG.md, npm pack output expectations, carry-forward condition |
| M2: WASM export name not specified | Major | Fixed: Added canonical name note: `searchBoosted` (camelCase js_name) |
| M3: Miri on Day 6 falls outside end-of-week review scope | Major | Fixed: Moved Miri to Day 5. End-of-week review now includes Miri results |
| M4: No hour estimates for Track C tasks | Major | Fixed: Added optimistic estimates per task (2h + 1.5h + 2h + 2h = 7.5h optimistic) |
| M5: No GATE_W49_COMPLETE.md deliverable | Major | Fixed: Added as explicit Day 6 task |
| m1: "5 demo pages" unverified claim | Minor | Fixed: Removed specific count, said "multiple demo pages" |
| m2: PQ hero stat could overstate production-readiness | Minor | Fixed: Added note that any PQ stat must use memory angle ("8B/vec") not compression ratio, and label experimental |
| m3: Filter Playground self-referencing link to index.html | Minor | Fixed: Added explicit note to fix href when demoting to grid |
| m4: SVG icon size confusion (14x14 category vs 18x18 card) | Minor | Fixed: Made explicit: "card-level SVG: 18x18 viewBox, stroke-width 1.5; category-level SVG: 14x14 viewBox, stroke-width 2" |
| m5: "5-7 tweets" with 6 enumerated items | Minor | Fixed: Changed to "6-tweet thread" with listed items |
| m6: Track A/B missing Owner column | Minor | Fixed: Added Owner to all track tables |
| m7: hub.html has user-scalable=no (WCAG violation) | Minor | Fixed: Added explicit instruction to remove user-scalable=no and maximum-scale=1.0 |

---

## PROMPT FOR /planner-weekly 49

You are PLANNER. Create the Week 49 WEEKLY_TASK_PLAN.md for EdgeVec.

### CURRENT STATE (Read These First)

| Document | Path | Why |
|:---------|:-----|:----|
| ROADMAP v7.4 | `docs/planning/ROADMAP.md` | Phase 11 (v1.0) starts — Milestone 11.1 Security Audit. **NOTE:** ROADMAP says M11.1 is "Week 48-49" — this is STALE. Update to "Week 49-50" because W48 was the Growth Pivot, not security. |
| GATE_W48_COMPLETE.md | `.claude/GATE_W48_COMPLETE.md` | Carry-forward items (4 items — ALL must appear in W49) |
| W49_MANDATORY_ADDONS.md | `docs/planning/weeks/week_49/W49_MANDATORY_ADDONS.md` | P0 Demo Hub update spec |
| W48 WEEKLY_TASK_PLAN.md | `docs/planning/weeks/week_48/WEEKLY_TASK_PLAN.md` | Reference for format, rigor level, acceptance criteria density — MATCH THIS EXACTLY |
| Demo Hub (current) | `docs/demo/hub.html` | Current state: v0.7.0, stale badges, no Entity-RAG card. NOTE: line 5 has `user-scalable=no` and `maximum-scale=1.0` which are WCAG 2.1 AA violations — MUST be removed |
| Entity-RAG demo | `docs/demo/entity-rag/index.html` | Reference for cyberpunk aesthetic, design system, animation patterns |
| README.md | `README.md` | Current state for any updates |
| CHANGELOG.md | `CHANGELOG.md` | Current state for W49 entries |

### SPRINT IDENTITY

**Week 49: Launch & Polish — Demo Hub v0.9.0 + Distribution + v1.0 Prep**

This is a **hybrid frontend-engineering + distribution + v1.0-prep sprint**. Three tracks:

1. **Track A (P0, Days 1-2): Demo Hub Overhaul** — Frontend engineering at maximum quality. Owner: DOCWRITER + WASM_SPECIALIST (verification)
2. **Track B (Days 3-4): Distribution Pipeline** — Blog publish, GitHub Pages, social. Owner: DOCWRITER + PLANNER
3. **Track C (Days 5-6): v1.0 Security Audit Kickoff** — Phase 11 Milestone 11.1 begins (first half of W49-W50). Owner: RUST_ENGINEER + TEST_ENGINEER

### TRACK A: DEMO HUB OVERHAUL — FRONTEND ENGINEERING BRIEF

**This is NOT a simple version bump.** The Demo Hub is EdgeVec's public face — the first thing a developer sees. It must be **world-class frontend engineering**.

#### Design System Constraints (Non-Negotiable)

The existing cyberpunk design system is mature and consistent across multiple demo pages and the Entity-RAG demo. The hub update MUST:

1. **Preserve the design language exactly:**
   - Color palette: `--bg-void: #030306`, `--cyan: #00ffff`, `--magenta: #ff00ff`, `--green: #00ff88`, `--purple: #9945ff`
   - Typography: Orbitron (display) + JetBrains Mono (body)
   - Matrix rain background canvas with `prefers-reduced-motion` respect
   - Card system: `.card`, `.card--featured`, `.card--local` with hover animations (translateY, glow, top-line reveal)
   - Badge system: `.card__badge--version`, `.card__badge--new`, `.card__badge--featured`

2. **Elevate, don't redesign:**
   - New Entity-RAG featured card MUST feel like it belongs — same card anatomy, same badge placement, same hover behavior
   - New "AI / RAG" category section needs a distinctive but harmonious category icon
     - **Category-level SVG:** 14x14 viewBox, stroke-width 2 (matches existing category icons in hub.html lines 246-249)
     - **Card-level SVG:** 18x18 viewBox, stroke-width 1.5 (matches existing card icons in hub.html lines 384-389)
   - Hero stats: consider adding PQ memory stat as 4th stat — **MUST use memory angle ("8B/vec") not compression ratio, and any PQ reference must note it is experimental**. Verify alignment at 4 stats on mobile (flex-wrap behavior)

3. **Quality bar (hostile reviewer will verify ALL of these):**
   - **WCAG 2.1 AA fix:** Remove `user-scalable=no` and `maximum-scale=1.0` from viewport meta tag (line 5 of current hub.html). These prevent zoom and violate WCAG SC 1.4.4.
   - Zero console errors (no 404s for images, fonts, or scripts)
   - Mobile responsive: 375px, 768px, 1024px, 1440px breakpoints ALL tested
   - Lighthouse Performance > 90 (static HTML, should be trivial)
   - All links resolve — **IMPORTANT:** When demoting Filter Playground from featured to Core Features grid, fix its `href="index.html"` (currently self-referencing when hub IS index.html). Verify the correct relative path.
   - No orphaned CSS (remove any styles for deleted elements)
   - `prefers-reduced-motion` disables matrix rain AND any new animations
   - Semantic HTML: proper heading hierarchy (h1 > h2 > h3), landmark elements (`<nav>`, `<main>`, `<section>`, `<footer>`)
   - Badge consistency: ONLY v0.9.0 items get "NEW", ONLY the featured item gets "FEATURED"
   - Footer version: `EDGEVEC v0.9.0`
   - Filter Playground demoted from featured -> Core Features grid (keep content, remove FEATURED/LATEST badges, fix href)
   - SIMD Benchmark: remove stale "NEW" badge (shipped v0.7.0)
   - Entity-RAG card links to `entity-rag/index.html` (relative path from hub.html)
   - **WASM canonical export name:** The demo's WASM API uses `searchBoosted` (camelCase, via `js_name` attribute). No changes needed to WASM code, but any documentation or verification must reference this name correctly.

4. **SVG icon for Entity-RAG card:**
   - Card icon: stroke-only, 18x18 viewBox, stroke-width 1.5 (matching existing card icons)
   - Concept: brain + nodes, or neural network, or entity graph — something that says "AI/RAG"
   - Must be inline SVG (no external files)

5. **Optional polish (if surplus time):**
   - Subtle entrance animation for cards (staggered fadeIn, 50ms delay per card) — must respect `prefers-reduced-motion`
   - "Last updated: March 2026" timestamp in footer
   - Open Graph meta tags for social sharing (og:title, og:description, og:image)

#### Frontend Engineering Skills Integration

Apply these principles during Demo Hub implementation:
- **Accessibility (WCAG 2.1 AA):** Color contrast ratios (4.5:1 minimum for text), focus indicators on interactive elements, decorative elements marked `aria-hidden="true"`, no zoom restrictions
- **Performance:** No layout shifts (CLS=0), fonts preloaded via `<link rel="preconnect">` (already present), critical CSS inlined (already is — single file)
- **Semantic markup:** `<nav>`, `<main>`, `<section>`, `<article>`, `<footer>` properly used
- **Progressive enhancement:** Page must be usable with JS disabled (matrix rain is decorative enhancement, all content visible without it)

#### Track A Estimation

| Task | Optimistic | Planned (1.3x) |
|:-----|:-----------|:----------------|
| Entity-RAG featured card + category section | 2h | 2.5h |
| Filter Playground demotion + badge cleanup | 1h | 1.3h |
| Version bump (all v0.9.0 references) | 0.5h | 0.7h |
| Hero stats update (optional PQ stat) | 0.5h | 0.7h |
| WCAG fixes (viewport, semantics, focus) | 1h | 1.3h |
| Mobile responsive testing + fixes | 1h | 1.3h |
| **Track A Total** | **6h** | **7.8h** |

**3x Rule Exception (documented):** This is HTML/CSS editing in a single file with an established design system. All card patterns, badge styles, and animations already exist — this is pattern-matching, not greenfield. 1.3x multiplier is appropriate.

### TRACK B: DISTRIBUTION PIPELINE

**Carry-forward items from GATE_W48_COMPLETE.md (ALL 4 accounted for):**

| Item | Priority | Day | Owner | Acceptance Criteria |
|:-----|:---------|:----|:------|:--------------------|
| Publish blog to dev.to | MEDIUM | Day 3 | DOCWRITER | Blog live on dev.to with correct Markdown formatting, all code blocks render with syntax highlighting, demo link `https://matte1782.github.io/edgevec/demo/entity-rag/index.html` works. Tags: #webdev #rust #wasm #vectordatabase. Source: `docs/blog/entity-enhanced-rag.md` |
| Deploy demo to GitHub Pages | MEDIUM | Day 3 | WASM_SPECIALIST | `https://matte1782.github.io/edgevec/demo/hub.html` loads the updated v0.9.0 hub. Entity-RAG demo accessible from hub card link. Method: GitHub Actions workflow OR manual `gh-pages` branch push. Verify both hub.html and entity-rag/index.html load without errors |
| Social media push (Twitter/X thread) | LOW | Day 4 | DOCWRITER | 6-tweet thread draft in `docs/planning/weeks/week_49/SOCIAL_THREAD_DRAFT.md`: (1) hook, (2) problem, (3) solution with code snippet, (4) demo link + screenshot, (5) comparison table, (6) CTA (star + install). User posts manually |
| npm publish edgevec-langchain@0.2.0 | LOW | Day 4 | PLANNER | Pre-publish checklist: (1) `edgevec-langchain/CHANGELOG.md` updated with v0.2.0 section listing FilterExpression support, (2) `npm pack` produces tarball without extraneous files (verify: `tar tzf edgevec-langchain-0.2.0.tgz` shows only expected files), (3) `npm publish --dry-run` succeeds. User handles actual `npm publish` with OTP. If user not available for OTP, carry forward to W50 with note in GATE_W49_COMPLETE.md |

**Additional distribution tasks:**

| Task | Day | Owner | Acceptance Criteria |
|:-----|:----|:------|:--------------------|
| Reddit post draft (r/rust, r/webdev) | Day 4 | DOCWRITER | Draft in `docs/planning/weeks/week_49/REDDIT_DRAFT.md`. Two posts: (1) r/rust — technical angle (WASM + SIMD + entity-enhanced RAG in Rust), (2) r/webdev — demo angle (in-browser vector search, zero API calls). Each 200-400 words. User posts manually |

#### Track B Estimation

| Task | Optimistic | Planned (1.5x) |
|:-----|:-----------|:----------------|
| dev.to blog publish + formatting | 1.5h | 2.3h |
| GitHub Pages deploy + verification | 2h | 3h |
| Social thread draft | 1h | 1.5h |
| npm pre-publish prep | 1h | 1.5h |
| Reddit drafts | 1h | 1.5h |
| **Track B Total** | **6.5h** | **9.8h** |

**3x Rule Exception (documented):** Blog publish is copy-paste from existing hostile-reviewed `docs/blog/entity-enhanced-rag.md`. Social/Reddit drafts are content writing with defined constraints. GitHub Pages deploy may hit unexpected issues — 1.5x multiplier applied across all Track B tasks (infra uncertainty factored into the GitHub Pages task estimate at 2h optimistic -> 3h planned).

### TRACK C: v1.0 SECURITY AUDIT KICKOFF

**ROADMAP Phase 11, Milestone 11.1 — first half (W49 of W49-W50 milestone).**

**IMPORTANT:** The ROADMAP currently says M11.1 is "Week 48-49, 16h". This is STALE — W48 was the Growth Pivot. Update ROADMAP to say "Week 49-50, 16h" as a Day 6 task.

**No modifications to `src/` this week.** Read-only analysis of `src/` (grep, miri, cargo audit) is fully in scope.

| Task | Day | Owner | Optimistic Hours | Acceptance Criteria |
|:-----|:----|:------|:-----------------|:--------------------|
| `unsafe` code inventory | Day 5 | RUST_ENGINEER | 2h | `docs/audits/UNSAFE_INVENTORY.md` listing every `unsafe` block in `src/` with: file path, line number, justification, proof of soundness (invariant + why it holds). Cross-validated against `grep -rn "unsafe" src/` output. Count of total unsafe blocks. Zero missed blocks. |
| Dependency audit | Day 5 | RUST_ENGINEER | 1.5h | `cargo audit` output appended to `docs/audits/DEPENDENCY_AUDIT.md`. `cargo deny check` if cargo-deny is installed. List all transitive deps. Flag any with known CVEs. If no issues found, document "0 known vulnerabilities" with audit date. |
| Fuzzing campaign plan | Day 5 | TEST_ENGINEER | 2h | `docs/audits/FUZZ_CAMPAIGN_PLAN.md` — (1) inventory of existing fuzz targets in `fuzz/`, (2) gap analysis: which parser/input boundaries lack fuzz targets, (3) 48h continuous run plan with infrastructure needs (local vs CI), (4) success criteria (0 crashes = pass). |
| Miri regression | Day 5 | TEST_ENGINEER | 2h | `cargo +nightly miri test` results documented in `docs/audits/MIRI_REGRESSION_W49.md`. Compare against W42 baseline (392+ pass, 0 UB). Document: (1) total tests run, (2) any new UB findings, (3) any tests that no longer pass under Miri (with root cause). If nightly toolchain install needed, budget extra time. |

#### Track C Estimation

| Task | Optimistic | Planned (1.5x) |
|:-----|:-----------|:----------------|
| Unsafe inventory | 2h | 3h |
| Dependency audit | 1.5h | 2.3h |
| Fuzzing campaign plan | 2h | 3h |
| Miri regression | 2h | 3h |
| **Track C Total** | **7.5h** | **11.3h** |

**3x Rule Exception (documented):** These are analysis/documentation tasks, not implementation. The codebase is well-known. Miri regression is a re-run of an established process (W42 baseline exists). 1.5x is appropriate for potential nightly toolchain issues or unexpected findings.

### DAY 6: OVERFLOW + GATES + COMMIT

| Task | Owner | Hours | Acceptance Criteria |
|:-----|:------|:------|:--------------------|
| Fix all hostile review findings (both rounds) | Per finding | 2h | All Critical + Major issues resolved from both reviews |
| Update ROADMAP.md v7.4 -> v7.5 | PLANNER | 1h | W49 actuals added. M11.1 week range corrected to "W49-W50". Phase 10 fully marked COMPLETE. |
| Create `.claude/GATE_W49_COMPLETE.md` | PLANNER | 0.5h | Gate file documents: both hostile review verdicts, carry-forward items from W48 (all resolved or explicitly carried to W50), all deliverables with status, regression results, any new carry-forward items |
| Full regression | TEST_ENGINEER | 0.5h | `cargo test --lib` + `cargo clippy -- -D warnings` + `cargo check --target wasm32-unknown-unknown` — all pass |
| Commit all W49 work | PLANNER | 0.5h | Conventional commit: `chore(w49): Demo Hub v0.9.0 + distribution + security audit kickoff` |

### HOSTILE REVIEW GATES

**Two hostile reviews required:**

1. **Mid-week (end of Day 2):** Demo Hub update review
   - Scope: `docs/demo/hub.html` (updated)
   - Attack vectors: (1) version consistency — grep all v0.7.0 references, must be 0 remaining, (2) badge logic — NEW only on v0.9.0 items, FEATURED only on Entity-RAG card, (3) mobile responsive — test at 375px width, verify cards stack properly, (4) Entity-RAG card links correctly to `entity-rag/index.html`, (5) Filter Playground properly demoted to Core Features grid with fixed href, (6) SIMD Benchmark "NEW" badge removed, (7) no console errors in Chrome DevTools, (8) no orphaned CSS rules, (9) semantic HTML — heading hierarchy, landmarks, (10) WCAG — `user-scalable=no` removed, color contrast adequate, (11) SVG icon is stroke-only, proportioned correctly, visually harmonious
   - **0 Critical, 0 Major required for GO**

2. **End-of-week (end of Day 5):** Security audit artifacts + Miri review
   - Scope: `docs/audits/UNSAFE_INVENTORY.md`, `docs/audits/FUZZ_CAMPAIGN_PLAN.md`, `docs/audits/DEPENDENCY_AUDIT.md`, `docs/audits/MIRI_REGRESSION_W49.md`
   - Attack vectors: (1) completeness — cross-validate unsafe inventory against `grep -rn "unsafe" src/` (zero missed blocks), (2) each unsafe block has a soundness proof with specific invariant, not handwaving, (3) fuzz campaign covers all parser/input boundaries identified in crate, (4) Miri test count >= W42 baseline (392+), (5) dependency audit is dated and lists tool versions, (6) no "TODO" or "TBD" in any audit document
   - **0 Critical, 0 Major required for GO**

### TOTAL ESTIMATION SUMMARY

| Track | Optimistic | Planned | 3x Ceiling |
|:------|:-----------|:--------|:-----------|
| A: Demo Hub | 6h | 7.8h | 18h |
| B: Distribution | 6.5h | 9.8h | 19.5h |
| C: Security Audit | 7.5h | 11.3h | 22.5h |
| Day 6: Overflow | — | 4.5h | — |
| **Total** | **20h** | **33.4h** | **60h** |

Planned total: ~33.4h across 6 days (~5.6h/day). Within the ~35h budget.

### FORMAT REQUIREMENTS

Follow W48's WEEKLY_TASK_PLAN.md format EXACTLY. The plan MUST contain ALL of these sections:

- [ ] Sprint header (status, goal, dates, prerequisites, milestone)
- [ ] Strategic context (why this week matters)
- [ ] Key references table (documents to read before any task)
- [ ] Estimation notes with 3x rule (with documented exceptions)
- [ ] Critical path diagram (ASCII art)
- [ ] Day-by-day breakdown with task tables: ID (W49.Na), Task, Owner, Est. Hours, Verification, Acceptance Criteria (binary pass/fail with specific numbers/commands)
- [ ] Daily execution files listed (DAY_1_TASKS.md through DAY_6_TASKS.md) as deliverables
- [ ] Track independence documentation
- [ ] Risk register (probability, impact, mitigation per risk)
- [ ] Dependencies table (depends-on AND blocks)
- [ ] NOT IN SCOPE section
- [ ] Carry-forward traceability from W48 (all 4 items accounted for)
- [ ] Anti-error checklist (self-validation)
- [ ] Sprint-level acceptance criteria table (all binary pass/fail)
- [ ] Double hostile review protocol with specific attack vectors per review
- [ ] GATE_W49_COMPLETE.md as explicit Day 6 deliverable
- [ ] APPROVALS section (PLANNER + HOSTILE_REVIEWER signatures)

### CONSTRAINTS

- Total planned hours: ~33-35h across 6 days (~5.5-5.8h/day)
- No task > 16 hours
- All acceptance criteria binary pass/fail with specific numbers or commands
- Demo Hub work is P0 — if it takes longer, distribution slides to surplus/carry-forward
- Security audit is foundational — even if incomplete in W49, the inventory and plan MUST be STARTED
- **No modifications to `src/` this week.** Read-only analysis (grep, miri, cargo audit) is in scope.
- No PQ modifications
- WASM build check after hub update (verify nothing broken): `cargo check --target wasm32-unknown-unknown`
- Demo Hub is HTML/CSS only — no new JS beyond existing matrix rain
- Blog publish is adaptation from `docs/blog/entity-enhanced-rag.md` to dev.to format — minimal changes
- All carry-forward items from GATE_W48_COMPLETE.md MUST appear (none silently dropped)

### SUCCESS METRICS

| Metric | Target |
|:-------|:-------|
| Demo Hub version | v0.9.0 everywhere (0 stale v0.7.0 references) |
| Entity-RAG card | Featured, linked to entity-rag/index.html, working |
| WCAG fix | `user-scalable=no` removed from viewport |
| Blog published | Live on dev.to |
| GitHub Pages | Hub accessible at `https://matte1782.github.io/edgevec/demo/hub.html` |
| Unsafe inventory | Complete (0 missed blocks, cross-validated via grep) |
| Fuzz campaign plan | Document exists, hostile-reviewed |
| Miri regression | >= 392 tests pass, 0 UB |
| Mobile responsive | 375px-1440px verified |
| Hostile reviews | 2x GO (0C, 0M each) |
| GATE_W49_COMPLETE.md | Created with all sections |
| W48 carry-forwards | All 4 resolved or explicitly carried to W50 |

---

**END OF PLANNER PROMPT DRAFT v2**
