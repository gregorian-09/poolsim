use std::process::ExitCode;

use poolsim_core::{telemetry::TelemetryRecommendation, types::SaturationLevel};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum DoctorStatus {
    Healthy,
    TooSmall,
    TooLarge,
    CloseToSaturation,
    Saturated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum DoctorSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DoctorFinding {
    pub name: String,
    pub severity: DoctorSeverity,
    pub message: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DoctorReport {
    pub status: DoctorStatus,
    pub service_name: Option<String>,
    pub window: Option<String>,
    pub observed_at: Option<String>,
    pub current_pool_size: u32,
    pub recommended_pool_size: u32,
    pub pool_size_delta: i64,
    pub current_rho: f64,
    pub current_p99_queue_wait_ms: f64,
    pub current_saturation: SaturationLevel,
    pub recommended_saturation: SaturationLevel,
    pub findings: Vec<DoctorFinding>,
    pub recommendation: TelemetryRecommendation,
}

impl DoctorStatus {
    pub(crate) fn exit_code(self, warn_exit: bool) -> ExitCode {
        match self {
            Self::Saturated => ExitCode::from(2),
            Self::TooSmall | Self::CloseToSaturation if warn_exit => ExitCode::from(3),
            Self::Healthy | Self::TooLarge | Self::TooSmall | Self::CloseToSaturation => {
                ExitCode::from(0)
            }
        }
    }
}

pub(crate) fn build_doctor_report(recommendation: TelemetryRecommendation) -> DoctorReport {
    let diff = &recommendation.diff;
    let status = diagnose_status(&recommendation);
    let findings = build_findings(&recommendation, status);

    DoctorReport {
        status,
        service_name: recommendation.service_name.clone(),
        window: recommendation.window.clone(),
        observed_at: recommendation.observed_at.clone(),
        current_pool_size: diff.current_pool_size,
        recommended_pool_size: diff.recommended_pool_size,
        pool_size_delta: diff.pool_size_delta,
        current_rho: diff.current_evaluation.utilisation_rho,
        current_p99_queue_wait_ms: diff.current_evaluation.p99_queue_wait_ms,
        current_saturation: diff.current_evaluation.saturation,
        recommended_saturation: diff.recommended_report.saturation,
        findings,
        recommendation,
    }
}

fn diagnose_status(recommendation: &TelemetryRecommendation) -> DoctorStatus {
    let diff = &recommendation.diff;

    if diff.current_evaluation.saturation == SaturationLevel::Critical
        || diff.recommended_report.saturation == SaturationLevel::Critical
    {
        return DoctorStatus::Saturated;
    }

    if diff.recommended_pool_size > diff.current_pool_size {
        return DoctorStatus::TooSmall;
    }

    if diff.current_evaluation.saturation == SaturationLevel::Warning {
        return DoctorStatus::CloseToSaturation;
    }

    if diff.recommended_pool_size < diff.current_pool_size {
        return DoctorStatus::TooLarge;
    }

    DoctorStatus::Healthy
}

fn build_findings(
    recommendation: &TelemetryRecommendation,
    status: DoctorStatus,
) -> Vec<DoctorFinding> {
    let diff = &recommendation.diff;
    let mut findings = Vec::new();

    findings.push(DoctorFinding {
        name: "current_pool".to_string(),
        severity: status_severity(status),
        message: status_message(status, recommendation),
        action: status_action(status, recommendation),
    });

    if diff.pool_size_delta != 0 {
        findings.push(DoctorFinding {
            name: "recommendation_diff".to_string(),
            severity: diff_severity(status),
            message: format!(
                "current pool size is {}; recommended pool size is {} ({:+})",
                diff.current_pool_size, diff.recommended_pool_size, diff.pool_size_delta
            ),
            action: if diff.pool_size_delta > 0 {
                format!(
                    "increase the configured pool by {} connection(s)",
                    diff.additional_connections_required
                )
            } else {
                format!(
                    "consider removing {} idle connection slot(s) after validating peak traffic",
                    diff.removable_connections
                )
            },
        });
    }

    if diff.current_evaluation.p99_queue_wait_ms > diff.recommended_report.p99_queue_wait_ms {
        findings.push(DoctorFinding {
            name: "queue_wait".to_string(),
            severity: DoctorSeverity::Info,
            message: format!(
                "recommended pool lowers p99 queue wait from {:.3}ms to {:.3}ms",
                diff.current_evaluation.p99_queue_wait_ms,
                diff.recommended_report.p99_queue_wait_ms
            ),
            action: "review the recommendation before applying it to production".to_string(),
        });
    }

    findings
}

fn status_severity(status: DoctorStatus) -> DoctorSeverity {
    match status {
        DoctorStatus::Saturated => DoctorSeverity::Critical,
        DoctorStatus::TooSmall | DoctorStatus::CloseToSaturation => DoctorSeverity::Warning,
        DoctorStatus::Healthy | DoctorStatus::TooLarge => DoctorSeverity::Info,
    }
}

fn diff_severity(status: DoctorStatus) -> DoctorSeverity {
    match status {
        DoctorStatus::Saturated => DoctorSeverity::Critical,
        DoctorStatus::TooSmall => DoctorSeverity::Warning,
        DoctorStatus::Healthy | DoctorStatus::TooLarge | DoctorStatus::CloseToSaturation => {
            DoctorSeverity::Info
        }
    }
}

fn status_message(status: DoctorStatus, recommendation: &TelemetryRecommendation) -> String {
    let diff = &recommendation.diff;
    match status {
        DoctorStatus::Healthy => format!(
            "current pool size {} matches the recommendation and is not saturated",
            diff.current_pool_size
        ),
        DoctorStatus::TooSmall => format!(
            "current pool size {} is below the recommended size {}",
            diff.current_pool_size, diff.recommended_pool_size
        ),
        DoctorStatus::TooLarge => format!(
            "current pool size {} is above the recommended size {}",
            diff.current_pool_size, diff.recommended_pool_size
        ),
        DoctorStatus::CloseToSaturation => format!(
            "current pool size {} is close to saturation (rho={:.3})",
            diff.current_pool_size, diff.current_evaluation.utilisation_rho
        ),
        DoctorStatus::Saturated => format!(
            "current or recommended pool is critically saturated (current={:?}, recommended={:?})",
            diff.current_evaluation.saturation, diff.recommended_report.saturation
        ),
    }
}

fn status_action(status: DoctorStatus, recommendation: &TelemetryRecommendation) -> String {
    let diff = &recommendation.diff;
    match status {
        DoctorStatus::Healthy => {
            "keep the current pool setting and continue monitoring".to_string()
        }
        DoctorStatus::TooSmall => format!(
            "raise the pool toward {} and verify the database connection budget first",
            diff.recommended_pool_size
        ),
        DoctorStatus::TooLarge => format!(
            "lower the pool toward {} if peak and incident traffic are represented",
            diff.recommended_pool_size
        ),
        DoctorStatus::CloseToSaturation => {
            "treat the pool as fragile; compare peak and incident scenarios before deployment"
                .to_string()
        }
        DoctorStatus::Saturated => {
            "increase capacity or reduce concurrency before deploying this workload".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use poolsim_core::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange},
        types::{EvaluationResult, SimulationReport},
    };

    use super::*;

    fn evaluation(
        pool_size: u32,
        rho: f64,
        p99: f64,
        saturation: SaturationLevel,
    ) -> EvaluationResult {
        EvaluationResult {
            pool_size,
            utilisation_rho: rho,
            mean_queue_wait_ms: 3.0,
            p99_queue_wait_ms: p99,
            saturation,
            warnings: Vec::new(),
        }
    }

    fn report(pool_size: u32, rho: f64, p99: f64, saturation: SaturationLevel) -> SimulationReport {
        SimulationReport {
            optimal_pool_size: pool_size,
            confidence_interval: (pool_size, pool_size),
            cold_start_min_pool_size: pool_size,
            utilisation_rho: rho,
            mean_queue_wait_ms: 2.0,
            p99_queue_wait_ms: p99,
            saturation,
            sensitivity: Vec::new(),
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn recommendation(
        current: u32,
        recommended: u32,
        current_saturation: SaturationLevel,
        recommended_saturation: SaturationLevel,
    ) -> TelemetryRecommendation {
        let delta = i64::from(recommended) - i64::from(current);
        let change = match delta.cmp(&0) {
            std::cmp::Ordering::Greater => PoolSizeChange::Increase,
            std::cmp::Ordering::Less => PoolSizeChange::Decrease,
            std::cmp::Ordering::Equal => PoolSizeChange::Keep,
        };

        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: Some("2026-05-16T00:00:00Z".to_string()),
            diff: PoolRecommendationDiff {
                current_pool_size: current,
                recommended_pool_size: recommended,
                pool_size_delta: delta,
                change,
                additional_connections_required: recommended.saturating_sub(current),
                removable_connections: current.saturating_sub(recommended),
                connection_change_percent: ((recommended as f64 - current as f64) / current as f64)
                    * 100.0,
                current_evaluation: evaluation(current, 0.88, 40.0, current_saturation),
                recommended_report: report(recommended, 0.72, 20.0, recommended_saturation),
            },
        }
    }

    #[test]
    fn doctor_classifies_healthy_too_small_too_large_and_close_to_saturation() {
        let healthy = build_doctor_report(recommendation(
            8,
            8,
            SaturationLevel::Ok,
            SaturationLevel::Ok,
        ));
        assert_eq!(healthy.status, DoctorStatus::Healthy);
        assert_eq!(healthy.findings[0].severity, DoctorSeverity::Info);
        let _ = healthy.status.exit_code(false);

        let too_small = build_doctor_report(recommendation(
            8,
            10,
            SaturationLevel::Ok,
            SaturationLevel::Ok,
        ));
        assert_eq!(too_small.status, DoctorStatus::TooSmall);
        assert!(too_small
            .findings
            .iter()
            .any(|finding| finding.name == "recommendation_diff"));
        let _ = too_small.status.exit_code(true);

        let too_large = build_doctor_report(recommendation(
            10,
            8,
            SaturationLevel::Ok,
            SaturationLevel::Ok,
        ));
        assert_eq!(too_large.status, DoctorStatus::TooLarge);
        assert!(too_large.findings[0].action.contains("lower"));

        let close = build_doctor_report(recommendation(
            8,
            8,
            SaturationLevel::Warning,
            SaturationLevel::Ok,
        ));
        assert_eq!(close.status, DoctorStatus::CloseToSaturation);
        assert!(close.findings[0].message.contains("close to saturation"));
    }

    #[test]
    fn doctor_classifies_critical_saturation_and_queue_wait_improvement() {
        let saturated = build_doctor_report(recommendation(
            8,
            12,
            SaturationLevel::Critical,
            SaturationLevel::Warning,
        ));
        assert_eq!(saturated.status, DoctorStatus::Saturated);
        assert_eq!(saturated.findings[0].severity, DoctorSeverity::Critical);
        assert!(saturated
            .findings
            .iter()
            .any(|finding| finding.name == "queue_wait"));
        let _ = saturated.status.exit_code(true);
    }
}
