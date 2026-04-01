use std::time::{Duration, Instant};

use poolsim_core::{
    simulate,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

#[test]
#[ignore = "hardware-dependent performance contract check for dedicated CI runners"]
fn full_simulation_under_200ms_on_reference_machine() {
    let workload = WorkloadConfig {
        requests_per_second: 500.0,
        latency_p50_ms: 12.0,
        latency_p95_ms: 80.0,
        latency_p99_ms: 200.0,
        raw_samples_ms: None,
        step_load_profile: None,
    };
    let pool = PoolConfig {
        max_server_connections: 120,
        connection_overhead_ms: 2.0,
        idle_timeout_ms: None,
        min_pool_size: 5,
        max_pool_size: 80,
    };
    let opts = SimulationOptions {
        iterations: 10_000,
        seed: Some(42),
        ..SimulationOptions::default()
    };

    let start = Instant::now();
    let _ = simulate(&workload, &pool, &opts).expect("simulation inputs should be valid");
    let elapsed = start.elapsed();

    assert!(
        elapsed <= Duration::from_millis(200),
        "expected <= 200ms, got {:?}",
        elapsed
    );
}
