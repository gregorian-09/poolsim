#!/usr/bin/env python3
"""Create anonymized, opt-in deployed-pool survey payloads."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Iterable, Mapping


SCHEMA_VERSION = "v1"
ALLOWED_FIELDS = {
    "framework",
    "database",
    "pool_size",
    "min_pool_size",
    "max_pool_size",
    "replicas",
    "rps_band",
    "saturation",
    "uses_proxy",
    "environment_class",
}
FORBIDDEN_HINTS = ("host", "hostname", "url", "uri", "dsn", "password", "secret", "token", "query", "sql", "service_name")


class SurveyError(RuntimeError):
    """Raised when a survey payload cannot be produced safely."""


def reject_forbidden_keys(entry: Mapping[str, object]) -> None:
    for key in entry:
        lowered = key.lower()
        if any(hint in lowered for hint in FORBIDDEN_HINTS):
            raise SurveyError(f"field {key!r} may contain identifying or sensitive data")


def sanitize_entry(entry: Mapping[str, object]) -> dict[str, object]:
    reject_forbidden_keys(entry)
    sanitized = {key: entry[key] for key in ALLOWED_FIELDS if key in entry}
    if "pool_size" not in sanitized:
        raise SurveyError("each survey entry must include pool_size")
    return dict(sorted(sanitized.items()))


def build_payload(entries: Iterable[Mapping[str, object]], consent: bool) -> dict[str, object]:
    if not consent:
        raise SurveyError("survey export requires explicit consent")
    sanitized = [sanitize_entry(entry) for entry in entries]
    return {
        "schema_version": SCHEMA_VERSION,
        "consent": True,
        "contains_application_data": False,
        "entries": sanitized,
    }


def load_entries(path: Path) -> list[Mapping[str, object]]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(raw, dict) and isinstance(raw.get("entries"), list):
        return raw["entries"]
    if isinstance(raw, list):
        return raw
    raise SurveyError("survey input must be a list or an object with an entries list")


def main() -> int:
    parser = argparse.ArgumentParser(description="Create an anonymized Poolsim deployed-pool survey payload.")
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--consent", action="store_true", help="required explicit opt-in consent")
    args = parser.parse_args()

    payload = build_payload(load_entries(args.input), args.consent)
    rendered = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
