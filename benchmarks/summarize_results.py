#!/usr/bin/env python3
"""Summarize Poolsim sizing benchmark results.

The benchmark harness writes JSON rows for real pool runs. This summarizer keeps the
published comparison math testable without requiring a database during unit tests.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Iterable, Mapping


def percent_error(predicted: float, actual: float) -> float:
    if actual == 0:
        return 0.0 if predicted == 0 else float("inf")
    return abs(predicted - actual) / actual * 100.0


def summarize(rows: Iterable[Mapping[str, float | int | str]]) -> dict[str, object]:
    details = []
    for row in rows:
        predicted = float(row["poolsim_predicted_p99_queue_wait_ms"])
        actual = float(row["observed_p99_queue_wait_ms"])
        details.append(
            {
                "target": row["target"],
                "framework": row["framework"],
                "recommended_pool_size": int(row["recommended_pool_size"]),
                "actual_pool_size": int(row["actual_pool_size"]),
                "predicted_p99_queue_wait_ms": predicted,
                "observed_p99_queue_wait_ms": actual,
                "p99_queue_wait_percent_error": percent_error(predicted, actual),
            }
        )

    mean_error = sum(item["p99_queue_wait_percent_error"] for item in details) / len(details) if details else 0.0
    return {
        "benchmark_count": len(details),
        "mean_p99_queue_wait_percent_error": mean_error,
        "results": details,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Summarize Poolsim benchmark result JSON.")
    parser.add_argument("input", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    rows = json.loads(args.input.read_text(encoding="utf-8"))
    report = summarize(rows)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
