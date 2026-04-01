use poolsim_core::{
    distribution::LatencyDistribution,
    erlang, monte_carlo, optimizer, sensitivity,
    error::PoolsimError,
    emit_performance_contract_warning, evaluate, simulate, sweep,
    types::{
        DistributionModel, PoolConfig, QueueModel, SaturationLevel, SimulationOptions, StepLoadPoint,
        WorkloadConfig,
    },
};

fn base_workload() -> WorkloadConfig {
    WorkloadConfig {
        requests_per_second: 250.0,
        latency_p50_ms: 8.0,
        latency_p95_ms: 40.0,
        latency_p99_ms: 110.0,
        raw_samples_ms: None,
        step_load_profile: None,
    }
}

fn base_pool() -> PoolConfig {
    PoolConfig {
        max_server_connections: 100,
        connection_overhead_ms: 2.0,
        idle_timeout_ms: None,
        min_pool_size: 3,
        max_pool_size: 30,
    }
}

fn base_options() -> SimulationOptions {
    SimulationOptions {
        iterations: 1_000,
        seed: Some(9),
        distribution: DistributionModel::LogNormal,
        queue_model: QueueModel::MMC,
        target_wait_p99_ms: 45.0,
        max_acceptable_rho: 0.85,
    }
}

#[test]
fn distribution_models_fit_and_percentiles_work() {
    let wl = base_workload();
    let mut rng = rand::thread_rng();

    let lognormal = LatencyDistribution::fit(&wl, DistributionModel::LogNormal).expect("lognormal fit");
    assert!(lognormal.mean_ms() > 0.0);
    assert!(lognormal.percentile_ms(0.95).expect("p95") >= wl.latency_p50_ms);
    assert!(lognormal.sample_ms(&mut rng) > 0.0);

    let exponential =
        LatencyDistribution::fit(&wl, DistributionModel::Exponential).expect("exponential fit");
    assert!(exponential.mean_ms() > 0.0);
    assert!(exponential.percentile_ms(0.99).expect("p99") > exponential.mean_ms());

    let gamma = LatencyDistribution::fit(&wl, DistributionModel::Gamma).expect("gamma fit");
    assert!(gamma.mean_ms() > 0.0);
    assert!(gamma.percentile_ms(0.50).expect("p50") > 0.0);
}

#[test]
fn empirical_distribution_overrides_requested_model() {
    let mut wl = base_workload();
    wl.raw_samples_ms = Some(vec![6.0, 7.0, 10.0, 15.0, 30.0, 50.0]);

    let dist = LatencyDistribution::fit(&wl, DistributionModel::Gamma).expect("empirical fit");
    let p90 = dist.percentile_ms(0.90).expect("empirical percentile");
    assert!(p90 >= 15.0);
}

#[test]
fn simulate_outputs_cold_start_and_step_load_analysis() {
    let mut wl = base_workload();
    wl.step_load_profile = Some(vec![
        StepLoadPoint {
            time_s: 0,
            requests_per_second: 200.0,
        },
        StepLoadPoint {
            time_s: 30,
            requests_per_second: 380.0,
        },
        StepLoadPoint {
            time_s: 60,
            requests_per_second: 320.0,
        },
    ]);

    let report = simulate(&wl, &base_pool(), &base_options()).expect("simulate should work");
    assert!(report.cold_start_min_pool_size >= base_pool().min_pool_size);
    assert!(report.cold_start_min_pool_size <= report.optimal_pool_size);
    assert_eq!(report.step_load_analysis.len(), 3);
    assert_eq!(report.step_load_analysis[1].time_s, 30);
}

#[test]
fn workload_validation_rejects_step_profile_order_and_rps() {
    let mut wl = base_workload();
    wl.step_load_profile = Some(vec![
        StepLoadPoint {
            time_s: 10,
            requests_per_second: 200.0,
        },
        StepLoadPoint {
            time_s: 5,
            requests_per_second: 220.0,
        },
    ]);
    let err = wl
        .validate()
        .expect_err("non-increasing step profile should fail");
    assert_eq!(err.code(), "INVALID_STEP_LOAD_PROFILE_ORDER");

    let mut wl2 = base_workload();
    wl2.step_load_profile = Some(vec![StepLoadPoint {
        time_s: 5,
        requests_per_second: 0.0,
    }]);
    let err2 = wl2
        .validate()
        .expect_err("non-positive step rps should fail");
    assert_eq!(err2.code(), "INVALID_STEP_LOAD_RPS");

    let mut wl3 = base_workload();
    wl3.latency_p95_ms = 6.0;
    let err3 = wl3
        .validate()
        .expect_err("invalid percentile order should fail");
    assert_eq!(err3.code(), "INVALID_LATENCY_ORDER");
    assert!(err3.details().is_some());
}

#[test]
fn pool_and_options_validation_cover_error_codes() {
    let mut pool = base_pool();
    pool.max_pool_size = 200;
    let err = pool
        .validate()
        .expect_err("max_pool_size above server max should fail");
    assert_eq!(err.code(), "POOL_EXCEEDS_SERVER_MAX");

    let mut opts = base_options();
    opts.max_acceptable_rho = 1.2;
    let err2 = opts
        .validate()
        .expect_err("rho outside [0,1) should fail");
    assert_eq!(err2.code(), "INVALID_MAX_ACCEPTABLE_RHO");

    let mut bad_pool = base_pool();
    bad_pool.max_server_connections = 0;
    assert_eq!(
        bad_pool
            .validate()
            .expect_err("zero max_server_connections should fail")
            .code(),
        "INVALID_MAX_SERVER_CONNECTIONS"
    );

    let mut bad_pool2 = base_pool();
    bad_pool2.connection_overhead_ms = -1.0;
    assert_eq!(
        bad_pool2
            .validate()
            .expect_err("negative overhead should fail")
            .code(),
        "INVALID_CONNECTION_OVERHEAD"
    );

    let mut bad_pool3 = base_pool();
    bad_pool3.min_pool_size = 0;
    assert_eq!(
        bad_pool3
            .validate()
            .expect_err("min pool size zero should fail")
            .code(),
        "INVALID_MIN_POOL_SIZE"
    );

    let mut bad_pool4 = base_pool();
    bad_pool4.min_pool_size = 10;
    bad_pool4.max_pool_size = 5;
    assert_eq!(
        bad_pool4
            .validate()
            .expect_err("invalid min/max ordering should fail")
            .code(),
        "INVALID_POOL_RANGE"
    );

    let mut bad_opts = base_options();
    bad_opts.iterations = 0;
    assert_eq!(
        bad_opts
            .validate()
            .expect_err("iterations zero should fail")
            .code(),
        "INVALID_ITERATIONS"
    );

    let mut bad_opts2 = base_options();
    bad_opts2.target_wait_p99_ms = 0.0;
    assert_eq!(
        bad_opts2
            .validate()
            .expect_err("target wait zero should fail")
            .code(),
        "INVALID_TARGET_WAIT"
    );
}

#[test]
fn saturation_level_thresholds_match_spec() {
    assert_eq!(SaturationLevel::from_rho(0.84), SaturationLevel::Ok);
    assert_eq!(SaturationLevel::from_rho(0.85), SaturationLevel::Warning);
    assert_eq!(SaturationLevel::from_rho(0.95), SaturationLevel::Critical);
}

#[test]
fn error_code_and_details_branches_are_exercised() {
    let invalid = PoolsimError::invalid_input("INVALID_X", "bad input", Some(serde_json::json!({"a": 1})));
    assert_eq!(invalid.code(), "INVALID_X");
    assert!(invalid.details().is_some());

    let saturated = PoolsimError::Saturated { rho: 1.2 };
    assert_eq!(saturated.code(), "SATURATED");
    assert!(saturated.details().is_none());

    let dist = PoolsimError::Distribution("dist".to_string());
    assert_eq!(dist.code(), "DISTRIBUTION_ERROR");
    assert!(dist.details().is_none());

    let sim = PoolsimError::Simulation("sim".to_string());
    assert_eq!(sim.code(), "SIMULATION_ERROR");
    assert!(sim.details().is_none());
}

#[test]
fn workload_validation_covers_sample_errors() {
    let mut wl = base_workload();
    wl.raw_samples_ms = Some(vec![1.0, 2.0]);
    assert_eq!(
        wl.validate()
            .expect_err("too-few samples should fail")
            .code(),
        "INVALID_SAMPLES"
    );

    let mut wl2 = base_workload();
    wl2.raw_samples_ms = Some(vec![1.0, -2.0, 3.0]);
    assert_eq!(
        wl2.validate()
            .expect_err("non-positive sample should fail")
            .code(),
        "INVALID_SAMPLES"
    );

    let mut wl3 = base_workload();
    wl3.step_load_profile = Some(Vec::new());
    assert_eq!(
        wl3.validate()
            .expect_err("empty step profile should fail")
            .code(),
        "INVALID_STEP_LOAD_PROFILE"
    );
}

#[test]
fn distribution_sampling_and_error_paths_are_covered() {
    let mut rng = rand::thread_rng();
    let wl = base_workload();

    let exp = LatencyDistribution::fit(&wl, DistributionModel::Exponential).expect("exp fit");
    assert!(exp.sample_ms(&mut rng) > 0.0);

    let gamma = LatencyDistribution::fit(&wl, DistributionModel::Gamma).expect("gamma fit");
    assert!(gamma.sample_ms(&mut rng) > 0.0);

    let mut empirical_wl = base_workload();
    empirical_wl.raw_samples_ms = Some(vec![5.0, 8.0, 13.0, 21.0]);
    let empirical = LatencyDistribution::fit(&empirical_wl, DistributionModel::LogNormal)
        .expect("empirical fit should work");
    assert!(empirical.sample_ms(&mut rng) > 0.0);
    assert!(empirical.mean_ms() > 0.0);

    let mut empty_empirical = base_workload();
    empty_empirical.raw_samples_ms = Some(Vec::new());
    let err = LatencyDistribution::fit(&empty_empirical, DistributionModel::Empirical)
        .expect_err("empty empirical samples should fail");
    assert_eq!(err.code(), "DISTRIBUTION_ERROR");

    let mut bad_lognormal = base_workload();
    bad_lognormal.latency_p50_ms = 10.0;
    bad_lognormal.latency_p95_ms = 10.0;
    bad_lognormal.latency_p99_ms = 10.0;
    let err = LatencyDistribution::fit(&bad_lognormal, DistributionModel::LogNormal)
        .expect_err("non-separable percentiles should fail");
    assert!(err.to_string().contains("lognormal sigma"));

    let mut nan_gamma = base_workload();
    nan_gamma.latency_p50_ms = f64::NAN;
    let gamma_from_nan = LatencyDistribution::fit(&nan_gamma, DistributionModel::Gamma)
        .expect("gamma fitter should clamp non-finite inputs to safe parameters");
    assert!(gamma_from_nan.mean_ms().is_finite());
}

#[test]
fn erlang_input_validation_paths_are_covered() {
    assert!(erlang::utilisation(100.0, 0.0, 1).is_infinite());

    assert_eq!(
        erlang::erlang_c(0.0, 1.0, 2).expect("zero arrival rate should return zero"),
        0.0
    );
    assert_eq!(
        erlang::mean_queue_wait_ms(0.0, 1.0, 2).expect("zero arrival should return zero"),
        0.0
    );
    assert_eq!(
        erlang::queue_wait_percentile_ms(0.0, 1.0, 2, 0.95).expect("zero arrival should return zero"),
        0.0
    );
    assert_eq!(
        erlang::queue_wait_percentile_ms(1.0, 2.0, 2, 0.0).expect("q=0 should return zero"),
        0.0
    );

    assert_eq!(
        erlang::erlang_c(1.0, 1.0, 0)
            .expect_err("c=0 should fail")
            .code(),
        "INVALID_SERVER_COUNT"
    );
    assert_eq!(
        erlang::erlang_c(1.0, 0.0, 1)
            .expect_err("mu<=0 should fail")
            .code(),
        "INVALID_SERVICE_RATE"
    );
    assert_eq!(
        erlang::erlang_c(100.0, 1.0, 2)
            .expect_err("saturated inputs should fail")
            .code(),
        "SATURATED"
    );
}

#[test]
fn monte_carlo_error_paths_are_covered() {
    let wl = base_workload();
    let mut opts = base_options();
    opts.iterations = 200;

    let dist = LatencyDistribution::fit(&wl, DistributionModel::LogNormal).expect("dist fit");
    assert_eq!(
        monte_carlo::run(&wl, 0, &dist, &opts)
            .expect_err("pool_size=0 should fail")
            .code(),
        "INVALID_POOL_SIZE"
    );

    let mut empty_opts = opts.clone();
    empty_opts.iterations = 0;
    let err = monte_carlo::run(&wl, 4, &dist, &empty_opts)
        .expect_err("zero iterations should fail to produce wait times");
    assert_eq!(err.code(), "SIMULATION_ERROR");

    let mut mdc_opts = opts;
    mdc_opts.queue_model = QueueModel::MDC;
    let mdc = monte_carlo::run(&wl, 4, &dist, &mdc_opts).expect("MDC run should succeed");
    assert!(mdc.p99 >= 0.0);
}

#[test]
fn optimizer_and_sensitivity_paths_are_covered() {
    let wl = base_workload();
    let mut pool = base_pool();
    pool.max_pool_size = 12;
    let dist = LatencyDistribution::fit(&wl, DistributionModel::LogNormal).expect("dist fit");

    let mut fallback_opts = base_options();
    fallback_opts.target_wait_p99_ms = 0.001;
    fallback_opts.max_acceptable_rho = 0.01;
    let fallback = optimizer::find_optimal(&wl, &pool, &dist, &fallback_opts)
        .expect("optimizer should return fallback result");
    assert_eq!(fallback.pool_size, pool.max_pool_size);
    assert!(
        fallback
            .warnings
            .iter()
            .any(|w| w.contains("No candidate pool size met target constraints"))
    );

    let mut mdc_opts = base_options();
    mdc_opts.queue_model = QueueModel::MDC;
    mdc_opts.seed = Some(123);
    let mdc = optimizer::find_optimal(&wl, &pool, &dist, &mdc_opts).expect("mdc optimizer should run");
    assert!(
        mdc.warnings
            .iter()
            .any(|w| w.contains("MDC mode uses Monte Carlo probe estimates"))
    );

    let rows_default = sensitivity::sweep(&wl, &pool).expect("default sweep should work");
    assert!(!rows_default.is_empty());
    let rows_target = sensitivity::sweep_with_target(&wl, &pool, 35.0).expect("target sweep should work");
    assert!(!rows_target.is_empty());
    let rows_model = sensitivity::sweep_with_target_and_model(&wl, &pool, 35.0, QueueModel::MDC)
        .expect("target+model sweep should work");
    assert!(!rows_model.is_empty());
}

#[test]
fn workload_validation_covers_remaining_scalar_checks() {
    let mut wl = base_workload();
    wl.requests_per_second = 0.0;
    assert_eq!(
        wl.validate()
            .expect_err("zero rps should fail")
            .code(),
        "INVALID_RPS"
    );

    let mut wl2 = base_workload();
    wl2.latency_p50_ms = 0.0;
    assert_eq!(
        wl2.validate()
            .expect_err("non-positive latency should fail")
            .code(),
        "INVALID_LATENCY"
    );
}

#[test]
fn public_wrapper_paths_and_saturation_warning_are_covered() {
    let wl = base_workload();
    let pool = base_pool();
    let opts = base_options();

    let rows = sweep(&wl, &pool).expect("public sweep wrapper should work");
    assert!(!rows.is_empty());

    assert_eq!(
        evaluate(&wl, 0, &opts)
            .expect_err("pool_size=0 should fail")
            .code(),
        "INVALID_POOL_SIZE"
    );

    let hot_workload = WorkloadConfig {
        requests_per_second: 5_000.0,
        ..wl.clone()
    };
    let small_pool = PoolConfig {
        min_pool_size: 2,
        max_pool_size: 5,
        max_server_connections: 5,
        ..pool
    };
    let report = simulate(&hot_workload, &small_pool, &opts).expect("hot simulation should run");
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("System utilisation is high")),
        "expected saturation warning for hot workload"
    );

    let mut inf_overhead_pool = small_pool.clone();
    inf_overhead_pool.connection_overhead_ms = f64::INFINITY;
    let report = simulate(&wl, &inf_overhead_pool, &opts).expect("infinite overhead should still simulate");
    assert_eq!(
        report.cold_start_min_pool_size,
        inf_overhead_pool.min_pool_size.min(report.optimal_pool_size)
    );

    emit_performance_contract_warning(100, 200);
    emit_performance_contract_warning(250, 200);
}
