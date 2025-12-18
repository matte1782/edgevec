# Week 24 Fuzz Campaign Instructions

**Status:** READY FOR MANUAL EXECUTION
**Date:** 2025-12-18
**Targets:** filter_simple, filter_deep

---

## Prerequisites

`cargo fuzz` requires **Linux** with libFuzzer. You'll need:

1. Linux environment (native, WSL2, or Docker)
2. Nightly Rust toolchain: `rustup toolchain install nightly`
3. cargo-fuzz: `cargo +nightly install cargo-fuzz`

---

## Quick Start (Copy-Paste Commands)

### Option A: WSL2 (Recommended for Windows)

```bash
# From WSL2 terminal, navigate to project
cd /mnt/c/Users/matte/Desktop/Desktop\ OLD/AI/Università\ AI/courses/personal_project/fortress_problem_driven/research_fortress/edgevec

# Enter fuzz directory
cd fuzz

# Run filter_simple (24 hours = 86400 seconds)
cargo +nightly fuzz run filter_simple -- -max_total_time=86400 2>&1 | tee artifacts/filter_simple_run_w24.log

# In a separate terminal, run filter_deep
cargo +nightly fuzz run filter_deep -- -max_total_time=86400 2>&1 | tee artifacts/filter_deep_run_w24.log
```

### Option B: Docker

```bash
# Build a fuzzing container
docker run -it --rm -v "$(pwd):/workspace" rust:nightly bash -c "
  cd /workspace/fuzz
  cargo install cargo-fuzz
  cargo +nightly fuzz run filter_simple -- -max_total_time=86400
"
```

---

## Corpus Seed Files Created

### filter_simple corpus (text-based)
- `seed_simple_eq.txt`: `x = 1`
- `seed_and.txt`: `a > 10 AND b < 20`
- `seed_string.txt`: `category = "test"`
- `seed_not.txt`: `NOT (x = 5)`
- `seed_null.txt`: `field IS NULL`
- `seed_in.txt`: `x IN (1, 2, 3)`
- `seed_complex.txt`: `(a = 1 OR b = 2) AND c = 3`

### filter_deep corpus (binary structured input)
- `seed_nested1.bin`: Binary data for nested AND/OR generation
- `seed_nested2.bin`: Binary data for deeper nesting
- `seed_not_nested.bin`: Binary data for NOT + nesting

---

## Expected Behavior

### filter_simple
- Parses arbitrary UTF-8 strings through `edgevec::filter::parse()`
- Should return `Result<Filter, ParseError>` for all inputs
- **MUST NOT PANIC** regardless of input

### filter_deep
- Generates deeply nested expressions (up to 50 levels)
- Tests stack overflow protection
- Tests recursive parser limits
- **MUST NOT PANIC** regardless of nesting depth

---

## Success Criteria (Day 7)

| Metric | Requirement |
|:-------|:------------|
| Duration | Minimum 24 hours per target |
| Crashes | 0 crashes |
| Panics | 0 panics |
| OOM | 0 out-of-memory events |

---

## If Crashes Are Found

1. **STOP** the campaign
2. **DO NOT PROCEED** to Day 2 tasks
3. Crash artifacts will be in `fuzz/artifacts/crashes/`
4. Reproduce with: `cargo +nightly fuzz run <target> <crash_file>`
5. Fix the bug in `src/filter/`
6. Re-run fuzzing from scratch
7. Document the fix in CHANGELOG.md

---

## Monitoring Progress

```bash
# Check fuzzer stats
watch -n 60 'ls -la fuzz/corpus/filter_simple/ | wc -l'

# Check for crashes
ls fuzz/artifacts/crashes/ 2>/dev/null || echo "No crashes yet"

# Check log for coverage stats
tail -f fuzz/artifacts/filter_simple_run_w24.log
```

---

## Background Execution (tmux/screen)

```bash
# Using tmux
tmux new-session -d -s fuzz_simple 'cd fuzz && cargo +nightly fuzz run filter_simple -- -max_total_time=86400 2>&1 | tee artifacts/filter_simple_run_w24.log'
tmux new-session -d -s fuzz_deep 'cd fuzz && cargo +nightly fuzz run filter_deep -- -max_total_time=86400 2>&1 | tee artifacts/filter_deep_run_w24.log'

# Check sessions
tmux ls

# Attach to monitor
tmux attach -t fuzz_simple
```

---

## Day 1 Completion Checklist

After starting both campaigns:

- [ ] filter_simple campaign running (check with `ps aux | grep filter_simple`)
- [ ] filter_deep campaign running (check with `ps aux | grep filter_deep`)
- [ ] Log files being written to `fuzz/artifacts/`
- [ ] No immediate crashes in first 5 minutes

---

**Note:** While fuzzing runs in background (24+ hours), agents will work on Day 2-6 tasks. Results verification happens on Day 7 before npm publish.
