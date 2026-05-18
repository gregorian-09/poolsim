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

## What Is New In `0.2.1`

`0.2.1` is an API-compatible patch over the additive `0.2.0` feature release. It keeps the CLI operational workflows and aligns the published crate set with the restored `poolsim-core` no-default-features WASM build.

New and expanded workflows include:

- `budget`: allocate a global database connection budget across services and replicas.
- `compare`: compare normal, peak, and incident scenarios side by side.
- `import telemetry`: load production telemetry from JSON or TOML and compute a recommendation diff.
- `import prometheus`: use captured or live Prometheus-compatible query responses.
- `gate`: fail CI when new assumptions exceed safety policy.
- `guard`: deployment-safe wrapper around gate output for CI/CD systems.
- `doctor`: diagnose whether a pool is too small, too large, close to saturation, or healthy.
- `generate-config`: generate runtime config snippets for HikariCP, Spring Boot, SQLAlchemy, Prisma, node-postgres, sqlx, and deadpool.
- Expanded JSON, CSV, and table output coverage across command workflows.
- Comprehensive executable docs fixtures and `100%` workspace line coverage enforcement.

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
- `poolsim doctor`: explain whether a current pool is healthy.
- `poolsim generate-config`: produce framework-specific pool config snippets.

Telemetry and CI commands:

- `poolsim import telemetry`: recommendation diff from telemetry files.
- `poolsim import prometheus`: recommendation diff from Prometheus-compatible data.
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
