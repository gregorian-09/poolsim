use poolsim_core::{
    evaluate, simulate, sweep_with_options,
    types::{DistributionModel, PoolConfig, QueueModel, SimulationOptions, WorkloadConfig},
    MIN_FULL_SIMULATION_ITERATIONS,
};

fn workload() -> WorkloadConfig {
    WorkloadConfig {
        requests_per_second: 300.0,
        latency_p50_ms: 10.0,
        latency_p95_ms: 50.0,
        latency_p99_ms: 120.0,
        raw_samples_ms: None,
        step_load_profile: None,
    }
}

fn pool() -> PoolConfig {
    PoolConfig {
        max_server_connections: 80,
        connection_overhead_ms: 2.0,
        idle_timeout_ms: None,
        min_pool_size: 4,
        max_pool_size: 20,
    }
}

#[test]
fn pool_config_accepts_connection_establishment_overhead_alias() {
    let raw = r#"
    {
      "max_server_connections": 100,
      "connection_establishment_overhead_ms": 3.5,
      "idle_timeout_ms": null,
      "min_pool_size": 4,
      "max_pool_size": 20
    }
    "#;

    let cfg: PoolConfig = serde_json::from_str(raw).expect("pool config should deserialize");
    assert!((cfg.connection_overhead_ms - 3.5).abs() < f64::EPSILON);
}

#[test]
fn mdc_model_changes_evaluation_output() {
    let wl = workload();

    let mmc_opts = SimulationOptions {
        iterations: 800,
        seed: Some(7),
        distribution: DistributionModel::LogNormal,
        queue_model: QueueModel::MMC,
        target_wait_p99_ms: 40.0,
        max_acceptable_rho: 0.85,
    };

    let mdc_opts = SimulationOptions {
        queue_model: QueueModel::MDC,
        ..mmc_opts.clone()
    };

    let mmc = evaluate(&wl, 10, &mmc_opts).expect("mmc evaluation should succeed");
    let mdc = evaluate(&wl, 10, &mdc_opts).expect("mdc evaluation should succeed");

    assert_ne!(mmc.p99_queue_wait_ms, mdc.p99_queue_wait_ms);
}

#[test]
fn sweep_with_options_respects_queue_model() {
    let wl = workload();
    let pl = pool();

    let mmc_opts = SimulationOptions {
        iterations: 400,
        seed: Some(1),
        distribution: DistributionModel::LogNormal,
        queue_model: QueueModel::MMC,
        target_wait_p99_ms: 40.0,
        max_acceptable_rho: 0.85,
    };

    let mdc_opts = SimulationOptions {
        queue_model: QueueModel::MDC,
        ..mmc_opts.clone()
    };

    let mmc_rows = sweep_with_options(&wl, &pl, &mmc_opts).expect("mmc sweep should succeed");
    let mdc_rows = sweep_with_options(&wl, &pl, &mdc_opts).expect("mdc sweep should succeed");

    let mmc_last = mmc_rows.last().expect("mmc rows should not be empty");
    let mdc_last = mdc_rows.last().expect("mdc rows should not be empty");

    assert_eq!(mmc_last.pool_size, mdc_last.pool_size);
    assert!(mdc_last.p99_queue_wait_ms <= mmc_last.p99_queue_wait_ms);
}

#[test]
fn full_simulate_enforces_minimum_iteration_budget() {
    let wl = workload();
    let pl = pool();
    let opts = SimulationOptions {
        iterations: 500,
        seed: Some(3),
        distribution: DistributionModel::LogNormal,
        queue_model: QueueModel::MMC,
        target_wait_p99_ms: 40.0,
        max_acceptable_rho: 0.85,
    };

    let report = simulate(&wl, &pl, &opts).expect("simulate should succeed with low iteration request");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains(&MIN_FULL_SIMULATION_ITERATIONS.to_string())),
        "simulate should include min-iteration warning"
    );
}
