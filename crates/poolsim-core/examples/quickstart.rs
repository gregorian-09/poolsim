use poolsim_core::{
    simulate,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};

fn build_inputs() -> (WorkloadConfig, PoolConfig, SimulationOptions) {
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
    (workload, pool, SimulationOptions::default())
}

fn run_example() -> u32 {
    let (workload, pool, options) = build_inputs();
    let report = simulate(&workload, &pool, &options).expect("simulation should succeed");
    report.optimal_pool_size
}

#[cfg(not(test))]
fn main() {
    let optimal = run_example();
    println!("recommended pool size: {optimal}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quickstart_example_runs() {
        let optimal = run_example();
        assert!(optimal >= 3);
    }
}
