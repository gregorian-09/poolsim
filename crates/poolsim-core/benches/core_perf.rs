use std::time::Instant;

use poolsim_core::{
    emit_performance_contract_warning,
    erlang,
    simulate,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

fn main() {
    bench_erlang_grid();
    bench_full_simulation();
}

fn bench_erlang_grid() {
    let mut total = 0.0;
    let start = Instant::now();
    for _ in 0..20_000 {
        for servers in [1u32, 2, 4, 8, 16, 32, 64, 100] {
            let rho = 0.8;
            let mu = 100.0;
            let lambda = rho * servers as f64 * mu;
            total += erlang::erlang_c(lambda, mu, servers).expect("valid erlang inputs");
        }
    }
    let elapsed = start.elapsed();
    println!("bench_erlang_grid elapsed={elapsed:?} checksum={total:.6}");
}

fn bench_full_simulation() {
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
    let _ = simulate(&workload, &pool, &opts).expect("simulation benchmark inputs must be valid");
    let elapsed = start.elapsed();
    println!("bench_full_simulation elapsed={elapsed:?}");
    emit_performance_contract_warning(elapsed.as_millis(), 200);
}
