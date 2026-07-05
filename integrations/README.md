# Poolsim Integrations

This directory contains opt-in integrations for teams that want to use Poolsim outside the Rust CLI/library directly.

The integrations do not change the sizing model and do not mutate production connection pool settings by default. They delegate to the stable Poolsim CLI or `poolsim-web` API so every environment uses the same recommendation engine.

## `action.yml` Versus `integrations/`

`action.yml` is the reusable GitHub Action entrypoint for this repository. It lets another repository run Poolsim as a capacity gate with a short workflow step:

```yaml
- uses: gregorian-09/poolsim@v0.2.1
  with:
    policy: capacity-policy.toml
    source: telemetry
    telemetry-config: telemetry.json
```

Use `action.yml` when you want a drop-in GitHub Actions gate that installs `poolsim-cli`, runs `poolsim gate`, and fails the CI job when the configured policy is unsafe.

The `integrations/` directory is broader. It contains examples and adapters for systems that are not GitHub Actions:

- `continuous/`: scheduled recommendation polling that emits `PoolRecommendationDiff` events and optional webhooks.
- `grafana/`: a Grafana panel package that renders `poolsim-web` sensitivity rows as a heatmap.
- `kubernetes/`: a sidecar metrics exporter and a controller for deployment recommendation annotations.
- `terraform/`: a Terraform/OpenTofu external-provider adapter for connection-pool sizing as infrastructure data.

## Choosing An Integration

Use GitHub Action capacity gate when:

- You want CI to block unsafe pool changes or unsafe traffic assumptions.
- You already have telemetry snapshots or captured Prometheus responses in CI.
- You want a simple `uses: gregorian-09/poolsim@...` workflow step.

Use Terraform/OpenTofu when:

- Pool size is part of infrastructure review.
- You want `terraform plan` to show recommended sizing values.
- You want to feed Poolsim output into IaC-managed service variables.

Use Kubernetes sidecar when:

- You want recommendation metrics beside a workload pod.
- Prometheus should scrape current and recommended pool settings.
- You do not want Poolsim to patch Kubernetes resources.

Use Kubernetes controller when:

- You want recommendations written back to deployment annotations.
- Platform tooling or alerts should read annotations from the Kubernetes API.
- You can provide RBAC permissions for `get`, `list`, and optionally `patch` on deployments.

Use Grafana when:

- Engineers already review capacity inside Grafana dashboards.
- You want a visual heatmap of candidate pool sizes and risk.
- `poolsim-web` is reachable from Grafana or from the user's browser, depending on deployment topology.

Use continuous mode when:

- You have periodically updated Prometheus response snapshots.
- You want drift events over time.
- You want to post recommendation changes to a webhook such as Slack, PagerDuty, or an internal automation endpoint.

## Common Runtime Requirement

Most integrations need the `poolsim` CLI binary:

```bash
cargo install poolsim-cli
poolsim --version
```

The Grafana panel is the exception: it talks to `poolsim-web` over HTTP instead of shelling out to the CLI.

## Public API Compatibility

These integrations are additive adoption layers. They do not remove, rename, or change Rust APIs, CLI commands, REST routes, WebSocket routes, config schemas, or output fields.

Where an integration consumes Poolsim output, it expects the existing JSON contract from `poolsim --format json` or `poolsim-web` routes.

## Validation

Run the integration checks from the repository root:

```bash
python3 -m unittest integrations/continuous/test_poolsim_continuous.py
python3 -m unittest integrations/terraform/external/test_poolsim_sizing.py
python3 -m unittest integrations/kubernetes/sidecar/test_poolsim_sidecar.py
python3 -m unittest integrations/kubernetes/test_controller.py
python3 integrations/grafana/tests/validate_grafana_plugin.py
```

These tests validate command construction, payload flattening, metrics rendering, controller annotations, event creation, and Grafana plugin metadata without requiring a live cluster, Terraform backend, Grafana server, or Prometheus server.

## Support

- Main documentation: <https://github.com/gregorian-09/poolsim/tree/main/docs>
- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
