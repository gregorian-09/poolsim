# poolsim-web

`poolsim-web` is the REST and WebSocket service layer for `poolsim-core`.

It exposes the sizing calculator over HTTP for service-to-service integration, dashboards, and remote tooling.

## Surface

It exposes:

- `GET /v1/health`
- `GET /v1/models`
- `POST /v1/simulate`
- `POST /v1/evaluate`
- `POST /v1/sensitivity`
- `POST /v1/batch`
- `GET /v1/live`

## Run

```bash
cargo run -p poolsim-web
```

By default it binds to `0.0.0.0:8080`.

## Example

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

## Embedding

If you do not want to run the provided binary, the crate also exports `build_app`, `AppState`, and `RateLimitState` so you can compose the router inside your own Axum service.

## See Also

- Workspace repository: <https://github.com/gregorian-09/poolsim>
- Detailed web guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/web-api.md>

## Notes

- This crate serves the sizing calculator; it does not manage live production connection pools.
