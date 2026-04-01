use std::{fs, path::PathBuf};

use poolsim_core::{
    distribution::LatencyDistribution,
    emit_performance_contract_warning,
    erlang,
    error::PoolsimError,
    monte_carlo,
    optimizer,
    sensitivity,
    simulate,
    sweep,
    sweep_with_options,
    DistributionModel,
    MIN_FULL_SIMULATION_ITERATIONS,
    PERFORMANCE_CONTRACT_WARNING,
    QueueModel,
    RiskLevel,
};
use poolsim_core::types::{
    PoolConfig, SaturationLevel, SimulationOptions, StepLoadPoint, WorkloadConfig,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct DocsConfig {
    workload: WorkloadConfig,
    pool: PoolConfig,
    #[serde(default)]
    options: SimulationOptions,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve")
}

fn fixture(path: &str) -> PathBuf {
    workspace_root().join(path)
}

fn load_docs_config() -> DocsConfig {
    let raw = fs::read_to_string(fixture("docs/fixtures/cli-config.json"))
        .expect("docs CLI fixture should be readable");
    serde_json::from_str(&raw).expect("docs CLI fixture should deserialize")
}

#[test]
fn docs_core_fixture_covers_top_level_and_helper_apis() {
    let docs = load_docs_config();

    assert_eq!(MIN_FULL_SIMULATION_ITERATIONS, 10_000);
    assert_eq!(
        PERFORMANCE_CONTRACT_WARNING,
        "performance contract not met: expected <= 200ms"
    );
    emit_performance_contract_warning(0, 1_000);

    docs.workload.validate().expect("workload fixture should validate");
    docs.pool.validate().expect("pool fixture should validate");
    docs.options.validate().expect("options fixture should validate");
    for point in docs
        .workload
        .step_load_profile
        .as_ref()
        .expect("step profile should exist in fixture")
    {
        point.validate().expect("step point should validate");
    }

    assert_eq!(SaturationLevel::from_rho(0.40), SaturationLevel::Ok);
    assert_eq!(SaturationLevel::from_rho(0.88), SaturationLevel::Warning);
    assert_eq!(SaturationLevel::from_rho(0.96), SaturationLevel::Critical);

    let report = simulate(&docs.workload, &docs.pool, &docs.options)
        .expect("docs simulate example should succeed");
    assert!(report.optimal_pool_size >= docs.pool.min_pool_size);
    assert!(report.optimal_pool_size <= docs.pool.max_pool_size);
    assert_eq!(report.step_load_analysis.len(), 2);

    let evaluation = poolsim_core::evaluate(&docs.workload, report.optimal_pool_size, &docs.options)
        .expect("docs evaluate example should succeed");
    assert_eq!(evaluation.pool_size, report.optimal_pool_size);

    let default_sweep = sweep(&docs.workload, &docs.pool).expect("top-level sweep should succeed");
    let explicit_sweep = sweep_with_options(&docs.workload, &docs.pool, &docs.options)
        .expect("top-level sweep_with_options should succeed");
    assert!(!default_sweep.is_empty());
    assert_eq!(explicit_sweep.len(), default_sweep.len());

    let dist = LatencyDistribution::fit(&docs.workload, DistributionModel::LogNormal)
        .expect("lognormal fit should succeed");
    assert!(dist.mean_ms() > 0.0);
    assert!(dist.percentile_ms(0.99).expect("p99 should compute") > 0.0);
    let mut rng = rand::thread_rng();
    assert!(dist.sample_ms(&mut rng) > 0.0);

    let lambda = docs.workload.requests_per_second;
    let mu = 1_000.0 / dist.mean_ms();
    let rho = erlang::utilisation(lambda, mu, report.optimal_pool_size);
    assert!(rho.is_finite());
    assert!(erlang::erlang_c(lambda, mu, report.optimal_pool_size).is_ok());
    assert!(erlang::mean_queue_wait_ms(lambda, mu, report.optimal_pool_size).is_ok());
    assert!(erlang::queue_wait_percentile_ms(lambda, mu, report.optimal_pool_size, 0.99).is_ok());

    let mc = monte_carlo::run(&docs.workload, report.optimal_pool_size, &dist, &docs.options)
        .expect("Monte Carlo run should succeed");
    assert!(!mc.wait_times_ms.is_empty());

    let optimal = optimizer::find_optimal(&docs.workload, &docs.pool, &dist, &docs.options)
        .expect("optimizer should succeed");
    assert!(optimal.pool_size >= docs.pool.min_pool_size);

    let sensitivity_default =
        sensitivity::sweep(&docs.workload, &docs.pool).expect("module sweep should succeed");
    let sensitivity_target = sensitivity::sweep_with_target(&docs.workload, &docs.pool, 45.0)
        .expect("targeted sweep should succeed");
    let sensitivity_target_model = sensitivity::sweep_with_target_and_model(
        &docs.workload,
        &docs.pool,
        45.0,
        QueueModel::MDC,
    )
    .expect("targeted modeled sweep should succeed");
    let sensitivity_opts = sensitivity::sweep_with_options(&docs.workload, &docs.pool, &docs.options)
        .expect("module sweep_with_options should succeed");
    assert_eq!(sensitivity_default.len(), sensitivity_target.len());
    assert_eq!(sensitivity_target_model.len(), sensitivity_opts.len());

    let error = PoolsimError::invalid_input(
        "INVALID_DOCS_FIXTURE",
        "fixture validation failed",
        Some(json!({"path": "docs/fixtures/cli-config.json"})),
    );
    assert_eq!(error.code(), "INVALID_DOCS_FIXTURE");
    assert_eq!(
        error.details().expect("error details should exist")["path"],
        "docs/fixtures/cli-config.json"
    );

    let risk: RiskLevel = sensitivity_default
        .iter()
        .map(|row| row.risk)
        .min()
        .expect("sensitivity rows should exist");
    assert!(matches!(
        risk,
        RiskLevel::Low | RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical
    ));
}

#[test]
fn docs_empirical_samples_fixture_covers_empirical_distribution_path() {
    let docs = load_docs_config();
    let samples = fs::read_to_string(fixture("docs/fixtures/latencies.txt"))
        .expect("latency sample fixture should be readable");
    let raw_samples_ms = samples
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<f64>()
                .expect("latency samples should parse as f64")
        })
        .collect::<Vec<_>>();

    let workload = WorkloadConfig {
        raw_samples_ms: Some(raw_samples_ms),
        step_load_profile: Some(vec![
            StepLoadPoint {
                time_s: 0,
                requests_per_second: docs.workload.requests_per_second,
            },
            StepLoadPoint {
                time_s: 30,
                requests_per_second: docs.workload.requests_per_second + 20.0,
            },
        ]),
        ..docs.workload
    };
    workload.validate().expect("empirical workload should validate");

    let dist = LatencyDistribution::fit(&workload, DistributionModel::Empirical)
        .expect("empirical fit should succeed");
    assert!(matches!(dist, LatencyDistribution::Empirical(_)));
    assert!(dist.percentile_ms(0.50).expect("median should compute") > 0.0);
    assert!(dist.mean_ms() > 0.0);
}
