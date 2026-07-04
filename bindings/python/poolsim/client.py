"""Small Python wrapper around the stable Poolsim CLI JSON interface."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any, Mapping, Sequence


class PoolsimError(RuntimeError):
    """Raised when the Poolsim CLI exits unsuccessfully or emits invalid JSON."""


class PoolsimClient:
    """Calls the `poolsim` executable and returns decoded JSON payloads."""

    def __init__(self, executable: str = "poolsim") -> None:
        self.executable = executable

    def simulate(self, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim simulate --config <path>` and return the report."""
        return self._run_json(["simulate", "--config", str(config)])

    def evaluate(self, config: str | Path, pool_size: int) -> Mapping[str, Any]:
        """Run `poolsim evaluate` for a fixed pool size."""
        return self._run_json(["evaluate", "--config", str(config), "--pool-size", str(pool_size)])

    def sweep(self, config: str | Path) -> list[Mapping[str, Any]]:
        """Run `poolsim sweep` and return sensitivity rows."""
        return list(self._run_json(["sweep", "--config", str(config)]))

    def batch(self, config: str | Path) -> list[Mapping[str, Any]]:
        """Run `poolsim batch` and return all simulation reports."""
        return list(self._run_json(["batch", "--config", str(config)]))

    def compare(self, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim compare` for named traffic scenarios."""
        return self._run_json(["compare", "--config", str(config)])

    def budget(self, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim budget` for a database connection budget plan."""
        return self._run_json(["budget", "--config", str(config)])

    def telemetry_recommend(self, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim import telemetry` and return the recommendation diff."""
        return self._run_json(["import", "telemetry", "--config", str(config)])

    def doctor(self, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim doctor telemetry` and return the diagnostic report."""
        return self._run_json(["doctor", "telemetry", "--config", str(config)])

    def generate_config(self, framework: str, config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim generate-config` from a simulation config."""
        return self._run_json([
            "generate-config",
            "--framework",
            framework,
            "simulate",
            "--config",
            str(config),
        ])

    def gate(self, policy: str | Path, telemetry_config: str | Path) -> Mapping[str, Any]:
        """Run `poolsim gate telemetry` and return the gate report."""
        return self._run_json([
            "gate",
            "--policy",
            str(policy),
            "telemetry",
            "--config",
            str(telemetry_config),
        ], allowed_exit_codes=(0, 2))

    def _run_json(
        self,
        args: Sequence[str],
        allowed_exit_codes: tuple[int, ...] = (0,),
    ) -> Any:
        command = [self.executable, "--format", "json", *args]
        result = subprocess.run(command, check=False, capture_output=True, text=True)
        if result.returncode not in allowed_exit_codes:
            raise PoolsimError(result.stderr.strip() or f"poolsim exited with {result.returncode}")
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            raise PoolsimError("poolsim did not emit valid JSON") from exc
