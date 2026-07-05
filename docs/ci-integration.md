# CI Integration

Poolsim provides drop-in CI assets for teams that want connection-pool safety checks in pull requests and deployment pipelines.

## GitHub Action

The repository root contains `action.yml`, a composite action that installs `poolsim-cli` and runs `poolsim gate`. GitHub's current composite-action model uses an action metadata file with `runs.using: composite`, so the gate can be reused from other repositories without requiring a JavaScript action bundle.

Example workflow:

```yaml
name: Capacity Gate

on:
  pull_request:

jobs:
  poolsim:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: gregorian-09/poolsim@v1
        with:
          policy: docs/fixtures/gate-policy.toml
          source: telemetry
          telemetry-config: docs/fixtures/telemetry.json
```

Prometheus response-file example:

```yaml
- uses: gregorian-09/poolsim@v1
  with:
    source: prometheus
    policy: docs/fixtures/gate-policy.toml
    prometheus-response-file: docs/fixtures/prometheus-responses.json
    service-name: checkout-api
    window: 5m
    current-pool-size: "8"
    max-server-connections: "100"
    connection-overhead-ms: "2"
    min: "2"
    max: "20"
```

The action fails the job using the existing `poolsim gate` exit-code contract.

## GitLab CI

Use [`templates/gitlab/poolsim-capacity-gate.gitlab-ci.yml`](../templates/gitlab/poolsim-capacity-gate.gitlab-ci.yml) as a copyable CI component:

```yaml
include:
  - local: templates/gitlab/poolsim-capacity-gate.gitlab-ci.yml
```

Override variables in your project pipeline when your policy or telemetry file lives elsewhere:

```yaml
variables:
  POOLSIM_POLICY: ops/poolsim/gate-policy.toml
  POOLSIM_TELEMETRY_CONFIG: ops/poolsim/telemetry.json
```

## GHCR Docker Image

The repository includes `.github/workflows/docker.yml` for publishing `poolsim-web` to GitHub Container Registry. It is intentionally not triggered on every push. It runs only for:

- manual `workflow_dispatch`
- version tags matching `v*`

The workflow uses maintained Docker actions and publishes `ghcr.io/<owner>/poolsim-web` from the checked-in `Dockerfile`.

The Dockerfile builds with `rust:1.84-alpine` by default and exposes a `RUST_VERSION` build argument for future toolchain updates:

```bash
docker build --build-arg RUST_VERSION=1.84 -t poolsim-web:local .
```

Use the GitHub Actions Docker workflow for full image verification when local disk space is constrained.

## Compatibility

These assets are wrappers around existing CLI commands. They do not change `poolsim gate`, `poolsim guard`, REST routes, config fields, or output schemas.
