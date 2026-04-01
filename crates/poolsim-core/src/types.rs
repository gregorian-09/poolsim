//! Public data models used by `poolsim-core`.
//!
//! The types in this module define:
//!
//! - workload inputs
//! - pool and server constraints
//! - simulation options
//! - risk and saturation labels
//! - simulation, evaluation, sensitivity, and step-load outputs
//!
//! Most library callers will build values from this module and then pass them
//! into crate-root helpers such as [`crate::simulate`] and [`crate::evaluate`].

use serde::{Deserialize, Serialize};

use crate::error::PoolsimError;

/// Latency distribution model used to fit service-time behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionModel {
    /// Log-normal distribution fitted from percentile inputs.
    LogNormal,
    /// Exponential distribution inferred from median latency.
    Exponential,
    /// Empirical distribution built from raw samples.
    Empirical,
    /// Gamma distribution fitted from percentile moments.
    Gamma,
}

/// Queueing model used for queue-wait estimation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueModel {
    /// M/M/c queue (stochastic service time).
    MMC,
    /// M/D/c queue (deterministic service time).
    MDC,
}

/// Risk class assigned to a sensitivity candidate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskLevel {
    /// Well within target constraints.
    Low,
    /// Acceptable but close to limits.
    Medium,
    /// Elevated risk.
    High,
    /// Unacceptable operating point.
    Critical,
}

/// Saturation status derived from utilization (`rho`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SaturationLevel {
    /// Utilization is healthy.
    Ok,
    /// Utilization is elevated.
    Warning,
    /// Utilization is critical.
    Critical,
}

impl SaturationLevel {
    /// Maps utilization ratio (`rho`) to [`SaturationLevel`].
    pub fn from_rho(rho: f64) -> Self {
        if rho >= 0.95 {
            Self::Critical
        } else if rho >= 0.85 {
            Self::Warning
        } else {
            Self::Ok
        }
    }
}

/// Workload inputs used by simulation and evaluation routines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkloadConfig {
    /// Arrival rate in requests per second.
    pub requests_per_second: f64,
    /// p50 end-to-end latency (milliseconds).
    pub latency_p50_ms: f64,
    /// p95 end-to-end latency (milliseconds).
    pub latency_p95_ms: f64,
    /// p99 end-to-end latency (milliseconds).
    pub latency_p99_ms: f64,
    /// Optional raw latency samples used for empirical fitting.
    #[serde(default)]
    pub raw_samples_ms: Option<Vec<f64>>,
    /// Optional step-load profile for burst analysis.
    #[serde(default)]
    pub step_load_profile: Option<Vec<StepLoadPoint>>,
}

impl WorkloadConfig {
    /// Validates workload values and ordering constraints.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] when fields are out of range or
    /// percentile/step-profile ordering is invalid.
    pub fn validate(&self) -> Result<(), PoolsimError> {
        if self.requests_per_second <= 0.0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_RPS",
                "requests_per_second must be greater than 0",
                None,
            ));
        }

        if self.latency_p50_ms <= 0.0 || self.latency_p95_ms <= 0.0 || self.latency_p99_ms <= 0.0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_LATENCY",
                "latency percentiles must be greater than 0",
                None,
            ));
        }

        if !(self.latency_p50_ms < self.latency_p95_ms && self.latency_p95_ms < self.latency_p99_ms) {
            return Err(PoolsimError::invalid_input(
                "INVALID_LATENCY_ORDER",
                "latency percentiles must be ordered: p50 < p95 < p99",
                Some(serde_json::json!({
                    "p50": self.latency_p50_ms,
                    "p95": self.latency_p95_ms,
                    "p99": self.latency_p99_ms,
                })),
            ));
        }

        if let Some(samples) = &self.raw_samples_ms {
            if samples.len() < 3 {
                return Err(PoolsimError::invalid_input(
                    "INVALID_SAMPLES",
                    "raw_samples_ms must contain at least 3 values",
                    None,
                ));
            }
            if samples.iter().any(|v| *v <= 0.0 || !v.is_finite()) {
                return Err(PoolsimError::invalid_input(
                    "INVALID_SAMPLES",
                    "raw_samples_ms values must be finite and greater than 0",
                    None,
                ));
            }
        }

        if let Some(profile) = &self.step_load_profile {
            if profile.is_empty() {
                return Err(PoolsimError::invalid_input(
                    "INVALID_STEP_LOAD_PROFILE",
                    "step_load_profile must not be empty when provided",
                    None,
                ));
            }

            let mut prev_time = None;
            for point in profile {
                point.validate()?;
                if let Some(prev) = prev_time {
                    if point.time_s <= prev {
                        return Err(PoolsimError::invalid_input(
                            "INVALID_STEP_LOAD_PROFILE_ORDER",
                            "step_load_profile time_s values must be strictly increasing",
                            Some(serde_json::json!({
                                "previous_time_s": prev,
                                "current_time_s": point.time_s,
                            })),
                        ));
                    }
                }
                prev_time = Some(point.time_s);
            }
        }

        Ok(())
    }
}

/// A single step-load point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepLoadPoint {
    /// Step timestamp in seconds from the scenario start.
    #[serde(alias = "time_seconds", alias = "at_second")]
    pub time_s: u32,
    /// Arrival rate at `time_s` in requests per second.
    pub requests_per_second: f64,
}

impl StepLoadPoint {
    /// Validates the step-load point payload.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] if `requests_per_second` is non-finite
    /// or not strictly positive.
    pub fn validate(&self) -> Result<(), PoolsimError> {
        if self.requests_per_second <= 0.0 || !self.requests_per_second.is_finite() {
            return Err(PoolsimError::invalid_input(
                "INVALID_STEP_LOAD_RPS",
                "step_load_profile requests_per_second must be finite and greater than 0",
                Some(serde_json::json!({
                    "time_s": self.time_s,
                    "requests_per_second": self.requests_per_second,
                })),
            ));
        }
        Ok(())
    }
}

/// Pool sizing and server-capacity constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolConfig {
    /// Maximum backend/server connections available.
    pub max_server_connections: u32,
    /// Connection establishment overhead (milliseconds).
    #[serde(alias = "connection_establishment_overhead_ms")]
    pub connection_overhead_ms: f64,
    /// Optional idle timeout in milliseconds.
    pub idle_timeout_ms: Option<u64>,
    /// Minimum pool size considered by optimization.
    pub min_pool_size: u32,
    /// Maximum pool size considered by optimization.
    pub max_pool_size: u32,
}

impl PoolConfig {
    /// Validates pool sizing constraints and capacity bounds.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] when the sizing range or
    /// server-capacity constraints are invalid.
    pub fn validate(&self) -> Result<(), PoolsimError> {
        if self.max_server_connections == 0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_MAX_SERVER_CONNECTIONS",
                "max_server_connections must be greater than 0",
                None,
            ));
        }

        if self.connection_overhead_ms < 0.0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_CONNECTION_OVERHEAD",
                "connection_overhead_ms must be non-negative",
                None,
            ));
        }

        if self.min_pool_size == 0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_MIN_POOL_SIZE",
                "min_pool_size must be greater than 0",
                None,
            ));
        }

        if self.min_pool_size > self.max_pool_size {
            return Err(PoolsimError::invalid_input(
                "INVALID_POOL_RANGE",
                "min_pool_size must be <= max_pool_size",
                None,
            ));
        }

        if self.max_pool_size > self.max_server_connections {
            return Err(PoolsimError::invalid_input(
                "POOL_EXCEEDS_SERVER_MAX",
                "max_pool_size must be <= max_server_connections",
                None,
            ));
        }

        Ok(())
    }
}

/// Simulation and optimization options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationOptions {
    /// Number of Monte Carlo iterations.
    #[serde(default = "default_iterations")]
    pub iterations: u32,
    /// Optional deterministic random seed.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Distribution model used to fit service times.
    #[serde(default = "default_distribution")]
    pub distribution: DistributionModel,
    /// Queue model used to estimate wait behavior.
    #[serde(default = "default_queue_model")]
    pub queue_model: QueueModel,
    /// Target p99 wait (ms) used for candidate acceptance and risk labels.
    #[serde(default = "default_target_wait_p99_ms")]
    pub target_wait_p99_ms: f64,
    /// Maximum acceptable utilization (`rho`) for candidate acceptance.
    #[serde(default = "default_max_acceptable_rho")]
    pub max_acceptable_rho: f64,
}

impl Default for SimulationOptions {
    fn default() -> Self {
        Self {
            iterations: default_iterations(),
            seed: None,
            distribution: default_distribution(),
            queue_model: default_queue_model(),
            target_wait_p99_ms: default_target_wait_p99_ms(),
            max_acceptable_rho: default_max_acceptable_rho(),
        }
    }
}

impl SimulationOptions {
    /// Validates simulation options.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] when iteration count, target wait,
    /// or utilization thresholds are invalid.
    pub fn validate(&self) -> Result<(), PoolsimError> {
        if self.iterations == 0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_ITERATIONS",
                "iterations must be greater than 0",
                None,
            ));
        }

        if self.target_wait_p99_ms <= 0.0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_TARGET_WAIT",
                "target_wait_p99_ms must be greater than 0",
                None,
            ));
        }

        if !(0.0..1.0).contains(&self.max_acceptable_rho) {
            return Err(PoolsimError::invalid_input(
                "INVALID_MAX_ACCEPTABLE_RHO",
                "max_acceptable_rho must be in [0, 1)",
                None,
            ));
        }

        Ok(())
    }
}

fn default_iterations() -> u32 {
    10_000
}

fn default_distribution() -> DistributionModel {
    DistributionModel::LogNormal
}

fn default_queue_model() -> QueueModel {
    QueueModel::MMC
}

fn default_target_wait_p99_ms() -> f64 {
    50.0
}

fn default_max_acceptable_rho() -> f64 {
    0.85
}

/// Sensitivity table row for a single candidate pool size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SensitivityRow {
    /// Candidate pool size.
    pub pool_size: u32,
    /// Utilization ratio (`rho`) at this pool size.
    pub utilisation_rho: f64,
    /// Mean queue wait in milliseconds.
    pub mean_queue_wait_ms: f64,
    /// p99 queue wait in milliseconds.
    pub p99_queue_wait_ms: f64,
    /// Qualitative risk label for this candidate.
    pub risk: RiskLevel,
}

/// End-to-end simulation result for a workload/pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimulationReport {
    /// Recommended warm pool size.
    pub optimal_pool_size: u32,
    /// Confidence interval around `optimal_pool_size`.
    pub confidence_interval: (u32, u32),
    /// Recommended minimum pool size for cold-start bursts.
    pub cold_start_min_pool_size: u32,
    /// Utilization ratio (`rho`) at the recommended pool size.
    pub utilisation_rho: f64,
    /// Mean queue wait in milliseconds at the recommended pool size.
    pub mean_queue_wait_ms: f64,
    /// p99 queue wait in milliseconds at the recommended pool size.
    pub p99_queue_wait_ms: f64,
    /// Saturation status at the recommended pool size.
    pub saturation: SaturationLevel,
    /// Per-pool-size sensitivity rows.
    pub sensitivity: Vec<SensitivityRow>,
    /// Step-load analysis rows (empty when no step profile is configured).
    #[serde(default)]
    pub step_load_analysis: Vec<StepLoadResult>,
    /// Human-readable warnings and advisory notes.
    pub warnings: Vec<String>,
}

/// Evaluation output for a single fixed pool size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationResult {
    /// Evaluated pool size.
    pub pool_size: u32,
    /// Utilization ratio (`rho`) for `pool_size`.
    pub utilisation_rho: f64,
    /// Mean queue wait in milliseconds for `pool_size`.
    pub mean_queue_wait_ms: f64,
    /// p99 queue wait in milliseconds for `pool_size`.
    pub p99_queue_wait_ms: f64,
    /// Saturation status for `pool_size`.
    pub saturation: SaturationLevel,
    /// Human-readable warnings and advisory notes.
    pub warnings: Vec<String>,
}

/// Result row for a single step-load point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepLoadResult {
    /// Step timestamp in seconds from scenario start.
    pub time_s: u32,
    /// Arrival rate in requests per second at this step.
    pub requests_per_second: f64,
    /// Utilization ratio (`rho`) at this step.
    pub utilisation_rho: f64,
    /// p99 queue wait in milliseconds at this step.
    pub p99_queue_wait_ms: f64,
    /// Saturation status at this step.
    pub saturation: SaturationLevel,
}
