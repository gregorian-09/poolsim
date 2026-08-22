# Database Contention Classification

`poolsim check db-contention` classifies whether observed connection wait is
most consistent with application-pool starvation, database-side contention, a
healthy system, or insufficient evidence.

This is a diagnostic classifier, not a database query tool and not a runtime
enforcer. It consumes a normalized JSON evidence snapshot. A PostgreSQL,
MySQL, managed-database, pooler, or observability adapter can produce that
snapshot without making the core crate depend on a database driver.

## Why Pool Size Is Not Always the Cause

An application can show pool acquisition wait because every connection is busy
executing useful work, but it can also show the same symptom because every
connection is blocked on a database lock, a long transaction, a saturated
database resource, or a slow query. Increasing the application pool in the
latter cases adds more blocked work and can make the incident worse.

PostgreSQL exposes one row per server process in `pg_stat_activity`, including
transaction/query timestamps, `state`, `wait_event_type`, and `wait_event`.
The PostgreSQL documentation specifically distinguishes `idle in transaction`
from ordinary `idle`, and notes that an active backend with a non-null wait
event is executing a query while blocked somewhere in the system. Its `Lock`
wait type represents waiting for heavyweight locks. See the
[PostgreSQL cumulative statistics documentation](https://www.postgresql.org/docs/current/monitoring-stats.html)
and [PostgreSQL lock monitoring documentation](https://www.postgresql.org/docs/current/monitoring-locks.html).

MySQL exposes current and historical wait events through Performance Schema,
and `data_lock_waits` describes the relationship between requesting and
blocking transactions. The relationship is important: a count of busy
connections alone cannot identify the blocker. See the
[MySQL `data_lock_waits` documentation](https://dev.mysql.com/doc/refman/8.0/en/performance-schema-data-lock-waits-table.html)
and [MySQL Performance Schema wait-event documentation](https://dev.mysql.com/doc/refman/8.0/en/performance-schema-wait-tables.html).

Poolsim intentionally normalizes these provider-specific views into portable
evidence fields. It does not claim that a PostgreSQL lock count and a MySQL
wait-event count have identical collection semantics.

## Quick Start

Run the checked-in contention fixture:

```bash
poolsim --format json --warn-exit check db-contention \
  --config docs/fixtures/database-contention.json
```

The fixture reports `database-contention`, identifies `lock-contention` as the
dominant cause, sets `suppress_pool_increase` to `true`, and exits with `2`.
The command exits non-zero intentionally because the evidence indicates that
adding connections is unsafe until the database-side cause is addressed.

Run a healthy, complete evidence snapshot:

```json
{
  "pool_wait_p99_ms": 40.0,
  "database_latency_p99_ms": 20.0,
  "lock_waiting_sessions": 0,
  "idle_in_transaction_sessions": 0,
  "deadlocks_per_second": 0.0,
  "active_sessions": 40,
  "max_connections": 100,
  "database_cpu_utilization": 0.40,
  "database_io_utilization": 0.40
}
```

With the default thresholds, the healthy snapshot is enough to classify high
pool wait as `pool-starvation`. That is not an unconditional recommendation to
increase the pool; it means the supplied database evidence did not identify a
database-side blocker. Validate workload, connection budget, and pool-scale
safety separately with [`pool-scale-safety.md`](pool-scale-safety.md).

## Evidence Model

The JSON input maps to
`poolsim_core::contention::DatabaseContentionInput`:

| Field | Type | Meaning |
| --- | --- | --- |
| `pool_wait_p99_ms` | number or null | p99 application-pool acquisition wait in milliseconds. |
| `database_latency_p99_ms` | number or null | p99 database service/query latency in milliseconds. |
| `lock_waiting_sessions` | integer or null | Sessions currently waiting on database locks. |
| `idle_in_transaction_sessions` | integer or null | Sessions inside an open transaction but not executing a query. |
| `longest_idle_in_transaction_seconds` | number or null | Longest observed idle-in-transaction duration. |
| `deadlocks_per_second` | number or null | Deadlock rate for the observation window. |
| `active_sessions` | integer or null | Active database sessions in the observed allocation. |
| `max_connections` | integer or null | Applicable maximum database connections. |
| `database_cpu_utilization` | number or null | CPU utilization as a fraction from `0.0` to `1.0`. |
| `database_io_utilization` | number or null | I/O utilization as a fraction from `0.0` to `1.0`. |
| `policy` | object | Optional threshold overrides. |

All numeric fields are optional so adapters can preserve partial evidence. The
classifier does not fill missing values with zero. A missing lock metric is not
the same as zero lock waits.

### Collection Mapping: PostgreSQL

A PostgreSQL adapter can map fields as follows:

| Poolsim field | Typical source | Collection note |
| --- | --- | --- |
| `lock_waiting_sessions` | `pg_stat_activity.wait_event_type = 'Lock'`, or ungranted `pg_locks` rows | Count a consistent observation, not a lifetime total. |
| `idle_in_transaction_sessions` | `pg_stat_activity.state IN ('idle in transaction', 'idle in transaction (aborted)')` | Keep aborted transactions visible; they still require cleanup. |
| `longest_idle_in_transaction_seconds` | `now() - xact_start` for idle-in-transaction rows | Use transaction age, not only query age. |
| `database_latency_p99_ms` | Query/service histogram or tracing metric | Keep the histogram window aligned with the activity snapshot. |
| `active_sessions` | Count of relevant `pg_stat_activity` rows | Define whether background workers and admin sessions are included. |
| `max_connections` | `SHOW max_connections` or an allocated service budget | `max_connections` is not automatically the service's full budget. |
| CPU/I/O utilization | Provider metrics, `pg_stat_io`, or host metrics | Preserve the metric's aggregation/window metadata upstream. |

PostgreSQL statistics views have collection and visibility constraints. The
current activity view can be more current than cumulative counters, and access
to other sessions may require the `pg_read_all_stats` role or superuser
privileges. An adapter should record collection timestamp, source, and scope
alongside the JSON snapshot even though those metadata fields are not required
by the current classifier.

### Collection Mapping: MySQL

A MySQL adapter can map fields from Performance Schema and server metrics:

| Poolsim field | Typical source | Collection note |
| --- | --- | --- |
| `lock_waiting_sessions` | `performance_schema.data_lock_waits` or metadata-lock wait views | Preserve requesting/blocking relationship when available. |
| `deadlocks_per_second` | InnoDB deadlock counter divided by observation duration | Use a rate, not a cumulative count without a window. |
| `database_latency_p99_ms` | statement/transaction event histograms or tracing | Ensure the percentile population matches application traffic. |
| `active_sessions` | Performance Schema `threads` or connection metrics | Document whether sleeping and administrative threads are excluded. |
| CPU/I/O utilization | provider or host metrics | Normalize to fractions before sending to Poolsim. |
| `max_connections` | MySQL `max_connections` or an allocated service budget | Include other services and administrative reserve in the budget decision. |

MySQL's lock-wait relationship tables distinguish the requesting transaction
from the blocking transaction. A normalized adapter should not collapse that
relationship into a generic busy-session count when blocker identity is
available.

## Threshold Policy

`poolsim_core::contention::DatabaseContentionPolicy` controls classification
thresholds. Defaults are deliberately conservative and are not universal
database tuning recommendations:

| Field | Default | Meaning |
| --- | ---: | --- |
| `pool_wait_p99_threshold_ms` | `25.0` | Pool p99 wait is elevated at or above 25 ms. |
| `database_latency_p99_threshold_ms` | `100.0` | Database p99 latency is elevated at or above 100 ms. |
| `idle_transaction_threshold_seconds` | `60.0` | An idle transaction is material at or above 60 seconds. |
| `deadlocks_per_second_threshold` | `0.0` | Any positive deadlock rate is material. |
| `database_cpu_utilization_threshold` | `0.90` | CPU is near capacity at or above 90%. |
| `database_io_utilization_threshold` | `0.90` | I/O is near capacity at or above 90%. |
| `connection_utilization_threshold` | `0.90` | Active/max connections are near the limit at or above 90%. |

Override thresholds when the provider, workload, or alerting policy justifies
it:

```json
{
  "pool_wait_p99_ms": 35.0,
  "database_latency_p99_ms": 80.0,
  "lock_waiting_sessions": 0,
  "idle_in_transaction_sessions": 0,
  "deadlocks_per_second": 0.0,
  "active_sessions": 30,
  "max_connections": 100,
  "database_cpu_utilization": 0.80,
  "database_io_utilization": 0.80,
  "policy": {
    "pool_wait_p99_threshold_ms": 50.0,
    "database_latency_p99_threshold_ms": 120.0,
    "idle_transaction_threshold_seconds": 120.0,
    "deadlocks_per_second_threshold": 0.0,
    "database_cpu_utilization_threshold": 0.95,
    "database_io_utilization_threshold": 0.95,
    "connection_utilization_threshold": 0.95
  }
}
```

Threshold changes should be reviewed like alert-policy changes. Do not lower a
threshold merely to make a CI job pass.

## Classification Rules

The classifier applies these rules in order:

1. Validate all supplied numeric values and policy thresholds.
2. Record every supplied evidence category. Missing evidence is not counted as
   a healthy zero.
3. Emit findings for elevated pool wait, lock waits, idle transactions,
   deadlocks, connection capacity, CPU, I/O, and database latency.
4. Classify database contention when lock waits, material idle transactions,
   deadlocks, near-limit connection capacity, near-limit CPU/I/O, or otherwise
   unexplained elevated database latency is present.
5. Classify `pool-starvation` only when pool wait is elevated, all core
   database-side evidence is present, and no database contention signal is
   elevated.
6. Return `needs-review` when evidence is incomplete or no evidence exists.
7. Set `suppress_pool_increase` only for `database-contention`; a review result
   is not permission to increase a pool.

The classifier can report `mixed` when more than one database-side cause is
present. It intentionally does not rank a lock wait against a deadlock or
idle transaction as if those causes were directly comparable; the findings
retain each signal and remediation.

## Statuses And Exit Codes

The JSON `status` is one of:

- `healthy`: complete supplied evidence shows no material contention.
- `pool-starvation`: pool wait is elevated and supplied database evidence is
  healthy.
- `database-contention`: at least one database-side contention signal is
  material; do not use a simple pool increase as the first remediation.
- `needs-review`: evidence is incomplete or insufficient to identify a cause.

The CLI exit codes are:

- `0`: `healthy` or `pool-starvation`.
- `2`: `database-contention`.
- `3`: `needs-review` with `--warn-exit`.
- `0`: `needs-review` without `--warn-exit`, for compatibility with advisory
  diagnostics.
- `1`: malformed input, unreadable config, invalid thresholds, or invalid
  evidence relationships.

The `2` result is intentionally CI-friendly. The classifier is not an
automatic kill-session tool; it only prevents a diagnostic workflow from
silently treating database contention as pool starvation.

## Output Contract

JSON output fields:

- `status`: classification status.
- `dominant_cause`: `none`, `pool-starvation`, `lock-contention`,
  `idle-transaction`, `deadlock`, `database-saturation`, `slow-database`,
  `mixed`, or `unknown`.
- `suppress_pool_increase`: true only for database contention.
- `evidence_categories`: number of supplied evidence categories used.
- `confidence`: `high`, `medium`, or `low`.
- `findings`: remediation-oriented findings with `code`, `risk`, `message`,
  and `remediation`.

CSV output contains the scalar fields and `finding_count`. Table output prints
the scalar fields followed by each finding. HTML uses the standard self-
contained report renderer and is suitable for incident notes or design review.

## Finding Codes

| Code | Risk | Meaning |
| --- | --- | --- |
| `POOL_WAIT_ELEVATED` | High | Application-pool acquisition wait is above threshold. |
| `LOCK_WAITING_SESSIONS` | High | Database sessions are waiting for locks. |
| `IDLE_IN_TRANSACTION` | High | Transactions remain open while sessions are idle. |
| `IDLE_TRANSACTION_DURATION_MISSING` | Medium | Idle sessions exist without transaction age. |
| `DEADLOCKS_DETECTED` | Critical | Deadlock rate is above policy. |
| `DATABASE_CONNECTIONS_NEAR_LIMIT` | High | Active sessions approach the supplied connection limit. |
| `CONNECTION_CAPACITY_EVIDENCE_INCOMPLETE` | Medium | Only one side of active/max connection evidence exists. |
| `DATABASE_CPU_NEAR_LIMIT` | High | CPU utilization approaches policy threshold. |
| `DATABASE_IO_NEAR_LIMIT` | High | I/O utilization approaches policy threshold. |
| `DATABASE_LATENCY_ELEVATED` | Medium | Database p99 latency exceeds policy threshold. |
| `POOL_STARVATION_LIKELY` | Medium | Pool wait is elevated while complete database evidence is healthy. |
| `DATABASE_EVIDENCE_INCOMPLETE` | Medium | Pool wait exists without complete database-side evidence. |
| `INSUFFICIENT_CONTENTION_EVIDENCE` | High | No evidence was supplied. |

Findings are additive and may expand in future releases. Consumers should
match stable codes they understand and preserve unknown codes.

## Rust API

The core API is reusable without spawning the CLI:

```rust
use poolsim_core::contention::{
    classify_database_contention, DatabaseContentionInput,
    DatabaseContentionStatus,
};

let input = DatabaseContentionInput::new()
    .with_pool_wait_p99_ms(80.0)
    .with_database_latency_p99_ms(120.0)
    .with_lock_waiting_sessions(3)
    .with_idle_in_transaction_sessions(0)
    .with_deadlocks_per_second(0.0)
    .with_active_sessions(70)
    .with_max_connections(100);

let report = classify_database_contention(&input)?;
if report.status == DatabaseContentionStatus::DatabaseContention {
    eprintln!("database contention found; do not increase the pool yet");
}
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Public Types

- `poolsim_core::contention::DatabaseContentionCause`: normalized dominant
  cause.
- `poolsim_core::contention::DatabaseContentionStatus`: final classification.
- `poolsim_core::contention::DatabaseContentionPolicy`: configurable numeric
  thresholds.
- `poolsim_core::contention::DatabaseContentionInput`: normalized evidence and
  policy.
- `poolsim_core::contention::DatabaseContentionFinding`: stable finding code,
  risk, explanation, and remediation.
- `poolsim_core::contention::DatabaseContentionReport`: status, cause,
  suppression decision, evidence count, confidence, and findings.
- `poolsim_core::contention::classify_database_contention`: classification
  function returning a report or `PoolsimError`.

### Constructors And Builders

`DatabaseContentionPolicy::new` returns the default policy. Its additive
builders are:

- `DatabaseContentionPolicy::with_pool_wait_p99_threshold_ms`
- `DatabaseContentionPolicy::with_database_latency_p99_threshold_ms`
- `DatabaseContentionPolicy::with_idle_transaction_threshold_seconds`
- `DatabaseContentionPolicy::with_deadlocks_per_second_threshold`
- `DatabaseContentionPolicy::with_database_cpu_utilization_threshold`
- `DatabaseContentionPolicy::with_database_io_utilization_threshold`
- `DatabaseContentionPolicy::with_connection_utilization_threshold`

`DatabaseContentionInput::new` and `DatabaseContentionInput::default` create
empty evidence with the default policy. Its builders are:

- `DatabaseContentionInput::with_pool_wait_p99_ms`
- `DatabaseContentionInput::with_database_latency_p99_ms`
- `DatabaseContentionInput::with_lock_waiting_sessions`
- `DatabaseContentionInput::with_idle_in_transaction_sessions`
- `DatabaseContentionInput::with_longest_idle_in_transaction_seconds`
- `DatabaseContentionInput::with_deadlocks_per_second`
- `DatabaseContentionInput::with_active_sessions`
- `DatabaseContentionInput::with_max_connections`
- `DatabaseContentionInput::with_database_cpu_utilization`
- `DatabaseContentionInput::with_database_io_utilization`
- `DatabaseContentionInput::with_policy`

All input and output types implement Serde serialization. The core structs are
non-exhaustive so future evidence and findings can be added without requiring
downstream callers to construct an exhaustive representation.

## CI And Incident Use

Use the classifier as an advisory or blocking diagnostic step:

```yaml
- name: Diagnose database contention
  run: >-
    poolsim --format json --warn-exit check db-contention
    --config artifacts/database-contention.json
```

Recommended response to `database-contention`:

1. Inspect blocking sessions and lock ownership.
2. Find long-lived or idle-in-transaction sessions.
3. Review transaction boundaries, lock ordering, and query plans.
4. Check deadlock retry multiplication before raising concurrency.
5. Re-run the classifier after remediation.

Recommended response to `pool-starvation`:

1. Validate that database metrics cover the same time window.
2. Run the pool scale-safety gate with the shared connection budget.
3. Compare normal, peak, and incident scenarios.
4. Change the runtime pool only after the budget and rollout plan are approved.

## Limitations

- The classifier does not query PostgreSQL, MySQL, a pooler, or a cloud API.
- A point-in-time snapshot can miss short lock waits or transient deadlocks.
- A high database latency percentile does not identify the exact query or
  blocker; use provider-specific query and lock views for that.
- `active_sessions` and `max_connections` must have compatible scope. Do not
  compare one service's sessions with a whole-cluster limit without accounting
  for other consumers.
- CPU and I/O utilization semantics vary by provider. Normalize them and keep
  source metadata outside the classifier input.
- `idle_in_transaction_sessions` without transaction age is review evidence,
  not proof that sessions have exceeded the configured duration threshold.
- Healthy classification means no material signal was found in supplied
  evidence; it does not prove the database is globally healthy.

## Compatibility

This feature adds one `poolsim-core` module, one `check db-contention` CLI
subcommand, and one fixture. Existing APIs, commands, output fields, REST
routes, WebSocket messages, and exit-code behavior are unchanged.

## Sources

- [PostgreSQL cumulative statistics and `pg_stat_activity`](https://www.postgresql.org/docs/current/monitoring-stats.html)
- [PostgreSQL lock monitoring](https://www.postgresql.org/docs/current/monitoring-locks.html)
- [MySQL Performance Schema wait events](https://dev.mysql.com/doc/refman/8.0/en/performance-schema-wait-tables.html)
- [MySQL `data_lock_waits`](https://dev.mysql.com/doc/refman/8.0/en/performance-schema-data-lock-waits-table.html)

## Public API Inventory

The CLI argument type is `poolsim_cli::args::DatabaseContentionCheckArgs`.
The complete public module is `poolsim_core::contention`. Its documented
symbols are `poolsim_core::contention::DatabaseContentionCause`,
`poolsim_core::contention::DatabaseContentionStatus`,
`poolsim_core::contention::DatabaseContentionPolicy`,
`poolsim_core::contention::DatabaseContentionPolicy::new`,
`poolsim_core::contention::DatabaseContentionPolicy::with_pool_wait_p99_threshold_ms`,
`poolsim_core::contention::DatabaseContentionPolicy::with_database_latency_p99_threshold_ms`,
`poolsim_core::contention::DatabaseContentionPolicy::with_idle_transaction_threshold_seconds`,
`poolsim_core::contention::DatabaseContentionPolicy::with_deadlocks_per_second_threshold`,
`poolsim_core::contention::DatabaseContentionPolicy::with_database_cpu_utilization_threshold`,
`poolsim_core::contention::DatabaseContentionPolicy::with_database_io_utilization_threshold`,
`poolsim_core::contention::DatabaseContentionPolicy::with_connection_utilization_threshold`,
`poolsim_core::contention::DatabaseContentionInput`,
`poolsim_core::contention::DatabaseContentionInput::new`,
`poolsim_core::contention::DatabaseContentionInput::default`,
`poolsim_core::contention::DatabaseContentionInput::with_pool_wait_p99_ms`,
`poolsim_core::contention::DatabaseContentionInput::with_database_latency_p99_ms`,
`poolsim_core::contention::DatabaseContentionInput::with_lock_waiting_sessions`,
`poolsim_core::contention::DatabaseContentionInput::with_idle_in_transaction_sessions`,
`poolsim_core::contention::DatabaseContentionInput::with_longest_idle_in_transaction_seconds`,
`poolsim_core::contention::DatabaseContentionInput::with_deadlocks_per_second`,
`poolsim_core::contention::DatabaseContentionInput::with_active_sessions`,
`poolsim_core::contention::DatabaseContentionInput::with_max_connections`,
`poolsim_core::contention::DatabaseContentionInput::with_database_cpu_utilization`,
`poolsim_core::contention::DatabaseContentionInput::with_database_io_utilization`,
`poolsim_core::contention::DatabaseContentionInput::with_policy`,
`poolsim_core::contention::DatabaseContentionFinding`,
`poolsim_core::contention::DatabaseContentionReport`, and
`poolsim_core::contention::classify_database_contention`.
