//! Monte Carlo queue simulation primitives.
//!
//! This module is used when `poolsim` needs a sampled queue-wait distribution
//! rather than only analytical queueing metrics.
//!
//! Typical uses:
//!
//! - full simulation through crate-root [`crate::simulate`]
//! - fixed-size evaluation through [`crate::evaluate`]
//! - M/D/c probing during optimization and sensitivity analysis
//!
//! The public entrypoint is [`crate::monte_carlo::run`], which returns
//! [`crate::monte_carlo::MonteCarloResult`].

use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::{Distribution, Exp};

use crate::{
    distribution::LatencyDistribution, error::PoolsimError, types::QueueModel,
    types::SimulationOptions, WorkloadConfig,
};

/// Monte Carlo queue-wait summary and raw sampled waits.
#[derive(Debug, Clone)]
pub struct MonteCarloResult {
    /// Sorted per-request queue waits in milliseconds.
    pub wait_times_ms: Vec<f64>,
    /// p50 queue wait (milliseconds).
    pub p50: f64,
    /// p95 queue wait (milliseconds).
    pub p95: f64,
    /// p99 queue wait (milliseconds).
    pub p99: f64,
    /// Mean queue wait (milliseconds).
    pub mean: f64,
}

/// Runs Monte Carlo queue-wait simulation.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] for invalid pool size, and
/// [`PoolsimError::Simulation`] when no wait samples are produced.
pub fn run(
    workload: &WorkloadConfig,
    pool_size: u32,
    dist: &LatencyDistribution,
    opts: &SimulationOptions,
) -> Result<MonteCarloResult, PoolsimError> {
    run_with_overhead(workload, pool_size, 0.0, dist, opts)
}

pub(crate) fn run_with_overhead(
    workload: &WorkloadConfig,
    pool_size: u32,
    connection_overhead_ms: f64,
    dist: &LatencyDistribution,
    opts: &SimulationOptions,
) -> Result<MonteCarloResult, PoolsimError> {
    if pool_size == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_POOL_SIZE",
            "pool_size must be > 0",
            None,
        ));
    }

    let iterations = opts.iterations as usize;
    let lambda = workload.requests_per_second;
    let base_seed = opts.seed.unwrap_or_else(rand::random::<u64>);
    let deterministic_service_ms = dist.mean_ms() + connection_overhead_ms;

    #[cfg(feature = "parallel")]
    let waits = {
        use rayon::prelude::*;

        let workers = rayon::current_num_threads().max(1);
        let chunk_count = workers.min(iterations.max(1));
        let chunk_size = iterations.div_ceil(chunk_count);

        (0..chunk_count)
            .into_par_iter()
            .map(|chunk_id| {
                let start = chunk_id * chunk_size;
                let end = ((chunk_id + 1) * chunk_size).min(iterations);
                if start >= end {
                    return Vec::new();
                }

                let seed = base_seed ^ ((chunk_id as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                let mut rng = StdRng::seed_from_u64(seed);
                simulate_chunk(
                    &mut rng,
                    end - start,
                    lambda,
                    pool_size,
                    connection_overhead_ms,
                    deterministic_service_ms,
                    opts.queue_model,
                    dist,
                )
            })
            .reduce(Vec::new, |mut left, mut right| {
                left.append(&mut right);
                left
            })
    };

    #[cfg(not(feature = "parallel"))]
    let waits = {
        let mut rng = StdRng::seed_from_u64(base_seed);
        simulate_chunk(
            &mut rng,
            iterations,
            lambda,
            pool_size,
            connection_overhead_ms,
            deterministic_service_ms,
            opts.queue_model,
            dist,
        )
    };

    build_result(waits)
}

fn simulate_chunk<R: Rng + ?Sized>(
    rng: &mut R,
    iterations: usize,
    lambda: f64,
    pool_size: u32,
    connection_overhead_ms: f64,
    deterministic_service_ms: f64,
    queue_model: QueueModel,
    dist: &LatencyDistribution,
) -> Vec<f64> {
    let mut waits = Vec::with_capacity(iterations);
    let mut arrival_time_s = 0.0;
    let mut server_free_at = vec![0.0f64; pool_size as usize];

    let inter_arrival = Exp::new(lambda).expect("lambda > 0 for exponential inter-arrival");

    for _ in 0..iterations {
        arrival_time_s += inter_arrival.sample(rng);

        let mut min_idx = 0usize;
        let mut min_free = server_free_at[0];
        for (idx, free_at) in server_free_at.iter().copied().enumerate().skip(1) {
            if free_at < min_free {
                min_idx = idx;
                min_free = free_at;
            }
        }

        let wait_s = (min_free - arrival_time_s).max(0.0);
        let service_ms = match queue_model {
            QueueModel::MMC => dist.sample_ms(rng) + connection_overhead_ms,
            QueueModel::MDC => deterministic_service_ms,
        };
        let service_s = service_ms / 1_000.0;

        server_free_at[min_idx] = arrival_time_s + wait_s + service_s;
        waits.push(wait_s * 1_000.0);
    }

    waits
}

fn build_result(mut waits: Vec<f64>) -> Result<MonteCarloResult, PoolsimError> {
    if waits.is_empty() {
        return Err(PoolsimError::Simulation(
            "no wait times were generated during simulation".to_string(),
        ));
    }

    let mean = waits.iter().sum::<f64>() / waits.len() as f64;
    waits.sort_by(|a, b| a.total_cmp(b));

    Ok(MonteCarloResult {
        p50: percentile(&waits, 0.50),
        p95: percentile(&waits, 0.95),
        p99: percentile(&waits, 0.99),
        mean,
        wait_times_ms: waits,
    })
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let p = p.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_returns_zero_for_empty_input() {
        assert_eq!(percentile(&[], 0.5), 0.0);
    }
}
