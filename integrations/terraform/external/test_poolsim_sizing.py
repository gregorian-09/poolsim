import json
import stat
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parent))
import poolsim_sizing


def fake_poolsim(tmp_path: Path, payload) -> Path:
    script = tmp_path / "poolsim"
    script.write_text(
        "#!/usr/bin/env python3\n"
        "import json, sys\n"
        f"sys.stdout.write({json.dumps(payload)!r})\n"
    )
    script.chmod(script.stat().st_mode | stat.S_IEXEC)
    return script


class TerraformAdapterTests(unittest.TestCase):
    def test_simulate_flattens_payload_for_external_provider(self):
        with TemporaryDirectory() as tmp:
            executable = fake_poolsim(Path(tmp), {
                "optimal_pool_size": 8,
                "confidence_interval": [7, 9],
                "p99_queue_wait_ms": 12.5,
                "saturation": "Ok",
            })
            result = poolsim_sizing.run_query({
                "poolsim_executable": str(executable),
                "command": "simulate",
                "config": "config.json",
            })
            self.assertEqual(result["optimal_pool_size"], "8")
            self.assertEqual(result["confidence_interval_min"], "7")
            self.assertEqual(result["confidence_interval_max"], "9")
            self.assertEqual(result["p99_queue_wait_ms"], "12.5")
            self.assertIn("raw_json", result)

    def test_rejects_unknown_command(self):
        with self.assertRaisesRegex(ValueError, "command must be"):
            poolsim_sizing.run_query({"command": "delete", "config": "config.json"})


if __name__ == "__main__":
    unittest.main()
