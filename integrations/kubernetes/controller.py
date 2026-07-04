#!/usr/bin/env python3
"""Kubernetes controller for Poolsim recommendation annotations.

The controller uses the in-cluster Kubernetes API and the stable `poolsim` CLI.
It does not modify application runtime pool settings. It writes recommendation
metadata back to deployment annotations so platform teams can observe drift and
wire alerts or policy around it.
"""

from __future__ import annotations

import json
import os
import ssl
import subprocess
import time
import urllib.request
from dataclasses import dataclass
from typing import Callable, Iterable, Mapping, Sequence


DEFAULT_API = "https://kubernetes.default.svc"
SERVICE_ACCOUNT_TOKEN = "/var/run/secrets/kubernetes.io/serviceaccount/token"
SERVICE_ACCOUNT_NAMESPACE = "/var/run/secrets/kubernetes.io/serviceaccount/namespace"


@dataclass(frozen=True)
class ControllerConfig:
    namespace: str
    api_server: str = DEFAULT_API
    token: str | None = None
    interval_secs: float = 60.0
    poolsim_cli: str = "poolsim"
    apply: bool = False


def read_service_account_namespace(path: str = SERVICE_ACCOUNT_NAMESPACE) -> str:
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read().strip() or "default"
    except FileNotFoundError:
        return "default"


def read_service_account_token(path: str = SERVICE_ACCOUNT_TOKEN) -> str | None:
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read().strip() or None
    except FileNotFoundError:
        return None


def config_from_env(env: Mapping[str, str] = os.environ) -> ControllerConfig:
    return ControllerConfig(
        namespace=env.get("POOLSIM_K8S_NAMESPACE") or read_service_account_namespace(),
        api_server=env.get("POOLSIM_K8S_API", DEFAULT_API),
        token=env.get("POOLSIM_K8S_TOKEN") or read_service_account_token(),
        interval_secs=float(env.get("POOLSIM_K8S_INTERVAL_SECS", "60")),
        poolsim_cli=env.get("POOLSIM_CLI", "poolsim"),
        apply=env.get("POOLSIM_K8S_APPLY", "false").lower() in {"1", "true", "yes"},
    )


def annotation(deployment: Mapping[str, object], key: str) -> str | None:
    metadata = deployment.get("metadata")
    if not isinstance(metadata, Mapping):
        return None
    annotations = metadata.get("annotations")
    if not isinstance(annotations, Mapping):
        return None
    value = annotations.get(key)
    return str(value) if value is not None else None


def name_of(deployment: Mapping[str, object]) -> str:
    metadata = deployment.get("metadata")
    if isinstance(metadata, Mapping) and metadata.get("name"):
        return str(metadata["name"])
    return "unknown"


def build_command(deployment: Mapping[str, object], poolsim_cli: str = "poolsim") -> list[str] | None:
    required = {
        "--rps": annotation(deployment, "poolsim.io/expected-rps"),
        "--p50": annotation(deployment, "poolsim.io/latency-p50-ms"),
        "--p95": annotation(deployment, "poolsim.io/latency-p95-ms"),
        "--p99": annotation(deployment, "poolsim.io/latency-p99-ms"),
        "--max-server-connections": annotation(deployment, "poolsim.io/max-server-connections"),
        "--min": annotation(deployment, "poolsim.io/min-pool-size"),
        "--max": annotation(deployment, "poolsim.io/max-pool-size"),
    }
    if any(value is None for value in required.values()):
        return None

    command = [poolsim_cli, "--format", "json", "simulate"]
    for flag, value in required.items():
        command.extend([flag, str(value)])

    if overhead := annotation(deployment, "poolsim.io/connection-overhead-ms"):
        command.extend(["--connection-overhead-ms", overhead])
    if iterations := annotation(deployment, "poolsim.io/iterations"):
        command.extend(["--iterations", iterations])
    return command


def run_poolsim(command: Sequence[str]) -> Mapping[str, object]:
    completed = subprocess.run(
        list(command),
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return json.loads(completed.stdout)


def recommendation_annotations(report: Mapping[str, object]) -> dict[str, str]:
    return {
        "poolsim.io/recommended-pool-size": str(report["optimal_pool_size"]),
        "poolsim.io/recommended-rho": str(report["utilisation_rho"]),
        "poolsim.io/recommended-p99-queue-wait-ms": str(report["p99_queue_wait_ms"]),
        "poolsim.io/recommended-saturation": str(report["saturation"]),
    }


def request_json(config: ControllerConfig, method: str, path: str, body: Mapping[str, object] | None = None) -> Mapping[str, object]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Accept": "application/json"}
    if body is not None:
        headers["Content-Type"] = "application/merge-patch+json"
    if config.token:
        headers["Authorization"] = f"Bearer {config.token}"
    request = urllib.request.Request(
        f"{config.api_server}{path}",
        data=data,
        headers=headers,
        method=method,
    )
    context = ssl.create_default_context()
    with urllib.request.urlopen(request, context=context, timeout=15) as response:
        if response.status == 204:
            return {}
        return json.loads(response.read().decode("utf-8"))


def list_deployments(config: ControllerConfig) -> Iterable[Mapping[str, object]]:
    payload = request_json(config, "GET", f"/apis/apps/v1/namespaces/{config.namespace}/deployments")
    items = payload.get("items", [])
    return items if isinstance(items, list) else []


def patch_deployment(config: ControllerConfig, deployment: Mapping[str, object], annotations: Mapping[str, str]) -> None:
    body = {"metadata": {"annotations": dict(annotations)}}
    request_json(
        config,
        "PATCH",
        f"/apis/apps/v1/namespaces/{config.namespace}/deployments/{name_of(deployment)}",
        body,
    )


def reconcile_once(
    config: ControllerConfig,
    deployments: Iterable[Mapping[str, object]],
    runner: Callable[[Sequence[str]], Mapping[str, object]] = run_poolsim,
    patcher: Callable[[ControllerConfig, Mapping[str, object], Mapping[str, str]], None] = patch_deployment,
) -> list[dict[str, object]]:
    events = []
    for deployment in deployments:
        command = build_command(deployment, config.poolsim_cli)
        if command is None:
            continue
        report = runner(command)
        annotations = recommendation_annotations(report)
        if config.apply:
            patcher(config, deployment, annotations)
        events.append({"deployment": name_of(deployment), "annotations": annotations, "applied": config.apply})
    return events


def run_forever(config: ControllerConfig) -> None:
    while True:
        events = reconcile_once(config, list_deployments(config))
        for event in events:
            print(json.dumps(event, sort_keys=True), flush=True)
        time.sleep(config.interval_secs)


def main() -> int:
    run_forever(config_from_env())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
