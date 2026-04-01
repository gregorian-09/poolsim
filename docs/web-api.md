# Poolsim Web API

## Purpose

This is the exhaustive usage reference for the current `poolsim-web` surface.

It covers:

- Every REST endpoint
- The WebSocket endpoint
- Request and response payloads
- Error responses
- Embedding `poolsim-web` as an Axum router
- `AppState`, `RateLimitState`, and `build_app`

Checked-in request bodies for the documented HTTP and WebSocket examples live under `docs/fixtures/`:

- `docs/fixtures/web-simulate.json`
- `docs/fixtures/web-evaluate.json`
- `docs/fixtures/web-sensitivity.json`
- `docs/fixtures/batch.json`
- `docs/fixtures/web-ws-request.json`

## Base Routes

Available routes:

- `GET /v1/health`
- `GET /v1/models`
- `POST /v1/simulate`
- `POST /v1/evaluate`
- `POST /v1/sensitivity`
- `POST /v1/batch`
- `GET /v1/live` (WebSocket upgrade)

All REST request bodies are JSON.

## REST Endpoints

### `GET /v1/health`

Purpose:

- liveness check
- version exposure

Example:

```bash
curl -s http://127.0.0.1:8080/v1/health
```

Response:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

Public response model:

- `HealthResponse.status`
- `HealthResponse.version`

### `GET /v1/models`

Purpose:

- list supported distribution models
- list supported queue models

Example:

```bash
curl -s http://127.0.0.1:8080/v1/models
```

Response:

```json
{
  "distribution_models": ["LogNormal", "Exponential", "Empirical", "Gamma"],
  "queue_models": ["MMC", "MDC"]
}
```

Public response model:

- `ModelsResponse.distribution_models`
- `ModelsResponse.queue_models`

### `POST /v1/simulate`

Purpose:

- run the full recommendation flow

Request model:

- `SimulationRequest.workload`
- `SimulationRequest.pool`
- `SimulationRequest.options`

Example:

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/simulate \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-simulate.json
```

Response shape:

- `optimal_pool_size`
- `confidence_interval`
- `cold_start_min_pool_size`
- `utilisation_rho`
- `mean_queue_wait_ms`
- `p99_queue_wait_ms`
- `saturation`
- `sensitivity`
- `step_load_analysis`
- `warnings`

### `POST /v1/evaluate`

Purpose:

- score one fixed pool size

Request model:

- `EvaluateRequest.workload`
- `EvaluateRequest.pool_size`
- `EvaluateRequest.options`

Example:

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/evaluate \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-evaluate.json
```

Response shape:

- `pool_size`
- `utilisation_rho`
- `mean_queue_wait_ms`
- `p99_queue_wait_ms`
- `saturation`
- `warnings`

### `POST /v1/sensitivity`

Purpose:

- generate sensitivity rows across a pool-size range

Request model:

- `SensitivityRequest.workload`
- `SensitivityRequest.pool`
- `SensitivityRequest.options`

Example:

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/sensitivity \
  -H 'content-type: application/json' \
  --data @docs/fixtures/web-sensitivity.json
```

Response is a JSON array of sensitivity rows:

```json
[
  {
    "pool_size": 3,
    "utilisation_rho": 0.91,
    "mean_queue_wait_ms": 44.0,
    "p99_queue_wait_ms": 230.0,
    "risk": "Critical"
  }
]
```

### `POST /v1/batch`

Purpose:

- run multiple simulation requests in one call

The request body is an array of `SimulationRequest`.

Example:

```bash
curl -s \
  -X POST http://127.0.0.1:8080/v1/batch \
  -H 'content-type: application/json' \
  --data @docs/fixtures/batch.json
```

Response:

- JSON array of `SimulationReport`

## Shared Payload Types

These are the request fields reused across endpoints.

### Workload payload

```json
{
  "requests_per_second": 220.0,
  "latency_p50_ms": 8.0,
  "latency_p95_ms": 32.0,
  "latency_p99_ms": 85.0,
  "raw_samples_ms": [7.0, 8.0, 10.0, 15.0],
  "step_load_profile": [
    { "time_s": 0, "requests_per_second": 180.0 },
    { "time_s": 30, "requests_per_second": 260.0 }
  ]
}
```

### Pool payload

```json
{
  "max_server_connections": 120,
  "connection_overhead_ms": 2.0,
  "idle_timeout_ms": 120000,
  "min_pool_size": 3,
  "max_pool_size": 24
}
```

The `connection_overhead_ms` field also accepts the alias:

- `connection_establishment_overhead_ms`

### Options payload

```json
{
  "iterations": 10000,
  "seed": 42,
  "distribution": "LogNormal",
  "queue_model": "MMC",
  "target_wait_p99_ms": 45.0,
  "max_acceptable_rho": 0.85
}
```

## Error Responses

The web layer maps failures to a consistent JSON shape:

```json
{
  "error": "human readable message",
  "code": "MACHINE_CODE",
  "details": null
}
```

### Common error codes

- `INVALID_JSON`
- `SIMULATION_TIMEOUT`
- `INTERNAL_JOIN_ERROR`
- `INVALID_RPS`
- `INVALID_LATENCY`
- `INVALID_LATENCY_ORDER`
- `INVALID_SAMPLES`
- `INVALID_STEP_LOAD_PROFILE`
- `INVALID_STEP_LOAD_PROFILE_ORDER`
- `INVALID_STEP_LOAD_RPS`
- `INVALID_MAX_SERVER_CONNECTIONS`
- `INVALID_CONNECTION_OVERHEAD`
- `INVALID_MIN_POOL_SIZE`
- `INVALID_POOL_RANGE`
- `POOL_EXCEEDS_SERVER_MAX`
- `INVALID_ITERATIONS`
- `INVALID_TARGET_WAIT`
- `INVALID_MAX_ACCEPTABLE_RHO`
- `INVALID_POOL_SIZE`
- `SATURATED`
- `DISTRIBUTION_ERROR`
- `SIMULATION_ERROR`

### Status mapping

- `400 Bad Request`: invalid JSON or invalid input
- `408 Request Timeout`: simulation timeout
- `422 Unprocessable Entity`: saturated queue state
- `500 Internal Server Error`: internal join/distribution/simulation failures

Example invalid-input response:

```json
{
  "error": "latency percentiles must be ordered: p50 < p95 < p99",
  "code": "INVALID_LATENCY_ORDER",
  "details": {
    "p50": 100.0,
    "p95": 50.0,
    "p99": 120.0
  }
}
```

## WebSocket API

### Route

- `GET /v1/live`

### Upgrade Behavior

The client must connect with WebSocket, then send one JSON frame as the initial request.

Accepted first frame types:

- UTF-8 text JSON
- binary frame containing UTF-8 JSON

### Initial request payload

Single simulation request:

The checked-in single-request fixture is `docs/fixtures/web-ws-request.json`.

```json
{
  "workload": {
    "requests_per_second": 180.0,
    "latency_p50_ms": 6.0,
    "latency_p95_ms": 25.0,
    "latency_p99_ms": 60.0
  },
  "pool": {
    "max_server_connections": 100,
    "connection_overhead_ms": 2.0,
    "min_pool_size": 2,
    "max_pool_size": 20
  },
  "options": {
    "iterations": 12000
  }
}
```

Batch request:

```json
[
  {
    "workload": {
      "requests_per_second": 180.0,
      "latency_p50_ms": 6.0,
      "latency_p95_ms": 25.0,
      "latency_p99_ms": 60.0
    },
    "pool": {
      "max_server_connections": 100,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 2,
      "max_pool_size": 20
    },
    "options": {
      "iterations": 12000
    }
  },
  {
    "workload": {
      "requests_per_second": 260.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 30.0,
      "latency_p99_ms": 70.0
    },
    "pool": {
      "max_server_connections": 120,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 3,
      "max_pool_size": 24
    },
    "options": {
      "iterations": 12000
    }
  }
]
```

### Tick frames

Intermediate progress frames:

```json
{
  "tick": 1000,
  "of": 10000,
  "p99_so_far_ms": 41.2,
  "optimal_so_far": 22
}
```

Batch tick frames add:

- `batch_index`
- `batch_total`

### Done frames

Single-request done frame:

```json
{
  "done": true,
  "report": {
    "optimal_pool_size": 8,
    "confidence_interval": [7, 9],
    "cold_start_min_pool_size": 6,
    "utilisation_rho": 0.74,
    "mean_queue_wait_ms": 3.2,
    "p99_queue_wait_ms": 18.0,
    "saturation": "Ok",
    "sensitivity": [],
    "step_load_analysis": [],
    "warnings": []
  }
}
```

Batch done frame:

```json
{
  "batch_done": true,
  "batch_total": 2,
  "reports": [
    {
      "optimal_pool_size": 8,
      "confidence_interval": [7, 9],
      "cold_start_min_pool_size": 6,
      "utilisation_rho": 0.74,
      "mean_queue_wait_ms": 3.2,
      "p99_queue_wait_ms": 18.0,
      "saturation": "Ok",
      "sensitivity": [],
      "step_load_analysis": [],
      "warnings": []
    }
  ]
}
```

### WebSocket error frames

Shape:

```json
{
  "error": "message",
  "code": "ERROR_CODE",
  "details": null
}
```

Possible codes include:

- `INVALID_UTF8`
- `INVALID_FRAME`
- `INVALID_JSON`
- any propagated core error code, such as `INVALID_LATENCY_ORDER`
- `SIMULATION_TIMEOUT`
- `INTERNAL_JOIN_ERROR`

Batch errors may include `batch_index`.

## Embedding `poolsim-web`

### `AppState`

Public fields:

- `simulation_timeout`
- `version`

Example:

```rust
use std::time::Duration;
use poolsim_web::state::AppState;

let state = AppState {
    simulation_timeout: Duration::from_secs(30),
    version: "0.1.0",
};

assert_eq!(state.version, "0.1.0");
```

### `RateLimitState`

Use this to configure in-memory per-IP rate limiting.

Example:

```rust
use std::time::Duration;
use poolsim_web::middleware::rate_limit::RateLimitState;

let limiter = RateLimitState::new(60, Duration::from_secs(60));
let _ = limiter;
```

### `build_app`

Use `build_app` to get the full Axum router.

```rust
use std::time::Duration;
use poolsim_web::{
    build_app,
    middleware::rate_limit::RateLimitState,
    state::AppState,
};

let state = AppState {
    simulation_timeout: Duration::from_secs(30),
    version: "0.1.0",
};

let rate_limit = RateLimitState::new(60, Duration::from_secs(60));
let app = build_app(state, rate_limit, "https://example.com,http://localhost:3000");

let _ = app;
```

Minimal same-origin setup:

```rust
use std::time::Duration;
use poolsim_web::{
    build_app,
    middleware::rate_limit::RateLimitState,
    state::AppState,
};

let app = build_app(
    AppState {
        simulation_timeout: Duration::from_secs(15),
        version: "dev",
    },
    RateLimitState::new(30, Duration::from_secs(60)),
    "",
);

let _ = app;
```

### `AppError`

`AppError` is the web-layer error type used by handlers. Most HTTP consumers see it only through serialized JSON responses, but embedder code may still handle it directly.

Variants:

- `Core`
- `InvalidJson`
- `Timeout`
- `Join`

## Operational Notes

- The web layer runs core simulation work in `spawn_blocking`.
- WebSocket progress frames are best-effort; slow clients may miss intermediate ticks.
- `POST /v1/batch` fails the whole request if any single batch item fails.
- The rate limiter is in-memory. It is process-local, not distributed.
