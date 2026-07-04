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
- [`../integrations/kubernetes/README.md`](../integrations/kubernetes/README.md): Kubernetes sidecar and controller patterns for surfacing sizing recommendations from deployment annotations.
- [`continuous-recommendations.md`](continuous-recommendations.md): Opt-in worker for repeated recommendation diff events and webhook delivery.
- [`../integrations/grafana/README.md`](../integrations/grafana/README.md): Grafana panel package for rendering `poolsim-web` sensitivity rows as a heatmap.
- [`../benchmarks/README.md`](../benchmarks/README.md): Benchmark result contract and summarizer for comparing Poolsim predictions to real pool runs.
- [`deployed-pool-survey.md`](deployed-pool-survey.md): Opt-in anonymized survey payload generator for pool configuration statistics.
- [`packaging.md`](packaging.md): Homebrew formula and release packaging notes.
- [`json-schema.md`](json-schema.md): JSON Schema files for documented config, telemetry, budget, scenario, batch, and gate-policy inputs.
- [`fixtures/README.md`](fixtures/README.md): Checked-in sample inputs used by the docs and docs-validation tests.

## Scope for Current Version

This documentation covers the sizing calculator implemented today in:

- `crates/poolsim-core`
- `crates/poolsim-cli`
- `crates/poolsim-web`

Runtime-enforcement documentation is outside this release; Poolsim remains a sizing, diagnostics, recommendation, and integration toolkit rather than a production connection-pool implementation.
