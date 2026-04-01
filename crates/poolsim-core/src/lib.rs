#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/poolsim-core/0.1.0")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#![deny(missing_docs)]

/// Distribution fitting and sampling utilities.
pub mod distribution;
/// Erlang-C queueing formulas.
pub mod erlang;
/// Error type and helpers.
pub mod error;
/// Monte Carlo queue simulation engine.
pub mod monte_carlo;
/// Pool-size optimization routines.
pub mod optimizer;
/// Sensitivity analysis routines.
pub mod sensitivity;
/// Public input/output data models.
pub mod types;

use distribution::LatencyDistribution;
use error::PoolsimError;
use optimizer::find_optimal;
use types::{
    EvaluationResult, PoolConfig, SaturationLevel, SensitivityRow, SimulationOptions, SimulationReport,
    StepLoadResult, WorkloadConfig,
};

/// Re-exported distribution model enum.
pub use types::DistributionModel;
/// Re-exported queue model enum.
pub use types::QueueModel;
/// Re-exported risk-level enum.
pub use types::RiskLevel;

/// Minimum iteration floor used by full simulation for stable estimates.
pub const MIN_FULL_SIMULATION_ITERATIONS: u32 = 10_000;
/// Performance warning text emitted by benchmark/helpers when threshold is exceeded.
pub const PERFORMANCE_CONTRACT_WARNING: &str = "performance contract not met: expected <= 200ms";

/// Emits the performance contract warning when elapsed time exceeds threshold.
pub fn emit_performance_contract_warning(elapsed_ms: u128, threshold_ms: u128) {
    if elapsed_ms > threshold_ms {
        eprintln!("{PERFORMANCE_CONTRACT_WARNING}");
    }
}

/// Runs full pool-size optimization and returns a simulation report.
///
/// # Errors
///
/// Returns [`error::PoolsimError`] for invalid inputs, distribution fitting failures,
/// or queue/simulation failures.
pub fn simulate(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    opts: &SimulationOptions,
) -> Result<SimulationReport, PoolsimError> {
    workload.validate()?;
    pool.validate()?;
    opts.validate()?;

    let mut effective_opts = opts.clone();
    let mut warnings = Vec::new();
    if effective_opts.iterations < MIN_FULL_SIMULATION_ITERATIONS {
        effective_opts.iterations = MIN_FULL_SIMULATION_ITERATIONS;
        warnings.push(format!(
            "iterations increased to {} for full simulation fidelity",
            MIN_FULL_SIMULATION_ITERATIONS
        ));
    }

    let dist = LatencyDistribution::fit(workload, effective_opts.distribution)?;
    let optimal = find_optimal(workload, pool, &dist, &effective_opts)?;
    let sensitivity = sensitivity::sweep_with_options(workload, pool, &effective_opts)?;
    let cold_start_min_pool_size =
        recommend_cold_start_pool_size(workload, pool, &dist, &effective_opts, optimal.pool_size);

    let mut step_opts = effective_opts.clone();
    if workload.step_load_profile.is_some() {
        let reduced = (effective_opts.iterations / 4).clamp(1_500, 5_000);
        if reduced < effective_opts.iterations {
            step_opts.iterations = reduced;
            warnings.push(format!(
                "step-load analysis used {} iterations per step for responsiveness",
                reduced
            ));
        }
    }
    let step_load_analysis = build_step_load_analysis(workload, optimal.pool_size, &step_opts)?;

    let saturation = SaturationLevel::from_rho(optimal.utilisation_rho);
    warnings.extend(optimal.warnings);
    if saturation != SaturationLevel::Ok {
        warnings.push(format!(
            "System utilisation is high at the recommended size (rho={:.3})",
            optimal.utilisation_rho
        ));
    }

    Ok(SimulationReport {
        optimal_pool_size: optimal.pool_size,
        confidence_interval: optimal.confidence_interval,
        cold_start_min_pool_size,
        utilisation_rho: optimal.utilisation_rho,
        mean_queue_wait_ms: optimal.mean_queue_wait_ms,
        p99_queue_wait_ms: optimal.p99_queue_wait_ms,
        saturation,
        sensitivity,
        step_load_analysis,
        warnings,
    })
}

/// Evaluates a fixed pool size against the workload/options.
///
/// # Errors
///
/// Returns [`error::PoolsimError`] for invalid inputs or queue/simulation failures.
pub fn evaluate(
    workload: &WorkloadConfig,
    pool_size: u32,
    opts: &SimulationOptions,
) -> Result<EvaluationResult, PoolsimError> {
    workload.validate()?;
    opts.validate()?;

    if pool_size == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_POOL_SIZE",
            "pool_size must be greater than 0",
            None,
        ));
    }

    let dist = LatencyDistribution::fit(workload, opts.distribution)?;
    let mc = monte_carlo::run(workload, pool_size, &dist, opts)?;

    let lambda = workload.requests_per_second;
    let mu = 1_000.0 / dist.mean_ms();
    let rho = erlang::utilisation(lambda, mu, pool_size);
    let mean_wait = match opts.queue_model {
        QueueModel::MMC => erlang::mean_queue_wait_ms(lambda, mu, pool_size).unwrap_or(mc.mean),
        QueueModel::MDC => mc.mean,
    };

    let saturation = SaturationLevel::from_rho(rho);
    let mut warnings = Vec::new();
    if saturation != SaturationLevel::Ok {
        warnings.push(format!("utilisation is elevated (rho={:.3})", rho));
    }

    Ok(EvaluationResult {
        pool_size,
        utilisation_rho: rho,
        mean_queue_wait_ms: mean_wait,
        p99_queue_wait_ms: mc.p99,
        saturation,
        warnings,
    })
}

/// Generates a sensitivity table using default simulation options.
///
/// # Errors
///
/// Returns [`error::PoolsimError`] for invalid inputs or queue/simulation failures.
pub fn sweep(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
) -> Result<Vec<SensitivityRow>, PoolsimError> {
    sweep_with_options(workload, pool, &SimulationOptions::default())
}

/// Generates a sensitivity table using explicit simulation options.
///
/// # Errors
///
/// Returns [`error::PoolsimError`] for invalid inputs or queue/simulation failures.
pub fn sweep_with_options(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    opts: &SimulationOptions,
) -> Result<Vec<SensitivityRow>, PoolsimError> {
    workload.validate()?;
    pool.validate()?;
    opts.validate()?;
    sensitivity::sweep_with_options(workload, pool, opts)
}

fn recommend_cold_start_pool_size(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    dist: &LatencyDistribution,
    opts: &SimulationOptions,
    recommended_pool_size: u32,
) -> u32 {
    let peak_rps = workload
        .step_load_profile
        .as_ref()
        .and_then(|profile| {
            profile
                .iter()
                .map(|point| point.requests_per_second)
                .max_by(|a, b| a.total_cmp(b))
        })
        .map(|peak| peak.max(workload.requests_per_second))
        .unwrap_or(workload.requests_per_second);

    let mu = 1_000.0 / (dist.mean_ms() + pool.connection_overhead_ms);
    if !mu.is_finite() || mu <= 0.0 {
        return pool.min_pool_size.min(recommended_pool_size);
    }

    let warm_rho_target = opts.max_acceptable_rho.min(0.70).max(0.35);
    let required = (peak_rps / (mu * warm_rho_target)).ceil().max(1.0) as u32;
    required
        .max(pool.min_pool_size)
        .min(recommended_pool_size)
}

fn build_step_load_analysis(
    workload: &WorkloadConfig,
    pool_size: u32,
    opts: &SimulationOptions,
) -> Result<Vec<StepLoadResult>, PoolsimError> {
    let Some(profile) = &workload.step_load_profile else {
        return Ok(Vec::new());
    };

    let mut rows = Vec::with_capacity(profile.len());
    for point in profile {
        let mut step_workload = workload.clone();
        step_workload.requests_per_second = point.requests_per_second;
        step_workload.step_load_profile = None;

        let step = evaluate(&step_workload, pool_size, opts)?;
        rows.push(StepLoadResult {
            time_s: point.time_s,
            requests_per_second: point.requests_per_second,
            utilisation_rho: step.utilisation_rho,
            p99_queue_wait_ms: step.p99_queue_wait_ms,
            saturation: step.saturation,
        });
    }

    Ok(rows)
}
