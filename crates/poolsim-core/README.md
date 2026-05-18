# poolsim-core

[![Crates.io](https://img.shields.io/crates/v/poolsim-core.svg)](https://crates.io/crates/poolsim-core)
[![docs.rs](https://img.shields.io/docsrs/poolsim-core)](https://docs.rs/poolsim-core)
[![CI](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml/badge.svg)](https://github.com/gregorian-09/poolsim/actions/workflows/ci.yml)
[![Docs Coverage](https://img.shields.io/badge/docs%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/docs/README.md)
[![Workspace Coverage](https://img.shields.io/badge/workspace%20line%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_coverage_threshold.py)
[![Examples Coverage](https://img.shields.io/badge/examples%20coverage-100%25-brightgreen)](https://github.com/gregorian-09/poolsim/blob/main/tools/check_examples_coverage.py)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/gregorian-09/poolsim/blob/main/LICENSE)

`poolsim-core` is the Rust library crate for connection-pool sizing, queue-pressure modeling, and telemetry-backed pool recommendations.

Use it when you want to embed Poolsim directly inside Rust code instead of shelling out to `poolsim-cli` or running `poolsim-web`.

## What Is New In `0.2.0`

`0.2.0` is an additive feature release across the Poolsim workspace.

For `poolsim-core`, the important user-facing capabilities are:

- Telemetry-backed recommendation diffs through `poolsim_core::telemetry::recommend_from_telemetry`.
- Stronger docs and executable examples for the public sizing model.
- A fully covered core source tree enforced by CI at `100%` line coverage.
- Stable typed outputs that can feed CLI workflows such as `doctor`, `gate`, `guard`, `generate-config`, and `budget`.

The database budget planner itself currently lives in `poolsim-cli` because it is an operational command workflow. Use `poolsim-core` to compute per-service recommendations, then use `poolsim-cli budget` to allocate a shared database connection budget across services.

## Install

```toml
[dependencies]
poolsim-core = "0.2.0"
```

Optional default feature:

```toml
[dependencies]
poolsim-core = { version = "0.2.0", default-features = false }
```

Default features enable parallel simulation support. Disable default features when you need a smaller dependency surface or a `wasm32-unknown-unknown` build.

## Core Concepts

`poolsim-core` models connection-pool sizing in four layers:

- Workload: request rate, latency percentiles, optional empirical latency samples, optional step-load profile.
- Pool bounds: database connection limit, connection overhead, idle timeout, minimum and maximum pool size.
- Simulation options: iteration count, random seed, latency distribution, queue model, wait target, utilization target.
- Reports: recommended pool size, confidence interval, queue wait, saturation, sensitivity rows, warnings.

It is not a runtime connection pool. It does not open database connections, replace `sqlx`, `bb8`, `deadpool`, HikariCP, SQLAlchemy, Prisma, or node-postgres, or enforce pool settings in production.

## Primary APIs

Use these crate-root APIs first:

- `simulate`: run the full recommendation workflow.
- `evaluate`: score one fixed pool size against a workload.
- `sweep`: produce sensitivity rows with default simulation options.
- `sweep_with_options`: produce sensitivity rows with explicit options.

Use these modules for advanced workflows:

- `poolsim_core::types`: public input and output structs.
- `poolsim_core::telemetry`: telemetry snapshots and recommendation diffs.
- `poolsim_core::distribution`: latency distribution fitting.
- `poolsim_core::erlang`: Erlang-C queue formulas.
- `poolsim_core::monte_carlo`: simulation primitives.
- `poolsim_core::optimizer`: pool-size search.
- `poolsim_core::sensitivity`: sensitivity table generation.
- `poolsim_core::error`: typed error handling.

## Quick Simulation Example

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
println!("recommended pool size: {}", report.optimal_pool_size);
println!("p99 queue wait: {:.3} ms", report.p99_queue_wait_ms);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Fixed Pool Evaluation

Use `evaluate` when you already have a configured pool size and want to know whether it is safe.

```rust
use poolsim_core::{
    evaluate,
    types::{SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 180.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 70.0,
    raw_samples_ms: None,
    step_load_profile: None,
};

let result = evaluate(&workload, 8, &SimulationOptions::default())?;
println!("rho={:.3}, saturation={:?}", result.utilisation_rho, result.saturation);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Sensitivity Sweep

Use `sweep_with_options` to see how queue behavior changes across candidate pool sizes.

```rust
use poolsim_core::{
    sweep_with_options,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let workload = WorkloadConfig {
    requests_per_second: 260.0,
    latency_p50_ms: 8.0,
    latency_p95_ms: 30.0,
    latency_p99_ms: 70.0,
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

let rows = sweep_with_options(&workload, &pool, &SimulationOptions::default())?;
for row in rows {
    println!("size={} rho={:.3} risk={:?}", row.pool_size, row.utilisation_rho, row.risk);
}
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Telemetry Recommendation Diff

Use telemetry when you want to compare the current production setting with a computed recommendation.

```rust
use poolsim_core::{
    telemetry::{recommend_from_telemetry, TelemetrySnapshot},
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

let snapshot = TelemetrySnapshot {
    service_name: Some("checkout-api".to_string()),
    window: Some("1h".to_string()),
    observed_at: None,
    current_pool_size: 8,
    workload: WorkloadConfig {
        requests_per_second: 180.0,
        latency_p50_ms: 8.0,
        latency_p95_ms: 30.0,
        latency_p99_ms: 70.0,
        raw_samples_ms: None,
        step_load_profile: None,
    },
    pool: PoolConfig {
        max_server_connections: 100,
        connection_overhead_ms: 2.0,
        idle_timeout_ms: None,
        min_pool_size: 2,
        max_pool_size: 20,
    },
};

let recommendation = recommend_from_telemetry(&snapshot, &SimulationOptions::default())?;
println!("current: {}", recommendation.diff.current_pool_size);
println!("recommended: {}", recommendation.diff.recommended_pool_size);
println!("delta: {}", recommendation.diff.pool_size_delta);
# Ok::<(), poolsim_core::error::PoolsimError>(())
```

## Getting The Most From The Library

- Feed realistic p50, p95, and p99 latency values from production, not only local benchmarks.
- Re-run recommendations after traffic, latency, replica count, query behavior, or database limits change.
- Use `SimulationOptions::seed` for deterministic CI checks and examples.
- Use `raw_samples_ms` when you have representative latency samples and want empirical fitting.
- Use `step_load_profile` to model ramp-up, peak windows, or incident traffic.
- Treat the recommended pool size as a per-replica setting; multiply it by replica count before comparing against database `max_connections`.
- Pair this crate with `poolsim-cli budget` when multiple services share one database connection limit.

## Quality And CI Guarantees

The repository CI currently enforces:

- `cargo check --workspace`
- `cargo test --workspace`
- `RUSTFLAGS="-D missing_docs"` for core, CLI, and web crates
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
- `cargo test --workspace --doc`
- executable docs fixtures
- `cargo test --workspace --examples`
- `cargo tarpaulin --workspace` with `100%` overall and core source coverage thresholds
- `cargo tarpaulin --workspace --examples` with `100%` example-file coverage
- `wasm32-unknown-unknown` check for `poolsim-core` without default features

## Support

- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
- Documentation: <https://docs.rs/poolsim-core>
- Changelog: <https://github.com/gregorian-09/poolsim/blob/main/CHANGELOG.md>

When opening an issue, include the crate version, Rust version, input workload or telemetry shape, expected behavior, actual behavior, and a minimal reproduction when possible.

## Related Crates

- `poolsim-cli`: command-line workflows, CI guard mode, doctor, config generator, and database budget planner.
- `poolsim-web`: REST and WebSocket service wrapper around the sizing engine.
