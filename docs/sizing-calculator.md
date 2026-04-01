# Poolsim Sizing Calculator

## Purpose

Poolsim is a sizing calculator for connection pools. It estimates the pool size that satisfies latency and utilization constraints under a given workload profile.

The calculator is exposed in three forms:

- Rust library (`poolsim-core`)
- CLI (`poolsim-cli`)
- HTTP/WebSocket service (`poolsim-web`)

## What It Computes

Given workload, pool bounds, and simulation options, Poolsim computes:

- Recommended pool size (`optimal_pool_size`)
- Confidence interval for the recommendation
- Utilization (`rho`)
- Mean and p99 queue wait
- Saturation classification (`Ok`, `Warning`, `Critical`)
- Sensitivity table across pool sizes
- Optional step-load analysis output

## Input Model

### Workload Input

- `requests_per_second`: arrival rate
- `latency_p50_ms`, `latency_p95_ms`, `latency_p99_ms`: latency percentiles
- `raw_samples_ms` (optional): empirical latency samples
- `step_load_profile` (optional): time-series load profile

### Pool Input

- `max_server_connections`
- `connection_overhead_ms`
- `idle_timeout_ms` (optional)
- `min_pool_size`
- `max_pool_size`

### Simulation Options

- `iterations`
- `seed` (optional)
- `distribution` (`LogNormal`, `Empirical`, `Gamma`)
- `queue_model` (`MMC`, `MDC`)
- target thresholds (through options/config used by optimizer)

## Calculation Pipeline

1. Validate workload/pool/options.
2. Fit latency distribution:
   - from p50/p95/p99 (model-based), or
   - from `raw_samples_ms` (empirical path).
3. Estimate queue metrics using Erlang-C style queueing calculations.
4. Run Monte Carlo simulation for tail and burst-aware behavior.
5. Search candidate pool sizes and select the smallest acceptable size.
6. Build confidence interval and sensitivity risk surface.
7. Optionally evaluate step-load profile impact.

## Output Interpretation

### Saturation

- `Ok`: healthy utilization headroom.
- `Warning`: elevated utilization; monitor closely.
- `Critical`: unstable/saturated region; not safe for production steady state.

### Sensitivity Table

Use the sensitivity rows to decide how much headroom you want, not just the minimum passing value.

- If adjacent pool sizes quickly move into `High`/`Critical`, keep buffer.
- If risk remains stable across nearby sizes, a tighter choice may be acceptable.

## Usage

### Library Usage (Rust)

```rust
use poolsim_core::{
    simulate,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 220.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 32.0,
    latency_p99_ms: 85.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 120,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 3,
    max_pool_size: 24,
};

let report = simulate(&workload, &pool, &SimulationOptions::default())?;
println!("recommended size: {}", report.optimal_pool_size);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### CLI Usage

Run full simulation:

```bash
poolsim simulate --config prod.json
```

Evaluate one fixed size:

```bash
poolsim evaluate --config prod.json --pool-size 20 --format json
```

Generate sensitivity surface:

```bash
poolsim sweep --config prod.json --format csv
```

Exit code behavior:

- `0`: success and non-critical outcome
- `1`: input/config/runtime error
- `2`: critical saturation
- `3`: warning-level exit (when warning exit policy is enabled)

### Web API Usage

Primary endpoints:

- `POST /v1/simulate`
- `POST /v1/evaluate`
- `POST /v1/sensitivity`
- `POST /v1/batch`
- `GET /v1/health`
- `GET /v1/models`
- `GET /v1/live` (WebSocket stream path)

## Recommended Operating Workflow

1. Collect real workload percentiles (or sample latencies).
2. Run `simulate` to get baseline recommendation.
3. Review `sensitivity` to choose safety margin.
4. Validate with step-load profile when available.
5. Apply selected pool size in runtime configuration.
6. Re-run on traffic pattern changes or latency shifts.

## Assumptions and Limits

- Output quality depends on input quality.
- Percentiles should reflect representative traffic windows.
- The model does not replace production monitoring.
- Pool sizing remains a control decision: choose based on risk tolerance and operational headroom.
