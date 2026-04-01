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
    parser = argparse.ArgumentParser(description="Validate that example files are fully covered.")
    parser.add_argument(
        "--report",
        default="coverage/examples/tarpaulin-report.json",
        help="Tarpaulin JSON report path",
    )
    args = parser.parse_args()

    report_path = Path(args.report)
    if not report_path.exists():
        print(f"coverage report not found: {report_path}")
        return 1

    data = json.loads(report_path.read_text(encoding="utf-8"))
    entries = data.get("files", [])

    example_rows: list[tuple[str, int, int]] = []
    for entry in entries:
        path = normalized_path(entry.get("path", []))
        if "/examples/" not in path:
            continue
        covered = int(entry.get("covered", 0))
        coverable = int(entry.get("coverable", 0))
        example_rows.append((path, covered, coverable))

    if not example_rows:
        print("no example files were found in the coverage report")
        return 1

    failures: list[str] = []
    for path, covered, coverable in sorted(example_rows):
        pct = (100.0 * covered / coverable) if coverable else 0.0
        print(f"example coverage: {path} => {pct:.2f}% ({covered}/{coverable})")
        if coverable == 0:
            failures.append(f"{path} has zero coverable lines")
        elif covered != coverable:
            failures.append(f"{path} is not fully covered ({covered}/{coverable})")

    if failures:
        print("example coverage validation failed:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print("example coverage validation passed (all example files are 100% covered).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
