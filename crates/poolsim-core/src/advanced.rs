//! Optional advanced sizing helpers for acquisition waits, transaction mixes, and leaks.
//!
//! These helpers are additive and do not change the default simulation model.

use crate::{erlang, error::PoolsimError, types::WorkloadConfig};

/// Two-stage acquisition wait estimate for a fixed pool size.
#[derive(Debug, Clone, PartialEq)]
pub struct AcquisitionEstimate {
    pool_size: u32,
    utilisation_rho: f64,
    mean_acquisition_wait_ms: f64,
    p99_acquisition_wait_ms: f64,
    acquisition_timeout_ms: f64,
    timeout_risk: bool,
}

impl AcquisitionEstimate {
    /// Fixed pool size used by the estimate.
    pub fn pool_size(&self) -> u32 {
        self.pool_size
    }

    /// Estimated utilisation ratio.
    pub fn utilisation_rho(&self) -> f64 {
        self.utilisation_rho
    }

    /// Estimated mean wait to acquire a pool slot in milliseconds.
    pub fn mean_acquisition_wait_ms(&self) -> f64 {
        self.mean_acquisition_wait_ms
    }

    /// Estimated p99 wait to acquire a pool slot in milliseconds.
    pub fn p99_acquisition_wait_ms(&self) -> f64 {
        self.p99_acquisition_wait_ms
    }

    /// Configured acquisition timeout in milliseconds.
    pub fn acquisition_timeout_ms(&self) -> f64 {
        self.acquisition_timeout_ms
    }

    /// Whether the p99 acquisition wait is at or above the timeout.
    pub fn timeout_risk(&self) -> bool {
        self.timeout_risk
    }
}

/// Estimates pool-slot acquisition wait before database service time.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when pool size or timeout is invalid,
/// and propagates workload validation or Erlang-C errors.
pub fn estimate_acquisition_wait(
    workload: &WorkloadConfig,
    pool_size: u32,
    acquisition_timeout_ms: f64,
) -> Result<AcquisitionEstimate, PoolsimError> {
    workload.validate()?;
    if pool_size == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_POOL_SIZE",
            "pool_size must be greater than 0",
            None,
        ));
    }
    if acquisition_timeout_ms <= 0.0 || !acquisition_timeout_ms.is_finite() {
        return Err(PoolsimError::invalid_input(
            "INVALID_ACQUISITION_TIMEOUT",
            "acquisition_timeout_ms must be finite and greater than 0",
            None,
        ));
    }

    let lambda = workload.requests_per_second;
    let service_mean_ms =
        (workload.latency_p50_ms + workload.latency_p95_ms + workload.latency_p99_ms) / 3.0;
    let mu = 1_000.0 / service_mean_ms;
    let rho = erlang::utilisation(lambda, mu, pool_size);
    let mean = erlang::mean_queue_wait_ms(lambda, mu, pool_size)?;
    let p99 = erlang::queue_wait_percentile_ms(lambda, mu, pool_size, 0.99)?;

    Ok(AcquisitionEstimate {
        pool_size,
        utilisation_rho: rho,
        mean_acquisition_wait_ms: mean,
        p99_acquisition_wait_ms: p99,
        acquisition_timeout_ms,
        timeout_risk: p99 >= acquisition_timeout_ms,
    })
}

/// One traffic class in a transaction-level workload mix.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionClass {
    name: String,
    requests_per_second: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
}

impl TransactionClass {
    /// Creates a transaction class with its own request rate and latency percentiles.
    pub fn new(
        name: impl Into<String>,
        requests_per_second: f64,
        latency_p50_ms: f64,
        latency_p95_ms: f64,
        latency_p99_ms: f64,
    ) -> Self {
        Self {
            name: name.into(),
            requests_per_second,
            latency_p50_ms,
            latency_p95_ms,
            latency_p99_ms,
        }
    }

    /// Transaction class name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Request rate for this class.
    pub fn requests_per_second(&self) -> f64 {
        self.requests_per_second
    }

    fn validate(&self) -> Result<(), PoolsimError> {
        if self.name.trim().is_empty() {
            return Err(PoolsimError::invalid_input(
                "INVALID_TRANSACTION_NAME",
                "transaction class name must not be empty",
                None,
            ));
        }
        WorkloadConfig {
            requests_per_second: self.requests_per_second,
            latency_p50_ms: self.latency_p50_ms,
            latency_p95_ms: self.latency_p95_ms,
            latency_p99_ms: self.latency_p99_ms,
            raw_samples_ms: None,
            step_load_profile: None,
        }
        .validate()
    }
}

/// A transaction-level workload mix.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionMix {
    classes: Vec<TransactionClass>,
}

impl TransactionMix {
    /// Creates a transaction mix after validating each class.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] if the mix is empty or a class is invalid.
    pub fn new(classes: Vec<TransactionClass>) -> Result<Self, PoolsimError> {
        if classes.is_empty() {
            return Err(PoolsimError::invalid_input(
                "INVALID_TRANSACTION_MIX",
                "transaction mix must contain at least one class",
                None,
            ));
        }
        for class in &classes {
            class.validate()?;
        }
        Ok(Self { classes })
    }

    /// Classes in this mix.
    pub fn classes(&self) -> &[TransactionClass] {
        &self.classes
    }

    /// Aggregates the transaction mix into a workload usable by the existing simulator.
    pub fn aggregate_workload(&self) -> WorkloadConfig {
        let total_rps: f64 = self
            .classes
            .iter()
            .map(|class| class.requests_per_second)
            .sum();
        let weighted = |latency: fn(&TransactionClass) -> f64| {
            self.classes
                .iter()
                .map(|class| latency(class) * class.requests_per_second / total_rps)
                .sum()
        };

        WorkloadConfig {
            requests_per_second: total_rps,
            latency_p50_ms: weighted(|class| class.latency_p50_ms),
            latency_p95_ms: weighted(|class| class.latency_p95_ms),
            latency_p99_ms: weighted(|class| class.latency_p99_ms),
            raw_samples_ms: None,
            step_load_profile: None,
        }
    }
}

/// Result of connection leak modeling.
#[derive(Debug, Clone, PartialEq)]
pub struct LeakSimulation {
    initial_pool_size: u32,
    final_available_connections: u32,
    leaked_connections: u32,
    minutes_to_exhaustion: Option<u32>,
}

impl LeakSimulation {
    /// Initial pool size before leakage.
    pub fn initial_pool_size(&self) -> u32 {
        self.initial_pool_size
    }

    /// Available connections at the modeled duration.
    pub fn final_available_connections(&self) -> u32 {
        self.final_available_connections
    }

    /// Total leaked connections by the modeled duration.
    pub fn leaked_connections(&self) -> u32 {
        self.leaked_connections
    }

    /// First minute where all pool slots are leaked, if exhaustion occurs.
    pub fn minutes_to_exhaustion(&self) -> Option<u32> {
        self.minutes_to_exhaustion
    }
}

/// Simulates gradual connection leakage over time.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when pool size, leak rate, or duration is invalid.
pub fn simulate_connection_leak(
    initial_pool_size: u32,
    leak_rate_per_minute: f64,
    duration_minutes: u32,
) -> Result<LeakSimulation, PoolsimError> {
    if initial_pool_size == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_POOL_SIZE",
            "initial_pool_size must be greater than 0",
            None,
        ));
    }
    if leak_rate_per_minute < 0.0 || !leak_rate_per_minute.is_finite() {
        return Err(PoolsimError::invalid_input(
            "INVALID_LEAK_RATE",
            "leak_rate_per_minute must be finite and non-negative",
            None,
        ));
    }

    let leaked = (leak_rate_per_minute * f64::from(duration_minutes)).floor() as u32;
    let capped_leaked = leaked.min(initial_pool_size);
    let minutes_to_exhaustion = if leak_rate_per_minute > 0.0 {
        Some((f64::from(initial_pool_size) / leak_rate_per_minute).ceil() as u32)
            .filter(|minutes| *minutes <= duration_minutes)
    } else {
        None
    };

    Ok(LeakSimulation {
        initial_pool_size,
        final_available_connections: initial_pool_size.saturating_sub(capped_leaked),
        leaked_connections: capped_leaked,
        minutes_to_exhaustion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workload() -> WorkloadConfig {
        WorkloadConfig {
            requests_per_second: 100.0,
            latency_p50_ms: 5.0,
            latency_p95_ms: 20.0,
            latency_p99_ms: 40.0,
            raw_samples_ms: None,
            step_load_profile: None,
        }
    }

    #[test]
    fn acquisition_wait_estimates_timeout_risk() {
        let estimate = estimate_acquisition_wait(&workload(), 4, 100.0).expect("estimate");
        assert_eq!(estimate.pool_size(), 4);
        assert!(estimate.utilisation_rho().is_finite());
        assert!(estimate.p99_acquisition_wait_ms() >= 0.0);
        assert_eq!(
            estimate.timeout_risk(),
            estimate.p99_acquisition_wait_ms() >= 100.0
        );
    }

    #[test]
    fn transaction_mix_aggregates_weighted_workload() {
        let mix = TransactionMix::new(vec![
            TransactionClass::new("fast-read", 80.0, 4.0, 12.0, 30.0),
            TransactionClass::new("slow-write", 20.0, 20.0, 80.0, 160.0),
        ])
        .expect("valid mix");
        assert_eq!(mix.classes()[0].name(), "fast-read");
        let workload = mix.aggregate_workload();
        assert_eq!(workload.requests_per_second, 100.0);
        assert!(workload.latency_p99_ms > 30.0);
    }

    #[test]
    fn connection_leak_reports_exhaustion() {
        let leak = simulate_connection_leak(10, 2.0, 6).expect("leak simulation");
        assert_eq!(leak.initial_pool_size(), 10);
        assert_eq!(leak.leaked_connections(), 10);
        assert_eq!(leak.final_available_connections(), 0);
        assert_eq!(leak.minutes_to_exhaustion(), Some(5));
    }

    #[test]
    fn invalid_advanced_inputs_are_rejected() {
        assert!(estimate_acquisition_wait(&workload(), 0, 100.0).is_err());
        assert!(TransactionMix::new(Vec::new()).is_err());
        assert!(TransactionMix::new(vec![TransactionClass::new("", 1.0, 1.0, 2.0, 3.0)]).is_err());
        assert!(simulate_connection_leak(1, f64::NAN, 10).is_err());
    }
}
