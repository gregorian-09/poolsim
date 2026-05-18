# Docs Fixtures

This directory contains checked-in sample inputs used by the documentation and validation scripts.

## Files

- `cli-config.json`: primary JSON config for CLI examples, including `poolsim generate-config simulate`
- `cli-config.toml`: primary TOML config for CLI examples
- `batch.json`: JSON batch input for `poolsim batch`
- `batch.toml`: TOML batch input for `poolsim batch`
- `scenarios.json`: JSON scenario comparison input for `poolsim compare`
- `scenarios.toml`: TOML scenario comparison input for `poolsim compare`
- `budget.json`: JSON database connection budget input for `poolsim budget`
- `budget.toml`: TOML database connection budget input for `poolsim budget`
- `telemetry.json`: telemetry import body for `poolsim import telemetry`, `poolsim gate telemetry`, `poolsim guard telemetry`, `poolsim doctor telemetry`, `poolsim generate-config telemetry`, and `POST /v1/telemetry/recommend`
- `prometheus-responses.json`: offline Prometheus API response bundle for `poolsim import prometheus`, `poolsim gate prometheus`, `poolsim guard prometheus`, `poolsim doctor prometheus`, and `poolsim generate-config prometheus`
- `gate-policy.toml`: CI capacity-gate and deployment-guard policy for `poolsim gate` and `poolsim guard`
- `latencies.txt`: empirical latency sample file for `--samples-file`
- `web-simulate.json`: request body for `POST /v1/simulate`
- `web-evaluate.json`: request body for `POST /v1/evaluate`
- `web-sensitivity.json`: request body for `POST /v1/sensitivity`
- `web-ws-request.json`: initial WebSocket request body for `GET /v1/live`
