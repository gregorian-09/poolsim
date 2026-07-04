import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from controller import ControllerConfig, build_command, reconcile_once, recommendation_annotations


DEPLOYMENT = {
    "metadata": {
        "name": "checkout-api",
        "annotations": {
            "poolsim.io/expected-rps": "180",
            "poolsim.io/latency-p50-ms": "8",
            "poolsim.io/latency-p95-ms": "30",
            "poolsim.io/latency-p99-ms": "70",
            "poolsim.io/max-server-connections": "100",
            "poolsim.io/min-pool-size": "2",
            "poolsim.io/max-pool-size": "20",
            "poolsim.io/connection-overhead-ms": "2",
        },
    }
}


class KubernetesControllerTests(unittest.TestCase):
    def test_build_command_from_annotations(self):
        command = build_command(DEPLOYMENT, "poolsim")
        self.assertEqual(command[:4], ["poolsim", "--format", "json", "simulate"])
        self.assertIn("--rps", command)
        self.assertIn("180", command)
        self.assertIn("--connection-overhead-ms", command)

    def test_skips_deployment_without_required_annotations(self):
        self.assertIsNone(build_command({"metadata": {"annotations": {}}}))

    def test_recommendation_annotations_are_stable(self):
        annotations = recommendation_annotations(
            {
                "optimal_pool_size": 9,
                "utilisation_rho": 0.72,
                "p99_queue_wait_ms": 18.5,
                "saturation": "Ok",
            }
        )
        self.assertEqual(annotations["poolsim.io/recommended-pool-size"], "9")
        self.assertEqual(annotations["poolsim.io/recommended-saturation"], "Ok")

    def test_reconcile_patches_when_apply_enabled(self):
        patched = []

        def runner(command):
            self.assertIn("simulate", command)
            return {
                "optimal_pool_size": 9,
                "utilisation_rho": 0.72,
                "p99_queue_wait_ms": 18.5,
                "saturation": "Ok",
            }

        def patcher(config, deployment, annotations):
            patched.append((deployment["metadata"]["name"], annotations["poolsim.io/recommended-pool-size"]))

        events = reconcile_once(
            ControllerConfig(namespace="default", apply=True),
            [DEPLOYMENT],
            runner=runner,
            patcher=patcher,
        )
        self.assertEqual(patched, [("checkout-api", "9")])
        self.assertTrue(events[0]["applied"])


if __name__ == "__main__":
    unittest.main()
