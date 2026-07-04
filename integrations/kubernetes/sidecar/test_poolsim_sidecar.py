import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from poolsim_sidecar import SidecarError, build_simulate_command, collect_metrics, render_metrics


BASE_ENV = {
    "POOLSIM_CLI": "poolsim",
    "POOLSIM_SERVICE_NAME": "checkout-api",
    "POD_NAMESPACE": "payments",
    "POD_NAME": "checkout-abc123",
    "POOLSIM_EXPECTED_RPS": "180",
    "POOLSIM_LATENCY_P50_MS": "8",
    "POOLSIM_LATENCY_P95_MS": "30",
    "POOLSIM_LATENCY_P99_MS": "70",
    "POOLSIM_CURRENT_POOL_SIZE": "8",
    "POOLSIM_MAX_SERVER_CONNECTIONS": "100",
    "POOLSIM_MIN_POOL_SIZE": "2",
    "POOLSIM_MAX_POOL_SIZE": "20",
    "POOLSIM_CONNECTION_OVERHEAD_MS": "2",
}


class PoolsimSidecarTests(unittest.TestCase):
    def test_builds_simulate_command_from_environment(self):
        command = build_simulate_command(BASE_ENV)
        self.assertEqual(command[:4], ["poolsim", "--format", "json", "simulate"])
        self.assertIn("--rps", command)
        self.assertIn("180", command)
        self.assertIn("--connection-overhead-ms", command)
        self.assertIn("2", command)

    def test_rejects_missing_required_environment(self):
        env = dict(BASE_ENV)
        env.pop("POOLSIM_EXPECTED_RPS")
        with self.assertRaises(SidecarError):
            build_simulate_command(env)

    def test_renders_prometheus_metrics(self):
        report = {
            "optimal_pool_size": 9,
            "utilisation_rho": 0.71,
            "p99_queue_wait_ms": 23.5,
        }
        metrics = render_metrics(report, BASE_ENV)
        self.assertIn('poolsim_recommended_pool_size{service="checkout-api",namespace="payments",pod="checkout-abc123"} 9', metrics)
        self.assertIn("poolsim_current_pool_size", metrics)

    def test_collect_metrics_delegates_to_runner(self):
        seen = []

        def runner(command):
            seen.extend(command)
            return {"optimal_pool_size": 7, "utilisation_rho": 0.6, "p99_queue_wait_ms": 12.0}

        metrics = collect_metrics(BASE_ENV, runner)
        self.assertIn("poolsim_recommended_pool_size", metrics)
        self.assertIn("7", metrics)
        self.assertIn("simulate", seen)


if __name__ == "__main__":
    unittest.main()
