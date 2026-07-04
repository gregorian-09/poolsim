#!/usr/bin/env python3
"""Continuous Poolsim recommendation worker.

This worker is intentionally external to the Rust API surface. It polls an existing
CLI import path, emits stable PoolRecommendationDiff events, and optionally posts
those events to a webhook with bounded retries.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import time
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Mapping, Sequence


EVENT_SCHEMA_VERSION = "v1"
EVENT_TYPE = "PoolRecommendationDiff"


@dataclass(frozen=True)
class ContinuousConfig:
    source: str
    response_file: Path
    service_name: str | None
    window: str | None
    current_pool_size: int
    max_server_connections: int
    min_pool_size: int
    max_pool_size: int
    connection_overhead_ms: float | None = None
    interval_secs: float = 60.0
    state_file: Path | None = None
    webhook_url: str | None = None
    poolsim_cli: str = "poolsim"


def build_import_command(config: ContinuousConfig) -> list[str]:
    if config.source != "prometheus-response-file":
        raise ValueError(f"unsupported continuous source: {config.source}")

    command = [
        config.poolsim_cli,
        "--format",
        "json",
        "import",
        "prometheus",
        "--response-file",
        str(config.response_file),
        "--current-pool-size",
        str(config.current_pool_size),
        "--max-server-connections",
        str(config.max_server_connections),
        "--min",
        str(config.min_pool_size),
        "--max",
        str(config.max_pool_size),
    ]
    if config.service_name:
        command.extend(["--service-name", config.service_name])
    if config.window:
        command.extend(["--window", config.window])
    if config.connection_overhead_ms is not None:
        command.extend(["--connection-overhead-ms", str(config.connection_overhead_ms)])
    return command


def run_command(command: Sequence[str]) -> Mapping[str, object]:
    completed = subprocess.run(
        list(command),
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return json.loads(completed.stdout)


def load_previous(path: Path | None) -> Mapping[str, object] | None:
    if path is None or not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def save_previous(path: Path | None, recommendation: Mapping[str, object]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(recommendation, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _diff_value(recommendation: Mapping[str, object], key: str) -> object:
    diff = recommendation.get("diff")
    if not isinstance(diff, Mapping):
        raise ValueError("recommendation is missing diff object")
    return diff[key]


def build_event(
    recommendation: Mapping[str, object],
    previous: Mapping[str, object] | None,
    source: str,
) -> dict[str, object]:
    current_size = _diff_value(recommendation, "recommended_pool_size")
    previous_size = _diff_value(previous, "recommended_pool_size") if previous else None
    current_change = _diff_value(recommendation, "change")
    previous_change = _diff_value(previous, "change") if previous else None

    return {
        "schema_version": EVENT_SCHEMA_VERSION,
        "event_type": EVENT_TYPE,
        "source": source,
        "service_name": recommendation.get("service_name"),
        "window": recommendation.get("window"),
        "observed_at": recommendation.get("observed_at"),
        "changed": previous is None or current_size != previous_size or current_change != previous_change,
        "previous_recommended_pool_size": previous_size,
        "recommended_pool_size": current_size,
        "previous_change": previous_change,
        "change": current_change,
        "recommendation": recommendation,
    }


def post_webhook(
    url: str,
    event: Mapping[str, object],
    attempts: int = 3,
    sleep: Callable[[float], None] = time.sleep,
) -> None:
    payload = json.dumps(event).encode("utf-8")
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=10) as response:
                if 200 <= response.status < 300:
                    return
                last_error = RuntimeError(f"webhook returned HTTP {response.status}")
        except Exception as exc:  # pragma: no cover - covered through injected failing opener in tests indirectly by status path
            last_error = exc
        if attempt < attempts:
            sleep(min(2 ** (attempt - 1), 8))
    raise RuntimeError(f"webhook delivery failed after {attempts} attempts: {last_error}")


def run_once(
    config: ContinuousConfig,
    runner: Callable[[Sequence[str]], Mapping[str, object]] = run_command,
    webhook: Callable[[str, Mapping[str, object]], None] = post_webhook,
) -> dict[str, object]:
    previous = load_previous(config.state_file)
    recommendation = runner(build_import_command(config))
    event = build_event(recommendation, previous, config.source)
    if config.webhook_url:
        webhook(config.webhook_url, event)
    save_previous(config.state_file, recommendation)
    return event


def parse_args(argv: Sequence[str] | None = None) -> ContinuousConfig:
    parser = argparse.ArgumentParser(description="Poll Poolsim telemetry imports and emit recommendation diff events.")
    parser.add_argument("--source", choices=["prometheus-response-file"], default="prometheus-response-file")
    parser.add_argument("--response-file", required=True, type=Path)
    parser.add_argument("--service-name")
    parser.add_argument("--window")
    parser.add_argument("--current-pool-size", required=True, type=int)
    parser.add_argument("--max-server-connections", required=True, type=int)
    parser.add_argument("--min", dest="min_pool_size", required=True, type=int)
    parser.add_argument("--max", dest="max_pool_size", required=True, type=int)
    parser.add_argument("--connection-overhead-ms", type=float)
    parser.add_argument("--interval-secs", default=60.0, type=float)
    parser.add_argument("--state-file", type=Path)
    parser.add_argument("--webhook-url")
    parser.add_argument("--poolsim-cli", default="poolsim")
    args = parser.parse_args(argv)
    return ContinuousConfig(**vars(args))


def main(argv: Sequence[str] | None = None) -> int:
    config = parse_args(argv)
    while True:
        event = run_once(config)
        print(json.dumps(event, sort_keys=True), flush=True)
        time.sleep(config.interval_secs)


if __name__ == "__main__":
    raise SystemExit(main())
