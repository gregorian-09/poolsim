//! Safety checks for recommendations that would increase an application pool.
//!
//! A recommendation is not automatically safe to apply just because its queue
//! metrics improve. This module combines the existing telemetry-quality report
//! with deployment topology and database connection-budget evidence, then
//! returns an explicit allow, review, or block decision.

use serde::{Deserialize, Serialize};

use crate::{
    error::PoolsimError,
    pooler::EvidenceConfidence,
    telemetry::{PoolSizeChange, TelemetryRecommendation},
    telemetry_quality::{TelemetryQualityReport, TelemetryQualityStatus},
    types::RiskLevel,
};

/// Decision returned by the pool scale-safety gate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PoolScaleGateStatus {
    /// The recommendation does not violate the supplied safety evidence.
    Allowed,
    /// The recommendation may be reasonable, but evidence is incomplete or limited.
    NeedsReview,
    /// Applying the increase would violate an explicit safety condition.
    Blocked,
}

/// A machine-readable explanation produced by the pool scale-safety gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolScaleGateFinding {
    /// Stable machine-readable finding code.
    pub code: String,
    /// Risk associated with the finding.
    pub risk: RiskLevel,
    /// Human-readable explanation.
    pub message: String,
    /// Recommended remediation or verification step.
    pub remediation: String,
}

impl PoolScaleGateFinding {
    fn new(
        code: &'static str,
        risk: RiskLevel,
        message: &'static str,
        remediation: &'static str,
    ) -> Self {
        Self {
            code: code.to_string(),
            risk,
            message: message.to_string(),
            remediation: remediation.to_string(),
        }
    }
}

/// Evidence and topology supplied to the pool scale-safety gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PoolScaleGateInput {
    /// Existing recommendation and current-versus-recommended pool diff.
    pub recommendation: TelemetryRecommendation,
    /// Quality assessment for the telemetry behind the recommendation.
    pub telemetry_quality: TelemetryQualityReport,
    /// Maximum database connections available to this allocation.
    #[serde(default)]
    pub database_max_connections: Option<u32>,
    /// Database connection slots reserved for administration or other workloads.
    #[serde(default)]
    pub reserved_connections: u32,
    /// Additional connection headroom held back as an operational safety margin.
    #[serde(default)]
    pub safety_margin_connections: u32,
    /// Number of application replicas that will each receive the recommended pool.
    #[serde(default = "default_replica_count")]
    pub replica_count: u32,
    /// Current total connections consumed by this service, when observed directly.
    #[serde(default)]
    pub current_total_connections: Option<u32>,
}

impl PoolScaleGateInput {
    /// Creates a gate input with one replica and no declared database budget.
    pub fn new(
        recommendation: TelemetryRecommendation,
        telemetry_quality: TelemetryQualityReport,
    ) -> Self {
        Self {
            recommendation,
            telemetry_quality,
            database_max_connections: None,
            reserved_connections: 0,
            safety_margin_connections: 0,
            replica_count: default_replica_count(),
            current_total_connections: None,
        }
    }

    /// Sets the database budget and connection slots excluded from application use.
    #[must_use]
    pub fn with_database_budget(
        mut self,
        max_connections: u32,
        reserved_connections: u32,
        safety_margin_connections: u32,
    ) -> Self {
        self.database_max_connections = Some(max_connections);
        self.reserved_connections = reserved_connections;
        self.safety_margin_connections = safety_margin_connections;
        self
    }

    /// Sets the number of application replicas receiving the pool.
    #[must_use]
    pub fn with_replica_count(mut self, replica_count: u32) -> Self {
        self.replica_count = replica_count;
        self
    }

    /// Sets the directly observed current connection total for this service.
    #[must_use]
    pub fn with_current_total_connections(mut self, current_total_connections: u32) -> Self {
        self.current_total_connections = Some(current_total_connections);
        self
    }
}

/// Result of checking whether a recommendation may increase a deployed pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PoolScaleGateReport {
    /// Final safety decision.
    pub status: PoolScaleGateStatus,
    /// Whether the recommendation asks for more connections per replica.
    pub scale_up_requested: bool,
    /// Current pool size per replica.
    pub current_pool_size: u32,
    /// Recommended pool size per replica.
    pub recommended_pool_size: u32,
    /// Additional connections per replica requested by the recommendation.
    pub additional_connections_per_replica: u32,
    /// Additional connections across all replicas.
    pub additional_connections_total: u32,
    /// Replica count used for the projection.
    pub replica_count: u32,
    /// Current service connection total, observed or inferred when an increase is checked.
    #[serde(default)]
    pub current_total_connections: Option<u32>,
    /// Projected service connection total after the increase, when calculable.
    #[serde(default)]
    pub projected_total_connections: Option<u32>,
    /// Effective application capacity after reserved slots and safety margin.
    #[serde(default)]
    pub effective_database_capacity: Option<u32>,
    /// Original telemetry-quality evidence used by the decision.
    pub telemetry_quality: TelemetryQualityReport,
    /// Explanations and remediations for the decision.
    pub findings: Vec<PoolScaleGateFinding>,
    /// Confidence in the combined decision evidence.
    pub confidence: EvidenceConfidence,
}

/// Checks whether increasing the recommended pool is supported by telemetry and budget evidence.
///
/// An increase is blocked when the telemetry is rejected or when the projected
/// service connection total exceeds effective database capacity. Missing budget
/// or directly observed totals produce `NeedsReview`, rather than an optimistic
/// automatic approval. Recommendations that keep or reduce the pool are allowed
/// by this scale-up-specific gate and still return the complete quality report.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when replica counts are zero, budget
/// reservations exceed the database maximum, or projected arithmetic overflows.
pub fn check_pool_scale_gate(
    input: &PoolScaleGateInput,
) -> Result<PoolScaleGateReport, PoolsimError> {
    if input.replica_count == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_REPLICA_COUNT",
            "replica_count must be greater than 0",
            None,
        ));
    }

    let recommendation = &input.recommendation.diff;
    let scale_up_requested = recommendation.change == PoolSizeChange::Increase;
    let additional_connections_total = recommendation
        .additional_connections_required
        .checked_mul(input.replica_count)
        .ok_or_else(|| {
            PoolsimError::invalid_input(
                "CONNECTION_PROJECTION_OVERFLOW",
                "additional connection projection exceeds u32 capacity",
                None,
            )
        })?;

    let effective_database_capacity = match input.database_max_connections {
        Some(max_connections) => {
            let excluded = input
                .reserved_connections
                .checked_add(input.safety_margin_connections)
                .ok_or_else(|| {
                    PoolsimError::invalid_input(
                        "INVALID_DATABASE_BUDGET",
                        "reserved connections and safety margin exceed u32 capacity",
                        None,
                    )
                })?;
            if excluded > max_connections {
                return Err(PoolsimError::invalid_input(
                    "INVALID_DATABASE_BUDGET",
                    "reserved connections and safety margin cannot exceed database_max_connections",
                    None,
                ));
            }
            Some(max_connections - excluded)
        }
        None => None,
    };

    let inferred_current_total = recommendation
        .current_pool_size
        .checked_mul(input.replica_count)
        .ok_or_else(|| {
            PoolsimError::invalid_input(
                "CONNECTION_PROJECTION_OVERFLOW",
                "current connection projection exceeds u32 capacity",
                None,
            )
        })?;
    let current_total_connections = if scale_up_requested {
        Some(
            input
                .current_total_connections
                .unwrap_or(inferred_current_total),
        )
    } else {
        input.current_total_connections
    };
    let projected_total_connections = if scale_up_requested {
        Some(
            current_total_connections
                .unwrap_or(0)
                .checked_add(additional_connections_total)
                .ok_or_else(|| {
                    PoolsimError::invalid_input(
                        "CONNECTION_PROJECTION_OVERFLOW",
                        "projected connection total exceeds u32 capacity",
                        None,
                    )
                })?,
        )
    } else {
        None
    };

    let mut findings = Vec::new();
    let mut status = PoolScaleGateStatus::Allowed;
    let mut confidence = input.telemetry_quality.confidence;

    if !scale_up_requested {
        findings.push(PoolScaleGateFinding::new(
            "NO_SCALE_UP_REQUESTED",
            RiskLevel::Low,
            "the recommendation keeps or reduces the current pool size",
            "apply the existing recommendation workflow; this gate only blocks unsafe increases",
        ));
    } else {
        match input.telemetry_quality.status {
            TelemetryQualityStatus::Rejected => {
                status = PoolScaleGateStatus::Blocked;
                findings.push(PoolScaleGateFinding::new(
                    "TELEMETRY_QUALITY_REJECTED",
                    RiskLevel::Critical,
                    "the telemetry evidence was rejected for capacity planning",
                    "collect a defensible open-loop capture or correct the reported quality risks before increasing the pool",
                ));
            }
            TelemetryQualityStatus::NeedsReview => {
                status = PoolScaleGateStatus::NeedsReview;
                findings.push(PoolScaleGateFinding::new(
                    "TELEMETRY_QUALITY_NEEDS_REVIEW",
                    RiskLevel::Medium,
                    "the telemetry has limitations that require human review before scaling up",
                    "review the telemetry-quality findings and repeat the capture if the limitation affects the decision",
                ));
            }
            TelemetryQualityStatus::Valid => {}
        }

        if input.database_max_connections.is_none() {
            status = max_status(status, PoolScaleGateStatus::NeedsReview);
            confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
            findings.push(PoolScaleGateFinding::new(
                "DATABASE_BUDGET_MISSING",
                RiskLevel::Medium,
                "no database connection budget was supplied for the requested increase",
                "provide max connections, reserved slots, and an operational safety margin before automating the change",
            ));
        }

        if input.current_total_connections.is_none() {
            confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
            if status == PoolScaleGateStatus::Allowed {
                status = PoolScaleGateStatus::NeedsReview;
            }
            findings.push(PoolScaleGateFinding::new(
                "CURRENT_CONNECTION_TOTAL_INFERRED",
                RiskLevel::Medium,
                "current service connections were inferred from pool size multiplied by replica count",
                "supply the observed service connection total to account for other pools and connection consumers",
            ));
        }

        if let (Some(capacity), Some(projected)) =
            (effective_database_capacity, projected_total_connections)
        {
            if projected > capacity {
                status = PoolScaleGateStatus::Blocked;
                findings.push(PoolScaleGateFinding::new(
                    "DATABASE_BUDGET_EXCEEDED",
                    RiskLevel::Critical,
                    "the projected service connection total exceeds effective database capacity",
                    "reduce the per-replica pool, reduce replicas, increase the approved database budget, or allocate capacity across services",
                ));
            }
        }
    }

    Ok(PoolScaleGateReport {
        status,
        scale_up_requested,
        current_pool_size: recommendation.current_pool_size,
        recommended_pool_size: recommendation.recommended_pool_size,
        additional_connections_per_replica: recommendation.additional_connections_required,
        additional_connections_total,
        replica_count: input.replica_count,
        current_total_connections,
        projected_total_connections,
        effective_database_capacity,
        telemetry_quality: input.telemetry_quality.clone(),
        findings,
        confidence,
    })
}

fn default_replica_count() -> u32 {
    1
}

fn lower_confidence(
    current: EvidenceConfidence,
    candidate: EvidenceConfidence,
) -> EvidenceConfidence {
    match (current, candidate) {
        (EvidenceConfidence::Low, _) | (_, EvidenceConfidence::Low) => EvidenceConfidence::Low,
        (EvidenceConfidence::Medium, _) | (_, EvidenceConfidence::Medium) => {
            EvidenceConfidence::Medium
        }
        _ => EvidenceConfidence::High,
    }
}

fn max_status(current: PoolScaleGateStatus, candidate: PoolScaleGateStatus) -> PoolScaleGateStatus {
    match (current, candidate) {
        (PoolScaleGateStatus::Blocked, _) | (_, PoolScaleGateStatus::Blocked) => {
            PoolScaleGateStatus::Blocked
        }
        (PoolScaleGateStatus::NeedsReview, _) | (_, PoolScaleGateStatus::NeedsReview) => {
            PoolScaleGateStatus::NeedsReview
        }
        _ => PoolScaleGateStatus::Allowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange, TelemetryRecommendation},
        telemetry_quality::{TelemetryArrivalModel, TelemetryQualityInput, TelemetryQualityStatus},
        types::{EvaluationResult, SaturationLevel, SimulationReport},
    };

    fn recommendation(change: PoolSizeChange) -> TelemetryRecommendation {
        let (current, recommended) = match change {
            PoolSizeChange::Increase => (4, 8),
            PoolSizeChange::Decrease => (8, 4),
            PoolSizeChange::Keep => (4, 4),
        };
        let current_evaluation = EvaluationResult {
            pool_size: current,
            utilisation_rho: 0.5,
            mean_queue_wait_ms: 1.0,
            p99_queue_wait_ms: 2.0,
            saturation: SaturationLevel::Ok,
            warnings: Vec::new(),
        };
        let recommended_report = SimulationReport {
            optimal_pool_size: recommended,
            confidence_interval: (recommended, recommended),
            cold_start_min_pool_size: recommended,
            utilisation_rho: 0.5,
            mean_queue_wait_ms: 1.0,
            p99_queue_wait_ms: 2.0,
            saturation: SaturationLevel::Ok,
            sensitivity: Vec::new(),
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        };
        TelemetryRecommendation {
            service_name: Some("checkout".to_string()),
            window: Some("15m".to_string()),
            observed_at: None,
            diff: PoolRecommendationDiff {
                current_pool_size: current,
                recommended_pool_size: recommended,
                pool_size_delta: i64::from(recommended) - i64::from(current),
                change,
                additional_connections_required: recommended.saturating_sub(current),
                removable_connections: current.saturating_sub(recommended),
                connection_change_percent: 100.0,
                current_evaluation,
                recommended_report,
            },
        }
    }

    fn quality(status: TelemetryQualityStatus) -> TelemetryQualityReport {
        let input = TelemetryQualityInput::new(TelemetryArrivalModel::OpenLoop)
            .with_expected_requests_per_second(100.0)
            .with_observed_requests_per_second(100.0)
            .with_duration_seconds(60.0)
            .with_sample_count(6_000)
            .with_timeout_count(0)
            .with_error_count(0)
            .with_latency_percentiles(5.0, 10.0, 20.0)
            .with_pool_wait_p99_ms(2.0)
            .with_database_latency_p99_ms(18.0);
        let mut report = crate::telemetry_quality::assess_telemetry_quality(&input)
            .expect("complete quality input should assess");
        report.status = status;
        report
    }

    #[test]
    fn blocks_rejected_telemetry_before_budget_check() {
        let input = PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Rejected),
        )
        .with_database_budget(100, 10, 10)
        .with_replica_count(2)
        .with_current_total_connections(8);
        let report =
            check_pool_scale_gate(&input).expect("rejected quality should produce a report");
        assert_eq!(report.status, PoolScaleGateStatus::Blocked);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "TELEMETRY_QUALITY_REJECTED"));
    }

    #[test]
    fn blocks_projected_budget_excess() {
        let input = PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Valid),
        )
        .with_database_budget(20, 2, 2)
        .with_replica_count(2)
        .with_current_total_connections(14);
        let report = check_pool_scale_gate(&input).expect("budget excess should produce a report");
        assert_eq!(report.projected_total_connections, Some(22));
        assert_eq!(report.effective_database_capacity, Some(16));
        assert_eq!(report.status, PoolScaleGateStatus::Blocked);
    }

    #[test]
    fn requests_review_when_budget_or_observed_total_is_missing() {
        let report = check_pool_scale_gate(&PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Valid),
        ))
        .expect("missing budget should produce a review report");
        assert_eq!(report.status, PoolScaleGateStatus::NeedsReview);
        assert_eq!(report.current_total_connections, Some(4));
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "DATABASE_BUDGET_MISSING"));
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "CURRENT_CONNECTION_TOTAL_INFERRED"));
    }

    #[test]
    fn allows_increase_with_complete_evidence() {
        let input = PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Valid),
        )
        .with_database_budget(50, 5, 5)
        .with_replica_count(2)
        .with_current_total_connections(8);
        let report = check_pool_scale_gate(&input).expect("complete scale evidence should pass");
        assert_eq!(report.status, PoolScaleGateStatus::Allowed);
        assert_eq!(report.additional_connections_total, 8);
        assert_eq!(report.projected_total_connections, Some(16));
        assert_eq!(report.confidence, EvidenceConfidence::High);
    }

    #[test]
    fn non_increase_is_allowed_by_scale_up_gate() {
        let report = check_pool_scale_gate(&PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Decrease),
            quality(TelemetryQualityStatus::Rejected),
        ))
        .expect("non-increase should produce an allowed report");
        assert_eq!(report.status, PoolScaleGateStatus::Allowed);
        assert!(!report.scale_up_requested);
        assert_eq!(report.projected_total_connections, None);
    }

    #[test]
    fn rejects_zero_replicas_and_invalid_budget() {
        let zero = PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Valid),
        )
        .with_replica_count(0);
        assert_eq!(
            check_pool_scale_gate(&zero)
                .expect_err("zero replicas should be rejected")
                .code(),
            "INVALID_REPLICA_COUNT"
        );

        let invalid = PoolScaleGateInput::new(
            recommendation(PoolSizeChange::Increase),
            quality(TelemetryQualityStatus::Valid),
        )
        .with_database_budget(10, 6, 5);
        assert_eq!(
            check_pool_scale_gate(&invalid)
                .expect_err("invalid budget should be rejected")
                .code(),
            "INVALID_DATABASE_BUDGET"
        );
    }
}
