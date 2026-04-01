# Poolsim Library API

## Purpose

This document is the exhaustive usage reference for the current public `poolsim-core` API.

It covers:

- Every exported top-level function
- Every exported constant
- Every exported enum and struct in the public data model
- Every exported helper in the public submodules
- Practical usage patterns for real applications

The primary crate is `poolsim-core`.

## Import Patterns

Minimal top-level import:

```rust
use poolsim_core::{
    evaluate,
    simulate,
    sweep,
    sweep_with_options,
    DistributionModel,
    QueueModel,
    RiskLevel,
};
```

Common type import:

```rust
use poolsim_core::types::{
    EvaluationResult,
    PoolConfig,
    SaturationLevel,
    SensitivityRow,
    SimulationOptions,
    SimulationReport,
    StepLoadPoint,
    StepLoadResult,
    WorkloadConfig,
};
```

Module-oriented import:

```rust
use poolsim_core::{distribution, erlang, error, monte_carlo, optimizer, sensitivity};
```

## Top-Level API

### `simulate`

Use `simulate` when you want the full recommendation flow:

- validation
- latency fitting
- Monte Carlo optimization
- confidence interval
- sensitivity surface
- cold-start recommendation
- optional step-load analysis

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

let options = SimulationOptions::default();
let report = simulate(&workload, &pool, &options)?;

assert!(report.optimal_pool_size >= pool.min_pool_size);
assert!(report.optimal_pool_size <= pool.max_pool_size);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Simulation with explicit options:

```rust
use poolsim_core::{
    simulate,
    DistributionModel,
    QueueModel,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 350.0,
    latency_p50_ms: 10.0,
    latency_p95_ms: 40.0,
    latency_p99_ms: 100.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 180,
    connection_overhead_ms: 3.0,
    idle_timeout_ms: Some(120_000),
    min_pool_size: 4,
    max_pool_size: 40,
};

let options = SimulationOptions {
    iterations: 12_000,
    seed: Some(7),
    distribution: DistributionModel::Gamma,
    queue_model: QueueModel::MMC,
    target_wait_p99_ms: 60.0,
    max_acceptable_rho: 0.82,
};

let report = simulate(&workload, &pool, &options)?;
assert!(report.mean_queue_wait_ms >= 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Simulation with step-load analysis:

```rust
use poolsim_core::{
    simulate,
    types::{PoolConfig, SimulationOptions, StepLoadPoint, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 25.0,
    latency_p99_ms: 60.0,
    raw_samples_ms: None,
    step_load_profile: Some(vec![
        StepLoadPoint { time_s: 0, requests_per_second: 180.0 },
        StepLoadPoint { time_s: 30, requests_per_second: 260.0 },
        StepLoadPoint { time_s: 60, requests_per_second: 310.0 },
    ]),
};

let pool = PoolConfig {
    max_server_connections: 150,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 3,
    max_pool_size: 30,
};

let report = simulate(&workload, &pool, &SimulationOptions::default())?;
assert_eq!(report.step_load_analysis.len(), 3);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `evaluate`

Use `evaluate` when the pool size is already chosen and you want to score it.

```rust
use poolsim_core::{
    evaluate,
    types::{SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 240.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 80.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let result = evaluate(&workload, 10, &SimulationOptions::default())?;
assert_eq!(result.pool_size, 10);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `sweep`

Use `sweep` for a default sensitivity run across the configured pool range.

```rust
use poolsim_core::{
    sweep,
    types::{PoolConfig, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 200.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 70.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 12,
};

let rows = sweep(&workload, &pool)?;
assert_eq!(rows.len(), 11);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `sweep_with_options`

Use `sweep_with_options` when the default risk threshold or model is not enough.

```rust
use poolsim_core::{
    sweep_with_options,
    DistributionModel,
    QueueModel,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 260.0,
    latency_p50_ms: 9.0,
    latency_p95_ms: 35.0,
    latency_p99_ms: 90.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 120,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 3,
    max_pool_size: 20,
};

let options = SimulationOptions {
    distribution: DistributionModel::LogNormal,
    queue_model: QueueModel::MDC,
    target_wait_p99_ms: 45.0,
    ..SimulationOptions::default()
};

let rows = sweep_with_options(&workload, &pool, &options)?;
assert!(!rows.is_empty());
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Re-exported enums

The crate root re-exports:

- `DistributionModel`
- `QueueModel`
- `RiskLevel`

This lets you write:

```rust
use poolsim_core::{DistributionModel, QueueModel, RiskLevel};

let model = DistributionModel::Gamma;
let queue = QueueModel::MDC;
let risk = RiskLevel::High;

assert!(matches!(model, DistributionModel::Gamma));
assert!(matches!(queue, QueueModel::MDC));
assert!(matches!(risk, RiskLevel::High));
```

### Constants

`MIN_FULL_SIMULATION_ITERATIONS` is the full-simulation floor.

```rust
use poolsim_core::MIN_FULL_SIMULATION_ITERATIONS;

assert_eq!(MIN_FULL_SIMULATION_ITERATIONS, 10_000);
```

`PERFORMANCE_CONTRACT_WARNING` is the standard warning text emitted by the helper.

```rust
use poolsim_core::PERFORMANCE_CONTRACT_WARNING;

assert!(PERFORMANCE_CONTRACT_WARNING.contains("performance contract"));
```

### `emit_performance_contract_warning`

Use this helper when benchmarking or wrapping the library in a service layer that wants a standardized warning string.

```rust
use poolsim_core::emit_performance_contract_warning;

emit_performance_contract_warning(250, 200);
emit_performance_contract_warning(150, 200);
```

## `types` Module

### `DistributionModel`

Available values:

- `LogNormal`
- `Exponential`
- `Empirical`
- `Gamma`

Example:

```rust
use poolsim_core::types::DistributionModel;

let default_model = DistributionModel::LogNormal;
let sample_driven = DistributionModel::Empirical;

assert!(matches!(default_model, DistributionModel::LogNormal));
assert!(matches!(sample_driven, DistributionModel::Empirical));
```

### `QueueModel`

Available values:

- `MMC`
- `MDC`

Example:

```rust
use poolsim_core::types::QueueModel;

let queue_model = QueueModel::MMC;
assert!(matches!(queue_model, QueueModel::MMC));
```

### `RiskLevel`

Used in sensitivity rows:

- `Low`
- `Medium`
- `High`
- `Critical`

```rust
use poolsim_core::types::RiskLevel;

let risk = RiskLevel::Critical;
assert!(risk >= RiskLevel::High);
```

### `SaturationLevel`

Used in reports and evaluation output:

- `Ok`
- `Warning`
- `Critical`

Example:

```rust
use poolsim_core::types::SaturationLevel;

assert_eq!(SaturationLevel::from_rho(0.70), SaturationLevel::Ok);
assert_eq!(SaturationLevel::from_rho(0.86), SaturationLevel::Warning);
assert_eq!(SaturationLevel::from_rho(0.96), SaturationLevel::Critical);
```

### `WorkloadConfig`

Fields:

- `requests_per_second`
- `latency_p50_ms`
- `latency_p95_ms`
- `latency_p99_ms`
- `raw_samples_ms`
- `step_load_profile`

Minimal percentile-based workload:

```rust
use poolsim_core::types::WorkloadConfig;

let workload = WorkloadConfig {
    requests_per_second: 220.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 32.0,
    latency_p99_ms: 85.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

workload.validate()?;
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Empirical-sample workload:

```rust
use poolsim_core::types::WorkloadConfig;

let workload = WorkloadConfig {
    requests_per_second: 220.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 32.0,
    latency_p99_ms: 85.0,
    raw_samples_ms: Some(vec![6.5, 7.8, 9.1, 10.2, 12.0]),
    step_load_profile: None,
};

workload.validate()?;
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `StepLoadPoint`

Use `StepLoadPoint` when modeling traffic changes over time.

```rust
use poolsim_core::types::StepLoadPoint;

let point = StepLoadPoint {
    time_s: 30,
    requests_per_second: 260.0,
};

point.validate()?;
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `PoolConfig`

Fields:

- `max_server_connections`
- `connection_overhead_ms`
- `idle_timeout_ms`
- `min_pool_size`
- `max_pool_size`

Example:

```rust
use poolsim_core::types::PoolConfig;

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: Some(120_000),
    min_pool_size: 2,
    max_pool_size: 20,
};

pool.validate()?;
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `SimulationOptions`

Fields:

- `iterations`
- `seed`
- `distribution`
- `queue_model`
- `target_wait_p99_ms`
- `max_acceptable_rho`

Defaults:

```rust
use poolsim_core::types::{DistributionModel, QueueModel, SimulationOptions};

let options = SimulationOptions::default();
assert_eq!(options.iterations, 10_000);
assert_eq!(options.distribution, DistributionModel::LogNormal);
assert_eq!(options.queue_model, QueueModel::MMC);
assert_eq!(options.target_wait_p99_ms, 50.0);
assert_eq!(options.max_acceptable_rho, 0.85);
```

Explicit tuning:

```rust
use poolsim_core::types::{DistributionModel, QueueModel, SimulationOptions};

let options = SimulationOptions {
    iterations: 20_000,
    seed: Some(42),
    distribution: DistributionModel::Gamma,
    queue_model: QueueModel::MDC,
    target_wait_p99_ms: 40.0,
    max_acceptable_rho: 0.80,
};

options.validate()?;
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `SensitivityRow`

Returned by sweep operations:

```rust
use poolsim_core::types::{RiskLevel, SensitivityRow};

let row = SensitivityRow {
    pool_size: 8,
    utilisation_rho: 0.74,
    mean_queue_wait_ms: 3.2,
    p99_queue_wait_ms: 18.0,
    risk: RiskLevel::Low,
};

assert_eq!(row.pool_size, 8);
```

### `SimulationReport`

Returned by `simulate`:

```rust
use poolsim_core::types::{RiskLevel, SaturationLevel, SensitivityRow, SimulationReport, StepLoadResult};

let report = SimulationReport {
    optimal_pool_size: 8,
    confidence_interval: (7, 9),
    cold_start_min_pool_size: 6,
    utilisation_rho: 0.74,
    mean_queue_wait_ms: 3.2,
    p99_queue_wait_ms: 18.0,
    saturation: SaturationLevel::Ok,
    sensitivity: vec![SensitivityRow {
        pool_size: 8,
        utilisation_rho: 0.74,
        mean_queue_wait_ms: 3.2,
        p99_queue_wait_ms: 18.0,
        risk: RiskLevel::Low,
    }],
    step_load_analysis: vec![StepLoadResult {
        time_s: 0,
        requests_per_second: 180.0,
        utilisation_rho: 0.71,
        p99_queue_wait_ms: 16.0,
        saturation: SaturationLevel::Ok,
    }],
    warnings: vec![],
};

assert_eq!(report.confidence_interval, (7, 9));
```

### `EvaluationResult`

Returned by `evaluate`:

```rust
use poolsim_core::types::{EvaluationResult, SaturationLevel};

let result = EvaluationResult {
    pool_size: 10,
    utilisation_rho: 0.79,
    mean_queue_wait_ms: 4.1,
    p99_queue_wait_ms: 22.0,
    saturation: SaturationLevel::Ok,
    warnings: vec![],
};

assert_eq!(result.pool_size, 10);
```

### `StepLoadResult`

Returned inside `SimulationReport.step_load_analysis`:

```rust
use poolsim_core::types::{SaturationLevel, StepLoadResult};

let row = StepLoadResult {
    time_s: 30,
    requests_per_second: 260.0,
    utilisation_rho: 0.88,
    p99_queue_wait_ms: 42.0,
    saturation: SaturationLevel::Warning,
};

assert_eq!(row.time_s, 30);
```

## `error` Module

### `PoolsimError`

Variants:

- `InvalidInput`
- `Saturated`
- `Distribution`
- `Simulation`

Build a standard invalid-input error:

```rust
use poolsim_core::error::PoolsimError;

let err = PoolsimError::invalid_input(
    "INVALID_RPS",
    "requests_per_second must be greater than 0",
    None,
);

assert_eq!(err.code(), "INVALID_RPS");
assert!(err.details().is_none());
```

Inspect code and details:

```rust
use poolsim_core::error::PoolsimError;
use serde_json::json;

let err = PoolsimError::invalid_input(
    "INVALID_LATENCY_ORDER",
    "p50 < p95 < p99 is required",
    Some(json!({"p50": 100.0, "p95": 50.0, "p99": 120.0})),
);

assert_eq!(err.code(), "INVALID_LATENCY_ORDER");
assert!(err.details().is_some());
```

## `distribution` Module

### `EmpiricalCdf`

`EmpiricalCdf` is exposed as part of `LatencyDistribution::Empirical`, but normal callers usually do not construct it directly. The public workflow is to call `LatencyDistribution::fit` with `raw_samples_ms`.

### `LatencyDistribution`

Variants:

- `LogNormal { mu, sigma }`
- `Exponential { mean_ms }`
- `Empirical(EmpiricalCdf)`
- `Gamma { shape, scale }`

Fit from workload:

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    types::{DistributionModel, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 6.0,
    latency_p95_ms: 25.0,
    latency_p99_ms: 60.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::LogNormal)?;
assert!(dist.mean_ms() > 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Fit empirical model:

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    types::{DistributionModel, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 6.0,
    latency_p95_ms: 25.0,
    latency_p99_ms: 60.0,
    raw_samples_ms: Some(vec![5.0, 6.0, 7.5, 9.0, 10.0, 15.0]),
    step_load_profile: None,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::Empirical)?;
let p95 = dist.percentile_ms(0.95)?;
assert!(p95 > 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Sample from the fitted distribution:

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    types::{DistributionModel, WorkloadConfig},
};
use rand::{rngs::StdRng, SeedableRng};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 6.0,
    latency_p95_ms: 25.0,
    latency_p99_ms: 60.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::LogNormal)?;
let mut rng = StdRng::seed_from_u64(42);
let sample = dist.sample_ms(&mut rng);
assert!(sample > 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

Read a percentile:

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    types::{DistributionModel, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 6.0,
    latency_p95_ms: 25.0,
    latency_p99_ms: 60.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::Gamma)?;
let p99 = dist.percentile_ms(0.99)?;
assert!(p99 >= 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## `erlang` Module

### `utilisation`

```rust
use poolsim_core::erlang::utilisation;

let rho = utilisation(120.0, 20.0, 8);
assert!(rho > 0.0);
```

### `erlang_c`

```rust
use poolsim_core::erlang::erlang_c;

let p_wait = erlang_c(8.0, 1.0, 10)?;
assert!(p_wait >= 0.0);
assert!(p_wait <= 1.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `mean_queue_wait_ms`

```rust
use poolsim_core::erlang::mean_queue_wait_ms;

let mean_wait_ms = mean_queue_wait_ms(8.0, 1.0, 10)?;
assert!(mean_wait_ms >= 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `queue_wait_percentile_ms`

```rust
use poolsim_core::erlang::queue_wait_percentile_ms;

let p99_wait_ms = queue_wait_percentile_ms(8.0, 1.0, 10, 0.99)?;
assert!(p99_wait_ms >= 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## `monte_carlo` Module

### `MonteCarloResult`

Fields:

- `wait_times_ms`
- `p50`
- `p95`
- `p99`
- `mean`

### `run`

Use this when you want raw queue-wait samples plus summary statistics for one fixed pool size.

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    monte_carlo::run,
    types::{DistributionModel, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 200.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 70.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::LogNormal)?;
let result = run(&workload, 8, &dist, &SimulationOptions::default())?;

assert!(result.p99 >= result.p95);
assert!(!result.wait_times_ms.is_empty());
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## `optimizer` Module

### `OptimalResult`

Fields:

- `pool_size`
- `confidence_interval`
- `utilisation_rho`
- `mean_queue_wait_ms`
- `p99_queue_wait_ms`
- `warnings`

### `find_optimal`

Use this when you want the optimizer only, without the top-level report assembly.

```rust
use poolsim_core::{
    distribution::LatencyDistribution,
    optimizer::find_optimal,
    types::{DistributionModel, PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 200.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 70.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 16,
};

let dist = LatencyDistribution::fit(&workload, DistributionModel::LogNormal)?;
let optimal = find_optimal(&workload, &pool, &dist, &SimulationOptions::default())?;

assert!(optimal.pool_size >= pool.min_pool_size);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## `sensitivity` Module

### `sweep`

```rust
use poolsim_core::{
    sensitivity,
    types::{PoolConfig, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 190.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 26.0,
    latency_p99_ms: 65.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 12,
};

let rows = sensitivity::sweep(&workload, &pool)?;
assert_eq!(rows.first().unwrap().pool_size, 2);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `sweep_with_target`

```rust
use poolsim_core::{
    sensitivity,
    types::{PoolConfig, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 190.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 26.0,
    latency_p99_ms: 65.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 12,
};

let rows = sensitivity::sweep_with_target(&workload, &pool, 35.0)?;
assert!(!rows.is_empty());
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `sweep_with_target_and_model`

```rust
use poolsim_core::{
    sensitivity,
    QueueModel,
    types::{PoolConfig, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 190.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 26.0,
    latency_p99_ms: 65.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 12,
};

let rows = sensitivity::sweep_with_target_and_model(&workload, &pool, 40.0, QueueModel::MDC)?;
assert!(!rows.is_empty());
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### `sweep_with_options`

```rust
use poolsim_core::{
    sensitivity,
    DistributionModel,
    QueueModel,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 190.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 26.0,
    latency_p99_ms: 65.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let pool = PoolConfig {
    max_server_connections: 100,
    connection_overhead_ms: 2.0,
    idle_timeout_ms: None,
    min_pool_size: 2,
    max_pool_size: 12,
};

let options = SimulationOptions {
    distribution: DistributionModel::Gamma,
    queue_model: QueueModel::MMC,
    target_wait_p99_ms: 45.0,
    max_acceptable_rho: 0.82,
    ..SimulationOptions::default()
};

let rows = sensitivity::sweep_with_options(&workload, &pool, &options)?;
assert_eq!(rows.len(), 11);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Practical Patterns

### Pattern: percentile-only sizing

Use when you only know p50/p95/p99:

```rust
use poolsim_core::{
    simulate,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let report = simulate(
    &WorkloadConfig {
        requests_per_second: 280.0,
        latency_p50_ms: 9.0,
        latency_p95_ms: 35.0,
        latency_p99_ms: 95.0,
        raw_samples_ms: None,
        step_load_profile: None,
    },
    &PoolConfig {
        max_server_connections: 160,
        connection_overhead_ms: 2.0,
        idle_timeout_ms: None,
        min_pool_size: 4,
        max_pool_size: 30,
    },
    &SimulationOptions::default(),
)?;

assert!(report.optimal_pool_size >= 4);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Pattern: empirical sample-driven sizing

Use when you have real latency samples:

```rust
use poolsim_core::{
    simulate,
    DistributionModel,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 7.0,
    latency_p95_ms: 20.0,
    latency_p99_ms: 40.0,
    raw_samples_ms: Some(vec![5.5, 6.0, 6.8, 7.2, 7.9, 10.0, 12.0, 18.0]),
    step_load_profile: None,
};

let options = SimulationOptions {
    distribution: DistributionModel::Empirical,
    ..SimulationOptions::default()
};

let report = simulate(
    &workload,
    &PoolConfig {
        max_server_connections: 80,
        connection_overhead_ms: 1.5,
        idle_timeout_ms: None,
        min_pool_size: 2,
        max_pool_size: 16,
    },
    &options,
)?;

assert!(report.p99_queue_wait_ms >= 0.0);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

### Pattern: deterministic-service approximation

Use `QueueModel::MDC` for more deterministic workloads:

```rust
use poolsim_core::{
    simulate,
    QueueModel,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let report = simulate(
    &WorkloadConfig {
        requests_per_second: 150.0,
        latency_p50_ms: 6.0,
        latency_p95_ms: 12.0,
        latency_p99_ms: 16.0,
        raw_samples_ms: None,
        step_load_profile: None,
    },
    &PoolConfig {
        max_server_connections: 60,
        connection_overhead_ms: 1.0,
        idle_timeout_ms: None,
        min_pool_size: 2,
        max_pool_size: 12,
    },
    &SimulationOptions {
        queue_model: QueueModel::MDC,
        ..SimulationOptions::default()
    },
)?;

assert!(report.optimal_pool_size >= 2);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```
