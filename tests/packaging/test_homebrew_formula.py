import unittest
from pathlib import Path


FORMULA = Path(__file__).resolve().parents[2] / "packaging/homebrew/poolsim.rb"


class HomebrewFormulaTests(unittest.TestCase):
    def test_formula_contains_required_metadata(self):
        text = FORMULA.read_text(encoding="utf-8")
        self.assertIn("class Poolsim < Formula", text)
        self.assertIn('homepage "https://github.com/gregorian-09/poolsim"', text)
        self.assertIn('license "MIT"', text)
        self.assertIn('depends_on "rust" => :build', text)

    def test_formula_installs_cli_and_web_crates(self):
        text = FORMULA.read_text(encoding="utf-8")
        self.assertIn('"--path", "crates/poolsim-cli"', text)
        self.assertIn('"--path", "crates/poolsim-web"', text)
        self.assertIn('"#{bin}/poolsim --help"', text)


if __name__ == "__main__":
    unittest.main()
