#!/usr/bin/env python3
"""
Benchmark Regression Detection Script

Task: W10.8 - Create Benchmark Validation Suite

This script compares criterion benchmark results against baselines and
detects performance regressions.

Usage:
    python benches/check_regression.py [--baseline baselines.json] [--results target/criterion]

Exit codes:
    0 - All benchmarks within threshold
    1 - Regression detected (>10% slower than baseline)
    2 - Error (missing files, parse errors)
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, Optional, Tuple

# Default paths
DEFAULT_BASELINE = Path(__file__).parent / "baselines.json"
DEFAULT_RESULTS = Path(__file__).parent.parent / "target" / "criterion"


def load_baselines(path: Path) -> Dict:
    """Load baseline values from JSON file."""
    if not path.exists():
        print(f"ERROR: Baseline file not found: {path}")
        sys.exit(2)

    with open(path) as f:
        return json.load(f)


def find_criterion_estimate(benchmark_dir: Path) -> Optional[float]:
    """
    Find the median estimate from criterion's estimates.json.

    Criterion stores results in:
    target/criterion/<group>/<benchmark>/new/estimates.json
    """
    estimates_path = benchmark_dir / "new" / "estimates.json"
    if not estimates_path.exists():
        # Try base.json for comparison runs
        estimates_path = benchmark_dir / "base" / "estimates.json"

    if not estimates_path.exists():
        return None

    with open(estimates_path) as f:
        data = json.load(f)

    # Criterion stores median in nanoseconds
    if "median" in data and "point_estimate" in data["median"]:
        return data["median"]["point_estimate"]

    return None


def convert_to_baseline_unit(value_ns: float, unit: str) -> float:
    """Convert nanoseconds to the baseline's unit."""
    if unit == "ns":
        return value_ns
    elif unit == "us":
        return value_ns / 1000.0
    elif unit == "ms":
        return value_ns / 1_000_000.0
    elif unit == "s":
        return value_ns / 1_000_000_000.0
    else:
        return value_ns


def check_regression(
    baseline: Dict, results_dir: Path, threshold: float = 1.1
) -> Tuple[bool, Dict]:
    """
    Check for regressions against baselines.

    Returns:
        (passed, results_dict)
    """
    results = {}
    all_passed = True

    benchmarks = baseline.get("benchmarks", {})

    for name, config in benchmarks.items():
        # Look for benchmark in criterion output
        # Criterion uses format: <group>/<benchmark>
        benchmark_path = results_dir / "validation" / name

        if not benchmark_path.exists():
            results[name] = {
                "status": "SKIP",
                "reason": "No results found",
            }
            continue

        current_ns = find_criterion_estimate(benchmark_path)
        if current_ns is None:
            results[name] = {
                "status": "SKIP",
                "reason": "Could not parse estimates.json",
            }
            continue

        unit = config.get("unit", "ns")
        current = convert_to_baseline_unit(current_ns, unit)
        baseline_p50 = config.get("p50", 0)
        baseline_p99 = config.get("p99", 0)
        hard_limit = config.get("hard_limit", float("inf"))

        # Calculate regression
        if baseline_p50 > 0:
            ratio = current / baseline_p50
        else:
            ratio = 0

        # Determine status
        if current > hard_limit:
            status = "FAIL"
            reason = f"Exceeds hard limit ({current:.2f} > {hard_limit:.2f} {unit})"
            all_passed = False
        elif ratio > threshold:
            status = "REGRESSION"
            reason = f"{ratio:.1%} of baseline (threshold: {threshold:.0%})"
            all_passed = False
        else:
            status = "PASS"
            reason = f"{ratio:.1%} of baseline"

        results[name] = {
            "status": status,
            "current": current,
            "baseline_p50": baseline_p50,
            "baseline_p99": baseline_p99,
            "ratio": ratio,
            "unit": unit,
            "reason": reason,
        }

    return all_passed, results


def print_results(results: Dict, verbose: bool = True):
    """Print results in a formatted table."""
    print("\n" + "=" * 70)
    print("BENCHMARK VALIDATION RESULTS")
    print("=" * 70)

    for name, data in results.items():
        status = data.get("status", "UNKNOWN")

        if status == "SKIP":
            print(f"\n{name}: SKIP - {data.get('reason', '')}")
            continue

        current = data.get("current", 0)
        baseline = data.get("baseline_p50", 0)
        unit = data.get("unit", "ns")
        reason = data.get("reason", "")

        # Status emoji
        if status == "PASS":
            emoji = "[PASS]"
        elif status == "REGRESSION":
            emoji = "[REGR]"
        else:
            emoji = "[FAIL]"

        print(f"\n{emoji} {name}")
        print(f"    Current:  {current:.2f} {unit}")
        print(f"    Baseline: {baseline:.2f} {unit} (P50)")
        print(f"    Result:   {reason}")

    print("\n" + "=" * 70)


def generate_pr_comment(results: Dict, passed: bool) -> str:
    """Generate a markdown comment for PR."""
    lines = ["## Benchmark Validation Results\n"]

    if passed:
        lines.append("All benchmarks within threshold.\n")
    else:
        lines.append("**Regression detected!** See details below.\n")

    lines.append("| Benchmark | Current | Baseline | Status |")
    lines.append("|:----------|--------:|---------:|:-------|")

    for name, data in results.items():
        status = data.get("status", "SKIP")
        if status == "SKIP":
            lines.append(f"| {name} | - | - | SKIP |")
            continue

        current = data.get("current", 0)
        baseline = data.get("baseline_p50", 0)
        unit = data.get("unit", "ns")

        status_icon = {"PASS": "", "REGRESSION": "", "FAIL": ""}.get(status, "")

        lines.append(
            f"| {name} | {current:.2f} {unit} | {baseline:.2f} {unit} | {status_icon} {status} |"
        )

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Check for benchmark regressions")
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
        default=1.1,
        help="Regression threshold multiplier (default: 1.1 = 10%%)",
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

    args = parser.parse_args()

    # Load baselines
    baseline = load_baselines(args.baseline)

    # Override threshold if specified in baselines
    threshold = baseline.get("thresholds", {}).get(
        "regression_multiplier", args.threshold
    )

    # Check for regressions
    passed, results = check_regression(baseline, args.results, threshold)

    # Output results
    if args.pr_comment:
        print(generate_pr_comment(results, passed))
    elif not args.quiet:
        print_results(results)

    # Final status
    if passed:
        print("\nResult: PASS - All benchmarks within threshold")
        sys.exit(0)
    else:
        print("\nResult: FAIL - Regression detected")
        sys.exit(1)


if __name__ == "__main__":
    main()
