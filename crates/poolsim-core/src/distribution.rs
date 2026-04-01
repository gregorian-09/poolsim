//! Distribution fitting and sampling for workload latency inputs.
//!
//! This module converts percentile-based or sample-based latency observations
//! into a service-time distribution usable by the optimizer and simulation
//! layers.
//!
//! Main entrypoints:
//!
//! - [`LatencyDistribution::fit`]
//! - [`LatencyDistribution::sample_ms`]
//! - [`LatencyDistribution::percentile_ms`]
//! - [`LatencyDistribution::mean_ms`]
//!
//! Selection rules:
//!
//! - percentile-only workloads fit one of the supported parametric models
//! - `raw_samples_ms` forces empirical fitting regardless of the requested model
//! - fitted distributions are reused by Monte Carlo and queueing helpers

use rand::Rng;
use rand_distr::{Distribution as RandDistribution, Exp, Gamma as RandGamma, LogNormal as RandLogNormal};
use statrs::distribution::{ContinuousCDF, Gamma as StatGamma, LogNormal as StatLogNormal, Normal};

use crate::{
    error::PoolsimError,
    types::{DistributionModel, WorkloadConfig},
};

/// Empirical cumulative distribution built from raw latency samples.
#[derive(Debug, Clone)]
pub struct EmpiricalCdf {
    samples: Vec<f64>,
}

impl EmpiricalCdf {
    fn new(mut samples: Vec<f64>) -> Result<Self, PoolsimError> {
        if samples.is_empty() {
            return Err(PoolsimError::Distribution(
                "empirical distribution requires at least one sample".to_string(),
            ));
        }
        samples.sort_by(|a, b| a.total_cmp(b));
        Ok(Self { samples })
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let idx = rng.gen_range(0..self.samples.len());
        self.samples[idx]
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let p = p.clamp(0.0, 1.0);
        let idx = ((self.samples.len() - 1) as f64 * p).round() as usize;
        self.samples[idx]
    }

    fn mean(&self) -> f64 {
        self.samples.iter().sum::<f64>() / self.samples.len() as f64
    }
}

/// Service-time distribution used during simulation and queue estimation.
#[derive(Debug, Clone)]
pub enum LatencyDistribution {
    /// Log-normal distribution parameters.
    LogNormal {
        /// Log-space mean.
        mu: f64,
        /// Log-space standard deviation.
        sigma: f64,
    },
    /// Exponential distribution with the provided mean.
    Exponential {
        /// Mean service time in milliseconds.
        mean_ms: f64,
    },
    /// Empirical distribution sampled directly from input samples.
    Empirical(EmpiricalCdf),
    /// Gamma distribution parameters.
    Gamma {
        /// Shape parameter (k).
        shape: f64,
        /// Scale parameter (theta).
        scale: f64,
    },
}

impl LatencyDistribution {
    /// Fits a latency distribution for a workload and selected model.
    ///
    /// If `workload.raw_samples_ms` is present, empirical fitting is used regardless
    /// of `model`.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::Distribution`] when distribution parameters cannot
    /// be derived.
    pub fn fit(workload: &WorkloadConfig, model: DistributionModel) -> Result<Self, PoolsimError> {
        if let Some(raw_samples) = &workload.raw_samples_ms {
            return EmpiricalCdf::new(raw_samples.clone()).map(Self::Empirical);
        }

        match model {
            DistributionModel::LogNormal | DistributionModel::Empirical => {
                let (mu, sigma) = fit_lognormal(workload)?;
                Ok(Self::LogNormal { mu, sigma })
            }
            DistributionModel::Exponential => {
                let mean_ms = workload.latency_p50_ms / std::f64::consts::LN_2;
                Ok(Self::Exponential { mean_ms })
            }
            DistributionModel::Gamma => {
                let (shape, scale) = fit_gamma(workload)?;
                Ok(Self::Gamma { shape, scale })
            }
        }
    }

    /// Draws one latency sample in milliseconds.
    pub fn sample_ms<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self {
            Self::LogNormal { mu, sigma } => {
                let dist = RandLogNormal::new(*mu, *sigma).expect("valid lognormal params");
                dist.sample(rng)
            }
            Self::Exponential { mean_ms } => {
                let dist = Exp::new(1.0 / mean_ms).expect("valid exponential rate");
                dist.sample(rng)
            }
            Self::Empirical(empirical) => empirical.sample(rng),
            Self::Gamma { shape, scale } => {
                let dist = RandGamma::new(*shape, *scale).expect("valid gamma params");
                dist.sample(rng)
            }
        }
    }

    /// Returns the requested percentile in milliseconds.
    ///
    /// `p` is clamped into `[0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::Distribution`] when percentile evaluation fails for
    /// the current parameterization.
    pub fn percentile_ms(&self, p: f64) -> Result<f64, PoolsimError> {
        let p = p.clamp(0.0, 1.0);
        match self {
            Self::LogNormal { mu, sigma } => {
                let dist =
                    StatLogNormal::new(*mu, *sigma).map_err(|e| PoolsimError::Distribution(e.to_string()))?;
                Ok(dist.inverse_cdf(p))
            }
            Self::Exponential { mean_ms } => Ok(-mean_ms * (1.0 - p).ln()),
            Self::Empirical(empirical) => Ok(empirical.percentile(p)),
            Self::Gamma { shape, scale } => {
                let dist =
                    StatGamma::new(*shape, *scale).map_err(|e| PoolsimError::Distribution(e.to_string()))?;
                Ok(dist.inverse_cdf(p))
            }
        }
    }

    /// Returns the mean service time in milliseconds.
    pub fn mean_ms(&self) -> f64 {
        match self {
            Self::LogNormal { mu, sigma } => (mu + 0.5 * sigma * sigma).exp(),
            Self::Exponential { mean_ms } => *mean_ms,
            Self::Empirical(empirical) => empirical.mean(),
            Self::Gamma { shape, scale } => shape * scale,
        }
    }
}

fn fit_lognormal(workload: &WorkloadConfig) -> Result<(f64, f64), PoolsimError> {
    let mu = workload.latency_p50_ms.ln();
    let normal = Normal::new(0.0, 1.0).map_err(|e| PoolsimError::Distribution(e.to_string()))?;

    let mut sigmas = Vec::new();
    if workload.latency_p95_ms > workload.latency_p50_ms {
        let z95 = normal.inverse_cdf(0.95);
        sigmas.push((workload.latency_p95_ms / workload.latency_p50_ms).ln() / z95);
    }
    if workload.latency_p99_ms > workload.latency_p50_ms {
        let z99 = normal.inverse_cdf(0.99);
        sigmas.push((workload.latency_p99_ms / workload.latency_p50_ms).ln() / z99);
    }

    let sigma = sigmas
        .into_iter()
        .filter(|s| s.is_finite() && *s > 0.0)
        .sum::<f64>();

    if sigma <= 0.0 {
        return Err(PoolsimError::Distribution(
            "unable to derive positive lognormal sigma from percentiles".to_string(),
        ));
    }

    let count = if workload.latency_p99_ms > workload.latency_p50_ms { 2.0 } else { 1.0 };
    Ok((mu, sigma / count))
}

fn fit_gamma(workload: &WorkloadConfig) -> Result<(f64, f64), PoolsimError> {
    let mean = (workload.latency_p50_ms + workload.latency_p95_ms + workload.latency_p99_ms) / 3.0;
    let std_est = ((workload.latency_p99_ms - workload.latency_p50_ms) / 2.326_347_874).max(1e-6);
    let var = std_est * std_est;
    let shape = (mean * mean / var).max(1e-6);
    let scale = (var / mean).max(1e-6);

    Ok((shape, scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empirical_percentile_returns_zero_for_empty_internal_samples() {
        let empirical = EmpiricalCdf { samples: Vec::new() };
        assert_eq!(empirical.percentile(0.5), 0.0);
    }

    #[test]
    fn fit_gamma_clamps_non_finite_derived_parameters_to_safe_minimum() {
        let workload = WorkloadConfig {
            requests_per_second: 100.0,
            latency_p50_ms: 10.0,
            latency_p95_ms: 20.0,
            latency_p99_ms: f64::INFINITY,
            raw_samples_ms: None,
            step_load_profile: None,
        };

        let (shape, scale) =
            fit_gamma(&workload).expect("non-finite intermediates should clamp to safe values");
        assert_eq!(shape, 1e-6);
        assert_eq!(scale, 1e-6);
    }
}
