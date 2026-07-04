#!/usr/bin/env python3
"""Expose Poolsim sizing recommendations as Prometheus metrics in Kubernetes.

The sidecar intentionally delegates sizing to the stable `poolsim` CLI contract instead
of reimplementing formulas. Kubernetes annotations can be projected into the process
environment with the Downward API; see `deployment.yaml` in this directory.
"""

from __future__ import annotations

import json
import os
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Callable, Mapping, Sequence


DEFAULT_PORT = 9464


class SidecarError(RuntimeError):
    """Raised when the sidecar cannot produce a recommendation."""


def required(env: Mapping[str, str], key: str) -> str:
    value = env.get(key, "").strip()
    if not value:
        raise SidecarError(f"missing required environment variable {key}")
    return value


def optional(env: Mapping[str, str], key: str) -> str | None:
    value = env.get(key, "").strip()
    return value or None


def build_simulate_command(env: Mapping[str, str]) -> list[str]:
    """Build a `poolsim simulate` command from sidecar environment variables."""
    binary = env.get("POOLSIM_CLI", "poolsim")
    cmd = [
        binary,
        "--format",
        "json",
        "simulate",
        "--rps",
        required(env, "POOLSIM_EXPECTED_RPS"),
        "--p50",
        required(env, "POOLSIM_LATENCY_P50_MS"),
        "--p95",
        required(env, "POOLSIM_LATENCY_P95_MS"),
        "--p99",
        required(env, "POOLSIM_LATENCY_P99_MS"),
        "--max-server-connections",
        required(env, "POOLSIM_MAX_SERVER_CONNECTIONS"),
        "--min",
        required(env, "POOLSIM_MIN_POOL_SIZE"),
        "--max",
        required(env, "POOLSIM_MAX_POOL_SIZE"),
    ]

    optional_flags = [
        ("POOLSIM_CONNECTION_OVERHEAD_MS", "--connection-overhead-ms"),
        ("POOLSIM_IDLE_TIMEOUT_MS", "--idle-timeout-ms"),
        ("POOLSIM_ITERATIONS", "--iterations"),
        ("POOLSIM_SEED", "--seed"),
        ("POOLSIM_TARGET_WAIT_P99_MS", "--target-wait-p99-ms"),
        ("POOLSIM_MAX_ACCEPTABLE_RHO", "--max-acceptable-rho"),
    ]
    for env_key, flag in optional_flags:
        if value := optional(env, env_key):
            cmd.extend([flag, value])

    if value := optional(env, "POOLSIM_DISTRIBUTION"):
        cmd.extend(["--distribution", value])
    if value := optional(env, "POOLSIM_QUEUE_MODEL"):
        cmd.extend(["--queue-model", value])

    return cmd


def run_command(command: Sequence[str]) -> dict:
    completed = subprocess.run(
        list(command),
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return json.loads(completed.stdout)


def metric_label_value(raw: str) -> str:
    return raw.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")


def render_metrics(report: Mapping[str, object], env: Mapping[str, str]) -> str:
    service = metric_label_value(env.get("POOLSIM_SERVICE_NAME", "unknown"))
    namespace = metric_label_value(env.get("POD_NAMESPACE", "default"))
    pod = metric_label_value(env.get("POD_NAME", "unknown"))
    labels = f'{{service="{service}",namespace="{namespace}",pod="{pod}"}}'

    metrics = [
        "# HELP poolsim_recommended_pool_size Recommended connection pool size.",
        "# TYPE poolsim_recommended_pool_size gauge",
        f"poolsim_recommended_pool_size{labels} {report['optimal_pool_size']}",
        "# HELP poolsim_recommendation_rho Utilization ratio at the recommended pool size.",
        "# TYPE poolsim_recommendation_rho gauge",
        f"poolsim_recommendation_rho{labels} {report['utilisation_rho']}",
        "# HELP poolsim_recommendation_p99_queue_wait_ms Predicted p99 queue wait at the recommended pool size.",
        "# TYPE poolsim_recommendation_p99_queue_wait_ms gauge",
        f"poolsim_recommendation_p99_queue_wait_ms{labels} {report['p99_queue_wait_ms']}",
        "# HELP poolsim_current_pool_size Current configured pool size projected from Kubernetes metadata.",
        "# TYPE poolsim_current_pool_size gauge",
        f"poolsim_current_pool_size{labels} {required(env, 'POOLSIM_CURRENT_POOL_SIZE')}",
    ]
    return "\n".join(metrics) + "\n"


def collect_metrics(
    env: Mapping[str, str],
    runner: Callable[[Sequence[str]], Mapping[str, object]] = run_command,
) -> str:
    return render_metrics(runner(build_simulate_command(env)), env)


class MetricsHandler(BaseHTTPRequestHandler):
    """HTTP handler for `/metrics` and health probes."""

    runner: Callable[[Sequence[str]], Mapping[str, object]] = run_command

    def do_GET(self) -> None:  # noqa: N802 - stdlib callback name
        if self.path == "/healthz":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok\n")
            return
        if self.path != "/metrics":
            self.send_response(404)
            self.end_headers()
            self.wfile.write(b"not found\n")
            return

        try:
            body = collect_metrics(os.environ, self.runner).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
            self.end_headers()
            self.wfile.write(body)
        except Exception as exc:  # pragma: no cover - exercised through handler integration in deployments
            self.send_response(500)
            self.end_headers()
            self.wfile.write(f"poolsim sidecar error: {exc}\n".encode("utf-8"))

    def log_message(self, format: str, *args: object) -> None:
        if os.environ.get("POOLSIM_SIDECAR_ACCESS_LOG") == "1":
            super().log_message(format, *args)


def main() -> None:
    port = int(os.environ.get("POOLSIM_SIDECAR_PORT", DEFAULT_PORT))
    server = ThreadingHTTPServer(("0.0.0.0", port), MetricsHandler)
    server.serve_forever()


if __name__ == "__main__":
    main()
