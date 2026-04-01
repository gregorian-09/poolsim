#!/usr/bin/env python3
import argparse
import json
import re
import sys
from pathlib import Path


def parse_spec_ids(spec_text: str) -> list[str]:
    ids = re.findall(r"\|\s*((?:FR|CLI|WEB|NFR)-\d+)\s*\|", spec_text)
    return sorted(set(ids))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate spec-to-code traceability map completeness.")
    parser.add_argument("--spec", default="poolsim_specification.md", help="Path to markdown spec")
    parser.add_argument(
        "--map",
        default="tools/spec_traceability.json",
        help="Path to JSON traceability map"
    )
    args = parser.parse_args()

    spec_path = Path(args.spec)
    map_path = Path(args.map)
    spec_ids = parse_spec_ids(spec_path.read_text(encoding="utf-8"))
    trace = json.loads(map_path.read_text(encoding="utf-8"))

    map_ids = set(trace.keys())
    spec_set = set(spec_ids)

    missing = sorted(spec_set - map_ids)
    extra = sorted(map_ids - spec_set)

    incomplete: list[str] = []
    for req_id in spec_ids:
        entry = trace.get(req_id, {})
        if not entry.get("implemented", False):
            incomplete.append(f"{req_id}: implemented=false")
        if not entry.get("tested", False):
            incomplete.append(f"{req_id}: tested=false")
        if not entry.get("code_refs"):
            incomplete.append(f"{req_id}: missing code_refs")
        if not entry.get("test_refs"):
            incomplete.append(f"{req_id}: missing test_refs")

    if missing or extra or incomplete:
        print("Spec traceability validation failed.")
        if missing:
            print("Missing requirement IDs in map:")
            for req in missing:
                print(f"  - {req}")
        if extra:
            print("Extra IDs in map not present in spec:")
            for req in extra:
                print(f"  - {req}")
        if incomplete:
            print("Incomplete mapping entries:")
            for msg in incomplete:
                print(f"  - {msg}")
        return 1

    print(f"Spec traceability OK: {len(spec_ids)}/{len(spec_ids)} requirements mapped and marked implemented+tested.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
