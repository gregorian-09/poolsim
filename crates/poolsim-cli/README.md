# poolsim-cli

[![Crates.io](https://img.shields.io/crates/v/poolsim-cli.svg)](https://crates.io/crates/poolsim-cli)
[![docs.rs](https://img.shields.io/docsrs/poolsim-cli)](https://docs.rs/poolsim-cli)
[![CI](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml/badge.svg)](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml)
[![Docs Coverage](https://img.shields.io/badge/docs%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/docs/README.md)
[![Workspace Coverage](https://img.shields.io/badge/workspace%20line%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_coverage_threshold.py)
[![Examples Coverage](https://img.shields.io/badge/examples%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_examples_coverage.py)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/gregorian-09/poolsim/blob/main/LICENSE)

`poolsim-cli` is the command-line interface for the Poolsim connection-pool sizing calculator.

It is built for backend engineers, platform teams, SREs, and CI pipelines that need repeatable, machine-readable pool-sizing recommendations without embedding Rust code directly.

## What Is New In `0.3.0`

`0.3.0` is an additive CLI release that turns Poolsim from a local sizing calculator into a broader backend capacity-planning toolkit. Existing `simulate`, `evaluate`, `sweep`, `batch`, output formats, config-file behavior, and exit-code contracts remain available. The release adds and documents workflows for telemetry ingestion, CI safety gates, framework config generation, platform integrations, and release packaging.

### Telemetry And Observability Workflows

The CLI can now work with several production-data paths:

- `poolsim import telemetry`: reads checked-in or exported telemetry JSON/TOML and computes a current-vs-recommended pool diff.
- `poolsim import prometheus`: reads captured Prometheus response files or live Prometheus-compatible query results.
- `poolsim import otlp`: reads OpenTelemetry OTLP metric-export JSON and maps request-rate and latency metrics into a Poolsim workload.

These commands are useful when teams want recommendations based on observed traffic instead of hand-written estimates.

### CI And Deployment Safety Workflows

`0.3.0` documents and validates the CI-facing commands around the same sizing model:

- `poolsim gate`: evaluates telemetry against a TOML policy and returns failure when thresholds are unsafe.
- `poolsim guard`: produces deployment-friendly fields such as `deployment_safe`, `exit_code`, and `reason` for CI/CD systems.
- `--warn-exit`: lets warning/advisory outcomes return exit code `3` when CI needs to distinguish warning from pass.

The repository now includes GitHub Action, GitLab CI, Docker/GHCR, and Homebrew packaging documentation so teams can adopt these gates without writing custom glue from scratch.

### Diagnostics And Explainability

The release expands human-facing workflows:

- `poolsim doctor` classifies a pool as healthy, too small, too large, close to saturation, or critical.
- `--explain` emits prose reasoning while preserving machine-readable stdout for JSON/CSV/table consumers.
- `--format html` creates self-contained shareable reports for sizing discussions and design reviews.

These are intended for code review, incident follow-up, and capacity planning conversations where raw JSON is not enough.

### Planning And Adoption Workflows

The CLI also includes workflows for turning recommendations into operational decisions:

- `poolsim budget`: allocates a shared database `max_connections` budget across services and replicas.
- `poolsim compare`: compares normal, peak, and incident scenarios side by side.
- `poolsim generate-config`: emits framework snippets for HikariCP, Spring Boot, SQLAlchemy, Prisma, node-postgres, `sqlx`, and `deadpool`.
- `poolsim init`: creates starter simulation config and capacity-gate policy files.

These commands keep the sizing recommendation separate from runtime enforcement. Poolsim recommends, explains, and guards; your runtime pool still enforces the actual connection limits.

### Non-Rust And Platform Integrations

The release documentation now covers how non-Rust users consume the CLI contract:

- Python package wrapper.
- TypeScript package wrapper.
- Go module wrapper.
- Terraform/OpenTofu external provider adapter.
- Kubernetes sidecar metrics exporter and controller annotations.
- Grafana sensitivity heatmap panel.
- Continuous recommendation-diff worker with webhook delivery.

All of these delegate to the stable CLI JSON or web API contracts instead of reimplementing queueing formulas.

### Compatibility And Quality Notes

This release is intended to be backward-compatible with `0.2.x` command usage. It does not intentionally remove, rename, or narrow existing commands, flags, config keys, output fields, REST routes, or exit-code semantics. The upstream repository enforces documentation coverage, executable docs, examples coverage, and `100%` workspace line coverage as part of CI.

### When To Upgrade

Upgrade to `0.3.0` if you want CI gates, telemetry imports, OTLP ingestion, config generation, doctor diagnostics, HTML/explainable reporting, platform integration docs, updated package metadata, and the current release automation. Existing basic sizing scripts should continue to work.

## Install

```bash
cargo install poolsim-cli
```

Verify the install:

```bash
poolsim --help
poolsim --version
```

Depending on your Cargo install layout, the installed binary may be available as `poolsim`. In this README, examples use `poolsim` for installed usage and `cargo run -p poolsim-cli --` for workspace usage.

## Command Summary

Core sizing commands:

- `poolsim simulate`: full recommendation workflow.
- `poolsim evaluate`: score a fixed pool size.
- `poolsim sweep`: generate sensitivity rows.
- `poolsim batch`: run several simulation requests from one file.

Operational commands:

- `poolsim compare`: compare named traffic scenarios.
- `poolsim budget`: allocate one database connection budget across services.
- `poolsim plan serverless`: calculate serverless app-side pool footprint across concurrent execution environments.
- `poolsim graph ownership`: map which layer owns app, pooler, and database backend connection capacity.
- `poolsim classify endpoint`: identify direct, pooled, proxied, edge-managed, or unknown database endpoints and redact secrets.
- `poolsim check pooler`: detect external-pooler/session-feature compatibility risks.
- `poolsim check session-state`: add client-aware prepared-statement and session-state remediation for popular backend libraries.
- `poolsim import pooler-evidence`: summarize observed pooler client/backend counters from a JSON snapshot.
- `poolsim import pgbouncer-pools`: parse captured PgBouncer `SHOW POOLS` output into the same pooler evidence report.
- `poolsim doctor`: explain whether a current pool is healthy.
- `poolsim doctor pgbouncer-pools`: compare application-pool counters with downstream PgBouncer client/backend evidence.
- `poolsim generate-config`: produce framework-specific pool config snippets.

Telemetry and CI commands:

- `poolsim import telemetry`: recommendation diff from telemetry files.
- `poolsim import prometheus`: recommendation diff from Prometheus-compatible data.
- `poolsim import otlp`: recommendation diff from OpenTelemetry OTLP metric-export JSON.
- `poolsim gate`: policy check for traffic and latency assumptions.
- `poolsim guard`: deployment guard output for CI/CD pipelines.

Global output formats:

- `--format table`
- `--format json`
- `--format csv`

Global exit behavior:

- `--warn-exit`: warning outcomes return exit code `3` instead of `0`.

## Quick Start

Run a recommendation from flags:

```bash
poolsim --format json simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24
```

Run the checked-in fixture from a workspace checkout:

```bash
cargo run -p poolsim-cli -- --format json simulate --config docs/fixtures/cli-config.json
```

Interpret the important fields:

- `optimal_pool_size`: per-replica pool size recommendation.
- `confidence_interval`: uncertainty band around the recommendation.
- `cold_start_min_pool_size`: useful lower bound for warm startup.
- `utilisation_rho`: modeled utilization ratio.
- `p99_queue_wait_ms`: modeled p99 wait before a connection is available.
- `saturation`: `Ok`, `Warning`, or `Critical`.

## Endpoint And Pooler Examples

Classify a provider-managed pooled endpoint before using it for a workflow:

```bash
poolsim --format json classify endpoint \
  --endpoint 'postgres://user:secret@aws-0-us.pooler.supabase.com:6543/postgres?password=secret' \
  --workflow migration
```

Check session-feature compatibility with transaction pooling:

```bash
poolsim --format json check pooler \
  --pooler pg-bouncer \
  --mode transaction \
  --uses temporary-tables \
  --uses advisory-locks
```

Add client-aware prepared-statement guidance:

```bash
poolsim --format json check session-state \
  --client sqlx \
  --pooler pg-bouncer \
  --mode transaction \
  --uses prepared-statements \
  --max-prepared-statements 100
```

The session-state check reuses the generic pooler compatibility result, then adds `client_guidance` for Prisma, node-postgres, `sqlx`, SQLAlchemy asyncpg, PostgREST, PgJDBC, generic PostgreSQL clients, and unknown clients.

Import observed pooler counters:

```bash
poolsim --format json import pooler-evidence \
  --config docs/fixtures/pooler-evidence.json
```

The pooler evidence import reports observed client connections, backend connections, utilization against known limits, waiting clients, saturation, and confidence.

Import PgBouncer native `SHOW POOLS` output without manually normalizing JSON:

```bash
poolsim --format json import pgbouncer-pools \
  --file docs/fixtures/pgbouncer-show-pools.csv \
  --label checkout-pgbouncer \
  --pooler-backend-limit 30
```

Use `psql -p 6432 -d pgbouncer --csv -c "SHOW POOLS;"` for repeatable captures. The command also accepts default aligned `psql` table output for copy-paste diagnostics.

Poolsim exits with code `2` for clear incompatibility, and the JSON output includes remediation-oriented findings.

Diagnose whether application or downstream pooler capacity is the limiting layer:

```bash
poolsim --format json doctor pgbouncer-pools \
  --file docs/fixtures/pgbouncer-show-pools.csv \
  --label checkout-production \
  --pooler-backend-limit 15 \
  --application-active 4 \
  --application-max 16 \
  --application-waiting 0
```

The command preserves the nested `PoolerEvidenceReport` and adds a cross-layer status. `downstream-pooler-saturated` means the pooler backend reached its supplied limit; `downstream-pooler-waiting` means clients queued for backend capacity; `application-pool-saturated` means the app pool reached its own limit; and `needs-review` means the comparison lacks required evidence or is close to a limit. See [`docs/downstream-pooler-diagnosis.md`](../../docs/downstream-pooler-diagnosis.md) for all flags, output fields, precedence rules, and library examples.

## Serverless Concurrency Example

Calculate worst-case app-side pool footprint for a Lambda-style workload:

```bash
poolsim --format json plan serverless \
  --platform aws-lambda \
  --max-concurrent-invocations 120 \
  --reserved-concurrency 80 \
  --pool-size 2 \
  --database-backend-limit 240 \
  --warm-reuse-ratio 0.72
```

The key output is `worst_case_app_pool_connections`. Poolsim calculates it as `effective_concurrency * app_pool_size_per_environment`. If traffic uses an external pooler, Poolsim still reports the app-side footprint but leaves `direct_database_backend_upper_bound` as `null` until backend pooler evidence proves the real database footprint.

## Connection Ownership Graph Example

Map the connection layers for a service using RDS Proxy:

```bash
poolsim --format json graph ownership \
  --service-name checkout-api \
  --runtime-units 12 \
  --pool-size 10 \
  --endpoint-kind database-proxy \
  --external-pooler rds-proxy \
  --pooler-client-limit 1000 \
  --pooler-backend-limit 90 \
  --database-backend-limit 120 \
  --session-pinning-risk medium
```

The output separates `application-pool`, `pooler-client`, `pooler-backend`, and `database-backend` nodes. Use this before deciding whether a connection count is only client-side capacity or real backend database capacity.

## Telemetry Diff Example

Use telemetry when production already has a pool size and you want to know whether to change it.

```bash
poolsim --format json import telemetry --config docs/fixtures/telemetry.json
```

Override the current pool size without editing the file:

```bash
poolsim --format json import telemetry \
  --config docs/fixtures/telemetry.json \
  --current-pool-size 10
```

The output includes:

- current pool size
- recommended pool size
- signed delta
- increase/decrease/keep classification
- current pool evaluation
- recommended pool report

## Prometheus Import Example

Use captured Prometheus responses for deterministic local or CI checks:

```bash
poolsim --format json import prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

Use live Prometheus by providing `--endpoint` plus query flags:

```bash
poolsim --format json import prometheus \
  --endpoint http://localhost:9090 \
  --rps-query 'sum(rate(http_requests_total[5m]))' \
  --p50-query 'histogram_quantile(0.50, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000' \
  --p95-query 'histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000' \
  --p99-query 'histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000' \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --min 2 \
  --max 20
```

## Scenario Comparison Example

Compare normal, peak, and incident workloads without running separate commands manually:

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

Use another baseline:

```bash
poolsim --format csv compare \
  --config docs/fixtures/scenarios.json \
  --baseline peak
```

Use this in design reviews to answer:

- How much larger should the pool be at peak?
- Which incident scenario drives the worst saturation?
- How much p99 queue wait changes from baseline?

## Database Budget Planner Example

Use `budget` after each service has a per-replica recommendation. It plans against a global database connection ceiling.

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

TOML input is also supported:

```bash
poolsim --format table budget --config docs/fixtures/budget.toml
```

Minimal budget input shape:

```json
{
  "max_connections": 120,
  "reserved_connections": 20,
  "safety_margin_connections": 10,
  "services": [
    {
      "name": "checkout-api",
      "replicas": 6,
      "current_pool_size": 8,
      "min_pool_size": 4,
      "max_pool_size": 12,
      "recommended_pool_size": 10,
      "priority": 5
    }
  ]
}
```

Budget statuses:

- `Pass`: every requested service pool fits.
- `Warning`: minimums fit, but at least one recommendation is reduced.
- `Critical`: service minimums do not fit.

## CI Gate Example

Fail a pull request or deployment if new assumptions exceed policy:

```bash
poolsim --format json gate \
  --policy docs/fixtures/gate-policy.toml \
  telemetry \
  --config docs/fixtures/telemetry.json
```

Use `guard` when CI/CD systems need explicit deployment fields:

```bash
poolsim --format json guard \
  --policy docs/fixtures/gate-policy.toml \
  --max-current-rho 0.95 \
  telemetry \
  --config docs/fixtures/telemetry.json
```

`guard` returns fields like:

- `deployment_safe`
- `exit_code`
- `reason`
- nested gate checks

## Doctor Example

Diagnose current pool health:

```bash
poolsim --format json doctor telemetry --config docs/fixtures/telemetry.json
```

`doctor` is useful when you need an explanation for humans, not only a size delta. It reports findings such as:

- too small
- too large
- close to saturation
- critical saturation
- healthy

## Config Generator Example

Generate runtime configuration from a recommendation:

```bash
poolsim --format json generate-config \
  --framework sqlx \
  --pool-name checkout-pool \
  telemetry \
  --config docs/fixtures/telemetry.json
```

Supported frameworks:

- `hikaricp`
- `spring-boot`
- `sqlalchemy`
- `prisma`
- `node-pg`
- `sqlx`
- `deadpool`

Always compare the generated per-replica pool size against your total database connection budget. Use `poolsim budget` when several services share the same database.

## Exit Codes

- `0`: success, healthy, or warning without `--warn-exit`.
- `1`: command failure, parse failure, validation failure, IO failure, or internal error.
- `2`: critical outcome.
- `3`: warning/advisory outcome when `--warn-exit` is enabled.

## Getting The Most From The CLI

- Use JSON output in CI and automation.
- Use table output for human reviews.
- Use CSV output for spreadsheets and ad-hoc analysis.
- Commit representative scenario and budget files with your service repository.
- Re-run `simulate`, `doctor`, and `budget` whenever traffic, latency, replicas, database limits, or query behavior changes.
- Keep administrative and migration connections in `reserved_connections` when using the budget planner.
- Use `--seed` for reproducible CI outputs.

## Quality And CI Guarantees

The upstream repository currently enforces:

- `cargo check --workspace`
- `cargo test --workspace`
- `RUSTFLAGS="-D missing_docs"` checks for all crates
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- `cargo test --workspace --doc`
- executable docs fixtures for CLI examples
- `cargo test --workspace --examples`
- `100%` workspace line coverage
- `100%` `poolsim-core/src` line coverage
- `100%` example-file coverage
- docs-folder and public-API documentation coverage scripts

## Support

- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
- Detailed CLI guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/cli-reference.md>
- Changelog: <https://github.com/gregorian-09/poolsim/blob/main/CHANGELOG.md>

When opening an issue, include the command, full flags with secrets removed, input file shape, CLI version, operating system, expected output, actual output, and exit code.

## Quick Start Config Generation

Create starter files with `poolsim init --framework sqlx --database postgres --expected-rps 180`. The command writes a runnable `poolsim.json` simulation config and `poolsim-gate-policy.toml` capacity-gate policy, refusing to overwrite existing files unless `--force` is passed.

## Explainable Output

Add `--explain` to `simulate`, `evaluate`, or `sweep` to emit prose reasoning on stderr while keeping stdout in the selected `--format`. This lets CI keep parsing JSON while humans read why a pool size is safe, close to saturation, or unsafe.
