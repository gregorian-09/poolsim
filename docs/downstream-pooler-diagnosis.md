# Downstream Pooler Diagnosis

Poolsim can compare application-pool pressure with observed capacity in a downstream connection pooler. This answers a common incident question:

> Is the application pool too small, or does the application still have spare slots while the pooler is unable to obtain another database connection?

The diagnosis is additive to the existing pooler evidence import. It does not open a network connection to PgBouncer, change runtime pool settings, or replace the existing `PoolerEvidenceReport` contract.

## When To Use It

Use this workflow when an application reports connection-acquisition delays and the deployment includes an intermediate pooler such as PgBouncer or a managed proxy.

It is especially useful when:

- the application pool has unused capacity but requests still wait;
- PgBouncer reports `cl_waiting` clients;
- backend/server connections are at a configured pooler limit;
- increasing every application pool would risk exhausting the database budget;
- an incident review needs evidence for which layer constrained capacity.

The workflow consumes a captured PgBouncer `SHOW POOLS` result. Capture it with the PostgreSQL client:

```bash
psql -p 6432 -d pgbouncer --csv -c "SHOW POOLS;" > show-pools.csv
```

The capture can also be default aligned `psql` output copied from an incident terminal. See [`pgbouncer-show-pools-import.md`](pgbouncer-show-pools-import.md) for parser details.

## CLI Workflow

Run the additive doctor source with application counters and the captured pooler output:

```bash
poolsim --format json doctor pgbouncer-pools \
  --file docs/fixtures/pgbouncer-show-pools.csv \
  --label checkout-production \
  --pooler-backend-limit 15 \
  --application-active 4 \
  --application-max 16 \
  --application-waiting 0
```

The checked-in fixture has 42 active clients, no waiting clients, and 15 observed backend connections. With a backend limit of 15, it demonstrates a downstream saturation result without requiring a live PgBouncer instance.

### Input flags

The doctor command reuses every `import pgbouncer-pools` flag:

- `--file <path>` or `--input <path>`: captured CSV or aligned `SHOW POOLS` output.
- `--label <value>`: optional service, environment, or cluster label.
- `--mode <none|session|transaction|statement|provider-managed|unknown>`: optional mode override when the capture does not contain `pool_mode`.
- `--pooler-client-limit <n>`: optional client connection cap.
- `--pooler-backend-limit <n>`: backend/server connection cap used to calculate downstream utilization.

The diagnosis adds application-side flags:

- `--application-active <n>`: active connections in the application pool.
- `--application-max <n>`: configured maximum application connections.
- `--application-waiting <n>`: application requests waiting for a pool slot, when the client exposes this metric.

Application active and maximum values are optional at the CLI boundary so incomplete incident captures produce a review result instead of an argument error. For a useful cross-layer comparison, provide both.

### Status precedence

The diagnosis has one primary status and keeps independent findings for all observed constraints:

| Status | Meaning |
| --- | --- |
| `healthy` | Application and pooler evidence have known headroom and no observed client queue. |
| `downstream-pooler-waiting` | Pooler clients are waiting for backend capacity, even if backend utilization is below a supplied limit. |
| `downstream-pooler-saturated` | Pooler backend connections are at or above the supplied backend limit. This takes precedence over application saturation because increasing app pools cannot create backend capacity. |
| `application-pool-saturated` | The application active count is at or above its configured maximum and the downstream pooler has no stronger waiting/saturation signal. |
| `needs-review` | Application counters are incomplete, the application is at least 80% utilized, or the normalized pooler report lacks enough evidence. |

If both application and pooler layers are constrained, the primary status identifies the downstream condition and `findings` also contains `APPLICATION_POOL_SATURATED` and `APPLICATION_REQUESTS_WAITING` when those counters were supplied. This prevents the top-level status from hiding a second capacity problem.

### Output fields

JSON output contains:

- `status`: the primary `DownstreamPoolerDiagnosisStatus` value.
- `application_utilization`: active application connections divided by the configured maximum, when both are known.
- `application_waiting`: application-side waiting count, when supplied.
- `pooler`: the unchanged normalized `PoolerEvidenceReport`, including observed client/backend totals, limits, utilization, findings, and confidence.
- `findings`: cross-layer findings with stable codes, risk, message, and remediation.
- `confidence`: `high`, `medium`, or `low` evidence confidence.

The nested `pooler` object is deliberately retained. Existing consumers of `PoolerEvidenceReport` can continue to inspect the original evidence while new consumers use the cross-layer status.

### Exit codes

The command follows the diagnostic conventions used by other capacity checks:

- `0`: healthy, or advisory status without `--warn-exit`.
- `2`: application pool or downstream pooler is saturated.
- `3`: downstream clients are waiting or evidence needs review when `--warn-exit` is enabled.
- `1`: file access, parsing, or other command failure.

Use `--warn-exit` in CI when a waiting queue or incomplete evidence should block a deployment without treating every advisory result as a hard failure.

## Rust Library API

The library API accepts an already normalized `PoolerEvidenceReport`. This keeps collection and diagnosis separate, lets callers use PgBouncer, Prometheus exporter, Supavisor, or another collector, and avoids embedding network credentials in `poolsim-core`.

```rust
use poolsim_core::pooler::{
    diagnose_downstream_pooler, summarize_pooler_evidence, ApplicationPoolEvidence,
    DownstreamPoolerDiagnosisInput, DownstreamPoolerDiagnosisStatus,
    ExternalPoolerKind, MultiplexingMode, PoolerEvidenceSnapshot,
};

let pooler = summarize_pooler_evidence(
    &PoolerEvidenceSnapshot::new(
        ExternalPoolerKind::PgBouncer,
        MultiplexingMode::Transaction,
    )
    .with_client_active(180)
    .with_client_waiting(4)
    .with_server_active(30)
    .with_server_idle(0)
    .with_pooler_backend_limit(30),
);
let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
    ApplicationPoolEvidence::new(4, 16),
    pooler,
));

assert_eq!(
    report.status,
    DownstreamPoolerDiagnosisStatus::DownstreamPoolerSaturated
);
assert_eq!(report.application_utilization, Some(0.25));
```

For captured PgBouncer output, compose the parser, native summarizer, and diagnosis:

```rust
use poolsim_core::pooler::{
    diagnose_downstream_pooler, parse_pgbouncer_show_pools,
    summarize_pgbouncer_show_pools, ApplicationPoolEvidence,
    DownstreamPoolerDiagnosisInput, PgbouncerShowPoolsSnapshot,
};

let rows = parse_pgbouncer_show_pools(
    "database,user,cl_active,cl_waiting,sv_active,sv_idle,pool_mode\n\
     checkout,web,40,4,30,0,transaction\n",
)?;
let pooler = summarize_pgbouncer_show_pools(
    &PgbouncerShowPoolsSnapshot::new(rows).with_pooler_backend_limit(30),
)?;
let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
    ApplicationPoolEvidence::new(4, 16),
    pooler,
));
println!("{:?}", report.status);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

`ApplicationPoolEvidence::default()` is useful when a collector could not obtain application counters. Set optional values with `with_active`, `with_limit`, and `with_waiting`; the diagnosis returns `NeedsReview` and lowers confidence rather than fabricating utilization.

## Operational Interpretation

Do not respond to a downstream pooler bottleneck by automatically increasing every application pool. First compare:

1. application pool active and waiting counts;
2. pooler `cl_waiting` and backend/server counts;
3. pooler backend limits and database `max_connections` budget;
4. query latency, long transactions, session pinning, and database health;
5. replica count and the aggregate application connection footprint.

For PgBouncer, `cl_waiting` means client connections have sent queries but do not yet have a server connection. `sv_active` counts servers linked to clients and `sv_idle` counts immediately usable servers. A backend limit is required for a saturation ratio; without it, poolsim reports review-required evidence rather than inferring a provider limit.

This is a diagnostic comparison, not runtime enforcement. The runtime pool remains responsible for connection timeouts, maximum size, leak detection, and graceful shutdown.

## Compatibility Contract

The feature is additive and preserves:

- existing `PoolerEvidenceSnapshot`, `PoolerEvidenceReport`, and `summarize_pooler_evidence` behavior;
- existing `import pooler-evidence` and `import pgbouncer-pools` commands;
- existing JSON fields and exit codes for those commands;
- existing Rust sizing, telemetry, web, and binding APIs.

New public symbols are `ApplicationPoolEvidence`, `DownstreamPoolerDiagnosisInput`, `DownstreamPoolerDiagnosisReport`, `DownstreamPoolerDiagnosisStatus`, and `diagnose_downstream_pooler`.

The exact inventory names are `poolsim_cli::args::PgbouncerPoolsDoctorArgs`, `poolsim_core::pooler::ApplicationPoolEvidence`, `poolsim_core::pooler::ApplicationPoolEvidence::new`, `poolsim_core::pooler::ApplicationPoolEvidence::with_active`, `poolsim_core::pooler::ApplicationPoolEvidence::with_limit`, `poolsim_core::pooler::ApplicationPoolEvidence::with_waiting`, `poolsim_core::pooler::DownstreamPoolerDiagnosisInput`, `poolsim_core::pooler::DownstreamPoolerDiagnosisInput::new`, `poolsim_core::pooler::DownstreamPoolerDiagnosisReport`, and `poolsim_core::pooler::DownstreamPoolerDiagnosisStatus`.

## Research Sources

- [PgBouncer `SHOW POOLS` documentation](https://www.pgbouncer.org/usage.html)
- [Prometheus PgBouncer exporter metric mapping](https://github.com/prometheus-community/pgbouncer_exporter)
- [PostgreSQL `psql --csv` documentation](https://www.postgresql.org/docs/current/app-psql.html)
