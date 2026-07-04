# Grafana Panel

Poolsim includes a minimal Grafana panel package for displaying `poolsim-web` sensitivity rows as a heatmap with the current pool size overlaid.

## Package

The panel lives at `integrations/grafana/poolsim-panel`.

It follows Grafana's panel-plugin model: `plugin.json` declares a panel plugin, and `src/module.tsx` exports a `PanelPlugin` React component.

## Data Flow

1. Run `poolsim-web` where Grafana can reach it.
2. Configure the panel's `Poolsim Web URL` option.
3. Paste a request body compatible with `POST /v1/sensitivity`.
4. The panel renders each sensitivity row as a heatmap cell.
5. The current pool size is highlighted with a distinct color.

The panel uses the existing `POST /v1/sensitivity` route. No new REST endpoint is required.

## Local Validation

```bash
python3 integrations/grafana/tests/validate_grafana_plugin.py
```

A full Grafana plugin build requires Node dependencies from Grafana's plugin tooling. The checked-in validation test intentionally avoids installing packages during normal repository tests.

## Sources

- Grafana plugin anatomy: https://grafana.com/developers/plugin-tools/key-concepts/anatomy-of-a-plugin
- Grafana panel plugin tutorial: https://grafana.com/developers/plugin-tools/tutorials/build-a-panel-plugin
