//! Sensitivity analysis across a configured pool-size range.
//!
//! These helpers generate one [`crate::types::SensitivityRow`] per candidate
//! pool size so callers can inspect how risk and queue-wait behavior change as
//! the pool grows.
//!
//! Use this module when you need the full tradeoff surface instead of only the
//! single recommended result returned by [`crate::simulate`].

use crate::{
    distribution::LatencyDistribution,
    erlang,
    error::PoolsimError,
    monte_carlo,
    types::{
        DistributionModel, PoolConfig, QueueModel, RiskLevel, SensitivityRow, SimulationOptions, WorkloadConfig,
    },
};

/// Generates sensitivity rows using default simulation options.
///
/// # Errors
///
/// Returns distribution/simulation errors for invalid inputs or unstable queue states.
pub fn sweep(workload: &WorkloadConfig, pool: &PoolConfig) -> Result<Vec<SensitivityRow>, PoolsimError> {
    sweep_with_options(workload, pool, &SimulationOptions::default())
}

/// Generates sensitivity rows with a custom target p99 wait threshold.
///
/// # Errors
///
/// Returns distribution/simulation errors for invalid inputs or unstable queue states.
pub fn sweep_with_target(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    target_wait_p99_ms: f64,
) -> Result<Vec<SensitivityRow>, PoolsimError> {
    let opts = SimulationOptions {
        target_wait_p99_ms,
        ..SimulationOptions::default()
    };
    sweep_with_options(workload, pool, &opts)
}

/// Generates sensitivity rows with a custom target and queue model.
///
/// # Errors
///
/// Returns distribution/simulation errors for invalid inputs or unstable queue states.
pub fn sweep_with_target_and_model(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    target_wait_p99_ms: f64,
    queue_model: QueueModel,
) -> Result<Vec<SensitivityRow>, PoolsimError> {
    let opts = SimulationOptions {
        queue_model,
        target_wait_p99_ms,
        distribution: DistributionModel::LogNormal,
        ..SimulationOptions::default()
    };
    sweep_with_options(workload, pool, &opts)
}

/// Generates sensitivity rows across all candidate pool sizes.
///
/// # Errors
///
/// Returns distribution/simulation errors for invalid inputs or unstable queue states.
pub fn sweep_with_options(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    opts: &SimulationOptions,
) -> Result<Vec<SensitivityRow>, PoolsimError> {
    let dist = LatencyDistribution::fit(workload, opts.distribution)?;
    let mu = 1_000.0 / (dist.mean_ms() + pool.connection_overhead_ms);
    let lambda = workload.requests_per_second;
    let target_wait_p99_ms = opts.target_wait_p99_ms;

    let mut rows = Vec::with_capacity((pool.max_pool_size - pool.min_pool_size + 1) as usize);

    for size in pool.min_pool_size..=pool.max_pool_size {
        let rho = erlang::utilisation(lambda, mu, size);

        let (mean_wait, p99_wait, risk) = if rho >= 1.0 {
            (f64::MAX, f64::MAX, RiskLevel::Critical)
        } else {
            let (mean, p99) = match opts.queue_model {
                QueueModel::MMC => (
                    erlang::mean_queue_wait_ms(lambda, mu, size)?,
                    erlang::queue_wait_percentile_ms(lambda, mu, size, 0.99)?,
                ),
                QueueModel::MDC => {
                    let probe_opts = mdc_probe_options(opts, size);
                    let probe = monte_carlo::run_with_overhead(
                        workload,
                        size,
                        pool.connection_overhead_ms,
                        &dist,
                        &probe_opts,
                    )?;
                    (probe.mean, probe.p99)
                }
            };
            let risk = classify_risk(rho, p99, target_wait_p99_ms);
            (mean, p99, risk)
        };

        rows.push(SensitivityRow {
            pool_size: size,
            utilisation_rho: rho,
            mean_queue_wait_ms: mean_wait,
            p99_queue_wait_ms: p99_wait,
            risk,
        });
    }

    Ok(rows)
}

fn mdc_probe_options(opts: &SimulationOptions, size: u32) -> SimulationOptions {
    let mut probe_opts = opts.clone();
    probe_opts.iterations = (opts.iterations / 4).clamp(400, 2_000);
    if let Some(seed) = opts.seed {
        probe_opts.seed = Some(seed ^ ((size as u64 + 1).wrapping_mul(0x517C_C1B7_2722_0A95)));
    }
    probe_opts
}

fn classify_risk(rho: f64, p99_wait_ms: f64, target_wait_p99_ms: f64) -> RiskLevel {
    if rho >= 0.90 {
        return RiskLevel::Critical;
    }
    if rho >= 0.80 {
        return RiskLevel::High;
    }
    if rho < 0.70 && p99_wait_ms < target_wait_p99_ms / 2.0 {
        return RiskLevel::Low;
    }
    if rho < 0.80 || p99_wait_ms < target_wait_p99_ms {
        return RiskLevel::Medium;
    }
    RiskLevel::High
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_risk_falls_back_to_high_for_nan_inputs() {
        let risk = classify_risk(f64::NAN, f64::NAN, 50.0);
        assert_eq!(risk, RiskLevel::High);
    }
}
