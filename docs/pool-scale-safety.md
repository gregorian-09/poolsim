# Pool Scale Safety Gate

The pool scale-safety gate is an opt-in check for a specific operational risk:
an otherwise reasonable sizing recommendation may increase the number of
database connections beyond the capacity actually available to a service.

The gate combines two kinds of evidence:

1. The existing telemetry recommendation and current-versus-recommended diff.
2. A [`TelemetryQualityReport`](https://docs.rs/poolsim-core/latest/poolsim_core/telemetry_quality/struct.TelemetryQualityReport.html)
   plus deployment topology and database connection-budget evidence.

It does not change the recommendation. It does not modify a running pool. It
does not replace `poolsim gate`, `poolsim guard`, or the database's own
connection admission control. It is a separate, conservative decision layer
that answers: "May this recommendation increase the pool with the evidence we
have?"

## Why This Check Exists

Pool size is a per-process or per-replica setting, while database capacity is a
shared limit. A recommendation of 12 connections is not a 12-connection
change when a deployment has 10 replicas: it is a potential 120-connection
allocation. The gate makes this multiplication explicit.

Increasing a pool can also be the wrong response to database contention. The
[HikariCP pool-sizing guidance](https://github.com/brettwooldridge/HikariCP/wiki/About-Pool-Sizing)
describes how oversized pools can increase contention and response time, and
recommends validating settings with load tests. PostgreSQL documents that
`max_connections` is the maximum concurrent connection count and that raising
it increases resource allocation; reserved connection slots are not ordinary
application capacity. See the
[PostgreSQL connection configuration documentation](https://www.postgresql.org/docs/current/runtime-config-connection.html).

The gate therefore follows these rules:

- A recommendation that keeps or decreases the pool is not a scale-up and is
  allowed by this scale-up-specific check.
- A scale-up with rejected telemetry is blocked.
- A scale-up with telemetry needing review returns `needs-review`.
- A scale-up without a declared database budget returns `needs-review`, never
  automatic approval.
- A scale-up without a directly observed service connection total uses the
  current pool size multiplied by replicas, reports that inference, and
  returns `needs-review`.
- A projected total above effective database capacity is blocked.
- A database-contention report with material database-side causes blocks a
  scale-up even when the connection budget has room. Adding application
  connections cannot resolve lock waits, idle transactions, deadlocks, or
  backend saturation.
- Existing recommendation outputs remain unchanged regardless of the gate
  result.

## CLI Command

The command is nested under `check`:

```bash
poolsim --format json check pool-scale \
  --quality-config docs/fixtures/telemetry-quality-open-loop.json \
  --database-max-connections 100 \
  --reserved-connections 10 \
  --safety-margin-connections 10 \
  --replicas 3 \
  --current-total-connections 6 \
  telemetry \
  --config docs/fixtures/telemetry.json \
  --current-pool-size 2
```

The `--current-pool-size 2` override makes the example request a scale-up when
the fixture's workload is evaluated. In production, use the actual current
pool size from the same observation window as the quality evidence.

The source is one of the existing telemetry import sources:

```bash
# File or TOML telemetry input.
poolsim --format json check pool-scale \
  --quality-config quality.json \
  --database-max-connections 200 \
  --reserved-connections 20 \
  --safety-margin-connections 20 \
  --replicas 6 \
  telemetry --config telemetry.json

# Captured Prometheus API response or live Prometheus-compatible endpoint.
poolsim --format json check pool-scale \
  --quality-config quality.json \
  --database-max-connections 200 \
  --reserved-connections 20 \
  --safety-margin-connections 20 \
  --replicas 6 \
  prometheus --response-file prometheus-responses.json \
  --current-pool-size 8 \
  --max-server-connections 200 \
  --min 2 --max 40

# OpenTelemetry OTLP metric-export JSON.
poolsim --format json check pool-scale \
  --quality-config quality.json \
  --database-max-connections 200 \
  --reserved-connections 20 \
  --safety-margin-connections 20 \
  --replicas 6 \
  otlp --config otlp-metrics.json \
  --current-pool-size 8 \
  --max-server-connections 200 \
  --min 2 --max 40
```

The quality input is separate from the telemetry recommendation source on
purpose. Teams can retain a signed or reviewed quality assessment alongside a
raw telemetry export, while the recommendation source can be refreshed from a
different adapter.

## CLI Options

Options before the nested source apply to the gate:

- `--quality-config <path>`: JSON file containing a
  `poolsim_core::telemetry_quality::TelemetryQualityInput`.
- `--database-max-connections <count>`: maximum connection slots allocated to
  this service's database budget. Omit it only for an intentional review-only
  result.
- `--reserved-connections <count>`: slots reserved for administrators or other
  workloads. Defaults to `0`.
- `--safety-margin-connections <count>`: operational headroom excluded from
  the application budget. Defaults to `0`.
- `--replicas <count>`: application replicas receiving the recommended pool.
  Defaults to `1` and must be greater than zero.
- `--current-total-connections <count>`: directly observed total connections
  consumed by this service across replicas. If omitted, the gate infers it as
  `current_pool_size * replicas` and returns `needs-review` for a scale-up.
- `--contention-config <path>`: optional JSON file containing a
  `poolsim_core::contention::DatabaseContentionInput`. A classified
  `database-contention` result blocks a scale-up; incomplete evidence adds a
  review finding; `healthy` and `pool-starvation` do not block it.

The nested `telemetry`, `prometheus`, and `otlp` source flags are the same
flags documented in [`cli-reference.md`](cli-reference.md). This reuse avoids
parallel config formats and keeps all recommendation paths on the same core
telemetry model.

## Decision Math

For a recommendation that increases the pool, the gate calculates:

```text
additional_connections_total =
    additional_connections_per_replica * replica_count

effective_database_capacity =
    database_max_connections
    - reserved_connections
    - safety_margin_connections

projected_total_connections =
    current_total_connections
    + additional_connections_total
```

`current_total_connections` is measured directly when supplied. Otherwise the
gate uses:

```text
current_pool_size * replica_count
```

The inferred value is deliberately visible in the report. It does not account
for migration pools, background workers, administrative sessions, sidecars,
other services, or connection churn. Supply an observed total when those
consumers matter.

The budget check blocks only when:

```text
projected_total_connections > effective_database_capacity
```

Equality is allowed because reserved slots and the declared safety margin have
already been excluded. If a deployment needs additional slack, increase
`--safety-margin-connections` rather than relying on an undocumented threshold.

All multiplication and addition use checked arithmetic. Invalid budgets and
overflow return a normal Poolsim invalid-input error instead of wrapping.

## Status And Exit Codes

The JSON `status` field is one of:

- `allowed`: no scale-up was requested, or all required scale-up evidence was
  present and within budget.
- `needs-review`: a scale-up is requested but quality, budget, or topology
  evidence is incomplete or limited.
- `blocked`: telemetry was rejected or the projected connection total exceeds
  effective database capacity.

The process exit behavior is:

- `0`: `allowed`, or `needs-review` without `--warn-exit`.
- `2`: `blocked`.
- `3`: `needs-review` with `--warn-exit`.
- `1`: invalid input, unreadable files, malformed JSON, or recommendation
  failure.

This contract is independent from the existing `gate` and `guard` exit
contracts. Adding `check pool-scale` does not change their output or status
mapping.

## Output Contract

The JSON report includes:

- `status`: final scale-safety decision.
- `scale_up_requested`: whether the recommendation's change is `Increase`.
- `current_pool_size`: current size per replica.
- `recommended_pool_size`: recommended size per replica.
- `additional_connections_per_replica`: requested per-replica increase.
- `additional_connections_total`: increase multiplied by replicas.
- `replica_count`: replica count used by the projection.
- `current_total_connections`: observed or inferred current service total for
  a scale-up.
- `projected_total_connections`: projected service total for a scale-up.
- `effective_database_capacity`: database maximum after reserved slots and
  safety margin.
- `database_contention`: optional normalized contention report used by the
  decision. It is omitted when `--contention-config` is not supplied.
- `telemetry_quality`: complete quality report used by the gate.
- `findings`: stable, remediation-oriented gate findings.
- `confidence`: combined evidence confidence.

The CSV output contains the scalar report fields and a `finding_count`. The
table output prints the same scalar fields followed by each finding. HTML is a
self-contained report generated by the standard CLI HTML renderer.

## Finding Codes

| Code | Risk | Meaning | Action |
| --- | --- | --- | --- |
| `NO_SCALE_UP_REQUESTED` | `Low` | The recommendation keeps or reduces the current pool. | Use the normal recommendation workflow; this check is specifically for increases. |
| `TELEMETRY_QUALITY_REJECTED` | `Critical` | Quality assessment rejected the evidence for capacity planning. | Collect a defensible open-loop capture or correct the quality risks. |
| `TELEMETRY_QUALITY_NEEDS_REVIEW` | `Medium` | Quality evidence has limitations requiring review. | Review the quality findings and repeat the capture if needed. |
| `DATABASE_BUDGET_MISSING` | `Medium` | No database maximum was supplied for a requested increase. | Provide max connections, reserved slots, and safety margin. |
| `CURRENT_CONNECTION_TOTAL_INFERRED` | `Medium` | Current service total was inferred from pool size and replicas. | Supply the observed service total when other consumers exist. |
| `DATABASE_BUDGET_EXCEEDED` | `Critical` | Projected service total exceeds effective capacity. | Reduce pool/replicas, increase approved budget, or reallocate capacity. |
| `DATABASE_CONTENTION_DETECTED` | `Critical` | Database-side contention makes a scale-up unsafe. | Resolve lock waits, idle transactions, deadlocks, or backend saturation first. |
| `DATABASE_CONTENTION_NEEDS_REVIEW` | `Medium` | Contention evidence is incomplete. | Collect the missing database-side evidence before approving the increase. |

Telemetry-quality findings remain nested under `telemetry_quality.findings`;
the gate does not copy or rewrite them, so the original evidence explanation is
preserved.

## Rust API

The reusable implementation is in `poolsim-core`, so applications do not need
to shell out to the CLI. The API is additive and composes existing
`TelemetryRecommendation` and `TelemetryQualityReport` values:

```rust
use poolsim_core::scale_gate::{
    check_pool_scale_gate, PoolScaleGateInput, PoolScaleGateStatus,
};

# fn recommendation() -> poolsim_core::telemetry::TelemetryRecommendation { todo!() }
# fn quality() -> poolsim_core::telemetry_quality::TelemetryQualityReport { todo!() }
# fn contention_report() -> poolsim_core::contention::DatabaseContentionReport { todo!() }
let input = PoolScaleGateInput::new(recommendation(), quality())
    .with_database_budget(200, 20, 20)
    .with_replica_count(6)
    .with_current_total_connections(48)
    .with_database_contention_report(contention_report());

let report = check_pool_scale_gate(&input)?;
if report.status == PoolScaleGateStatus::Blocked {
    return Err("do not apply the scale-up".into());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

The public core types are:

- `PoolScaleGateStatus`: `Allowed`, `NeedsReview`, or `Blocked`.
- `PoolScaleGateFinding`: stable `code`, `risk`, `message`, and `remediation`
  fields.
- `PoolScaleGateInput`: recommendation, quality report, optional budget, and
  replica topology evidence.
- `PoolScaleGateReport`: final decision, projections, nested quality report,
  findings, and confidence.
- `check_pool_scale_gate`: validates evidence and returns the report or a
  `PoolsimError`.

`PoolScaleGateInput::new` defaults to one replica, no database budget, no
reserved slots, no safety margin, and no observed total. The builder methods
are:

- `PoolScaleGateInput::with_database_budget(max_connections,
  reserved_connections, safety_margin_connections)`.
- `PoolScaleGateInput::with_replica_count(replica_count)`.
- `PoolScaleGateInput::with_current_total_connections(current_total_connections)`.
- `PoolScaleGateInput::with_database_contention_report(report)` adds the
  optional `DatabaseContentionReport` produced by
  `poolsim_core::contention::classify_database_contention`.

The types are serialized with Serde for storage and service boundaries. The
status and findings are intentionally explicit so an automation caller can
block only `Blocked`, turn on review failures with `NeedsReview`, and preserve
unknown future enum variants without assuming they do not exist.

## CI Usage

Use the command as a second, opt-in safety stage after generating a
recommendation:

```yaml
- name: Check pool scale safety
  run: >-
    poolsim --format json --warn-exit check pool-scale
    --quality-config artifacts/telemetry-quality.json
    --database-max-connections 400
    --reserved-connections 30
    --safety-margin-connections 30
    --replicas 8
    --current-total-connections 176
    --contention-config artifacts/database-contention.json
    prometheus --response-file artifacts/prometheus.json
    --current-pool-size 22
    --max-server-connections 400
    --min 2 --max 60
```

Do not interpret exit code `0` as proof that a production database can accept
connections from every service. The check is scoped to the budget supplied in
the invocation. For shared databases, run the database budget planner across
all services first and pass each service's allocated capacity into its own
scale-safety check.

## Operational Limitations

- The gate does not observe a database or pooler directly.
- It cannot discover other services using the same database unless their
  connections are included in the supplied budget or current total.
- It does not model connection establishment bursts, transaction pinning,
  connection leaks, or pooler multiplexing by itself.
- It does not decide whether increasing database capacity is operationally
  appropriate; it only checks the supplied numeric budget.
- Quality evidence and recommendation telemetry should cover the same workload
  window. Mixing unrelated windows can produce a mathematically valid but
  operationally misleading result.

For downstream PgBouncer or managed-pooler evidence, run the existing
[`poolsim doctor pgbouncer-pools`](downstream-pooler-diagnosis.md) workflow
before approving a scale-up. A saturated downstream pooler is not fixed by
blindly increasing the application pool.

## Compatibility

This feature adds one `check` subcommand and one public `poolsim-core` module.
It does not remove or change existing APIs, CLI commands, flags, serialized
fields, REST routes, WebSocket messages, or existing exit-code mappings.

## Public API Inventory

The documented CLI argument type is `poolsim_cli::args::PoolScaleGateArgs`.
The core module is `poolsim_core::scale_gate`. Its public symbols are
`poolsim_core::scale_gate::PoolScaleGateStatus`,
`poolsim_core::scale_gate::PoolScaleGateFinding`,
`poolsim_core::scale_gate::PoolScaleGateInput`,
`poolsim_core::scale_gate::PoolScaleGateInput::new`,
`poolsim_core::scale_gate::PoolScaleGateInput::with_database_budget`,
`poolsim_core::scale_gate::PoolScaleGateInput::with_replica_count`,
`poolsim_core::scale_gate::PoolScaleGateInput::with_current_total_connections`,
`poolsim_core::scale_gate::PoolScaleGateInput::with_database_contention_report`,
`poolsim_core::scale_gate::PoolScaleGateReport`, and
`poolsim_core::scale_gate::check_pool_scale_gate`.
