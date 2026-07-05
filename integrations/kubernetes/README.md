# Kubernetes Integrations

Poolsim ships two opt-in Kubernetes integrations for teams that want sizing recommendations visible in cluster operations without letting Poolsim mutate production pool settings automatically.

- Sidecar metrics exporter: exposes recommendation metrics from the same pod.
- Controller: watches deployment annotations, computes recommendations, and optionally writes recommendation annotations back to deployments.

Both integrations delegate to the stable `poolsim --format json simulate` CLI path. Neither reimplements the sizing model.

## Which Mode Should You Use?

Use the sidecar when:

- You want per-pod Prometheus metrics.
- You want the app pod to expose current and recommended pool values together.
- You do not want Kubernetes API permissions for patching deployments.
- You are comfortable projecting annotations into environment variables with the Downward API.

Use the controller when:

- You want deployment-level recommendation annotations.
- You want one process to scan many deployments.
- You want platform tooling or alerting to read recommendations from Kubernetes metadata.
- You can provide RBAC permissions to list deployments and optionally patch deployment annotations.

## Safety Model

Poolsim does not change runtime pool settings in these integrations.

The sidecar only exposes metrics. The controller defaults to dry-run behavior unless `POOLSIM_K8S_APPLY=true` is set. Even when apply mode is enabled, the controller writes only `poolsim.io/recommended-*` annotations; it does not edit application environment variables, ConfigMaps, Secrets, or Deployment container specs.

## Runtime Requirements

- Python 3.9 or newer in the sidecar/controller image.
- The Rust `poolsim` executable available in the container image.
- Kubernetes deployment annotations containing workload assumptions.
- Prometheus if you want to scrape the sidecar metrics.
- RBAC permissions if you use controller mode.

## Annotation Contract

Required annotations for sidecar and controller sizing:

| Annotation | Meaning |
| --- | --- |
| `poolsim.io/expected-rps` | Expected service request rate. |
| `poolsim.io/latency-p50-ms` | Observed or assumed p50 latency in milliseconds. |
| `poolsim.io/latency-p95-ms` | Observed or assumed p95 latency in milliseconds. |
| `poolsim.io/latency-p99-ms` | Observed or assumed p99 latency in milliseconds. |
| `poolsim.io/max-server-connections` | Database connection cap visible to this service. |
| `poolsim.io/min-pool-size` | Minimum candidate pool size. |
| `poolsim.io/max-pool-size` | Maximum candidate pool size. |

Required only for sidecar metrics:

| Annotation | Meaning |
| --- | --- |
| `poolsim.io/current-pool-size` | Current configured application pool size. |

Optional annotations:

| Annotation | Meaning |
| --- | --- |
| `poolsim.io/connection-overhead-ms` | Additional per-connection overhead assumption. |
| `poolsim.io/idle-timeout-ms` | Idle timeout assumption for sidecar CLI calls. |
| `poolsim.io/iterations` | Monte Carlo iteration count. |
| `poolsim.io/seed` | Random seed for deterministic simulation. |
| `poolsim.io/target-wait-p99-ms` | Target p99 queue wait threshold. |
| `poolsim.io/max-acceptable-rho` | Maximum acceptable utilization ratio. |
| `poolsim.io/distribution` | Latency distribution for sidecar mode. |
| `poolsim.io/queue-model` | Queue model for sidecar mode. |

The checked-in controller currently passes `connection-overhead-ms` and `iterations` as optional annotations. The sidecar also supports `idle-timeout-ms`, `seed`, `target-wait-p99-ms`, `max-acceptable-rho`, `distribution`, and `queue-model` through environment variables.

## Sidecar Metrics Exporter

The sidecar is a small HTTP server that exposes:

- `GET /healthz`: liveness response.
- `GET /metrics`: Prometheus text exposition.

It reads environment variables, builds a `poolsim simulate` command, and renders metrics.

### Sidecar Files

- `sidecar/poolsim_sidecar.py`: HTTP metrics exporter.
- `sidecar/deployment.yaml`: example deployment with annotations and Downward API environment projection.
- `sidecar/test_poolsim_sidecar.py`: unit tests.

### Sidecar Environment Variables

Required variables:

| Variable | Source |
| --- | --- |
| `POOLSIM_EXPECTED_RPS` | `poolsim.io/expected-rps` annotation. |
| `POOLSIM_LATENCY_P50_MS` | `poolsim.io/latency-p50-ms` annotation. |
| `POOLSIM_LATENCY_P95_MS` | `poolsim.io/latency-p95-ms` annotation. |
| `POOLSIM_LATENCY_P99_MS` | `poolsim.io/latency-p99-ms` annotation. |
| `POOLSIM_CURRENT_POOL_SIZE` | `poolsim.io/current-pool-size` annotation. |
| `POOLSIM_MAX_SERVER_CONNECTIONS` | `poolsim.io/max-server-connections` annotation. |
| `POOLSIM_MIN_POOL_SIZE` | `poolsim.io/min-pool-size` annotation. |
| `POOLSIM_MAX_POOL_SIZE` | `poolsim.io/max-pool-size` annotation. |

Optional variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `POOLSIM_CLI` | `poolsim` | CLI binary path. |
| `POOLSIM_SERVICE_NAME` | `unknown` | Service label used in Prometheus metrics. |
| `POD_NAMESPACE` | `default` | Namespace label used in metrics. |
| `POD_NAME` | `unknown` | Pod label used in metrics. |
| `POOLSIM_CONNECTION_OVERHEAD_MS` | unset | Adds `--connection-overhead-ms`. |
| `POOLSIM_IDLE_TIMEOUT_MS` | unset | Adds `--idle-timeout-ms`. |
| `POOLSIM_ITERATIONS` | unset | Adds `--iterations`. |
| `POOLSIM_SEED` | unset | Adds `--seed`. |
| `POOLSIM_TARGET_WAIT_P99_MS` | unset | Adds `--target-wait-p99-ms`. |
| `POOLSIM_MAX_ACCEPTABLE_RHO` | unset | Adds `--max-acceptable-rho`. |
| `POOLSIM_DISTRIBUTION` | unset | Adds `--distribution`. |
| `POOLSIM_QUEUE_MODEL` | unset | Adds `--queue-model`. |
| `POOLSIM_SIDECAR_PORT` | `9464` | HTTP listen port. |
| `POOLSIM_SIDECAR_ACCESS_LOG` | unset | Set to `1` to enable access logs. |

### Sidecar Command Shape

The sidecar builds a command like:

```bash
poolsim --format json simulate \
  --rps 180 \
  --p50 8 \
  --p95 30 \
  --p99 70 \
  --max-server-connections 100 \
  --min 2 \
  --max 20 \
  --connection-overhead-ms 2
```

### Sidecar Metrics

The exporter emits these gauges:

| Metric | Meaning |
| --- | --- |
| `poolsim_recommended_pool_size` | Recommended connection pool size. |
| `poolsim_recommendation_rho` | Utilization ratio at the recommended pool size. |
| `poolsim_recommendation_p99_queue_wait_ms` | Predicted p99 queue wait at the recommended pool size. |
| `poolsim_current_pool_size` | Current configured pool size from Kubernetes metadata. |

Labels:

- `service`
- `namespace`
- `pod`

Keep labels low-cardinality. Do not add request IDs, user IDs, or other high-cardinality values.

### Deploy The Sidecar Example

Review and edit the example image before applying:

```bash
kubectl apply -f integrations/kubernetes/sidecar/deployment.yaml
```

Port-forward for local testing:

```bash
kubectl port-forward deploy/checkout-api 9464:9464
curl http://localhost:9464/healthz
curl http://localhost:9464/metrics
```

## Controller Mode

The controller scans deployments in a namespace, finds deployments with Poolsim annotations, computes recommendations, and emits events to stdout. In apply mode, it patches recommendation annotations onto each deployment.

### Controller Files

- `controller.py`: Kubernetes API controller.
- `test_controller.py`: unit tests for command generation, annotation rendering, and patch behavior.

### Controller Environment Variables

| Variable | Default | Meaning |
| --- | --- | --- |
| `POOLSIM_K8S_NAMESPACE` | service-account namespace, then `default` | Namespace to reconcile. |
| `POOLSIM_K8S_API` | `https://kubernetes.default.svc` | Kubernetes API base URL. |
| `POOLSIM_K8S_TOKEN` | mounted service-account token | Bearer token. |
| `POOLSIM_K8S_INTERVAL_SECS` | `60` | Reconcile interval. |
| `POOLSIM_K8S_APPLY` | `false` | Set to `true`, `1`, or `yes` to patch deployments. |
| `POOLSIM_CLI` | `poolsim` | CLI binary path. |

### Recommendation Annotations Written By Controller

When `POOLSIM_K8S_APPLY=true`, the controller patches:

| Annotation | Meaning |
| --- | --- |
| `poolsim.io/recommended-pool-size` | Recommended pool size. |
| `poolsim.io/recommended-rho` | Utilization ratio at recommendation. |
| `poolsim.io/recommended-p99-queue-wait-ms` | Predicted p99 queue wait. |
| `poolsim.io/recommended-saturation` | Saturation classification. |

### Minimal RBAC

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

If you run the controller in dry-run mode only, remove `patch` from the verbs.

### Controller Event Output

The controller prints JSON events like:

```json
{
  "deployment": "checkout-api",
  "annotations": {
    "poolsim.io/recommended-pool-size": "9",
    "poolsim.io/recommended-rho": "0.72",
    "poolsim.io/recommended-p99-queue-wait-ms": "18.5",
    "poolsim.io/recommended-saturation": "Ok"
  },
  "applied": true
}
```

Ship stdout to your normal log collector if you want an audit trail.

## Run Tests

```bash
python3 -m unittest integrations/kubernetes/sidecar/test_poolsim_sidecar.py
python3 -m unittest integrations/kubernetes/test_controller.py
```

These tests validate command construction, missing-input errors, Prometheus metrics rendering, stable recommendation annotations, and apply-mode patch calls without requiring a live Kubernetes cluster.

## Operational Guidance

- Start controller mode with `POOLSIM_K8S_APPLY=false` and inspect stdout before enabling patches.
- Use conservative annotation values and review them like any other capacity assumption.
- Keep `max-server-connections` scoped to the budget available to this service, not necessarily the database global maximum.
- Alert on severe saturation or large recommendation drift, but require a human approval path before changing production pool settings automatically.
- Keep Poolsim image versions pinned in manifests.

## Sources

- Kubernetes annotations: <https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/>
- Kubernetes recommended labels: <https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/>
- Prometheus data model: <https://prometheus.io/docs/concepts/data_model/>

## Compatibility

These integrations are additive. They shell out to the existing `poolsim --format json simulate` command and do not change Rust APIs, CLI output schemas, REST routes, WebSocket routes, or config files.
