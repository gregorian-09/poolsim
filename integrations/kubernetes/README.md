# Kubernetes Sidecar

Poolsim ships an opt-in Kubernetes sidecar pattern for teams that want sizing recommendations visible in Prometheus without letting Poolsim mutate production pool settings.

## What It Does

- Reads `poolsim.io/*` annotations projected into environment variables with the Kubernetes Downward API.
- Runs the stable `poolsim --format json simulate` CLI path.
- Exposes Prometheus text metrics on `/metrics`.
- Leaves the application pool configuration untouched.

This follows Kubernetes guidance that annotations hold arbitrary non-identifying metadata and recommended `app.kubernetes.io/*` labels describe applications.

## Files

- `sidecar/poolsim_sidecar.py`: small HTTP exporter that delegates sizing to the CLI.
- `sidecar/deployment.yaml`: example deployment with annotations, Downward API env projection, and Prometheus scrape annotations.
- `sidecar/test_poolsim_sidecar.py`: unit tests for command construction and metrics rendering.

## Annotation Contract

Required annotations:

- `poolsim.io/expected-rps`
- `poolsim.io/latency-p50-ms`
- `poolsim.io/latency-p95-ms`
- `poolsim.io/latency-p99-ms`
- `poolsim.io/current-pool-size`
- `poolsim.io/max-server-connections`
- `poolsim.io/min-pool-size`
- `poolsim.io/max-pool-size`

Optional annotations:

- `poolsim.io/connection-overhead-ms`
- `poolsim.io/idle-timeout-ms`
- `poolsim.io/iterations`
- `poolsim.io/seed`
- `poolsim.io/target-wait-p99-ms`
- `poolsim.io/max-acceptable-rho`

## Metrics

The exporter emits:

- `poolsim_recommended_pool_size`
- `poolsim_recommendation_rho`
- `poolsim_recommendation_p99_queue_wait_ms`
- `poolsim_current_pool_size`

Labels are `service`, `namespace`, and `pod`. Keep those labels low-cardinality; Prometheus treats every unique label set as a separate time series.

## Run Tests

```bash
python3 -m unittest integrations/kubernetes/sidecar/test_poolsim_sidecar.py
```

## Sources

- Kubernetes annotations: https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/
- Kubernetes recommended labels: https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/
- Prometheus data model: https://prometheus.io/docs/concepts/data_model/
