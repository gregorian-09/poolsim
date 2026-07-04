import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from summarize_results import percent_error, summarize


class BenchmarkSummaryTests(unittest.TestCase):
    def test_percent_error_handles_zero_actual(self):
        self.assertEqual(percent_error(0, 0), 0.0)
        self.assertEqual(percent_error(25, 100), 75.0)

    def test_summarize_reports_mean_error_and_details(self):
        report = summarize(
            [
                {
                    "target": "postgres-local",
                    "framework": "sqlx",
                    "recommended_pool_size": 8,
                    "actual_pool_size": 8,
                    "poolsim_predicted_p99_queue_wait_ms": 24.5,
                    "observed_p99_queue_wait_ms": 27.0,
                }
            ]
        )
        self.assertEqual(report["benchmark_count"], 1)
        self.assertAlmostEqual(report["mean_p99_queue_wait_percent_error"], 9.259259, places=5)
        self.assertEqual(report["results"][0]["framework"], "sqlx")


if __name__ == "__main__":
    unittest.main()
