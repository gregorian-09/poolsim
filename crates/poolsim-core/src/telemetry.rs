//! Telemetry import models and recommendation-diff helpers.
//!
//! This module keeps telemetry-derived sizing workflows in the core crate so
//! the CLI and web layers can share one recommendation implementation.

use serde::{Deserialize, Serialize};

use crate::{
    error::PoolsimError,
    evaluate, simulate,
    types::{
        EvaluationResult, PoolConfig, SaturationLevel, SimulationOptions, SimulationReport,
        WorkloadConfig,
    },
};

/// Imported production telemetry for a single pool sizing decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelemetrySnapshot {
    /// Optional service, application, or pool name.
    #[serde(default)]
    pub service_name: Option<String>,
    /// Optional telemetry window label, such as `15m`, `1h`, or `2026-05-15T10:00Z/2026-05-15T11:00Z`.
    #[serde(default)]
    pub window: Option<String>,
    /// Optional timestamp or timestamp range indicating when the snapshot was observed.
    #[serde(default)]
    pub observed_at: Option<String>,
    /// Current production pool size to compare against the recommendation.
    pub current_pool_size: u32,
    /// Workload inferred from telemetry.
    pub workload: WorkloadConfig,
    /// Pool bounds and backend connection limits to use for recommendation.
    pub pool: PoolConfig,
}

impl TelemetrySnapshot {
    /// Validates the imported telemetry snapshot before recommendation.
    ///
    /// # Errors
    ///
    /// Returns [`PoolsimError::InvalidInput`] when the telemetry payload, workload,
    /// pool constraints, or current production pool size are invalid.
    pub fn validate(&self) -> Result<(), PoolsimError> {
        self.workload.validate()?;
        self.pool.validate()?;

        if self.current_pool_size == 0 {
            return Err(PoolsimError::invalid_input(
                "INVALID_CURRENT_POOL_SIZE",
                "current_pool_size must be greater than 0",
                None,
            ));
        }

        if self.current_pool_size > self.pool.max_server_connections {
            return Err(PoolsimError::invalid_input(
                "INVALID_CURRENT_POOL_SIZE",
                "current_pool_size cannot exceed max_server_connections",
                Some(serde_json::json!({
                    "current_pool_size": self.current_pool_size,
                    "max_server_connections": self.pool.max_server_connections,
                })),
            ));
        }

        Ok(())
    }
}

/// Direction of change between the current and recommended pool sizes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PoolSizeChange {
    /// The recommendation needs more connections than production currently uses.
    Increase,
    /// The recommendation uses fewer connections than production currently uses.
    Decrease,
    /// The recommendation matches the current production pool size.
    Keep,
}

impl PoolSizeChange {
    fn from_delta(delta: i64) -> Self {
        match delta.cmp(&0) {
            std::cmp::Ordering::Greater => Self::Increase,
            std::cmp::Ordering::Less => Self::Decrease,
            std::cmp::Ordering::Equal => Self::Keep,
        }
    }
}

/// Recommendation diff between an imported production pool and a computed recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolRecommendationDiff {
    /// Current production pool size from telemetry.
    pub current_pool_size: u32,
    /// Recommended pool size from the sizing engine.
    pub recommended_pool_size: u32,
    /// Signed difference: `recommended_pool_size - current_pool_size`.
    pub pool_size_delta: i64,
    /// Direction of the sizing change.
    pub change: PoolSizeChange,
    /// Extra connections required when [`Self::change`] is [`PoolSizeChange::Increase`].
    pub additional_connections_required: u32,
    /// Connections that can be removed when [`Self::change`] is [`PoolSizeChange::Decrease`].
    pub removable_connections: u32,
    /// Signed percentage change relative to the current pool size.
    pub connection_change_percent: f64,
    /// Evaluation of the current production pool size.
    pub current_evaluation: EvaluationResult,
    /// Full simulation report for the recommended pool size.
    pub recommended_report: SimulationReport,
}

impl PoolRecommendationDiff {
    fn new(
        current_pool_size: u32,
        current_evaluation: EvaluationResult,
        recommended_report: SimulationReport,
    ) -> Self {
        let recommended_pool_size = recommended_report.optimal_pool_size;
        let pool_size_delta = i64::from(recommended_pool_size) - i64::from(current_pool_size);
        let change = PoolSizeChange::from_delta(pool_size_delta);
        let additional_connections_required = pool_size_delta.max(0) as u32;
        let removable_connections = (-pool_size_delta).max(0) as u32;
        let connection_change_percent = (pool_size_delta as f64 / current_pool_size as f64) * 100.0;

        Self {
            current_pool_size,
            recommended_pool_size,
            pool_size_delta,
            change,
            additional_connections_required,
            removable_connections,
            connection_change_percent,
            current_evaluation,
            recommended_report,
        }
    }

    /// Returns the worst saturation level across the current and recommended evaluations.
    pub fn worst_saturation(&self) -> SaturationLevel {
        if self.current_evaluation.saturation == SaturationLevel::Critical
            || self.recommended_report.saturation == SaturationLevel::Critical
        {
            SaturationLevel::Critical
        } else if self.current_evaluation.saturation == SaturationLevel::Warning
            || self.recommended_report.saturation == SaturationLevel::Warning
        {
            SaturationLevel::Warning
        } else {
            SaturationLevel::Ok
        }
    }
}

/// Recommendation output produced from imported telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelemetryRecommendation {
    /// Optional service, application, or pool name copied from telemetry.
    #[serde(default)]
    pub service_name: Option<String>,
    /// Optional telemetry window copied from telemetry.
    #[serde(default)]
    pub window: Option<String>,
    /// Optional observation timestamp copied from telemetry.
    #[serde(default)]
    pub observed_at: Option<String>,
    /// Detailed current-vs-recommended diff.
    pub diff: PoolRecommendationDiff,
}

/// Computes a pool-size recommendation and diff from imported telemetry.
///
/// # Errors
///
/// Returns [`PoolsimError`] when telemetry validation, fixed-pool evaluation, or
/// full simulation fails.
pub fn recommend_from_telemetry(
    snapshot: &TelemetrySnapshot,
    opts: &SimulationOptions,
) -> Result<TelemetryRecommendation, PoolsimError> {
    snapshot.validate()?;
    opts.validate()?;

    let current_evaluation = evaluate(&snapshot.workload, snapshot.current_pool_size, opts)?;
    let recommended_report = simulate(&snapshot.workload, &snapshot.pool, opts)?;
    let diff = PoolRecommendationDiff::new(
        snapshot.current_pool_size,
        current_evaluation,
        recommended_report,
    );

    Ok(TelemetryRecommendation {
        service_name: snapshot.service_name.clone(),
        window: snapshot.window.clone(),
        observed_at: snapshot.observed_at.clone(),
        diff,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DistributionModel, QueueModel};

    fn snapshot(current_pool_size: u32) -> TelemetrySnapshot {
        TelemetrySnapshot {
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: Some("2026-05-15T10:00:00Z".to_string()),
            current_pool_size,
            workload: WorkloadConfig {
                requests_per_second: 180.0,
                latency_p50_ms: 8.0,
                latency_p95_ms: 28.0,
                latency_p99_ms: 75.0,
                raw_samples_ms: None,
                step_load_profile: None,
            },
            pool: PoolConfig {
                max_server_connections: 100,
                connection_overhead_ms: 1.5,
                idle_timeout_ms: Some(60_000),
                min_pool_size: 2,
                max_pool_size: 20,
            },
        }
    }

    fn options() -> SimulationOptions {
        SimulationOptions {
            iterations: 1_200,
            seed: Some(42),
            distribution: DistributionModel::LogNormal,
            queue_model: QueueModel::MMC,
            target_wait_p99_ms: 45.0,
            max_acceptable_rho: 0.85,
        }
    }

    #[test]
    fn validates_current_pool_size_bounds() {
        let zero = snapshot(0);
        let err = zero
            .validate()
            .expect_err("zero current pool size should fail");
        assert_eq!(err.code(), "INVALID_CURRENT_POOL_SIZE");

        let too_large = snapshot(101);
        let err = too_large
            .validate()
            .expect_err("current pool above backend limit should fail");
        assert_eq!(err.code(), "INVALID_CURRENT_POOL_SIZE");
    }

    #[test]
    fn recommends_from_telemetry_and_keeps_metadata() {
        let recommendation = recommend_from_telemetry(&snapshot(6), &options())
            .expect("telemetry recommendation should run");

        assert_eq!(recommendation.service_name.as_deref(), Some("checkout-api"));
        assert_eq!(recommendation.window.as_deref(), Some("1h"));
        assert_eq!(recommendation.diff.current_pool_size, 6);
        assert!(recommendation.diff.recommended_pool_size >= 2);
        assert!(recommendation.diff.recommended_pool_size <= 20);
        assert_eq!(
            recommendation.diff.pool_size_delta,
            i64::from(recommendation.diff.recommended_pool_size) - 6
        );
        assert!(matches!(
            recommendation.diff.change,
            PoolSizeChange::Increase | PoolSizeChange::Decrease | PoolSizeChange::Keep
        ));
    }

    fn evaluation(pool_size: u32, saturation: SaturationLevel) -> EvaluationResult {
        EvaluationResult {
            pool_size,
            utilisation_rho: 0.50,
            mean_queue_wait_ms: 1.0,
            p99_queue_wait_ms: 5.0,
            saturation,
            warnings: Vec::new(),
        }
    }

    fn report(pool_size: u32, saturation: SaturationLevel) -> SimulationReport {
        SimulationReport {
            optimal_pool_size: pool_size,
            confidence_interval: (5, 7),
            cold_start_min_pool_size: 5,
            utilisation_rho: 0.90,
            mean_queue_wait_ms: 2.0,
            p99_queue_wait_ms: 20.0,
            saturation,
            sensitivity: Vec::new(),
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn computes_decrease_diff_and_warning_saturation() {
        let diff = PoolRecommendationDiff::new(
            8,
            evaluation(8, SaturationLevel::Ok),
            report(6, SaturationLevel::Warning),
        );
        assert_eq!(diff.change, PoolSizeChange::Decrease);
        assert_eq!(diff.pool_size_delta, -2);
        assert_eq!(diff.additional_connections_required, 0);
        assert_eq!(diff.removable_connections, 2);
        assert_eq!(diff.connection_change_percent, -25.0);
        assert_eq!(diff.worst_saturation(), SaturationLevel::Warning);
    }

    #[test]
    fn computes_increase_and_keep_diffs() {
        let increase = PoolRecommendationDiff::new(
            8,
            evaluation(8, SaturationLevel::Ok),
            report(10, SaturationLevel::Ok),
        );
        assert_eq!(increase.change, PoolSizeChange::Increase);
        assert_eq!(increase.pool_size_delta, 2);
        assert_eq!(increase.additional_connections_required, 2);
        assert_eq!(increase.removable_connections, 0);
        assert_eq!(increase.connection_change_percent, 25.0);
        assert_eq!(increase.worst_saturation(), SaturationLevel::Ok);

        let keep = PoolRecommendationDiff::new(
            8,
            evaluation(8, SaturationLevel::Critical),
            report(8, SaturationLevel::Ok),
        );
        assert_eq!(keep.change, PoolSizeChange::Keep);
        assert_eq!(keep.pool_size_delta, 0);
        assert_eq!(keep.additional_connections_required, 0);
        assert_eq!(keep.removable_connections, 0);
        assert_eq!(keep.connection_change_percent, 0.0);
        assert_eq!(keep.worst_saturation(), SaturationLevel::Critical);
    }
}
