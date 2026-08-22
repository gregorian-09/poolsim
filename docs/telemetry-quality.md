# Telemetry Quality And Coordinated Omission

Poolsim recommendations are only as defensible as the evidence used to build
them. A p50/p95/p99 latency triple without an arrival model, achieved rate,
timeouts, errors, pool wait, and database latency can produce a plausible
number while hiding the reason the system slowed down.

The telemetry-quality check is an independent preflight assessment. It does
not change `TelemetrySnapshot`, `TelemetryRecommendation`, or any existing
`simulate`, `import`, `gate`, `guard`, or `doctor` behavior. Run it before a
capacity decision when the source is a load test, a telemetry export, or a
historical production window.

The public library API is in
`poolsim_core::telemetry_quality`:

- `poolsim_core::telemetry_quality::TelemetryQualityInput` describes the
  evidence;
- `poolsim_core::telemetry_quality::assess_telemetry_quality` evaluates it;
- `poolsim_core::telemetry_quality::TelemetryQualityReport` is the structured
  result; and
- `poolsim_core::telemetry_quality::TelemetryQualityFinding` explains each
  missing signal or risk.

## Why This Check Exists

### Closed-loop measurements can hide stalls

In a closed-loop load generator, a worker often sends the next request only
after the previous response completes. If the service pauses for one second,
that worker is not issuing requests during the pause. The measured request
latency may still be correct for the requests that were sent, but the sample
set can omit the time during which the load generator was unable to maintain
the intended arrival schedule. This is coordinated omission.

An open-loop or constant-rate model schedules arrivals independently from
completion. A corrected closed-loop histogram can also be useful, but the
correction method and expected interval must be recorded with the evidence.

Sources:

- [`wrk2` constant-throughput and latency methodology](https://github.com/giltene/wrk2)
- [HdrHistogram coordinated-omission correction](https://hdrhistogram.github.io/HdrHistogram/JavaDoc/org/HdrHistogram/AbstractHistogram.html)
- [Prometheus histograms and summaries](https://prometheus.io/docs/practices/histograms/)
- [Prometheus `histogram_quantile`](https://prometheus.io/docs/prometheus/latest/querying/functions/#histogram_quantile)
- [OpenTelemetry metrics concepts](https://opentelemetry.io/docs/concepts/signals/metrics/)
- [OpenTelemetry metrics API](https://opentelemetry.io/docs/specs/otel/metrics/api/)

### Percentiles are not enough

Percentiles describe the distribution of recorded latency observations. They
do not identify whether the run reached the intended rate, how many requests
timed out, whether application-pool acquisition was slow, or whether database
service time increased.

Poolsim therefore asks for evidence that answers four separate questions:

1. **Was the intended workload actually offered?** Compare expected and
   observed request rates.
2. **Could the arrival model omit stalled work?** Record open-loop,
   closed-loop, or unknown and whether correction was applied.
3. **What happened to requests that did not complete normally?** Export errors
   and timeouts separately.
4. **Where was time spent?** Export application-pool wait and database service
   latency in addition to end-to-end latency.

## Input Contract

`poolsim_core::telemetry_quality::TelemetryQualityInput` is deliberately
separate from the existing sizing input. It can be serialized as JSON and
stored beside the normal `TelemetrySnapshot` without changing that published
schema.

### Required for a valid capacity decision

The check treats these as essential evidence:

- `arrival_model`;
- `expected_requests_per_second`;
- `observed_requests_per_second`;
- `duration_seconds`;
- `sample_count`;
- `timeout_count`;
- `error_count`;
- `pool_wait_p99_ms`; and
- `database_latency_p99_ms`.

The latency percentiles are validated when supplied. They are not required by
the quality checker itself because another pipeline may keep them in the
existing `TelemetrySnapshot`, but a complete production workflow should supply
`latency_p50_ms`, `latency_p95_ms`, and `latency_p99_ms` too.

All floating-point values must be finite and non-negative. Expected rate and
duration must be greater than zero. `sample_count` must be greater than zero.
When multiple percentiles are supplied, they must be monotonically ordered:

```text
latency_p50_ms <= latency_p95_ms <= latency_p99_ms
```

Invalid numeric input returns `INVALID_TELEMETRY_QUALITY` instead of producing
a low-confidence report with fabricated values.

### Arrival models

`poolsim_core::telemetry_quality::TelemetryArrivalModel` has three values:

| Value | Meaning | Default interpretation |
| --- | --- | --- |
| `open-loop` | Request arrivals are scheduled independently from response completion. | Preferred for capacity planning. |
| `closed-loop` | A worker schedules the next request after a prior request completes. | High coordinated-omission risk unless corrected. |
| `unknown` | The source did not preserve the arrival model. | Not safe for automatic decisions. |

`corrected_for_coordinated_omission` should be `true` only when the source
actually applied a documented correction using the expected measurement
interval. It is not a flag to set merely because a test used a high request
rate.

## Report Semantics

`poolsim_core::telemetry_quality::TelemetryQualityStatus` contains:

| Status | Meaning | Recommended action |
| --- | --- | --- |
| `valid` | Open-loop evidence is complete, the achieved rate is close to the intended rate, and no quality finding remains. | The evidence can proceed to sizing, subject to normal database-budget review. |
| `needs-review` | The evidence is useful but has missing signals, an unknown model, a corrected closed-loop model, or moderate offered-rate drift. | Review the findings and record the limitation before acting. |
| `rejected` | The run has high coordinated-omission risk or achieved less than 90% of the intended rate. | Do not use the run as the basis for an automatic pool-size change; rerun or repair collection. |

`observed_rate_ratio` is:

```text
observed_requests_per_second / expected_requests_per_second
```

The thresholds are intentionally conservative:

- below `0.90`: `OFFERED_RATE_NOT_REACHED`, status `rejected`;
- `0.90` through below `0.95`: `OFFERED_RATE_DRIFT`, status at least
  `needs-review`; and
- `0.95` or higher: no offered-rate finding.

These thresholds do not claim that a 5% difference is universally safe. They
identify evidence that needs review; teams can apply stricter policy in CI.

## Findings

`poolsim_core::telemetry_quality::TelemetryQualityFinding` provides a stable
`code`, `risk`, `message`, and `remediation`. Match `code` in automation, not
the human-readable message.

| Code | Trigger | Risk |
| --- | --- | --- |
| `COORDINATED_OMISSION_RISK` | Closed-loop source, corrected or uncorrected. | Medium when corrected, High otherwise. |
| `ARRIVAL_MODEL_UNKNOWN` | Source did not identify its arrival model. | High. |
| `OFFERED_RATE_NOT_REACHED` | Observed rate is below 90% of expected. | Critical. |
| `OFFERED_RATE_DRIFT` | Observed rate is below 95% but at least 90% of expected. | High. |
| `ARRIVAL_RATE_MISSING` | Expected or observed request rate is absent. | High. |
| `SAMPLE_COUNT_MISSING` | Latency observation count is absent. | Medium. |
| `DURATION_MISSING` | Capture duration is absent. | Medium. |
| `TIMEOUT_COUNT_MISSING` | Timeout count is absent. | High. |
| `ERROR_COUNT_MISSING` | Error count is absent. | Medium. |
| `POOL_WAIT_METRIC_MISSING` | Application-pool acquisition p99 is absent. | Medium. |
| `DATABASE_LATENCY_METRIC_MISSING` | Database service latency p99 is absent. | Medium. |

Missing operational signals lower confidence but do not cause a parse error.
This distinction lets a dashboard show that a capture exists while still
preventing it from being silently treated as complete evidence.

## Rust API

### Build complete open-loop evidence

```rust
use poolsim_core::telemetry_quality::{
    assess_telemetry_quality, TelemetryArrivalModel, TelemetryQualityInput,
    TelemetryQualityStatus,
};

let input = TelemetryQualityInput::new(TelemetryArrivalModel::OpenLoop)
    .with_expected_requests_per_second(1_000.0)
    .with_observed_requests_per_second(1_000.0)
    .with_duration_seconds(60.0)
    .with_sample_count(60_000)
    .with_timeout_count(2)
    .with_error_count(5)
    .with_latency_percentiles(10.0, 30.0, 70.0)
    .with_pool_wait_p99_ms(4.0)
    .with_database_latency_p99_ms(18.0);

let report = assess_telemetry_quality(&input)?;
assert_eq!(report.status, TelemetryQualityStatus::Valid);
assert_eq!(report.observed_rate_ratio, Some(1.0));
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Detect an uncorrected closed-loop run

```rust
use poolsim_core::telemetry_quality::{
    assess_telemetry_quality, TelemetryArrivalModel, TelemetryQualityInput,
    TelemetryQualityStatus,
};

let input = TelemetryQualityInput::new(TelemetryArrivalModel::ClosedLoop)
    .with_expected_requests_per_second(500.0)
    .with_observed_requests_per_second(500.0)
    .with_duration_seconds(60.0)
    .with_sample_count(30_000)
    .with_timeout_count(0)
    .with_error_count(0)
    .with_latency_percentiles(8.0, 25.0, 60.0)
    .with_pool_wait_p99_ms(2.0)
    .with_database_latency_p99_ms(15.0);

let report = assess_telemetry_quality(&input)?;
assert_eq!(report.status, TelemetryQualityStatus::Rejected);
assert_eq!(report.coordinated_omission_risk, poolsim_core::RiskLevel::High);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Inspect findings

```rust
use poolsim_core::telemetry_quality::{
    assess_telemetry_quality, TelemetryArrivalModel, TelemetryQualityInput,
};

let input = TelemetryQualityInput::new(TelemetryArrivalModel::OpenLoop)
    .with_expected_requests_per_second(100.0)
    .with_observed_requests_per_second(85.0);
let report = assess_telemetry_quality(&input)?;

for finding in &report.findings {
    println!("{}: {}", finding.code, finding.remediation);
}
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

The builders are optional. JSON deserialization can populate the same public
fields, which is useful for language bindings and CI artifacts.

## CLI Usage

Create `telemetry-quality.json`:

```json
{
  "arrival_model": "open-loop",
  "expected_requests_per_second": 1000,
  "observed_requests_per_second": 1000,
  "duration_seconds": 60,
  "sample_count": 60000,
  "timeout_count": 2,
  "error_count": 5,
  "latency_p50_ms": 10,
  "latency_p95_ms": 30,
  "latency_p99_ms": 70,
  "pool_wait_p99_ms": 4,
  "database_latency_p99_ms": 18
}
```

Run the check:

```bash
poolsim --format json --warn-exit check telemetry-quality \
  --config telemetry-quality.json
```

All output formats are supported:

```bash
poolsim --format table check telemetry-quality --config telemetry-quality.json
poolsim --format json check telemetry-quality --config telemetry-quality.json
poolsim --format csv check telemetry-quality --config telemetry-quality.json
poolsim --format html check telemetry-quality --config telemetry-quality.json
```

Exit behavior:

- `0`: valid, or needs review when `--warn-exit` is not used;
- `2`: rejected evidence;
- `3`: needs review when `--warn-exit` is used; and
- `1`: unreadable JSON or invalid numeric input.

This makes the command suitable for a CI preflight without changing the exit
codes of existing `simulate`, `import`, `gate`, or `guard` commands.

## Prometheus And OpenTelemetry Collection

### Prometheus

Use request counters and histograms rather than exporting only precomputed
quantiles when multiple service instances must be aggregated. For a classic
histogram, a typical PromQL pattern is:

```promql
sum(rate(http_server_request_duration_seconds_bucket[5m]))
```

Then use `histogram_quantile` with `le` retained for classic histograms:

```promql
histogram_quantile(
  0.99,
  sum by (le) (rate(http_server_request_duration_seconds_bucket[5m]))
)
```

Export the request counter rate separately so the quality input can compare
the intended offered rate and the achieved rate. Export timeout and error
counters separately; do not infer either from a latency percentile.

### OpenTelemetry

OpenTelemetry histograms are appropriate for request-duration distributions.
Preserve metric name, unit, resource identity, service identity, and the
aggregation window when converting an OTLP export into a quality input. Avoid
high-cardinality attributes such as raw URLs, user IDs, or unbounded query
text. Keep the aggregation labels consistent across replicas so the resulting
histograms can be combined correctly.

## What The Check Does Not Prove

Even `valid` telemetry does not prove that a larger pool is safe. It does not
measure:

- database `max_connections` budget across all services;
- failover behavior or connection-retry storms;
- lock waits, idle transactions, or transaction duration;
- network/NAT/conntrack capacity;
- provider-specific proxy pinning; or
- whether the selected latency window represents peak and incident traffic.

Use the quality report as an evidence gate, then use `poolsim doctor`, pooler
diagnostics, and `poolsim budget` to interpret the capacity decision.

## Safe Operating Procedure

1. Run an open-loop or constant-rate test when possible.
2. Record intended rate, achieved rate, test duration, sample count, errors, and
   timeouts.
3. Export end-to-end latency, application-pool wait, and database service
   latency from the same window.
4. Run `poolsim check telemetry-quality` and preserve its JSON report.
5. Reject or repair evidence with `rejected` status before sizing.
6. Review every finding before applying a recommendation.
7. Compare the final recommendation against replica count and the shared
   database connection budget.

## Public API Inventory

This guide documents every new public item:

- `poolsim_core::telemetry_quality`;
- `poolsim_core::telemetry_quality::TelemetryArrivalModel`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::new`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_expected_requests_per_second`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_observed_requests_per_second`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_duration_seconds`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_sample_count`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_timeout_count`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_error_count`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_latency_percentiles`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_pool_wait_p99_ms`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_database_latency_p99_ms`;
- `poolsim_core::telemetry_quality::TelemetryQualityInput::with_coordinated_omission_correction`;
- `poolsim_core::telemetry_quality::TelemetryQualityFinding`;
- `poolsim_core::telemetry_quality::TelemetryQualityStatus`;
- `poolsim_core::telemetry_quality::TelemetryQualityReport`; and
- `poolsim_core::telemetry_quality::assess_telemetry_quality`.

The additive CLI argument type is
`poolsim_cli::args::TelemetryQualityCheckArgs`, exposed through
`check telemetry-quality`.

## Troubleshooting

### `rejected` with `COORDINATED_OMISSION_RISK`

The source used a closed-loop arrival model without recorded correction. Run
an open-loop constant-rate test or document the expected interval and use a
corrected histogram implementation. Do not simply relabel the run as
open-loop.

### `rejected` with `OFFERED_RATE_NOT_REACHED`

The generator or system completed less than 90% of the intended rate. Check
generator CPU, connection limits, client-side queues, timeouts, errors, and
server admission controls. A lower achieved rate is not equivalent to a
successful test at the intended rate.

### `needs-review` with missing signals

The report is not a parse failure. It means the capture can be inspected but
does not contain enough independent signals for an automatic capacity change.
Add the missing metrics and rerun the check.

### Support

Open an issue at
<https://github.com/gregorian-09/poolsim/issues>. Include the poolsim version,
the sanitized quality-input JSON, the load-generator arrival model, the
analysis window, and the exact finding codes. Remove credentials, customer
data, raw URLs, and unrestricted query text before sharing telemetry.
