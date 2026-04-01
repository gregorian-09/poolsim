# poolsim-core

`poolsim-core` is the Rust library crate for connection-pool sizing.

It is the simulation and analysis engine behind the `poolsim` workspace.

## Scope

The current release focuses on sizing calculation, not runtime pool enforcement.

It provides:

- workload and pool validation
- latency distribution fitting
- Erlang-C helpers
- Monte Carlo queue simulation
- pool-size optimization
- sensitivity analysis
- step-load analysis

## Install

```toml
[dependencies]
poolsim-core = "0.1.0"
```

## Primary APIs

Use the crate-root APIs for the most common workflows:

- `simulate`: full recommendation workflow
- `evaluate`: score a fixed pool size
- `sweep`: generate sensitivity rows with default options
- `sweep_with_options`: generate sensitivity rows with explicit options

Important public modules:

- `poolsim_core::types`
- `poolsim_core::distribution`
- `poolsim_core::erlang`
- `poolsim_core::monte_carlo`
- `poolsim_core::optimizer`
- `poolsim_core::sensitivity`
- `poolsim_core::error`

## Example

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

let report = simulate(&workload, &pool, &SimulationOptions::default()).unwrap();
assert!(report.optimal_pool_size >= 3);
```

## Output

The main simulation output includes:

- recommended pool size
- confidence interval
- cold-start minimum pool size
- utilisation ratio
- queue-wait metrics
- sensitivity rows
- optional step-load analysis
- warnings

## See Also

- Workspace repository: <https://github.com/gregorian-09/poolsim>
- Detailed library guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/library-api.md>
- Sizing calculator guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/sizing-calculator.md>

## Notes

- For CLI usage, see the `poolsim-cli` crate.
- For HTTP and WebSocket usage, see the `poolsim-web` crate.
