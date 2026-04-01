#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path


def normalized_path(parts: list[str]) -> str:
    if parts and parts[0] == "/":
        return "/" + "/".join(parts[1:])
    return "/".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate tarpaulin coverage thresholds.")
    parser.add_argument("--report", default="coverage/tarpaulin-report.json", help="Tarpaulin JSON report path")
    parser.add_argument("--core-min", type=float, default=80.0, help="Minimum coverage for poolsim-core src")
    parser.add_argument("--overall-min", type=float, default=0.0, help="Optional global minimum coverage")
    args = parser.parse_args()

    report_path = Path(args.report)
    data = json.loads(report_path.read_text(encoding="utf-8"))
    entries = data.get("files", [])

    total_cov = total_coverable = 0
    core_cov = core_coverable = 0

    for entry in entries:
        cov = int(entry.get("covered", 0))
        coverable = int(entry.get("coverable", 0))
        path = normalized_path(entry.get("path", []))
        total_cov += cov
        total_coverable += coverable
        if "/crates/poolsim-core/src/" in path:
            core_cov += cov
            core_coverable += coverable

    overall_pct = (100.0 * total_cov / total_coverable) if total_coverable else 0.0
    core_pct = (100.0 * core_cov / core_coverable) if core_coverable else 0.0

    print(
        f"Coverage summary: overall={overall_pct:.2f}% ({total_cov}/{total_coverable}), "
        f"poolsim-core/src={core_pct:.2f}% ({core_cov}/{core_coverable})"
    )

    failures = []
    if core_pct < args.core_min:
        failures.append(f"poolsim-core/src coverage {core_pct:.2f}% is below {args.core_min:.2f}%")
    if overall_pct < args.overall_min:
        failures.append(f"overall coverage {overall_pct:.2f}% is below {args.overall_min:.2f}%")

    if failures:
        print("Coverage threshold validation failed:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print("Coverage thresholds satisfied.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
