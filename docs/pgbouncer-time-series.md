# PgBouncer Time-Series Evidence

Poolsim can compare two timestamped PgBouncer observations to determine whether
downstream queue pressure is absent, present, or growing. This is a diagnostic
workflow, not a live PgBouncer client: it does not open a database connection,
run SQL, query Prometheus, or change pooler configuration.

The feature combines:

- cumulative counters from PgBouncer `SHOW STATS`, especially
  `total_query_count` and `total_wait_time`;
- queue gauges from the matching `SHOW POOLS` capture, especially `maxwait` and
  `cl_waiting`; and
- two timestamps that define the observation interval.

The core implementation is exposed by
`poolsim_core::pooler::PgbouncerStatsRow`,
`poolsim_core::pooler::PgbouncerShowStatsSnapshot`,
`poolsim_core::pooler::PgbouncerTimeSeriesSample`, and
`poolsim_core::pooler::PgbouncerTimeSeriesDeltaReport`.

## Why Two Kinds Of Evidence Matter

PgBouncer `SHOW STATS` counters are cumulative. They describe the amount of
work and client wait accumulated since PgBouncer started or since its counters
were reset. A single counter value is not a request rate or a wait rate.

`SHOW POOLS` provides instantaneous queue state:

- `cl_waiting` is the number of clients waiting for a server connection;
- `maxwait` is the age of the oldest waiting client; and
- `sv_active` and `sv_idle` describe backend/server usage.

Poolsim therefore calculates rates only from the difference between two
`PgbouncerTimeSeriesSample` values. It treats queue gauges as state at the
current capture and compares them across the interval when both captures
contain the same gauge.

The upstream PgBouncer documentation describes `total_query_count`,
`total_wait_time`, `avg_wait_time`, `cl_waiting`, and `maxwait` in its admin
console reference:

- [PgBouncer `SHOW STATS` and `SHOW POOLS` documentation](https://github.com/pgbouncer/pgbouncer.github.io/blob/master/usage.md)
- [Prometheus metric types](https://prometheus.io/docs/concepts/metric_types/)
- [Prometheus counter functions](https://prometheus.io/docs/prometheus/3.5/querying/functions/)
- [Prometheus instrumentation guidance](https://prometheus.io/docs/practices/the_zen/)
- [Prometheus Community PgBouncer exporter](https://github.com/prometheus-community/pgbouncer_exporter)

## Capture Contract

Each capture must contain:

1. A finite Unix timestamp in seconds.
2. At least one `SHOW STATS` row.
3. A cumulative `total_query_count` value for each row.
4. A cumulative `total_wait_time` value for each row.

For actionable queue trend detection, also capture both of these at the same
timestamp:

- `maxwait` as `maxwait_seconds`; and
- the aggregated `cl_waiting` count as `client_waiting`.

The gauges are optional in the data model because exporters and historical
archives may contain only statistics counters. Poolsim marks missing gauges as
`NeedsReview`; it never interprets missing queue evidence as a healthy queue.

### PgBouncer SQL Capture

An operator can capture the two sources from the PgBouncer admin database. The
exact capture command depends on the SQL client and authentication setup, but a
typical manual workflow is:

```sql
SHOW STATS;
SHOW POOLS;
```

Save the output from each command with the capture timestamp. The library
parser accepts default `psql` aligned output and `psql --csv` output. A
`SHOW STATS` CSV capture can look like this:

```csv
database,total_query_count,total_wait_time,avg_wait_time
checkout,1000,20000,20
jobs,250,5000,20
```

The parser does not require every PgBouncer column. It uses only the stable
cumulative counters needed for the delta and preserves `database` and
`avg_wait_time` when available.

### Exporter Capture

The Prometheus Community PgBouncer exporter exposes the same concepts as
metrics. Common mappings include:

| PgBouncer concept | Exporter metric concept | Type | Poolsim field |
| --- | --- | --- | --- |
| `total_query_count` | pooled query counter | counter | `total_query_count` |
| `total_wait_time` | client wait seconds counter | counter | `total_wait_time_us` after unit conversion |
| `maxwait` | client max-wait gauge | gauge | `maxwait_seconds` |
| `cl_waiting` | waiting-client gauge | gauge | `client_waiting` |

Keep raw counters in the source archive and derive rates from the counter
difference or a Prometheus `rate()` query. Do not sum already-derived rates
across labels. When aggregating multiple PgBouncer databases, aggregate the
cumulative counters first and then compute one interval rate.

## Rust Library Workflow

### Parse `SHOW STATS`

Use `poolsim_core::pooler::parse_pgbouncer_show_stats` for either aligned or
CSV output:

```rust
use poolsim_core::pooler::parse_pgbouncer_show_stats;

let rows = parse_pgbouncer_show_stats(
    "database,total_query_count,total_wait_time,avg_wait_time\ncheckout,1000,20000,20\n",
)?;

assert_eq!(rows[0].total_query_count, 1_000);
assert_eq!(rows[0].total_wait_time_us, 20_000);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

The parser returns `INVALID_PGBOUNCER_SHOW_STATS` for a missing header,
missing required column, malformed row, invalid unsigned counter, or malformed
CSV quoting. A capture with no data rows is also invalid.

### Build A Timestamped Sample

Use `poolsim_core::pooler::PgbouncerShowStatsSnapshot::new` to associate the
parsed rows with a timestamp, then add the queue gauges when they came from the
same observation:

```rust
use poolsim_core::pooler::{
    parse_pgbouncer_show_stats, summarize_pgbouncer_show_stats,
    PgbouncerShowStatsSnapshot,
};

let rows = parse_pgbouncer_show_stats(
    "database,total_query_count,total_wait_time\ncheckout,1000,20000\n",
)?;
let sample = summarize_pgbouncer_show_stats(
    &PgbouncerShowStatsSnapshot::new(1_700_000_000.0, rows)
        .with_label("checkout-pgbouncer")
        .with_maxwait_seconds(0.0)
        .with_client_waiting(0),
)?;

assert_eq!(sample.total_query_count, 1_000);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

`poolsim_core::pooler::summarize_pgbouncer_show_stats` aggregates rows with
saturating arithmetic. This avoids overflow turning a large multi-database
capture into a negative or wrapped value. It does not use the per-row
`avg_wait_time` as the aggregate average because a weighted interval average is
more reliable and is calculated from counter deltas.

### Compare Two Samples

Serialize the resulting `PgbouncerTimeSeriesSample` values in the application
that collects evidence, or pass them directly to
`poolsim_core::pooler::diff_pgbouncer_time_series`:

```rust
use poolsim_core::pooler::{
    diff_pgbouncer_time_series, PgbouncerTimeSeriesSample,
    PgbouncerTimeSeriesStatus,
};

let previous = PgbouncerTimeSeriesSample {
    timestamp_seconds: 100.0,
    label: Some("checkout-pgbouncer".to_string()),
    total_query_count: 1_000,
    total_wait_time_us: 20_000,
    maxwait_seconds: Some(0.1),
    client_waiting: Some(1),
};
let current = PgbouncerTimeSeriesSample {
    timestamp_seconds: 110.0,
    label: Some("checkout-pgbouncer".to_string()),
    total_query_count: 1_200,
    total_wait_time_us: 70_000,
    maxwait_seconds: Some(0.8),
    client_waiting: Some(4),
};

let report = diff_pgbouncer_time_series(&previous, &current)?;
assert_eq!(report.interval_seconds, 10.0);
assert_eq!(report.query_rate_per_second, 20.0);
assert_eq!(report.average_wait_ms_per_query, Some(0.25));
assert_eq!(report.status, PgbouncerTimeSeriesStatus::QueueGrowing);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

The report exposes:

- `interval_seconds`: current timestamp minus previous timestamp;
- `query_rate_per_second`: cumulative query delta divided by the interval;
- `wait_time_rate_us_per_second`: cumulative wait-time delta divided by the
  interval;
- `average_wait_ms_per_query`: wait-time delta divided by query delta, with
  microseconds converted to milliseconds;
- `current_maxwait_seconds` and `maxwait_delta_seconds`;
- `current_client_waiting`;
- `counter_resets`;
- `findings`; and
- `confidence`.

If no queries completed during the interval,
`average_wait_ms_per_query` is `None`. This is different from zero wait: it
means the interval did not provide a denominator.

## Status Semantics

`poolsim_core::pooler::PgbouncerTimeSeriesStatus` is deliberately conservative:

| Status | Meaning | Operational response |
| --- | --- | --- |
| `Healthy` | Both queue gauges are present, continuous, and zero at the current capture. | Continue observing; do not increase a pool solely from cumulative counters. |
| `QueuePresent` | Current `maxwait_seconds` is above zero or current `client_waiting` is above zero, but no gauge increased from the previous capture. | Check whether the queue is transient and correlate with backend latency. |
| `QueueGrowing` | `maxwait_seconds` increased or `client_waiting` increased. | Treat the downstream pooler as under pressure; inspect database service time, long transactions, and backend limits before increasing application pools. |
| `CounterReset` | One or more cumulative counters decreased, with no stronger current queue signal. | Verify PgBouncer restarts, exporter continuity, and label identity before using the interval for trend decisions. |
| `NeedsReview` | Queue gauges are missing from one or both captures and no stronger queue/reset status applies. | Capture `maxwait` and `cl_waiting` at both timestamps before making an automated decision. |

Queue status has precedence over counter-reset and missing-gauge status. For
example, a queue that is growing during a PgBouncer restart is still reported
as `QueueGrowing`, while `counter_resets` and the reset finding remain present
so the restart is not hidden.

### Findings

Findings are machine-readable through `PoolerFinding::code` and include a
human-readable message and remediation:

- `PGBOUNCER_QUEUE_GROWING`: queue pressure increased;
- `PGBOUNCER_QUEUE_PRESENT`: a current queue is present;
- `PGBOUNCER_COUNTER_RESET`: a cumulative counter decreased;
- `PGBOUNCER_MAXWAIT_MISSING`: oldest-client queue age was not captured at both
  timestamps; and
- `PGBOUNCER_WAITING_GAUGE_MISSING`: waiting-client count was not captured at
  both timestamps.

These codes are suitable for CI annotations and alert routing. The strings are
stable report data; callers should match codes rather than human-readable
messages.

## Counter Reset Behavior

Counters can decrease when PgBouncer restarts, an exporter changes source, a
label set changes, or a historical series is stitched incorrectly. Poolsim
does not emit negative rates. For a decreased counter, it uses the current
value as post-reset accumulation and adds the matching
`poolsim_core::pooler::PgbouncerCounterKind` to `counter_resets`.

This is intentionally similar to Prometheus reset-aware counter handling, but
it does not pretend that a reset interval is a clean steady-state interval.
The report lowers confidence to `Medium` and emits
`PGBOUNCER_COUNTER_RESET`. Persist the reset marker with the sample so a later
analysis can exclude or separately annotate the interval.

## CLI Workflow

The CLI command consumes JSON files containing
`poolsim_core::pooler::PgbouncerTimeSeriesSample` values. This keeps collection
and analysis separate: a collector can parse SQL output, convert exporter
metrics, or use an existing metrics pipeline, while CI and incident tooling
can compare immutable JSON artifacts.

The additive CLI argument model is represented by
`poolsim_cli::args::PgbouncerTimeseriesImportArgs`; it contains the `previous`
and `current` JSON paths and does not alter any existing import arguments.

Example `previous.json`:

```json
{
  "timestamp_seconds": 1700000000,
  "label": "checkout-pgbouncer",
  "total_query_count": 1000,
  "total_wait_time_us": 20000,
  "maxwait_seconds": 0.1,
  "client_waiting": 1
}
```

Example `current.json`:

```json
{
  "timestamp_seconds": 1700000010,
  "label": "checkout-pgbouncer",
  "total_query_count": 1200,
  "total_wait_time_us": 70000,
  "maxwait_seconds": 0.8,
  "client_waiting": 4
}
```

Run the comparison:

```bash
poolsim --format json --warn-exit import pgbouncer-timeseries \
  --previous previous.json \
  --current current.json
```

Supported output formats are:

```bash
poolsim --format table import pgbouncer-timeseries --previous previous.json --current current.json
poolsim --format json import pgbouncer-timeseries --previous previous.json --current current.json
poolsim --format csv import pgbouncer-timeseries --previous previous.json --current current.json
poolsim --format html import pgbouncer-timeseries --previous previous.json --current current.json
```

The command uses the same structured report for every format. JSON is best for
automation, CSV is useful for simple evidence archives, table is useful during
an incident, and HTML is useful for attaching a self-contained report to a
review or incident record.

## Exit Codes

Without `--warn-exit`, only a growing queue returns a non-zero diagnostic code:

- `0`: healthy, queue present, counter reset, or needs review;
- `2`: queue pressure is growing; and
- `1`: input could not be read or parsed, or the timestamps were invalid.

With `--warn-exit`:

- `0`: healthy;
- `2`: queue pressure is growing;
- `3`: queue present, counter reset, or incomplete evidence; and
- `1`: invalid command input or malformed JSON.

This allows a strict deployment gate to fail on growing pressure while a
monitoring job can opt into warnings for weaker evidence. Do not interpret
exit code `0` with missing gauges as proof that the downstream pooler is
healthy; use the JSON `status` and `confidence` fields.

## Production Collection Guidance

1. Keep the same service, pooler, and database label identity between captures.
2. Record the collector timestamp and use one clock source consistently.
3. Capture at a fixed interval appropriate to the incident or workload window.
4. Store raw cumulative counters so a reset can be investigated later.
5. Capture `maxwait` and `cl_waiting` with every sample.
6. Correlate queue growth with backend latency, long transactions, database
   saturation, and application-pool waiting.
7. Check the database connection budget before increasing application or
   pooler capacity.
8. Treat a counter reset as a data-quality event, not as evidence that load
   disappeared.

Time-series evidence identifies downstream queue pressure; it does not prove
that adding connections is safe. Use `poolsim doctor pgbouncer-pools` for a
single-capture application-versus-pooler diagnosis and
`poolsim budget` when several services share a database connection budget.

## Public API Inventory

The additive API documented by this guide is:

- `poolsim_core::pooler::PgbouncerStatsRow`;
- `poolsim_core::pooler::PgbouncerStatsRow::new`;
- `poolsim_core::pooler::PgbouncerStatsRow::with_database`;
- `poolsim_core::pooler::PgbouncerStatsRow::with_avg_wait_time_us`;
- `poolsim_core::pooler::PgbouncerShowStatsSnapshot`;
- `poolsim_core::pooler::PgbouncerShowStatsSnapshot::new`;
- `poolsim_core::pooler::PgbouncerShowStatsSnapshot::with_label`;
- `poolsim_core::pooler::PgbouncerShowStatsSnapshot::with_maxwait_seconds`;
- `poolsim_core::pooler::PgbouncerShowStatsSnapshot::with_client_waiting`;
- `poolsim_core::pooler::PgbouncerTimeSeriesSample`;
- `poolsim_core::pooler::PgbouncerCounterKind`;
- `poolsim_core::pooler::PgbouncerTimeSeriesStatus`;
- `poolsim_core::pooler::PgbouncerTimeSeriesDeltaReport`;
- `poolsim_core::pooler::parse_pgbouncer_show_stats`;
- `poolsim_core::pooler::summarize_pgbouncer_show_stats`; and
- `poolsim_core::pooler::diff_pgbouncer_time_series`.

All existing pooler evidence APIs remain available. This slice adds no network
dependency, no credential requirement, no feature flag, and no change to an
existing serialized field or CLI command.

## Troubleshooting

### `INVALID_PGBOUNCER_SHOW_STATS`

Check that the capture includes a header with `total_query_count` and
`total_wait_time`, that every data row has the same number of columns, and that
counter cells contain unsigned integers. For CSV, preserve quoted fields and
do not paste a partial row.

### `INVALID_PGBOUNCER_TIME_SERIES`

Check that timestamps are finite Unix seconds and that the current timestamp is
strictly greater than the previous timestamp. Also check that `maxwait_seconds`
is finite and non-negative.

### `NeedsReview` with zero current queue

This means the current queue is zero but one or both queue gauges were absent
from the pair. Capture both `maxwait` and `cl_waiting` at both timestamps before
turning the result into an automated healthy decision.

### Unexpected `CounterReset`

Check PgBouncer restart history, exporter target changes, scrape labels, and
whether the two artifacts came from the same pooler instance. Do not subtract
the values manually or clamp a negative delta to zero without retaining the
reset event.

### Support

Report issues at
<https://github.com/gregorian-09/poolsim/issues>. Include the poolsim version,
PgBouncer version, sanitized headers and sample rows, timestamps, output JSON,
and the exact command. Remove credentials, connection strings, customer data,
and unrestricted query text before posting captures.
