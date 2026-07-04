# Poolsim Documentation

This folder contains user-facing documentation for the current Poolsim capabilities.

## Contents

- [`sizing-calculator.md`](sizing-calculator.md): End-to-end guide for what the sizing calculator does, required inputs, calculation pipeline, output interpretation, and usage through library/CLI/web targets.
- [`library-api.md`](library-api.md): Exhaustive `poolsim-core` reference with examples for all exported functions, constants, modules, enums, and structs.
- [`cli-reference.md`](cli-reference.md): Exhaustive command-line reference covering every subcommand, flag, config shape, output format, and exit-code path.
- [`web-api.md`](web-api.md): Exhaustive REST/WebSocket reference plus embedding examples for `build_app`, `AppState`, and `RateLimitState`.
- [`terraform-opentofu.md`](terraform-opentofu.md): Terraform/OpenTofu external-data adapter for connection-pool sizing as code.
- [`language-bindings.md`](language-bindings.md): Python, TypeScript, and Go bindings that delegate to the stable CLI JSON contract.
- [`ci-integration.md`](ci-integration.md): GitHub Action, GitLab CI, and GHCR Docker image integration guide for capacity gates and web deployment.
- [`../integrations/kubernetes/README.md`](../integrations/kubernetes/README.md): Kubernetes sidecar pattern that exposes sizing recommendations as Prometheus metrics.
- [`continuous-recommendations.md`](continuous-recommendations.md): Opt-in worker for repeated recommendation diff events and webhook delivery.
- [`../integrations/grafana/README.md`](../integrations/grafana/README.md): Grafana panel package for rendering `poolsim-web` sensitivity rows as a heatmap.
- [`json-schema.md`](json-schema.md): JSON Schema files for documented config, telemetry, budget, scenario, batch, and gate-policy inputs.
- [`feature-roadmap.md`](feature-roadmap.md): Tracked implementation tasks and acceptance criteria for adoption, observability, deeper simulation, developer experience, and validation features.
- [`fixtures/README.md`](fixtures/README.md): Checked-in sample inputs used by the docs and docs-validation tests.

## Scope for Current Version

This documentation covers the sizing calculator implemented today in:

- `crates/poolsim-core`
- `crates/poolsim-cli`
- `crates/poolsim-web`

Future runtime-enforcement documentation is intentionally out of scope for now.
