#!/usr/bin/env python3
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "poolsim-panel"


def main() -> int:
    plugin = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    module = (ROOT / "src/module.tsx").read_text(encoding="utf-8")

    assert plugin["type"] == "panel"
    assert plugin["id"] == "gregorianrayne-poolsim-panel"
    assert plugin["info"]["version"] == package["version"]
    assert "PanelPlugin" in module
    assert "/v1/sensitivity" in module
    assert "currentPoolSize" in module
    assert "POST /v1/sensitivity" in (Path(__file__).resolve().parents[1] / "README.md").read_text(encoding="utf-8")
    print("grafana plugin validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
