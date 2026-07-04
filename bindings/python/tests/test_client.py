import json
import stat
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from poolsim import PoolsimClient, PoolsimError


def fake_poolsim(tmp_path: Path, payload, code: int = 0, stderr: str = "") -> Path:
    script = tmp_path / "poolsim"
    script.write_text(
        "#!/usr/bin/env python3\n"
        "import json, sys\n"
        f"sys.stderr.write({stderr!r})\n"
        f"sys.stdout.write({json.dumps(payload)!r})\n"
        f"sys.exit({code})\n"
    )
    script.chmod(script.stat().st_mode | stat.S_IEXEC)
    return script


class PoolsimClientTests(unittest.TestCase):
    def test_simulate_invokes_cli_and_decodes_json(self):
        with TemporaryDirectory() as tmp:
            executable = fake_poolsim(Path(tmp), {"optimal_pool_size": 8})
            report = PoolsimClient(str(executable)).simulate("config.json")
            self.assertEqual(report["optimal_pool_size"], 8)

    def test_all_methods_delegate_to_json_cli(self):
        with TemporaryDirectory() as tmp:
            executable = fake_poolsim(Path(tmp), {"status": "ok"})
            client = PoolsimClient(str(executable))
            self.assertEqual(client.evaluate("c.json", 8)["status"], "ok")
            self.assertEqual(client.compare("c.json")["status"], "ok")
            self.assertEqual(client.budget("c.json")["status"], "ok")
            self.assertEqual(client.telemetry_recommend("t.json")["status"], "ok")
            self.assertEqual(client.doctor("t.json")["status"], "ok")
            self.assertEqual(client.generate_config("sqlx", "c.json")["status"], "ok")

    def test_errors_include_cli_stderr(self):
        with TemporaryDirectory() as tmp:
            executable = fake_poolsim(Path(tmp), {}, code=1, stderr="bad input")
            with self.assertRaisesRegex(PoolsimError, "bad input"):
                PoolsimClient(str(executable)).simulate("config.json")


if __name__ == "__main__":
    unittest.main()
