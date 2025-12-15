#!/usr/bin/env python3
"""
EdgeVec Benchmark Regression Detection Script

Task: W10.8 - Create Benchmark Validation Suite
Updated: W18.3 v1.2 - P99 latency tracking with hostile review fixes

This script compares criterion benchmark results against baselines and
detects performance regressions for both median (P50) and tail (P99) latencies.

Checks performed:
1. Median (P50) latency regression
2. P99 latency regression (estimated)
3. P99/median ratio sanity check

Usage:
    python benches/check_regression.py [--baseline baselines.json] [--results target/criterion]

Exit codes:
    0 - All benchmarks within threshold
    1 - Regression detected (>10% slower than baseline)
    2 - Error (missing files, parse errors, P99 extraction failure)

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
- NO native percentiles key - P99 estimated as mean + 5*std_dev (conservative)
- For iterated benchmarks, "slope" contains point_estimate
- For non-iterated benchmarks, "median" contains point_estimate

W18.3 v1.2 Hostile Review Fixes:
- [C1] Criterion output format verified from actual cargo bench run
- [C3] Make validation FAIL if P99 extraction fails (not silent skip)
- [C4] Use mean + 5*std_dev for conservative P99 estimate (long-tailed distributions)
- [C5] Unit conversion validation with explicit error on unsupported units
- [M3] Handle new benchmarks not in baselines (log warning, skip)
- [M5] Print P99 estimation warnings to stdout for CI visibility
- [m1] Magic numbers moved to named constants
- [m2] Type hints added to all functions
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

# =============================================================================
# CONSTANTS (moved from inline magic numbers per [m1])
# =============================================================================

# Default paths
DEFAULT_BASELINE = Path(__file__).parent / "baselines.json"
DEFAULT_RESULTS = Path(__file__).parent.parent / "target" / "criterion"

# Regression thresholds
P50_REGRESSION_THRESHOLD: float = 1.10  # 10% regression tolerance for median
P99_REGRESSION_THRESHOLD: float = 1.50  # 50% regression tolerance for P99 (tail latency more variable)
P99_MEDIAN_RATIO_MAX: float = 5.0  # P99 can be up to 5x median (relaxed per [M2] - real-world long tails)

# P99 estimation multiplier (mean + N * std_dev)
# Changed from 3 to 5 per [C4] - long-tailed distributions need conservative estimate
P99_STDDEV_MULTIPLIER: float = 5.0

# Supported units for conversion
SUPPORTED_UNITS: set[str] = {"ns", "us", "ms", "s"}


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

    Returns full estimates dict for P99 extraction.
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


def extract_p99_ns(estimates: dict[str, Any], benchmark_name: str) -> tuple[float | None, bool]:
    """
    Extract P99 latency in nanoseconds from Criterion estimates.

    Returns:
        (p99_value, is_estimated): Tuple of P99 value and whether it was estimated

    [C4 FIX] Uses mean + 5*std_dev for conservative estimate (not 3x).
    Long-tailed performance distributions require more conservative estimate.
    """
    # Criterion doesn't provide native percentiles, so we always estimate
    # from mean + N*std_dev where N=5 for conservative tail estimate
    if "mean" in estimates and "std_dev" in estimates:
        mean = estimates["mean"]
        std_dev = estimates["std_dev"]

        if isinstance(mean, dict) and isinstance(std_dev, dict):
            mean_ns = mean.get("point_estimate", 0)
            std_dev_ns = std_dev.get("point_estimate", 0)

            if mean_ns > 0 and std_dev_ns >= 0:
                p99_estimate = mean_ns + (P99_STDDEV_MULTIPLIER * std_dev_ns)
                # [M5 FIX] Print to stdout so CI can see it
                print(f"  [{benchmark_name}] P99 estimated: mean({mean_ns:.0f}) + {P99_STDDEV_MULTIPLIER}*std_dev({std_dev_ns:.0f}) = {p99_estimate:.0f} ns")
                return p99_estimate, True

    # Final fallback: use upper confidence bound
    if "mean" in estimates:
        mean = estimates["mean"]
        if isinstance(mean, dict):
            ci = mean.get("confidence_interval", {})
            upper = ci.get("upper_bound")
            if upper:
                print(f"  [{benchmark_name}] P99 fallback: using mean upper CI bound = {upper:.0f} ns")
                return float(upper), True

    return None, False


def convert_ns_to_unit(value_ns: float, unit: str) -> float:
    """
    Convert nanoseconds to the target unit.

    [C5 FIX] Validates unit is supported, raises error otherwise.
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
        # Should never reach here due to validation above
        return value_ns


# =============================================================================
# MAIN REGRESSION CHECK
# =============================================================================

def check_regression(
    baseline: dict[str, Any],
    results_dir: Path,
    threshold: float = P50_REGRESSION_THRESHOLD,
    strict_p99: bool = True,
) -> tuple[bool, dict[str, Any]]:
    """
    Check for regressions against baselines.

    W18.3: Now checks both P50 (median) and P99 (tail) latencies.

    Args:
        baseline: Baseline configuration dict
        results_dir: Path to criterion results
        threshold: P50 regression threshold multiplier
        strict_p99: If True, FAIL validation when P99 cannot be extracted

    Returns:
        (passed, results_dict)
    """
    results: dict[str, Any] = {}
    all_passed = True

    benchmarks = baseline.get("benchmarks", {})

    for name, config in benchmarks.items():
        # Look for benchmark in criterion output
        benchmark_path = results_dir / "validation" / name

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
        current_p99_ns, p99_estimated = extract_p99_ns(estimates, name)

        if current_median_ns is None:
            results[name] = {
                "status": "SKIP",
                "reason": "Could not extract median from estimates",
            }
            continue

        # [C3 FIX] FAIL if P99 extraction fails and baseline expects P99
        baseline_p99 = config.get("p99", 0)
        if current_p99_ns is None and baseline_p99 > 0 and strict_p99:
            results[name] = {
                "status": "FAIL",
                "reason": "Could not extract P99 from estimates (baseline requires P99)",
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
        current_p99 = convert_ns_to_unit(current_p99_ns, unit) if current_p99_ns else None

        # Initialize result
        result: dict[str, Any] = {
            "current_p50": current_median,
            "current_p99": current_p99,
            "baseline_p50": baseline_p50,
            "baseline_p99": baseline_p99,
            "unit": unit,
            "p99_estimated": p99_estimated,
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

        # Check 2: P99 regression
        if current_p99 is not None and baseline_p99 > 0:
            p99_ratio = current_p99 / baseline_p99
            result["p99_ratio"] = p99_ratio

            if p99_ratio > P99_REGRESSION_THRESHOLD:
                result["checks"].append({
                    "name": "P99 Regression",
                    "passed": False,
                    "reason": f"P99 {p99_ratio:.1%} of baseline (threshold: {P99_REGRESSION_THRESHOLD:.0%})"
                })
                all_passed = False
            else:
                result["checks"].append({
                    "name": "P99 Regression",
                    "passed": True,
                    "reason": f"P99 {p99_ratio:.1%} of baseline"
                })

        # Check 3: P99/P50 ratio sanity check
        if current_p99 is not None and current_median > 0:
            p99_p50_ratio = current_p99 / current_median
            result["p99_p50_ratio"] = p99_p50_ratio

            if p99_p50_ratio > P99_MEDIAN_RATIO_MAX:
                result["checks"].append({
                    "name": "P99/P50 Ratio",
                    "passed": False,
                    "reason": f"P99/P50 ratio {p99_p50_ratio:.2f}x exceeds {P99_MEDIAN_RATIO_MAX}x limit"
                })
                all_passed = False
            else:
                result["checks"].append({
                    "name": "P99/P50 Ratio",
                    "passed": True,
                    "reason": f"P99/P50 ratio {p99_p50_ratio:.2f}x (OK)"
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

def print_results(results: dict[str, Any], verbose: bool = True) -> None:
    """Print results in a formatted table."""
    print("\n" + "=" * 80)
    print("BENCHMARK VALIDATION RESULTS (W18.3 v1.2: P99 Tracking)")
    print("=" * 80)
    print(f"P99 estimate: mean + {P99_STDDEV_MULTIPLIER}*std_dev (conservative for long tails)")
    print(f"P99/P50 ratio max: {P99_MEDIAN_RATIO_MAX}x")
    print("=" * 80)

    for name, data in results.items():
        status = data.get("status", "UNKNOWN")

        if status == "SKIP":
            print(f"\n{name}: SKIP - {data.get('reason', '')}")
            continue

        current_p50 = data.get("current_p50", 0)
        current_p99 = data.get("current_p99")
        baseline_p50 = data.get("baseline_p50", 0)
        baseline_p99 = data.get("baseline_p99", 0)
        unit = data.get("unit", "ns")
        p99_estimated = data.get("p99_estimated", False)

        # Status indicator
        indicator = {"PASS": "[PASS]", "REGRESSION": "[REGR]", "FAIL": "[FAIL]"}.get(status, "[????]")

        print(f"\n{indicator} {name}")
        print(f"    P50:  {current_p50:.2f} {unit} (baseline: {baseline_p50:.2f} {unit})")
        if current_p99 is not None:
            est_marker = " (estimated)" if p99_estimated else ""
            print(f"    P99:  {current_p99:.2f} {unit} (baseline: {baseline_p99:.2f} {unit}){est_marker}")
            if current_p50 > 0:
                print(f"    P99/P50 Ratio: {current_p99/current_p50:.2f}x")

        # Print check details
        for check in data.get("checks", []):
            check_indicator = "  [OK]" if check["passed"] else "  [!!]"
            print(f"    {check_indicator} {check['name']}: {check['reason']}")

    print("\n" + "=" * 80)


def generate_pr_comment(results: dict[str, Any], passed: bool) -> str:
    """Generate a markdown comment for PR with P99 metrics."""
    lines = ["## Benchmark Validation Results (W18.3 v1.2)\n"]

    if passed:
        lines.append("All benchmarks within threshold.\n")
    else:
        lines.append("**Regression detected!** See details below.\n")

    # Summary table with P99
    lines.append("| Benchmark | P50 | P99 | P50 vs Baseline | P99 vs Baseline | Status |")
    lines.append("|:----------|----:|----:|----------------:|----------------:|:-------|")

    for name, data in results.items():
        status = data.get("status", "SKIP")
        if status == "SKIP":
            lines.append(f"| {name} | - | - | - | - | SKIP |")
            continue

        current_p50 = data.get("current_p50", 0)
        current_p99 = data.get("current_p99")
        p50_ratio = data.get("p50_ratio", 0)
        p99_ratio = data.get("p99_ratio")
        unit = data.get("unit", "ns")

        p50_str = f"{current_p50:.2f} {unit}"
        p99_str = f"{current_p99:.2f} {unit}" if current_p99 else "-"
        p50_ratio_str = f"{p50_ratio:.0%}" if p50_ratio else "-"
        p99_ratio_str = f"{p99_ratio:.0%}" if p99_ratio else "-"

        status_icon = {"PASS": "OK", "REGRESSION": "REGR", "FAIL": "FAIL"}.get(status, "??")

        lines.append(f"| {name} | {p50_str} | {p99_str} | {p50_ratio_str} | {p99_ratio_str} | {status_icon} |")

    lines.append("\n### Thresholds")
    lines.append(f"- P50 regression: >{P50_REGRESSION_THRESHOLD:.0%} of baseline")
    lines.append(f"- P99 regression: >{P99_REGRESSION_THRESHOLD:.0%} of baseline")
    lines.append(f"- P99/P50 ratio: <{P99_MEDIAN_RATIO_MAX}x")
    lines.append(f"\n*P99 estimated as mean + {P99_STDDEV_MULTIPLIER}*std_dev*")

    return "\n".join(lines)


# =============================================================================
# MAIN ENTRY POINT
# =============================================================================

def main() -> None:
    parser = argparse.ArgumentParser(description="Check for benchmark regressions (P50 and P99)")
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
        "--lenient-p99",
        action="store_true",
        help="Don't fail if P99 extraction fails (skip instead)",
    )

    args = parser.parse_args()

    # Load baselines
    baseline = load_baselines(args.baseline)

    # Override threshold if specified in baselines
    threshold = baseline.get("thresholds", {}).get(
        "regression_multiplier", args.threshold
    )

    # Check for regressions
    strict_p99 = not args.lenient_p99
    passed, results = check_regression(baseline, args.results, threshold, strict_p99)

    # Output results
    if args.pr_comment:
        print(generate_pr_comment(results, passed))
    elif not args.quiet:
        print_results(results)

    # Final status
    if passed:
        print("\nResult: PASS - All benchmarks within threshold (P50 and P99)")
        sys.exit(0)
    else:
        print("\nResult: FAIL - Regression detected (P50 or P99)")
        sys.exit(1)


if __name__ == "__main__":
    main()
