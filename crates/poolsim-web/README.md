# poolsim-web

[![Crates.io](https://img.shields.io/crates/v/poolsim-web.svg)](https://crates.io/crates/poolsim-web)
[![docs.rs](https://img.shields.io/docsrs/poolsim-web)](https://docs.rs/poolsim-web)
[![CI](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml/badge.svg)](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml)
[![Docs Coverage](https://img.shields.io/badge/docs%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/docs/README.md)
[![Workspace Coverage](https://img.shields.io/badge/workspace%20line%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_coverage_threshold.py)
[![Examples Coverage](https://img.shields.io/badge/examples%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_examples_coverage.py)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/gregorian-09/poolsim/blob/main/LICENSE)

`poolsim-web` is the REST and WebSocket service layer for `poolsim-core`.

Use it when you want to expose the sizing calculator to dashboards, internal developer platforms, remote CI jobs, or non-Rust services over HTTP.

## What Is New In `0.3.0`

`0.3.0` is an additive web-service release for teams that want Poolsim available over HTTP, WebSocket, dashboards, and internal developer platforms. Existing REST routes, WebSocket behavior, JSON response fields, and embedding APIs remain available.

### Direct OTLP Recommendation Endpoint

The web service now documents and validates `POST /v1/otlp/recommend`, which accepts OpenTelemetry OTLP metric-export JSON and returns the same `TelemetryRecommendation` diff model used by the CLI and core library. This is useful for teams already exporting request-rate and latency metrics through OpenTelemetry collectors.

The endpoint supports explicit metric-name mapping so callers can adapt their own metric names into Poolsim's expected workload fields:

- request rate
- p50 latency in milliseconds
- p95 latency in milliseconds
- p99 latency in milliseconds

Missing required OTLP metrics are returned as structured HTTP errors instead of ambiguous server failures.

### Dashboard And Internal Platform Support

The release includes documentation and integration assets that make `poolsim-web` easier to use from dashboards:

- Grafana panel package for rendering `POST /v1/sensitivity` rows as a heatmap.
- Built-in browser UI served from `GET /` for interactive sizing experiments.
- WebSocket live progress documentation for interactive clients and batch simulations.
- Clear request/response fixtures for simulate, evaluate, sensitivity, batch, telemetry recommendation, OTLP recommendation, and WebSocket flows.

These surfaces are intended for internal tools, not as a public unauthenticated internet service.

### Stable Embedding Surface

The documented embedding API remains centered on:

- `build_app`
- `AppState`
- `RateLimitState`

Use these when you want to mount the Poolsim router inside a larger Axum service while controlling timeout, version, rate-limit, and CORS behavior yourself.

### Operations And Packaging

`0.3.0` updates the Docker/GHCR documentation and Dockerfile toolchain so `poolsim-web` can be built as a container image in CI. The Docker workflow is intentionally not triggered on every push; it is designed for manual runs and version tags.

Recommended operating posture:

- run behind an internal auth gateway when exposed beyond localhost,
- set explicit CORS origins,
- configure simulation timeout and rate-limit values for your UI/CI budget,
- keep request payloads free of secrets,
- pin container and crate versions for repeatable deployments.

### Compatibility And Quality Notes

This release is intended to be backward-compatible with `0.2.x` web consumers. It does not intentionally remove REST routes, WebSocket frame shapes, public embedding types, or serialized response fields. CI validates REST/WebSocket docs fixtures, route behavior, public API documentation, rustdoc warnings, and workspace coverage.

### When To Upgrade

Upgrade to `0.3.0` if you want the OTLP recommendation endpoint, updated web API documentation, dashboard integration guidance, Docker/GHCR release metadata, and current package versions. Existing `/v1/simulate`, `/v1/evaluate`, `/v1/sensitivity`, `/v1/batch`, `/v1/telemetry/recommend`, and `/v1/live` clients should continue to work.

## Install Or Run

Run from a workspace checkout:

```bash
cargo run -p poolsim-web
```

Install the binary crate from crates.io:

```bash
cargo install poolsim-web
```

By default the service binds to `0.0.0.0:8080`.

Common environment variables:

- `POOLSIM_HOST`: bind host.
- `POOLSIM_PORT`: bind port.
- `POOLSIM_CORS_ALLOW_ORIGIN`: allowed CORS origin.
- `POOLSIM_TIMEOUT_MS`: simulation timeout.
- `POOLSIM_RATE_LIMIT_RPM`: rate limit per minute.
- `RUST_LOG`: tracing filter.

## HTTP Surface

Available REST endpoints:

- `GET /v1/health`
- `GET /v1/models`
- `POST /v1/simulate`
- `POST /v1/evaluate`
- `POST /v1/sensitivity`
- `POST /v1/batch`
- `POST /v1/telemetry/recommend`
- `POST /v1/otlp/recommend`

Available WebSocket endpoint:

- `GET /v1/live`

All JSON error responses use a stable error envelope with code and message fields.

## Health Check

```bash
curl -s http://127.0.0.1:8080/v1/health
```

Example response:

```json
{
  "status": "ok",
  "version": "0.3.0"
}
```

## Model Metadata

```bash
curl -s http://127.0.0.1:8080/v1/models
```

Use this endpoint to discover supported distribution and queue models for UI selectors.

## Simulation Request

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/simulate \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-simulate.json
```

Inline request example:

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/simulate \
  -H 'content-type: application/json' \
  -d '{
    "workload": {
      "requests_per_second": 220.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 32.0,
      "latency_p99_ms": 85.0
    },
    "pool": {
      "max_server_connections": 120,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 3,
      "max_pool_size": 24
    },
    "options": {
      "iterations": 10000,
      "distribution": "LogNormal",
      "queue_model": "MMC"
    }
  }'
```

## Fixed Evaluation Request

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/evaluate \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-evaluate.json
```

Use this endpoint when a service already has a configured pool size and you want to evaluate saturation and queue wait.

## Sensitivity Request

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/sensitivity \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-sensitivity.json
```

Use sensitivity output for dashboards, charts, or design-review tables.

## Batch Request

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/batch \
  -H 'content-type: application/json' \
  -d '{
    "requests": [
      {
        "workload": {
          "requests_per_second": 180.0,
          "latency_p50_ms": 8.0,
          "latency_p95_ms": 30.0,
          "latency_p99_ms": 70.0
        },
        "pool": {
          "max_server_connections": 100,
          "connection_overhead_ms": 2.0,
          "min_pool_size": 2,
          "max_pool_size": 20
        }
      }
    ]
  }'
```

## Telemetry Recommendation Request

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/telemetry/recommend \
  -H 'content-type: application/json' \
  --data @docs/fixtures/telemetry.json
```

Use this endpoint to let non-Rust services submit observed traffic and latency data and receive a current-vs-recommended pool diff.

## WebSocket Live Streaming

The WebSocket endpoint accepts a simulation request or batch of simulation requests and streams newline-delimited progress frames before the final report.

Endpoint:

```text
ws://127.0.0.1:8080/v1/live
```

Checked-in fixture:

```text
docs/fixtures/web-ws-request.json
```

This is useful for dashboards and UI flows where users should see progress while simulations run.

## Embedding In Your Own Axum Service

If you do not want to run the provided binary, compose the router directly:

```rust
use std::time::Duration;

use poolsim_web::{build_app, middleware::rate_limit::RateLimitState, state::AppState};

let state = AppState {
    simulation_timeout: Duration::from_secs(5),
    version: env!("CARGO_PKG_VERSION"),
};
let limiter = RateLimitState::new(60, Duration::from_secs(60));
let app = build_app(state, limiter, "https://internal.example.com");
# let _ = app;
```

## Getting The Most From The Web Service

- Put it behind your internal auth gateway if exposed outside localhost.
- Keep CORS explicit; avoid broad production origins unless intentional.
- Set a simulation timeout that matches your UI and CI latency budget.
- Use telemetry recommendation endpoints for non-Rust services.
- Use WebSocket live streaming for dashboards and long-running batch simulations.
- Use `poolsim-cli budget` separately when you need cross-service database budget allocation.

## Quality And CI Guarantees

The upstream repository currently enforces:

- `cargo check --workspace`
- `cargo test --workspace`
- `RUSTFLAGS="-D missing_docs"` checks for all crates
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- REST and WebSocket executable docs fixtures
- integration tests for REST routes, rate limiting, error mapping, and WebSocket frames
- `100%` workspace line coverage
- `100%` example-file coverage
- public API documentation coverage checks

## Support

- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
- Detailed web guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/web-api.md>
- Changelog: <https://github.com/gregorian-09/poolsim/blob/main/CHANGELOG.md>

When opening an issue, include the crate version, endpoint, request body with secrets removed, response status, response body, logs if available, and reproduction steps.

## Related Crates

- `poolsim-core`: Rust sizing engine and telemetry recommendation model.
- `poolsim-cli`: command-line simulation, doctor, guard, config generator, and budget planner.
