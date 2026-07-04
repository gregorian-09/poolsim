# Poolsim CLI Reference

## Purpose

This is the exhaustive user guide for the `poolsim` command-line interface.

It covers:

- Every subcommand
- Every global flag
- Every subcommand-specific flag
- JSON and TOML config formats
- Batch input formats
- Telemetry import and recommendation diff
- Database connection budget planning
- Sample-file input
- Output formats
- Exit-code behavior

The CLI binary is `poolsim`.

Checked-in runnable fixture files live under `docs/fixtures/`:

- `docs/fixtures/cli-config.json`
- `docs/fixtures/cli-config.toml`
- `docs/fixtures/batch.json`
- `docs/fixtures/batch.toml`
- `docs/fixtures/scenarios.json`
- `docs/fixtures/scenarios.toml`
- `docs/fixtures/budget.json`
- `docs/fixtures/budget.toml`
- `docs/fixtures/telemetry.json`
- `docs/fixtures/prometheus-responses.json`
- `docs/fixtures/gate-policy.toml`
- `docs/fixtures/latencies.txt`

## Command Summary

Available subcommands:

- `simulate`
- `evaluate`
- `sweep`
- `batch`
- `compare`
- `budget`
- `import telemetry`
- `import prometheus`
- `gate telemetry`
- `gate prometheus`
- `guard telemetry`
- `guard prometheus`
- `doctor telemetry`
- `doctor prometheus`
- `generate-config telemetry`
- `generate-config prometheus`
- `generate-config simulate`

Global flags:

- `--format <table|json|csv|html>`
- `--warn-exit`

## Global Options

### `--format`

Controls output format.

Supported values:

- `table`
- `json`
- `csv`
- `html`

Examples:

```bash
poolsim --format table simulate --config docs/fixtures/cli-config.toml
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 12
poolsim --format csv sweep --config docs/fixtures/cli-config.json
```
poolsim --format html simulate --config docs/fixtures/cli-config.json > poolsim-report.html
```

Use `html` when you need a self-contained report that can be shared in review comments, artifacts, or incident notes. The HTML report includes a summary plus the raw JSON payload so machine-readable values remain visible.

### `--warn-exit`

When enabled, warning-level outcomes can return exit code `3` instead of `0`.

Example:

```bash
poolsim --warn-exit simulate --config docs/fixtures/cli-config.json
```

## `simulate`

### Purpose

Runs the full recommendation workflow.

This is the highest-level CLI command and usually the first one users should try.

### Forms

Config-driven:

```bash
poolsim simulate --config docs/fixtures/cli-config.json
```

Flags-only:

```bash
poolsim simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24
```

Config plus overrides:

```bash
poolsim simulate \
  --config docs/fixtures/cli-config.toml \
  --rps 260 \
  --iterations 20000 \
  --distribution gamma \
  --queue-model mdc
```

### `simulate`-specific flags

#### `--pool-size`

Evaluates a single pool size from within the `simulate` command path.

Example:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --pool-size 10 --format json
```

#### `--sweep`

Generates the full sensitivity surface from the `simulate` command path.

Example:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --sweep --format csv
```

Conflict rule:

- `--pool-size` and `--sweep` cannot be used together.

## `evaluate`

### Purpose

Scores one fixed pool size against the workload.

### Example

```bash
poolsim evaluate --config docs/fixtures/cli-config.json --pool-size 12
```

With explicit flags:

```bash
poolsim evaluate \
  --pool-size 12 \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --iterations 10000 \
  --distribution log-normal \
  --queue-model mmc
```

## `sweep`

### Purpose

Returns all candidate pool sizes in the configured range together with queue-wait and risk metrics.

### Example

```bash
poolsim sweep --config docs/fixtures/cli-config.json
```

Tighter range override:

```bash
poolsim sweep --config docs/fixtures/cli-config.json --min 4 --max 18 --format json
```

## `batch`

### Purpose

Runs multiple simulation requests from a single batch file.

### Example

```bash
poolsim batch --config docs/fixtures/batch.json --format json
```

## `compare`

### Purpose

Runs named scenarios side by side and reports how each scenario differs from a baseline.

Use `compare` when you want one report for normal traffic, peak traffic, and incident traffic instead of running separate simulations by hand.

The JSON output is a `ScenarioComparisonReport`:

- `baseline`: scenario name used as the delta reference
- `worst_saturation`: worst saturation level across all scenarios
- `rows`: one row per scenario

Each row includes:

- `name`: scenario name
- `is_baseline`: whether the row is the baseline scenario
- `requests_per_second`: workload request rate for the scenario
- `optimal_pool_size`: recommended pool size for the scenario
- `pool_size_delta`: recommended-size delta against the baseline
- `p99_queue_wait_ms`: scenario p99 queue wait
- `p99_queue_wait_delta_ms`: p99 queue-wait delta against the baseline
- `mean_queue_wait_ms`: scenario mean queue wait
- `mean_queue_wait_delta_ms`: mean queue-wait delta against the baseline
- `utilisation_rho`: scenario utilization ratio
- `utilisation_rho_delta`: utilization delta against the baseline
- `saturation`: scenario saturation label
- `report`: full underlying `SimulationReport`

Default baseline behavior:

- `--baseline <name>` overrides the file baseline
- file-level `baseline` is used when present
- otherwise the first scenario in the file is used

### JSON scenario comparison example

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

### CSV scenario comparison example

```bash
poolsim --format csv compare \
  --config docs/fixtures/scenarios.json \
  --baseline peak
```

### TOML scenario comparison example

```bash
poolsim --format table compare --config docs/fixtures/scenarios.toml
```

### `compare` flags

#### `--config <path>`

Required path to a JSON or TOML scenario comparison file.

#### `--baseline <name>`

Optional scenario name to use as the delta baseline.

## `budget`

### Purpose

Allocates a shared database `max_connections` budget across multiple services.

Use `budget` after generating service-level recommendations when the database has one global connection cap but several applications, workers, or replicas compete for it. The command keeps each service at or above its declared minimum when possible, honors service-level maximums, and distributes remaining capacity by priority and replica count.

The JSON output is a `BudgetPlanReport`:

- `status`: `Pass`, `Warning`, or `Critical`
- `max_connections`: database connection ceiling
- `reserved_connections`: connections reserved for maintenance, migrations, consoles, or other non-service use
- `safety_margin_connections`: extra capacity intentionally left unused
- `available_connections`: connections available for listed services after reservations
- `current_total_connections`: total current demand when every service provides `current_pool_size`
- `requested_total_connections`: total demand if every service uses its recommended or capped desired size
- `min_required_connections`: total demand for all service minimums
- `allocated_total_connections`: final planner allocation
- `unused_connections`: remaining unallocated budget
- `over_budget_connections`: requested demand above available budget
- `services`: one allocation row per service
- `warnings`: operational explanations for capped or reduced plans

Each service allocation includes:

- `name`: service name
- `replicas`: number of running replicas
- `priority`: relative allocation priority; higher values receive scarce capacity first
- `current_pool_size`: current configured per-replica pool size, when known
- `min_pool_size`: smallest acceptable per-replica pool size
- `max_pool_size`: optional service-specific upper bound
- `recommended_pool_size`: per-replica recommendation from `simulate`, telemetry import, or another sizing source
- `desired_pool_size`: recommendation after applying `max_pool_size`
- `allocated_pool_size`: final per-replica pool size to configure
- `current_total_connections`: current per-service total, when known
- `requested_total_connections`: desired per-service total
- `allocated_total_connections`: final per-service total
- `pool_size_delta_from_current`: per-replica change from the current value, when known
- `reduction_from_recommended`: per-replica reduction when budget pressure prevents the recommendation
- `capped_by_service_max`: whether `max_pool_size` capped the recommendation
- `meets_minimum`: whether the final allocation satisfies `min_pool_size`

### JSON budget example

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

### CSV budget example

```bash
poolsim --format csv budget --config docs/fixtures/budget.json
```

### TOML budget example

```bash
poolsim --format table budget --config docs/fixtures/budget.toml
```

### `budget` flags

#### `--config <path>`

Required path to a JSON or TOML budget plan file.

### Budget JSON format

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

### Budget TOML format

```toml
max_connections = 120
reserved_connections = 20
safety_margin_connections = 10

[[services]]
name = "checkout-api"
replicas = 6
current_pool_size = 8
min_pool_size = 4
max_pool_size = 12
recommended_pool_size = 10
priority = 5
```

### Budget status behavior

- `Pass`: every requested service pool fits inside the available budget
- `Warning`: minimums fit, but at least one requested pool must be reduced
- `Critical`: service minimums do not fit inside the available budget

`--warn-exit` makes `Warning` return exit code `3`. `Critical` returns exit code `2`.

## `import telemetry`

### Purpose

Imports observed production telemetry and computes a recommendation diff against the current pool size.

Use this command when you already have telemetry from Prometheus, OpenTelemetry, logs, APM, or an internal metrics job and want Poolsim to answer:

- what pool size it recommends
- whether production should increase, decrease, or keep the current setting
- how many connections are required or removable
- how the current pool scores against the same workload model

### Example

```bash
poolsim import telemetry --config docs/fixtures/telemetry.json --format table
poolsim import telemetry --config docs/fixtures/telemetry.json --format json
poolsim import telemetry --config docs/fixtures/telemetry.json --format csv
```

Override the current production pool size without editing the file:

```bash
poolsim import telemetry \
  --config docs/fixtures/telemetry.json \
  --current-pool-size 10 \
  --format json
```

### `import telemetry` flags

#### `--config <path>`

Required path to a JSON or TOML telemetry file.

The file can use a wrapped format:

```json
{
  "telemetry": {
    "service_name": "checkout-api",
    "window": "1h",
    "observed_at": "2026-05-15T10:00:00Z",
    "current_pool_size": 8,
    "workload": {
      "requests_per_second": 180.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 30.0,
      "latency_p99_ms": 70.0
    },
    "pool": {
      "max_server_connections": 100,
      "connection_overhead_ms": 2.0,
      "idle_timeout_ms": 120000,
      "min_pool_size": 2,
      "max_pool_size": 20
    }
  },
  "options": {
    "iterations": 1200,
    "seed": 9,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 40.0,
    "max_acceptable_rho": 0.85
  }
}
```

The file can also use a direct telemetry snapshot without `options`; Poolsim will use `SimulationOptions::default`.

#### `--current-pool-size <n>`

Optional override for `telemetry.current_pool_size`.

This is useful when a metrics export is reused for what-if diffs against several production settings.

### Output fields

The JSON output is a `TelemetryRecommendation`:

- `service_name`, `window`, and `observed_at`
- `diff.current_pool_size`
- `diff.recommended_pool_size`
- `diff.pool_size_delta`
- `diff.change`
- `diff.additional_connections_required`
- `diff.removable_connections`
- `diff.connection_change_percent`
- `diff.current_evaluation`
- `diff.recommended_report`

## `import prometheus`

### Purpose

Queries Prometheus-compatible instant-query responses, converts them into a `TelemetrySnapshot`, and returns the same `TelemetryRecommendation` diff produced by `import telemetry`.

This command is intended for Prometheus servers and OpenTelemetry metrics pipelines that expose Prometheus-compatible metrics.

The command uses the Prometheus instant query API:

- `GET /api/v1/query`
- one query for request rate
- one query for each latency percentile

Each query must return exactly one scalar or one instant-vector series. If a query returns multiple series, aggregate it with PromQL first.

### Live Prometheus example

```bash
poolsim import prometheus \
  --endpoint http://prometheus:9090 \
  --rps-query 'sum(rate(http_requests_total{service="checkout-api"}[5m]))' \
  --p50-query 'histogram_quantile(0.50, sum by (le) (rate(http_request_duration_seconds_bucket{service="checkout-api"}[5m]))) * 1000' \
  --p95-query 'histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket{service="checkout-api"}[5m]))) * 1000' \
  --p99-query 'histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket{service="checkout-api"}[5m]))) * 1000' \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20 \
  --format json
```

Latency queries must return milliseconds. If your histogram is in seconds, multiply by `1000` in PromQL as shown above.

### Offline response-file example

Use `--response-file` for reproducible tests, CI, examples, or environments where HTTPS/authentication is handled by another tool.

```bash
poolsim import prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20 \
  --format json
```

### `import prometheus` source flags

#### `--endpoint <url>`

Prometheus base URL. Native endpoint mode currently supports `http://` URLs.

Examples:

```bash
--endpoint http://localhost:9090
--endpoint http://prometheus.monitoring.svc:9090/prometheus
```

#### `--response-file <path>`

Reads a JSON file containing already-captured Prometheus API responses.

Expected shape:

```json
{
  "rps": { "status": "success", "data": { "resultType": "vector", "result": [] } },
  "p50": { "status": "success", "data": { "resultType": "vector", "result": [] } },
  "p95": { "status": "success", "data": { "resultType": "vector", "result": [] } },
  "p99": { "status": "success", "data": { "resultType": "vector", "result": [] } }
}
```

Use the checked-in example at `docs/fixtures/prometheus-responses.json`.

#### `--header 'Name: value'`

Adds a header to live Prometheus HTTP requests. This can be repeated.

Example:

```bash
--header 'Authorization: Bearer token'
```

### `import prometheus` query flags

These are required with `--endpoint` and ignored with `--response-file`:

- `--rps-query`
- `--p50-query`
- `--p95-query`
- `--p99-query`

Each query must return one numeric value.

### `import prometheus` metadata and pool flags

Required:

- `--current-pool-size`
- `--max-server-connections`
- `--min`
- `--max`

Optional:

- `--service-name`
- `--window`
- `--observed-at`
- `--connection-overhead-ms`
- `--connection-establishment-overhead-ms`
- `--idle-timeout-ms`
- `--iterations`
- `--seed`
- `--distribution`
- `--queue-model`
- `--target-wait-p99-ms`
- `--max-acceptable-rho`

The output is the same `TelemetryRecommendation` JSON/table/CSV shape documented in `import telemetry`.

## `gate`

### Purpose

Runs a CI-friendly capacity gate against imported telemetry and exits according to policy.

Use `gate` when a deployment, pull request, or scheduled capacity job needs a hard pass/fail answer instead of only a sizing recommendation.

The command reuses the same telemetry import paths as `import telemetry` and `import prometheus`, then evaluates a `GateReport`:

- `status`: `Pass`, `Warning`, or `Critical`
- `checks`: one row per policy rule
- `recommendation`: the full `TelemetryRecommendation` used by the gate
- process exit code: `0` for pass, `1` for warning policy failure, `2` for critical policy failure

Unlike global `--warn-exit`, `gate` has dedicated CI exit codes and does not require `--warn-exit`.

### Telemetry-file gate example

```bash
poolsim --format json gate \
  --policy docs/fixtures/gate-policy.toml \
  telemetry \
  --config docs/fixtures/telemetry.json
```

### Prometheus response-file gate example

```bash
poolsim --format json gate \
  --policy docs/fixtures/gate-policy.toml \
  prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

### Direct policy flags

All policy-file fields can also be supplied directly as CLI flags.

```bash
poolsim --format json gate \
  --max-saturation warning \
  --max-pool-increase-percent 50 \
  --max-additional-connections 8 \
  --max-recommended-pool-size 20 \
  --max-recommended-p99-queue-wait-ms 80 \
  --max-recommended-mean-queue-wait-ms 20 \
  --max-recommended-rho 0.90 \
  --max-current-p99-queue-wait-ms 100 \
  --max-current-mean-queue-wait-ms 25 \
  --max-current-rho 0.95 \
  telemetry \
  --config docs/fixtures/telemetry.json
```

CLI policy flags override values loaded from `--policy`.

### Policy file

`--policy` accepts JSON or TOML. The checked-in fixture is `docs/fixtures/gate-policy.toml`.

```toml
max_saturation = "Warning"
max_pool_increase_percent = 100
max_additional_connections = 10
max_recommended_pool_size = 20
max_recommended_p99_queue_wait_ms = 80
max_recommended_mean_queue_wait_ms = 20
max_recommended_rho = 0.90
max_current_p99_queue_wait_ms = 100
max_current_mean_queue_wait_ms = 25
max_current_rho = 0.95
```

Supported policy fields:

- `max_saturation`: allowed worst saturation, one of `Ok`, `Warning`, or `Critical`; default is `Warning`
- `max_pool_increase_percent`: maximum allowed positive pool-size increase percentage
- `max_additional_connections`: maximum allowed additional connections
- `max_recommended_pool_size`: maximum allowed recommended pool size
- `max_recommended_p99_queue_wait_ms`: maximum allowed recommended p99 queue wait in milliseconds
- `max_recommended_mean_queue_wait_ms`: maximum allowed recommended mean queue wait in milliseconds
- `max_recommended_rho`: maximum allowed recommended utilization ratio
- `max_current_p99_queue_wait_ms`: maximum allowed p99 queue wait for the currently configured production pool
- `max_current_mean_queue_wait_ms`: maximum allowed mean queue wait for the currently configured production pool
- `max_current_rho`: maximum allowed utilization ratio for the currently configured production pool
- `expected_pool_size`: exact recommended pool size expected by a checked-in config

### Gate source subcommands

#### `gate telemetry`

Uses the same flags as `import telemetry`:

- `--config <path>`
- `--current-pool-size <n>`

#### `gate prometheus`

Uses the same flags as `import prometheus`:

- `--endpoint <url>` or `--response-file <path>`
- `--rps-query`
- `--p50-query`
- `--p95-query`
- `--p99-query`
- `--header`
- telemetry metadata, pool, and simulation-option flags

### GitHub Actions example

```yaml
name: capacity-gate

on:
  pull_request:

jobs:
  poolsim:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo run -p poolsim-cli -- --format json gate --policy docs/fixtures/gate-policy.toml telemetry --config docs/fixtures/telemetry.json
```

## `guard`

### Purpose

Runs CI guard mode against imported telemetry and returns deployment-oriented output.

`guard` uses the same policy engine as `gate`, but wraps the result in a `GuardReport` with fields that are easier for CI systems to consume:

- `status`: `Pass`, `Warning`, or `Critical`
- `deployment_safe`: `true` only when the policy passes
- `exit_code`: numeric process exit code that the CLI returns
- `reason`: short human-readable deployment summary
- `gate`: the underlying `GateReport` with all policy checks and the full recommendation

Use `guard` when you want a deployment or pull request to fail if the currently configured pool becomes unsafe under new traffic or latency assumptions.

Exit codes:

- `0`: deployment is within policy
- `1`: deployment has warning-level policy failures
- `2`: deployment has critical policy failures

### Telemetry-file guard example

```bash
poolsim --format json guard \
  --policy docs/fixtures/gate-policy.toml \
  --max-current-rho 0.95 \
  telemetry \
  --config docs/fixtures/telemetry.json
```

### Prometheus response-file guard example

```bash
poolsim --format json guard \
  --max-current-p99-queue-wait-ms 100 \
  --max-current-mean-queue-wait-ms 20 \
  --max-current-rho 0.95 \
  prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

### Failing guard example

```bash
poolsim --format json guard \
  --max-current-rho 0.01 \
  telemetry \
  --config docs/fixtures/telemetry.json
```

### Guard source subcommands

#### `guard telemetry`

Uses the same flags as `import telemetry`:

- `--config <path>`
- `--current-pool-size <n>`

#### `guard prometheus`

Uses the same flags as `import prometheus`:

- `--endpoint <url>` or `--response-file <path>`
- `--rps-query`
- `--p50-query`
- `--p95-query`
- `--p99-query`
- `--header`
- telemetry metadata, pool, and simulation-option flags

### GitHub Actions guard example

```yaml
name: pool-guard

on:
  pull_request:

jobs:
  guard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo run -p poolsim-cli -- --format json guard --policy docs/fixtures/gate-policy.toml --max-current-rho 0.95 telemetry --config docs/fixtures/telemetry.json
```

## `doctor`

### Purpose

Diagnoses the configured production pool against observed telemetry.

Use `doctor` when you want an operational explanation instead of only a recommendation diff. It answers:

- whether the current pool is healthy
- whether it is too small
- whether it is too large
- whether it is close to saturation
- whether the current or recommended pool is critically saturated

The command reuses the same telemetry import paths as `import telemetry`, `import prometheus`, and `gate`.

The JSON output is a `DoctorReport`:

- `status`: `Healthy`, `TooSmall`, `TooLarge`, `CloseToSaturation`, or `Saturated`
- `current_pool_size`: the configured production pool size
- `recommended_pool_size`: the size selected by the sizing engine
- `pool_size_delta`: positive when the pool should grow, negative when it can shrink
- `current_rho`: current fixed-pool utilization ratio
- `current_p99_queue_wait_ms`: current fixed-pool p99 queue wait
- `current_saturation`: current fixed-pool saturation label
- `recommended_saturation`: recommended-pool saturation label
- `findings`: explanation and action rows
- `recommendation`: the full underlying `TelemetryRecommendation`

Default exit behavior is advisory:

- `Saturated` returns exit code `2`
- `Healthy`, `TooSmall`, `TooLarge`, and `CloseToSaturation` return exit code `0`
- with global `--warn-exit`, `TooSmall` and `CloseToSaturation` return exit code `3`

### Telemetry-file doctor example

```bash
poolsim --format json doctor telemetry \
  --config docs/fixtures/telemetry.json
```

With warning exit behavior:

```bash
poolsim --warn-exit --format json doctor telemetry \
  --config docs/fixtures/telemetry.json
```

### Prometheus response-file doctor example

```bash
poolsim --format json doctor prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

### Doctor source subcommands

#### `doctor telemetry`

Uses the same flags as `import telemetry`:

- `--config <path>`
- `--current-pool-size <n>`

#### `doctor prometheus`

Uses the same flags as `import prometheus`:

- `--endpoint <url>` or `--response-file <path>`
- `--rps-query`
- `--p50-query`
- `--p95-query`
- `--p99-query`
- `--header`
- telemetry metadata, pool, and simulation-option flags

## `generate-config`

### Purpose

Generates framework-specific pool configuration snippets from a Poolsim recommendation.

Use `generate-config` after `simulate`, `import telemetry`, or `import prometheus` when you want to turn the recommended pool size into copy-pasteable configuration for a real runtime pool.

Supported frameworks:

- `hikaricp`
- `spring-boot`
- `sqlalchemy`
- `prisma`
- `node-pg`
- `sqlx`
- `deadpool`

The command is intentionally additive: Poolsim still calculates settings, while the target framework enforces those settings at runtime.

### Telemetry-file config example

```bash
poolsim --format json generate-config \
  --framework sqlx \
  --pool-name checkout-pool \
  telemetry \
  --config docs/fixtures/telemetry.json
```

### Prometheus response-file config example

```bash
poolsim --format json generate-config \
  --framework spring-boot \
  prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --service-name checkout-api \
  --window 5m \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

### Simulation config example

```bash
poolsim --format csv generate-config \
  --framework node-pg \
  simulate \
  --config docs/fixtures/cli-config.json
```

### `generate-config` flags

#### `--framework <name>`

Required target framework.

Poolsim maps its recommendation to the framework's documented pool-size keys:

- HikariCP: `maximumPoolSize`, `minimumIdle`, `connectionTimeout`, `idleTimeout`
- Spring Boot Hikari: `spring.datasource.hikari.maximum-pool-size`, `minimum-idle`, `connection-timeout`, `idle-timeout`
- SQLAlchemy: `create_engine(pool_size=..., max_overflow=0, pool_timeout=...)`
- Prisma: `connection_limit` and `pool_timeout` for URL-based configuration; `max`, `connectionTimeoutMillis`, and `idleTimeoutMillis` for the v7 PostgreSQL adapter
- node-postgres: `new Pool({ max, min, connectionTimeoutMillis, idleTimeoutMillis })`
- sqlx: `PgPoolOptions::max_connections`, `min_connections`, `acquire_timeout`, and `idle_timeout`
- deadpool-postgres: environment keys under `PG__POOL__*`

#### `--min-idle <n>`

Optional minimum idle or warm-pool setting.

Default behavior uses the simulation's `cold_start_min_pool_size`, clamped so it never exceeds `recommended_pool_size`.

#### `--connection-timeout-ms <ms>`

Connection acquisition timeout in milliseconds.

Default: `30000`.

#### `--idle-timeout-ms <ms>`

Idle connection timeout in milliseconds.

Default: `600000`.

#### `--database-url-env <name>`

Environment variable used in snippets that need a database URL.

Default: `DATABASE_URL`.

#### `--pool-name <name>`

Pool name used by frameworks that expose a named pool setting.

Default: `poolsim-recommended-pool`.

### Source subcommands

#### `generate-config telemetry`

Uses the same flags as `import telemetry`:

- `--config <path>`
- `--current-pool-size <n>`

#### `generate-config prometheus`

Uses the same flags as `import prometheus`:

- `--endpoint <url>` or `--response-file <path>`
- `--rps-query`
- `--p50-query`
- `--p95-query`
- `--p99-query`
- `--header`
- telemetry metadata, pool, and simulation-option flags

#### `generate-config simulate`

Uses the same common input flags as `simulate`, including:

- `--config <path>`
- workload percentile flags
- pool limit flags
- simulation-option flags

### Output fields

The JSON output is a `ConfigSnippetReport`:

- `framework`: target framework
- `source`: `telemetry`, `prometheus`, or `simulate`
- `service_name`, `window`, and `observed_at` when available from telemetry
- `recommended_pool_size`
- `min_idle`
- `connection_timeout_ms`
- `idle_timeout_ms`
- `database_url_env`
- `pool_name`
- `max_server_connections`
- `utilisation_rho`
- `mean_queue_wait_ms`
- `p99_queue_wait_ms`
- `snippet`: generated framework configuration
- `notes`: operational cautions about database connection budgets and re-running Poolsim
- `references`: documentation URLs used for the framework-specific setting names

### Documentation references used by the generator

- HikariCP: <https://github.com/brettwooldridge/HikariCP>
- Spring Boot data access: <https://docs.spring.io/spring-boot/how-to/data-access.html>
- SQLAlchemy pooling: <https://docs.sqlalchemy.org/en/latest/core/pooling.html>
- Prisma connection pool: <https://www.prisma.io/docs/orm/prisma-client/setup-and-configuration/databases-connections/connection-pool>
- node-postgres Pool API: <https://node-postgres.com/apis/pool>
- sqlx PoolOptions: <https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html>
- deadpool-postgres Config: <https://docs.rs/deadpool-postgres/latest/deadpool_postgres/struct.Config.html>

## Common Input Flags

These flags are shared by `simulate`, `evaluate`, and `sweep` through the common CLI argument surface.

### `--config <path>`

Loads a JSON or TOML config file.

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json
poolsim simulate --config docs/fixtures/cli-config.toml
```

### `--rps`

Override `workload.requests_per_second`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --rps 275
```

### `--p50`, `--p95`, `--p99`

Override percentile latencies.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --p50 9 --p95 35 --p99 90
```

### `--samples-file`

Load empirical latency samples from a file. The checked-in sample file is `docs/fixtures/latencies.txt`.

The parser accepts values separated by:

- commas
- spaces
- tabs
- newlines

Example:

```bash
poolsim simulate \
  --rps 180 \
  --p50 6 \
  --p95 25 \
  --p99 60 \
  --samples-file latencies.txt \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

Sample file content:

```text
5.5
6.0
6.8
7.4
8.1
9.9
12.0
18.2
```

Or comma-separated:

```text
5.5,6.0,6.8,7.4,8.1,9.9,12.0,18.2
```

### `--max-server-connections`

Override `pool.max_server_connections`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --max-server-connections 150
```

### `--connection-overhead-ms`

Override `pool.connection_overhead_ms`.

The CLI also supports the alias:

- `--connection-establishment-overhead-ms`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --connection-overhead-ms 2.5
poolsim simulate --config docs/fixtures/cli-config.json --connection-establishment-overhead-ms 2.5
```

### `--idle-timeout-ms`

Override `pool.idle_timeout_ms`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --idle-timeout-ms 120000
```

### `--min`, `--max`

Override `pool.min_pool_size` and `pool.max_pool_size`.

```bash
poolsim sweep --config docs/fixtures/cli-config.json --min 4 --max 24
```

### `--iterations`

Override Monte Carlo iteration count.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --iterations 20000
```

### `--seed`

Set deterministic RNG seed.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --seed 42
```

### `--distribution`

Supported values:

- `log-normal`
- `exponential`
- `empirical`
- `gamma`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --distribution log-normal
poolsim simulate --config docs/fixtures/cli-config.json --distribution gamma
```

### `--queue-model`

Supported values:

- `mmc`
- `mdc`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --queue-model mmc
poolsim simulate --config docs/fixtures/cli-config.json --queue-model mdc
```

### `--target-wait-p99-ms`

Override the acceptance and risk threshold for p99 queue wait.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --target-wait-p99-ms 40
```

### `--max-acceptable-rho`

Override the utilization ceiling for candidate acceptance.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --max-acceptable-rho 0.80
```

## Config File Formats

## JSON config

```json
{
  "workload": {
    "requests_per_second": 220.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0,
    "raw_samples_ms": null,
    "step_load_profile": [
      { "time_s": 0, "requests_per_second": 180.0 },
      { "time_s": 30, "requests_per_second": 260.0 }
    ]
  },
  "pool": {
    "max_server_connections": 120,
    "connection_overhead_ms": 2.0,
    "idle_timeout_ms": 120000,
    "min_pool_size": 3,
    "max_pool_size": 24
  },
  "options": {
    "iterations": 12000,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
```

## TOML config

```toml
[workload]
requests_per_second = 220.0
latency_p50_ms = 8.0
latency_p95_ms = 32.0
latency_p99_ms = 85.0

[[workload.step_load_profile]]
time_s = 0
requests_per_second = 180.0

[[workload.step_load_profile]]
time_s = 30
requests_per_second = 260.0

[pool]
max_server_connections = 120
connection_overhead_ms = 2.0
idle_timeout_ms = 120000
min_pool_size = 3
max_pool_size = 24

[options]
iterations = 12000
seed = 7
distribution = "LogNormal"
queue_model = "MMC"
target_wait_p99_ms = 45.0
max_acceptable_rho = 0.85
```

## Batch File Formats

## JSON array batch

```json
[
  {
    "workload": {
      "requests_per_second": 180.0,
      "latency_p50_ms": 7.0,
      "latency_p95_ms": 25.0,
      "latency_p99_ms": 60.0
    },
    "pool": {
      "max_server_connections": 100,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 2,
      "max_pool_size": 20
    },
    "options": {
      "iterations": 10000
    }
  },
  {
    "workload": {
      "requests_per_second": 260.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 30.0,
      "latency_p99_ms": 70.0
    },
    "pool": {
      "max_server_connections": 120,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 3,
      "max_pool_size": 24
    },
    "options": {
      "iterations": 10000
    }
  }
]
```

## JSON object batch

```json
{
  "requests": [
    {
      "workload": {
        "requests_per_second": 180.0,
        "latency_p50_ms": 7.0,
        "latency_p95_ms": 25.0,
        "latency_p99_ms": 60.0
      },
      "pool": {
        "max_server_connections": 100,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 2,
        "max_pool_size": 20
      },
      "options": {
        "iterations": 10000
      }
    }
  ]
}
```

## TOML batch

```toml
[[requests]]
[requests.workload]
requests_per_second = 180.0
latency_p50_ms = 7.0
latency_p95_ms = 25.0
latency_p99_ms = 60.0

[requests.pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 20

[requests.options]
iterations = 10000
```

## Scenario Comparison File Formats

## JSON scenario comparison

```json
{
  "baseline": "normal",
  "scenarios": [
    {
      "name": "normal",
      "workload": {
        "requests_per_second": 180.0,
        "latency_p50_ms": 7.0,
        "latency_p95_ms": 25.0,
        "latency_p99_ms": 60.0
      },
      "pool": {
        "max_server_connections": 100,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 2,
        "max_pool_size": 20
      },
      "options": {
        "iterations": 10000,
        "seed": 17
      }
    },
    {
      "name": "peak",
      "workload": {
        "requests_per_second": 260.0,
        "latency_p50_ms": 8.0,
        "latency_p95_ms": 30.0,
        "latency_p99_ms": 70.0
      },
      "pool": {
        "max_server_connections": 120,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 3,
        "max_pool_size": 24
      },
      "options": {
        "iterations": 10000,
        "seed": 17
      }
    }
  ]
}
```

JSON also supports a direct array of scenario objects. In that form, the first scenario is the baseline unless `--baseline` is supplied.

## TOML scenario comparison

```toml
baseline = "normal"

[[scenarios]]
name = "normal"
[scenarios.workload]
requests_per_second = 180.0
latency_p50_ms = 7.0
latency_p95_ms = 25.0
latency_p99_ms = 60.0
[scenarios.pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 20
[scenarios.options]
iterations = 10000
seed = 17

[[scenarios]]
name = "peak"
[scenarios.workload]
requests_per_second = 260.0
latency_p50_ms = 8.0
latency_p95_ms = 30.0
latency_p99_ms = 70.0
[scenarios.pool]
max_server_connections = 120
connection_overhead_ms = 2.0
min_pool_size = 3
max_pool_size = 24
[scenarios.options]
iterations = 10000
seed = 17
```

## Output Formats

### Table output

Best for humans in terminals.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --format table
```

### JSON output

Best for automation, CI, scripts, and downstream APIs.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --format json
```

### CSV output

Best for spreadsheets and pipeline export.

```bash
poolsim sweep --config docs/fixtures/cli-config.json --format csv
```

## Exit Codes

- `0`: successful run with non-warning/non-critical outcome
- `1`: parse error, validation error, config error, I/O error, or execution failure
- `2`: critical saturation
- `3`: warning exit when `--warn-exit` is enabled

Practical CI pattern:

```bash
poolsim --warn-exit simulate --config docs/fixtures/cli-config.json --format json
status=$?
if [ "$status" -eq 2 ]; then
  echo "critical saturation"
elif [ "$status" -eq 3 ]; then
  echo "warning saturation"
fi
```

## End-to-End Examples

### Example: quick first run

```bash
poolsim simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24 \
  --format json
```

### Example: evaluate one candidate pool

```bash
poolsim evaluate \
  --config docs/fixtures/cli-config.json \
  --pool-size 12 \
  --format json
```

### Example: export sensitivity analysis

```bash
poolsim sweep \
  --config docs/fixtures/cli-config.json \
  --format csv > sensitivity.csv
```

### Example: sample-driven simulation

```bash
poolsim simulate \
  --rps 180 \
  --p50 6 \
  --p95 25 \
  --p99 60 \
  --samples-file latencies.txt \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20 \
  --distribution empirical \
  --format json
```

### Example: deterministic-service approximation

```bash
poolsim simulate \
  --config docs/fixtures/cli-config.json \
  --queue-model mdc \
  --format json
```

### Example: batch execution

```bash
poolsim batch --config docs/fixtures/batch.json --format json
```

### Example: scenario comparison

```bash
poolsim compare --config docs/fixtures/scenarios.json --format json
```

## Notes

- CLI flags override config-file values.
- `simulate --pool-size` is an evaluation shortcut through the `simulate` command path.
- `simulate --sweep` is a sweep shortcut through the `simulate` command path.
- If `raw_samples_ms` is present, the library uses empirical fitting regardless of the requested distribution model.
