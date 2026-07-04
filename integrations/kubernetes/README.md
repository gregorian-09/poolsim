# Kubernetes Sidecar

Poolsim ships two opt-in Kubernetes integrations for teams that want sizing recommendations visible in cluster operations without letting Poolsim mutate production pool settings.

- Sidecar metrics exporter: exposes recommendation metrics from the same pod.
- Controller: watches deployment annotations, computes recommendations, and writes recommendation annotations back to deployments.

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
- `controller.py`: Kubernetes API controller that patches recommendation annotations.
- `test_controller.py`: unit tests for controller command generation, annotation rendering, and patch behavior.

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
python3 -m unittest integrations/kubernetes/test_controller.py
```

## Controller Mode

Run the controller in a pod with a service account that can list and patch deployments in the target namespace.

Environment variables:

- `POOLSIM_K8S_NAMESPACE`: namespace to reconcile. Defaults to the service-account namespace.
- `POOLSIM_K8S_API`: Kubernetes API base URL. Defaults to `https://kubernetes.default.svc`.
- `POOLSIM_K8S_TOKEN`: bearer token. Defaults to the mounted service-account token.
- `POOLSIM_K8S_INTERVAL_SECS`: reconcile interval. Defaults to `60`.
- `POOLSIM_K8S_APPLY`: set to `true` to patch deployments. When false, the controller only prints recommendation events.
- `POOLSIM_CLI`: Poolsim CLI binary path. Defaults to `poolsim`.

The controller reads the same `poolsim.io/*` annotations documented above and patches:

- `poolsim.io/recommended-pool-size`
- `poolsim.io/recommended-rho`
- `poolsim.io/recommended-p99-queue-wait-ms`
- `poolsim.io/recommended-saturation`

Minimal RBAC:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: poolsim-controller
rules:
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "patch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: poolsim-controller
subjects:
  - kind: ServiceAccount
    name: poolsim-controller
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: poolsim-controller
```

## Sources

- Kubernetes annotations: https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/
- Kubernetes recommended labels: https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/
- Prometheus data model: https://prometheus.io/docs/concepts/data_model/
