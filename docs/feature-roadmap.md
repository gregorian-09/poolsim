# Feature Roadmap

This roadmap tracks the adoption, observability, simulation, developer-experience, and validation features planned for Poolsim. Every item follows the project compatibility rule: existing Rust APIs, CLI commands, flags, REST routes, WebSocket events, serialized fields, config keys, exit codes, and published examples must keep working unless a future release explicitly declares a breaking change.

## Compatibility Rules

Implementation work must follow these constraints:

- Add new commands, flags, routes, files, schemas, and output fields instead of renaming or removing existing ones.
- Keep existing defaults unchanged so current users receive identical behavior without opting in.
- Add optional config fields with defaults when a schema must expand.
- Keep old output formats stable; new formats are additive.
- Add tests for old behavior and new behavior in the same change when a feature touches an existing code path.
- Document every new public command, route, schema, package, or generated artifact before release.

## Integration And Adoption Tasks

### Language Bindings

Status: planned.

Scope:

- Python package installable as `pip install poolsim`.
- TypeScript package installable as `npm install poolsim`.
- Go package or command wrapper for Go services.

Acceptance criteria:

- Bindings call the stable sizing model without duplicating formulas by hand.
- Package APIs expose simulation, evaluation, sweep, telemetry recommendation, doctor, config generation, scenario comparison, and budget planning.
- Versioning is tied to the workspace `VERSION` file.
- Examples compile or run in each package test suite.
- README files explain parity with the Rust CLI and core crate.

### Terraform And OpenTofu Provider

Status: complete for external-data adapter; native provider remains future work.

Scope:

- Provide a `poolsim_sizing` resource or data source for connection pool sizing as code.
- Let teams commit workload assumptions and generated pool settings beside infrastructure.

Acceptance criteria:

- Provider schema mirrors the stable Poolsim config schema.
- Plans show recommendation diffs without mutating external systems.
- Acceptance tests cover configuration, drift, and invalid input.
- Documentation includes Terraform and OpenTofu examples.

### GitHub Action And GitLab CI Component

Status: in progress.

Scope:

- Add a drop-in capacity gate that reads a policy file and fails unsafe changes.
- Support GitHub Actions and GitLab CI with the existing `gate` command.

Acceptance criteria:

- CI examples use `docs/fixtures/gate-policy.toml` compatible policy fields.
- Failure mode uses the existing gate exit codes.
- Documentation shows telemetry-file and Prometheus-response workflows.
- Tests verify the checked-in templates reference valid commands and fixture paths.

### Kubernetes Sidecar Or Controller

Status: planned.

Scope:

- Watch deployment annotations such as `poolsim.io/expected-rps`.
- Expose recommendations through a metrics endpoint.

Acceptance criteria:

- Annotation schema is documented and versioned.
- Sidecar/controller does not alter runtime pool settings by default.
- Prometheus metrics have stable names and labels.
- Kubernetes manifests are tested with static validation.

## Observability And Live Data Tasks

### OpenTelemetry Native Ingestion

Status: planned.

Scope:

- Accept OTLP traces or metrics directly instead of only JSON snapshots or Prometheus response files.

Acceptance criteria:

- OTLP ingestion maps request rate and latency percentiles into `TelemetrySnapshot` without changing existing import behavior.
- Missing metric errors are explicit and documented.
- Tests cover metric name mapping, units, histograms, summaries, and invalid payloads.

### Grafana Plugin

Status: planned.

Scope:

- Provide a Grafana panel that queries `poolsim-web` and renders the sensitivity table as a heatmap with current pool size overlaid.

Acceptance criteria:

- Plugin uses existing REST endpoints unless a new endpoint is strictly additive.
- Build and test scripts validate the panel package.
- Documentation includes datasource setup and screenshots.

### Continuous Recommendation Mode

Status: planned.

Scope:

- Let `poolsim-web` poll Prometheus or InfluxDB on a schedule.
- Emit `PoolRecommendationDiff` events and optional webhooks for Slack or PagerDuty.

Acceptance criteria:

- Polling is opt-in and disabled by default.
- Event payloads include stable version metadata.
- Webhook delivery is retried with bounded backoff.
- Tests cover scheduler behavior, diff generation, and webhook failure paths.

## Deeper Simulation And Sizing Tasks

### Connection Overhead Profiles

Status: planned.

Scope:

- Ship named profiles for common databases and proxies, such as PostgreSQL, MySQL, and RDS Proxy.

Acceptance criteria:

- Profiles are additive presets over existing explicit `connection_overhead_ms` behavior.
- Explicit user values always override profile defaults.
- Documentation explains that profiles are sizing assumptions, not vendor guarantees.

### Connection Acquisition Time Modeling

Status: planned.

Scope:

- Model the two-stage queue: waiting for a pool slot, then waiting for database service.

Acceptance criteria:

- Existing single-stage queue behavior remains the default.
- New options are opt-in and documented with units.
- Tests cover acquisition timeout, p95/p99 acquisition wait, and saturated cases.

### Transaction-Level Simulation

Status: planned.

Scope:

- Model mixes of query types with different service times and arrival rates.

Acceptance criteria:

- Transaction profiles can be loaded from config files without breaking current workload config.
- Weighted mixes validate to 100 percent or a documented equivalent.
- Tests cover fast reads, slow writes, batch jobs, and malformed weights.

### Connection Leak Modeling

Status: planned.

Scope:

- Simulate gradual connection leakage and its effect on saturation over time.

Acceptance criteria:

- Leak modeling is opt-in and never changes existing simulations unless requested.
- Output identifies time-to-saturation and recommended guardrails.
- Documentation maps results to `connectionTimeout` and leak-detection settings.

## Developer Experience Tasks

### `poolsim init`

Status: in progress.

Scope:

- Add a setup command that generates a config file and capacity-gate policy from a small set of inputs.

Acceptance criteria:

- Non-interactive flags work in CI.
- Interactive prompting is optional and never required for automation.
- Generated files pass the docs fixture tests and schema validation.

### Explainable Output

Status: in progress.

Scope:

- Add `--explain` for prose output that explains why a recommendation is safe, marginal, or unsafe.

Acceptance criteria:

- Existing machine-readable output is unchanged unless `--explain` is passed.
- The explanation names pool size, request rate, rho, p99 queue wait, and risk reason.
- Tests cover simulation, fixed-size evaluation, and step-load explanations.

### Web UI

Status: planned.

Scope:

- Add a minimal browser UI for `poolsim-web` so users can paste workload numbers and inspect the sensitivity table live.

Acceptance criteria:

- UI uses existing public REST endpoints.
- Static assets are optional and do not change API behavior.
- Tests cover serving the page and submitting a simulation request.

## Community And Validation Tasks

### Sizing Benchmark Suite

Status: planned.

Scope:

- Run Poolsim recommendations against real pools under controlled load and publish the prediction error.

Acceptance criteria:

- Benchmarks are reproducible with documented infrastructure.
- Results compare recommended pool size, real p95/p99 latency, and predicted queue wait.
- HikariCP and sqlx are first-class benchmark targets.

### Deployed-Pool Survey

Status: planned.

Scope:

- Add opt-in anonymous telemetry for pool configuration statistics only.

Acceptance criteria:

- No application data, query text, credentials, hostnames, or private identifiers are collected.
- Opt-in consent is explicit and documented.
- Published aggregate data cannot identify individual services.

## Quick Wins

### Docker Image For `poolsim-web`

Status: in progress.

Acceptance criteria:

- Image build is documented for local use.
- GHCR publishing workflow is manual or tag-based, not every push.
- The workflow uses maintained action versions.

### JSON Schema For Config Files

Status: in progress.

Acceptance criteria:

- Schemas exist for simulation config, batch config, scenario comparison, budget plan, telemetry import, and gate policy.
- Documentation shows IDE integration and validation examples.
- Tests ensure schemas are valid JSON and example fixtures conform to the expected top-level shape.

### HTML Output

Status: in progress.

Acceptance criteria:

- `--format html` emits a self-contained report page.
- Existing table, JSON, and CSV outputs remain unchanged.
- Tests cover HTML output for all major commands.

### Homebrew Tap Formula

Status: planned.

Acceptance criteria:

- Formula points to published release artifacts.
- Documentation shows install and upgrade commands.
- Release process explains when the tap updates.
