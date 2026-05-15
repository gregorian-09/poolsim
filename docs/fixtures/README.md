# Docs Fixtures

This directory contains checked-in sample inputs used by the documentation and validation scripts.

## Files

- `cli-config.json`: primary JSON config for CLI examples
- `cli-config.toml`: primary TOML config for CLI examples
- `batch.json`: JSON batch input for `poolsim batch`
- `batch.toml`: TOML batch input for `poolsim batch`
- `telemetry.json`: telemetry import body for `poolsim import telemetry` and `POST /v1/telemetry/recommend`
- `latencies.txt`: empirical latency sample file for `--samples-file`
- `web-simulate.json`: request body for `POST /v1/simulate`
- `web-evaluate.json`: request body for `POST /v1/evaluate`
- `web-sensitivity.json`: request body for `POST /v1/sensitivity`
- `web-ws-request.json`: initial WebSocket request body for `GET /v1/live`
