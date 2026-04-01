//! Pool-size optimization routines.
//!
//! This module evaluates the configured candidate range and chooses the
//! smallest pool size that satisfies both:
//!
//! - `max_acceptable_rho`
//! - `target_wait_p99_ms`
//!
//! If no candidate satisfies both constraints, the optimizer falls back to
//! `max_pool_size` and emits advisory warnings in
//! [`crate::optimizer::OptimalResult`].

use crate::{
    distribution::LatencyDistribution,
    erlang,
    error::PoolsimError,
    monte_carlo,
    types::{PoolConfig, QueueModel, SimulationOptions, WorkloadConfig},
};

/// Optimization output for the selected pool size.
#[derive(Debug, Clone)]
pub struct OptimalResult {
    /// Selected pool size.
    pub pool_size: u32,
    /// Confidence interval around the selected pool size.
    pub confidence_interval: (u32, u32),
    /// Utilization ratio (`rho`) for `pool_size`.
    pub utilisation_rho: f64,
    /// Mean queue wait in milliseconds for `pool_size`.
    pub mean_queue_wait_ms: f64,
    /// p99 queue wait in milliseconds for `pool_size`.
    pub p99_queue_wait_ms: f64,
    /// Human-readable advisory notes from the optimizer.
    pub warnings: Vec<String>,
}

/// Finds the smallest pool size that satisfies target constraints.
///
/// Falls back to `pool.max_pool_size` when no candidate satisfies both
/// utilization and p99 wait targets.
///
/// # Errors
///
/// Returns propagated distribution/simulation/queue-model errors.
pub fn find_optimal(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    dist: &LatencyDistribution,
    opts: &SimulationOptions,
) -> Result<OptimalResult, PoolsimError> {
    let lambda = workload.requests_per_second;
    let mu = 1_000.0 / (dist.mean_ms() + pool.connection_overhead_ms);

    let mut candidate = None;
    let mut warnings = Vec::new();
    if opts.queue_model == QueueModel::MDC {
        warnings.push(
            "MDC mode uses Monte Carlo probe estimates for candidate search".to_string(),
        );
    }

    for size in pool.min_pool_size..=pool.max_pool_size {
        let rho = erlang::utilisation(lambda, mu, size);
        if rho >= 1.0 {
            continue;
        }

        let p99 = match opts.queue_model {
            QueueModel::MMC => erlang::queue_wait_percentile_ms(lambda, mu, size, 0.99)?,
            QueueModel::MDC => mdc_probe_p99(workload, pool, dist, opts, size)?,
        };
        if rho < opts.max_acceptable_rho && p99 <= opts.target_wait_p99_ms {
            candidate = Some(size);
            break;
        }
    }

    let chosen = candidate.unwrap_or(pool.max_pool_size);
    if candidate.is_none() {
        warnings.push(
            "No candidate pool size met target constraints; using max_pool_size fallback".to_string(),
        );
    }

    let mc = monte_carlo::run_with_overhead(workload, chosen, pool.connection_overhead_ms, dist, opts)?;

    let rho = erlang::utilisation(lambda, mu, chosen);
    let ci = bootstrap_ci(chosen, pool, &mc.wait_times_ms, opts.target_wait_p99_ms);

    Ok(OptimalResult {
        pool_size: chosen,
        confidence_interval: ci,
        utilisation_rho: rho,
        mean_queue_wait_ms: mc.mean,
        p99_queue_wait_ms: mc.p99,
        warnings,
    })
}

fn mdc_probe_p99(
    workload: &WorkloadConfig,
    pool: &PoolConfig,
    dist: &LatencyDistribution,
    opts: &SimulationOptions,
    size: u32,
) -> Result<f64, PoolsimError> {
    let probe_opts = mdc_probe_options(opts, size);
    let probe =
        monte_carlo::run_with_overhead(workload, size, pool.connection_overhead_ms, dist, &probe_opts)?;
    Ok(probe.p99)
}

fn mdc_probe_options(opts: &SimulationOptions, size: u32) -> SimulationOptions {
    let mut probe_opts = opts.clone();
    probe_opts.iterations = (opts.iterations / 4).clamp(400, 2_500);
    if let Some(seed) = opts.seed {
        probe_opts.seed = Some(seed ^ ((size as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
    }
    probe_opts
}

fn bootstrap_ci(chosen: u32, pool: &PoolConfig, wait_times: &[f64], target_wait_p99_ms: f64) -> (u32, u32) {
    if wait_times.is_empty() {
        return (chosen, chosen);
    }

    let mean = wait_times.iter().sum::<f64>() / wait_times.len() as f64;
    let variance = wait_times
        .iter()
        .map(|v| {
            let d = v - mean;
            d * d
        })
        .sum::<f64>()
        / wait_times.len() as f64;

    let stddev = variance.sqrt();
    let mut width = (stddev / target_wait_p99_ms).ceil() as u32;
    width = width.clamp(1, 5);

    (
        chosen.saturating_sub(width).max(pool.min_pool_size),
        chosen.saturating_add(width).min(pool.max_pool_size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_ci_returns_degenerate_interval_for_empty_waits() {
        let pool = PoolConfig {
            max_server_connections: 100,
            connection_overhead_ms: 2.0,
            idle_timeout_ms: None,
            min_pool_size: 2,
            max_pool_size: 20,
        };
        assert_eq!(bootstrap_ci(7, &pool, &[], 40.0), (7, 7));
    }
}
