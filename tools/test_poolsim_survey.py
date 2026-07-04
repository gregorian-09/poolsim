import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from poolsim_survey import SurveyError, build_payload, sanitize_entry


class PoolsimSurveyTests(unittest.TestCase):
    def test_requires_explicit_consent(self):
        with self.assertRaises(SurveyError):
            build_payload([{"pool_size": 8}], consent=False)

    def test_sanitizes_allowed_fields_only(self):
        entry = sanitize_entry(
            {
                "framework": "sqlx",
                "database": "postgres",
                "pool_size": 8,
                "rps_band": "100-250",
                "notes": "not exported",
            }
        )
        self.assertEqual(entry["framework"], "sqlx")
        self.assertNotIn("notes", entry)

    def test_rejects_identifying_fields(self):
        with self.assertRaises(SurveyError):
            sanitize_entry({"pool_size": 8, "database_url": "postgres://secret"})

    def test_builds_payload(self):
        payload = build_payload([{"pool_size": 8, "framework": "hikaricp"}], consent=True)
        self.assertEqual(payload["schema_version"], "v1")
        self.assertFalse(payload["contains_application_data"])
        self.assertEqual(payload["entries"][0]["pool_size"], 8)


if __name__ == "__main__":
    unittest.main()
