#!/usr/bin/env python3
"""
EdgeVec Benchmark Regression Detection Script

Task: W10.8 - Create Benchmark Validation Suite
Updated: W18.3 v1.3 - Hostile review fixes (calibrated baselines, correct naming)

This script compares criterion benchmark results against baselines and
detects performance regressions for both median (P50) and tail latencies.

Checks performed:
1. Median (P50) latency regression
2. Tail latency regression (conservative estimate)
3. Tail/median ratio sanity check

Usage:
    python benches/check_regression.py [--baseline baselines.json] [--results target/criterion]

Exit codes:
    0 - All benchmarks within threshold
    1 - Regression detected (>10% slower than baseline)
    2 - Error (missing files, parse errors, tail extraction failure)

CRITERION OUTPUT FORMAT (verified 2025-12-15 from actual cargo bench run):
Example estimates.json:
{
    "mean": {"point_estimate": 198003108.0, "confidence_interval": {...}},
    "median": {"point_estimate": 198200200.0, "confidence_interval": {...}},
    "std_dev": {"point_estimate": 2769826.78, "confidence_interval": {...}},
    "median_abs_dev": {"point_estimate": 3746189.13, ...},
    "slope": null  // or {"point_estimate": ..., "confidence_interval": {...}}
}
- Unit: nanoseconds (f64)
- NO native percentiles key

TAIL LATENCY ESTIMATION (W18.3 v1.3):
We estimate tail latency as: mean + 5*std_dev

This is NOT true P99. For normal distributions:
- P99 = mean + 2.326*std_dev
- P99.99997 = mean + 5*std_dev (our approach)

We intentionally use this CONSERVATIVE bound because:
1. It catches more regressions than true P99
2. Performance distributions are often long-tailed
3. Early detection is better than missed regressions

W18.3 v1.3 Hostile Review Fixes:
- [M4] Baselines tightened to measured values + 20% buffer
- [M5] Tail baselines calibrated from actual benchmark runs
- [C1] Renamed "P99" to "tail" for statistical accuracy
- [m1] Removed unused verbose parameter
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

# =============================================================================
# CONSTANTS
# =============================================================================

# Default paths
DEFAULT_BASELINE = Path(__file__).parent / "baselines.json"
DEFAULT_RESULTS = Path(__file__).parent.parent / "target" / "criterion"

# Regression thresholds
P50_REGRESSION_THRESHOLD: float = 1.10  # 10% regression tolerance for median
TAIL_REGRESSION_THRESHOLD: float = 1.50  # 50% regression tolerance for tail
TAIL_MEDIAN_RATIO_MAX: float = 5.0  # Tail can be up to 5x median

# Tail estimation multiplier (mean + N * std_dev)
# Using 5 for conservative tail estimate (~P99.99997 for normal distributions)
TAIL_STDDEV_MULTIPLIER: float = 5.0

# Supported units for conversion
SUPPORTED_UNITS: set[str] = {"ns", "us", "ms", "s"}

# Benchmark group name (hardcoded for now)
BENCHMARK_GROUP: str = "validation"


# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

def load_baselines(path: Path) -> dict[str, Any]:
    """Load baseline values from JSON file."""
    if not path.exists():
        print(f"ERROR: Baseline file not found: {path}")
        sys.exit(2)

    with open(path) as f:
        return json.load(f)


def find_criterion_estimate(benchmark_dir: Path) -> dict[str, Any] | None:
    """
    Find estimates from criterion's estimates.json.

    Criterion stores results in:
    target/criterion/<group>/<benchmark>/new/estimates.json

    Returns full estimates dict for tail extraction.
    """
    estimates_path = benchmark_dir / "new" / "estimates.json"
    if not estimates_path.exists():
        # Try base.json for comparison runs
        estimates_path = benchmark_dir / "base" / "estimates.json"

    if not estimates_path.exists():
        return None

    with open(estimates_path) as f:
        return json.load(f)


def extract_median_ns(estimates: dict[str, Any]) -> float | None:
    """Extract median in nanoseconds from Criterion estimates."""
    # Try median first (most accurate)
    if "median" in estimates:
        median = estimates["median"]
        if isinstance(median, dict) and "point_estimate" in median:
            return float(median["point_estimate"])

    # Fall back to slope (for iterated benchmarks)
    if "slope" in estimates and estimates["slope"] is not None:
        slope = estimates["slope"]
        if isinstance(slope, dict) and "point_estimate" in slope:
            return float(slope["point_estimate"])

    return None


def extract_tail_ns(estimates: dict[str, Any], benchmark_name: str) -> tuple[float | None, bool]:
    """
    Extract tail latency estimate in nanoseconds from Criterion estimates.

    Returns:
        (tail_value, is_estimated): Tuple of tail value and whether it was estimated

    Uses mean + 5*std_dev for conservative tail estimate.
    This is intentionally more conservative than true P99.
    """
    if "mean" in estimates and "std_dev" in estimates:
        mean = estimates["mean"]
        std_dev = estimates["std_dev"]

        if isinstance(mean, dict) and isinstance(std_dev, dict):
            mean_ns = mean.get("point_estimate", 0)
            std_dev_ns = std_dev.get("point_estimate", 0)

            if mean_ns > 0 and std_dev_ns >= 0:
                tail_estimate = mean_ns + (TAIL_STDDEV_MULTIPLIER * std_dev_ns)
                print(f"  [{benchmark_name}] Tail estimated: mean({mean_ns:.0f}) + {TAIL_STDDEV_MULTIPLIER}*std_dev({std_dev_ns:.0f}) = {tail_estimate:.0f} ns")
                return tail_estimate, True

    # Final fallback: use upper confidence bound
    if "mean" in estimates:
        mean = estimates["mean"]
        if isinstance(mean, dict):
            ci = mean.get("confidence_interval", {})
            upper = ci.get("upper_bound")
            if upper:
                print(f"  [{benchmark_name}] Tail fallback: using mean upper CI bound = {upper:.0f} ns")
                return float(upper), True

    return None, False


def convert_ns_to_unit(value_ns: float, unit: str) -> float:
    """
    Convert nanoseconds to the target unit.

    Validates unit is supported, raises error otherwise.
    """
    if unit not in SUPPORTED_UNITS:
        raise ValueError(f"Unsupported unit '{unit}'. Supported: {SUPPORTED_UNITS}")

    if unit == "ns":
        return value_ns
    elif unit == "us":
        return value_ns / 1_000.0
    elif unit == "ms":
        return value_ns / 1_000_000.0
    elif unit == "s":
        return value_ns / 1_000_000_000.0
    else:
        return value_ns


def get_baseline_tail(config: dict[str, Any]) -> float:
    """Get tail baseline, supporting both 'tail' and legacy 'p99' keys."""
    return config.get("tail", config.get("p99", 0))


# =============================================================================
# MAIN REGRESSION CHECK
# =============================================================================

def check_regression(
    baseline: dict[str, Any],
    results_dir: Path,
    threshold: float = P50_REGRESSION_THRESHOLD,
    strict_tail: bool = True,
) -> tuple[bool, dict[str, Any]]:
    """
    Check for regressions against baselines.

    Args:
        baseline: Baseline configuration dict
        results_dir: Path to criterion results
        threshold: P50 regression threshold multiplier
        strict_tail: If True, FAIL validation when tail cannot be extracted

    Returns:
        (passed, results_dict)
    """
    results: dict[str, Any] = {}
    all_passed = True

    benchmarks = baseline.get("benchmarks", {})

    for name, config in benchmarks.items():
        # Look for benchmark in criterion output
        benchmark_path = results_dir / BENCHMARK_GROUP / name

        if not benchmark_path.exists():
            results[name] = {
                "status": "SKIP",
                "reason": "No results found",
            }
            continue

        estimates = find_criterion_estimate(benchmark_path)
        if estimates is None:
            results[name] = {
                "status": "SKIP",
                "reason": "Could not parse estimates.json",
            }
            continue

        # Extract metrics in nanoseconds
        current_median_ns = extract_median_ns(estimates)
        current_tail_ns, tail_estimated = extract_tail_ns(estimates, name)

        if current_median_ns is None:
            results[name] = {
                "status": "SKIP",
                "reason": "Could not extract median from estimates",
            }
            continue

        # FAIL if tail extraction fails and baseline expects tail
        baseline_tail = get_baseline_tail(config)
        if current_tail_ns is None and baseline_tail > 0 and strict_tail:
            results[name] = {
                "status": "FAIL",
                "reason": "Could not extract tail from estimates (baseline requires tail)",
            }
            all_passed = False
            continue

        # Get baseline values and unit
        try:
            unit = config.get("unit", "ns")
            if unit not in SUPPORTED_UNITS:
                raise ValueError(f"Unsupported unit in baseline: {unit}")
        except ValueError as e:
            results[name] = {
                "status": "FAIL",
                "reason": str(e),
            }
            all_passed = False
            continue

        baseline_p50 = config.get("p50", 0)
        hard_limit = config.get("hard_limit", float("inf"))

        # Convert to target unit
        current_median = convert_ns_to_unit(current_median_ns, unit)
        current_tail = convert_ns_to_unit(current_tail_ns, unit) if current_tail_ns else None

        # Initialize result
        result: dict[str, Any] = {
            "current_p50": current_median,
            "current_tail": current_tail,
            "baseline_p50": baseline_p50,
            "baseline_tail": baseline_tail,
            "unit": unit,
            "tail_estimated": tail_estimated,
            "checks": [],
        }

        # Check 1: P50 (median) regression
        if baseline_p50 > 0:
            p50_ratio = current_median / baseline_p50
            result["p50_ratio"] = p50_ratio

            if current_median > hard_limit:
                result["checks"].append({
                    "name": "P50 Hard Limit",
                    "passed": False,
                    "reason": f"Exceeds hard limit ({current_median:.2f} > {hard_limit:.2f} {unit})"
                })
                all_passed = False
            elif p50_ratio > threshold:
                result["checks"].append({
                    "name": "P50 Regression",
                    "passed": False,
                    "reason": f"P50 {p50_ratio:.1%} of baseline (threshold: {threshold:.0%})"
                })
                all_passed = False
            else:
                result["checks"].append({
                    "name": "P50 Regression",
                    "passed": True,
                    "reason": f"P50 {p50_ratio:.1%} of baseline"
                })

        # Check 2: Tail regression
        if current_tail is not None and baseline_tail > 0:
            tail_ratio = current_tail / baseline_tail
            result["tail_ratio"] = tail_ratio

            if tail_ratio > TAIL_REGRESSION_THRESHOLD:
                result["checks"].append({
                    "name": "Tail Regression",
                    "passed": False,
                    "reason": f"Tail {tail_ratio:.1%} of baseline (threshold: {TAIL_REGRESSION_THRESHOLD:.0%})"
                })
                all_passed = False
            else:
                result["checks"].append({
                    "name": "Tail Regression",
                    "passed": True,
                    "reason": f"Tail {tail_ratio:.1%} of baseline"
                })

        # Check 3: Tail/P50 ratio sanity check
        if current_tail is not None and current_median > 0:
            tail_p50_ratio = current_tail / current_median
            result["tail_p50_ratio"] = tail_p50_ratio

            if tail_p50_ratio > TAIL_MEDIAN_RATIO_MAX:
                result["checks"].append({
                    "name": "Tail/P50 Ratio",
                    "passed": False,
                    "reason": f"Tail/P50 ratio {tail_p50_ratio:.2f}x exceeds {TAIL_MEDIAN_RATIO_MAX}x limit"
                })
                all_passed = False
            else:
                result["checks"].append({
                    "name": "Tail/P50 Ratio",
                    "passed": True,
                    "reason": f"Tail/P50 ratio {tail_p50_ratio:.2f}x (OK)"
                })

        # Determine overall status
        failed_checks = [c for c in result["checks"] if not c["passed"]]
        if failed_checks:
            result["status"] = "REGRESSION" if any("Regression" in c["name"] for c in failed_checks) else "FAIL"
            result["reason"] = "; ".join(c["reason"] for c in failed_checks)
        else:
            result["status"] = "PASS"
            result["reason"] = "All checks passed"

        results[name] = result

    return all_passed, results


# =============================================================================
# OUTPUT FORMATTERS
# =============================================================================

def print_results(results: dict[str, Any]) -> None:
    """Print results in a formatted table."""
    print("\n" + "=" * 80)
    print("BENCHMARK VALIDATION RESULTS (W18.3 v1.3: Calibrated Baselines)")
    print("=" * 80)
    print(f"Tail estimate: mean + {TAIL_STDDEV_MULTIPLIER}*std_dev (conservative bound)")
    print(f"Tail/P50 ratio max: {TAIL_MEDIAN_RATIO_MAX}x")
    print("=" * 80)

    for name, data in results.items():
        status = data.get("status", "UNKNOWN")

        if status == "SKIP":
            print(f"\n{name}: SKIP - {data.get('reason', '')}")
            continue

        current_p50 = data.get("current_p50", 0)
        current_tail = data.get("current_tail")
        baseline_p50 = data.get("baseline_p50", 0)
        baseline_tail = data.get("baseline_tail", 0)
        unit = data.get("unit", "ns")
        tail_estimated = data.get("tail_estimated", False)

        # Status indicator
        indicator = {"PASS": "[PASS]", "REGRESSION": "[REGR]", "FAIL": "[FAIL]"}.get(status, "[????]")

        print(f"\n{indicator} {name}")
        print(f"    P50:  {current_p50:.2f} {unit} (baseline: {baseline_p50:.2f} {unit})")
        if current_tail is not None:
            est_marker = " (estimated)" if tail_estimated else ""
            print(f"    Tail: {current_tail:.2f} {unit} (baseline: {baseline_tail:.2f} {unit}){est_marker}")
            if current_p50 > 0:
                print(f"    Tail/P50 Ratio: {current_tail/current_p50:.2f}x")

        # Print check details
        for check in data.get("checks", []):
            check_indicator = "  [OK]" if check["passed"] else "  [!!]"
            print(f"    {check_indicator} {check['name']}: {check['reason']}")

    print("\n" + "=" * 80)


def generate_pr_comment(results: dict[str, Any], passed: bool) -> str:
    """Generate a markdown comment for PR with tail metrics."""
    lines = ["## Benchmark Validation Results (W18.3 v1.3)\n"]

    if passed:
        lines.append("All benchmarks within threshold.\n")
    else:
        lines.append("**Regression detected!** See details below.\n")

    # Summary table with tail
    lines.append("| Benchmark | P50 | Tail | P50 vs Baseline | Tail vs Baseline | Status |")
    lines.append("|:----------|----:|-----:|----------------:|-----------------:|:-------|")

    for name, data in results.items():
        status = data.get("status", "SKIP")
        if status == "SKIP":
            lines.append(f"| {name} | - | - | - | - | SKIP |")
            continue

        current_p50 = data.get("current_p50", 0)
        current_tail = data.get("current_tail")
        p50_ratio = data.get("p50_ratio", 0)
        tail_ratio = data.get("tail_ratio")
        unit = data.get("unit", "ns")

        p50_str = f"{current_p50:.2f} {unit}"
        tail_str = f"{current_tail:.2f} {unit}" if current_tail else "-"
        p50_ratio_str = f"{p50_ratio:.0%}" if p50_ratio else "-"
        tail_ratio_str = f"{tail_ratio:.0%}" if tail_ratio else "-"

        status_icon = {"PASS": "OK", "REGRESSION": "REGR", "FAIL": "FAIL"}.get(status, "??")

        lines.append(f"| {name} | {p50_str} | {tail_str} | {p50_ratio_str} | {tail_ratio_str} | {status_icon} |")

    lines.append("\n### Thresholds")
    lines.append(f"- P50 regression: >{P50_REGRESSION_THRESHOLD:.0%} of baseline")
    lines.append(f"- Tail regression: >{TAIL_REGRESSION_THRESHOLD:.0%} of baseline")
    lines.append(f"- Tail/P50 ratio: <{TAIL_MEDIAN_RATIO_MAX}x")
    lines.append(f"\n*Tail estimated as mean + {TAIL_STDDEV_MULTIPLIER}*std_dev (conservative bound)*")

    return "\n".join(lines)


# =============================================================================
# MAIN ENTRY POINT
# =============================================================================

def main() -> None:
    parser = argparse.ArgumentParser(description="Check for benchmark regressions (P50 and Tail)")
    parser.add_argument(
        "--baseline",
        type=Path,
        default=DEFAULT_BASELINE,
        help="Path to baselines.json",
    )
    parser.add_argument(
        "--results",
        type=Path,
        default=DEFAULT_RESULTS,
        help="Path to criterion results directory",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=P50_REGRESSION_THRESHOLD,
        help=f"P50 regression threshold multiplier (default: {P50_REGRESSION_THRESHOLD})",
    )
    parser.add_argument(
        "--pr-comment",
        action="store_true",
        help="Generate PR comment markdown",
    )
    parser.add_argument(
        "--quiet",
        action="store_true",
        help="Only print final status",
    )
    parser.add_argument(
        "--lenient-tail",
        action="store_true",
        help="Don't fail if tail extraction fails (skip instead)",
    )

    args = parser.parse_args()

    # Load baselines
    baseline = load_baselines(args.baseline)

    # Override threshold if specified in baselines
    threshold = baseline.get("thresholds", {}).get(
        "regression_multiplier", args.threshold
    )

    # Check for regressions
    strict_tail = not args.lenient_tail
    passed, results = check_regression(baseline, args.results, threshold, strict_tail)

    # Output results
    if args.pr_comment:
        print(generate_pr_comment(results, passed))
    elif not args.quiet:
        print_results(results)

    # Final status
    if passed:
        print("\nResult: PASS - All benchmarks within threshold (P50 and Tail)")
        sys.exit(0)
    else:
        print("\nResult: FAIL - Regression detected (P50 or Tail)")
        sys.exit(1)


if __name__ == "__main__":
    main()
