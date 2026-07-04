import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from poolsim_continuous import ContinuousConfig, build_event, build_import_command, run_once


RECOMMENDATION = {
    "service_name": "checkout-api",
    "window": "5m",
    "observed_at": None,
    "diff": {
        "current_pool_size": 8,
        "recommended_pool_size": 10,
        "change": "Increase",
    },
}


class ContinuousRecommendationTests(unittest.TestCase):
    def test_builds_prometheus_response_file_command(self):
        config = ContinuousConfig(
            source="prometheus-response-file",
            response_file=Path("prometheus.json"),
            service_name="checkout-api",
            window="5m",
            current_pool_size=8,
            max_server_connections=100,
            min_pool_size=2,
            max_pool_size=20,
            connection_overhead_ms=2.0,
        )
        command = build_import_command(config)
        self.assertEqual(command[:5], ["poolsim", "--format", "json", "import", "prometheus"])
        self.assertIn("--response-file", command)
        self.assertIn("prometheus.json", command)
        self.assertIn("--service-name", command)

    def test_build_event_marks_changed_against_previous_state(self):
        previous = json.loads(json.dumps(RECOMMENDATION))
        previous["diff"]["recommended_pool_size"] = 8
        event = build_event(RECOMMENDATION, previous, "prometheus-response-file")
        self.assertEqual(event["event_type"], "PoolRecommendationDiff")
        self.assertTrue(event["changed"])
        self.assertEqual(event["previous_recommended_pool_size"], 8)
        self.assertEqual(event["recommended_pool_size"], 10)

    def test_run_once_saves_state_and_invokes_webhook(self):
        with tempfile.TemporaryDirectory() as tmp:
            state_file = Path(tmp) / "state.json"
            config = ContinuousConfig(
                source="prometheus-response-file",
                response_file=Path("prometheus.json"),
                service_name="checkout-api",
                window="5m",
                current_pool_size=8,
                max_server_connections=100,
                min_pool_size=2,
                max_pool_size=20,
                state_file=state_file,
                webhook_url="https://hooks.example.test/poolsim",
            )
            seen = []

            def runner(command):
                self.assertIn("prometheus", command)
                return RECOMMENDATION

            def webhook(url, event):
                seen.append((url, event["event_type"]))

            event = run_once(config, runner=runner, webhook=webhook)
            self.assertTrue(state_file.exists())
            self.assertTrue(event["changed"])
            self.assertEqual(seen, [("https://hooks.example.test/poolsim", "PoolRecommendationDiff")])


if __name__ == "__main__":
    unittest.main()
