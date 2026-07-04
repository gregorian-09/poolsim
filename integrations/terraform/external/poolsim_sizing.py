#!/usr/bin/env python3
"""Terraform/OpenTofu external data adapter for Poolsim sizing.

The external provider protocol passes a JSON query object on stdin and expects a
flat JSON object with string values on stdout.
"""

from __future__ import annotations

import json
import subprocess
import sys
from typing import Any


def main() -> int:
    try:
        query = json.load(sys.stdin)
        result = run_query(query)
        print(json.dumps(result, sort_keys=True))
        return 0
    except Exception as exc:  # Terraform surfaces stderr when the program fails.
        print(str(exc), file=sys.stderr)
        return 1


def run_query(query: dict[str, Any]) -> dict[str, str]:
    executable = str(query.get("poolsim_executable") or "poolsim")
    command = str(query.get("command") or "simulate")
    config = required(query, "config")

    args = [executable, "--format", "json"]
    if command == "simulate":
        args.extend(["simulate", "--config", config])
    elif command == "evaluate":
        args.extend(["evaluate", "--config", config, "--pool-size", required(query, "pool_size")])
    else:
        raise ValueError("command must be simulate or evaluate")

    proc = subprocess.run(args, check=False, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or f"poolsim exited with {proc.returncode}")

    payload = json.loads(proc.stdout)
    return flatten_payload(payload)


def required(query: dict[str, Any], key: str) -> str:
    value = query.get(key)
    if value is None or value == "":
        raise ValueError(f"missing required query field: {key}")
    return str(value)


def flatten_payload(payload: dict[str, Any]) -> dict[str, str]:
    fields = {
        "optimal_pool_size": payload.get("optimal_pool_size"),
        "pool_size": payload.get("pool_size"),
        "confidence_interval_min": sequence_value(payload.get("confidence_interval"), 0),
        "confidence_interval_max": sequence_value(payload.get("confidence_interval"), 1),
        "cold_start_min_pool_size": payload.get("cold_start_min_pool_size"),
        "utilisation_rho": payload.get("utilisation_rho"),
        "mean_queue_wait_ms": payload.get("mean_queue_wait_ms"),
        "p99_queue_wait_ms": payload.get("p99_queue_wait_ms"),
        "saturation": payload.get("saturation"),
    }
    out = {key: stringify(value) for key, value in fields.items() if value is not None}
    out["raw_json"] = json.dumps(payload, sort_keys=True)
    return out


def sequence_value(value: Any, index: int) -> Any:
    if isinstance(value, list) and len(value) > index:
        return value[index]
    return None


def stringify(value: Any) -> str:
    if isinstance(value, float):
        return format(value, ".12g")
    return str(value)


if __name__ == "__main__":
    raise SystemExit(main())
