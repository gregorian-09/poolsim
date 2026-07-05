# Grafana Panel Integration

Poolsim includes a minimal Grafana panel package for displaying `poolsim-web` sensitivity rows as a heatmap with the current pool size overlaid.

Use this integration when backend engineers already inspect capacity in Grafana and you want Poolsim's sizing table visible next to service dashboards.

## What It Does

- Defines a Grafana panel plugin in `poolsim-panel/plugin.json`.
- Exposes a React `PanelPlugin` in `poolsim-panel/src/module.tsx`.
- Calls `POST /v1/sensitivity` on `poolsim-web`.
- Renders each candidate pool size as a colored heatmap cell.
- Highlights the configured current pool size with a distinct color.
- Shows utilization ratio, p99 queue wait, and risk per candidate size.

It does not add new Poolsim REST routes and does not mutate any runtime pool setting.

## Current Maturity

This is a checked-in integration package and validation target, not a signed Grafana marketplace release.

Use it as:

- a reference panel implementation,
- a local/private Grafana plugin starting point,
- a dashboard integration example for teams running `poolsim-web` internally.

Before distributing it broadly, run the normal Grafana plugin build/signing process for your organization.

## Files

- `poolsim-panel/plugin.json`: Grafana plugin metadata.
- `poolsim-panel/package.json`: package metadata and validation script.
- `poolsim-panel/src/module.tsx`: panel implementation.
- `poolsim-panel/src/img/logo.svg`: plugin logo.
- `tests/validate_grafana_plugin.py`: lightweight repository validation.

## Runtime Requirements

- Grafana 10 or newer according to `plugin.json`.
- A reachable `poolsim-web` instance.
- A request body compatible with `POST /v1/sensitivity`.
- Browser/network access from the Grafana panel runtime to the configured `poolsim-web` URL.

## Data Flow

1. Run `poolsim-web` where the panel can reach it.
2. Add the Poolsim panel to a dashboard.
3. Configure `Poolsim Web URL`.
4. Set `Current pool size overlay`.
5. Paste a request body compatible with `POST /v1/sensitivity`.
6. The panel posts the request to `poolsim-web`.
7. The panel renders each sensitivity row as a heatmap cell.

The panel uses the existing `POST /v1/sensitivity` route. No new REST endpoint is required.

## Panel Options

| Option | Default | Meaning |
| --- | --- | --- |
| `Poolsim Web URL` | `http://localhost:8080` | Base URL for `poolsim-web`. |
| `Current pool size overlay` | `8` | Pool size to highlight in the heatmap. |
| `Sensitivity request JSON` | built-in example | Request body sent to `POST /v1/sensitivity`. |

## Sensitivity Request Example

```json
{
  "workload": {
    "requests_per_second": 180,
    "latency_p50_ms": 8,
    "latency_p95_ms": 30,
    "latency_p99_ms": 70
  },
  "pool": {
    "max_server_connections": 100,
    "connection_overhead_ms": 2,
    "min_pool_size": 2,
    "max_pool_size": 20
  },
  "options": {
    "iterations": 10000,
    "target_wait_p99_ms": 45,
    "max_acceptable_rho": 0.85
  }
}
```

Equivalent HTTP call:

```bash
curl -sS http://localhost:8080/v1/sensitivity \
  -H 'Content-Type: application/json' \
  -d @docs/fixtures/web-sensitivity.json
```

## Expected Response Shape

The panel expects an array of rows containing at least:

```json
[
  {
    "pool_size": 8,
    "utilisation_rho": 0.72,
    "mean_queue_wait_ms": 4.3,
    "p99_queue_wait_ms": 18.5,
    "risk": "Ok"
  }
]
```

The color mapping is intentionally simple:

- Current pool size: teal overlay.
- `Critical`: dark red.
- `High`: orange/red.
- `Medium`: yellow.
- Any other risk: green.

## Local Validation

Run the repository validation test:

```bash
python3 integrations/grafana/tests/validate_grafana_plugin.py
```

This check verifies:

- plugin type is `panel`,
- plugin ID is stable,
- plugin version matches `package.json`,
- the module exports a `PanelPlugin`,
- the module calls `/v1/sensitivity`,
- the README documents `POST /v1/sensitivity`.

## Package Script

From the panel package directory:

```bash
cd integrations/grafana/poolsim-panel
npm test
```

The checked-in `npm test` runs the Python validation script. It intentionally avoids a full Grafana plugin toolchain install during normal repository tests.

## Full Grafana Plugin Development

A complete Grafana plugin workflow normally involves Grafana's plugin tooling, local Grafana, and signing rules. The exact commands depend on your organization's plugin distribution path.

Typical private-plugin flow:

1. Install Node dependencies for the panel package.
2. Build the plugin with Grafana plugin tooling.
3. Mount or copy the built plugin into a Grafana plugins directory.
4. Configure Grafana to allow the unsigned plugin in development environments.
5. Verify the panel can reach `poolsim-web`.
6. Sign the plugin if distributing outside a local/private setup.

## Troubleshooting

### Panel shows `poolsim-web returned HTTP <status>`

The configured `Poolsim Web URL` is reachable but the request failed. Check the request JSON and test with `curl`.

### Panel shows a network error

The browser or Grafana runtime cannot reach `poolsim-web`. Check DNS, service routing, CORS/proxy setup, and whether `poolsim-web` is listening on the expected host/port.

### Cells render but values look wrong

Run the same request directly against `poolsim-web` and inspect the sensitivity rows:

```bash
curl -sS http://localhost:8080/v1/sensitivity \
  -H 'Content-Type: application/json' \
  -d @docs/fixtures/web-sensitivity.json
```

### Plugin does not load in Grafana

Check Grafana plugin signing and development-mode settings. The repository validation test confirms metadata consistency, but it is not a substitute for a full Grafana runtime test.

## Security And Operations

- Do not expose `poolsim-web` publicly unless you intentionally secure it.
- Prefer internal network access or a Grafana proxy/data-source path for production dashboards.
- Avoid putting secrets in the request JSON.
- Keep request bodies focused on workload and pool assumptions.
- Pin plugin and `poolsim-web` versions for reproducible dashboards.

## Sources

- Grafana plugin anatomy: <https://grafana.com/developers/plugin-tools/key-concepts/anatomy-of-a-plugin>
- Grafana panel plugin tutorial: <https://grafana.com/developers/plugin-tools/tutorials/build-a-panel-plugin>

## Compatibility

This integration is additive. It consumes the existing `POST /v1/sensitivity` route and does not change Rust APIs, CLI output schemas, REST routes, WebSocket routes, or config files.
