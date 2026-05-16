use std::process::ExitCode;

use serde::Serialize;

use crate::gate::{GateDecision, GateReport};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct GuardReport {
    pub status: GateDecision,
    pub deployment_safe: bool,
    pub exit_code: u8,
    pub reason: String,
    pub gate: GateReport,
}

impl GuardReport {
    pub(crate) fn exit_code(&self) -> ExitCode {
        self.status.exit_code()
    }
}

pub(crate) fn build_guard_report(gate: GateReport) -> GuardReport {
    let status = gate.status;
    GuardReport {
        status,
        deployment_safe: status == GateDecision::Pass,
        exit_code: exit_code_value(status),
        reason: reason(status),
        gate,
    }
}

fn exit_code_value(status: GateDecision) -> u8 {
    match status {
        GateDecision::Pass => 0,
        GateDecision::Warning => 1,
        GateDecision::Critical => 2,
    }
}

fn reason(status: GateDecision) -> String {
    match status {
        GateDecision::Pass => "deployment is within pool safety policy".to_string(),
        GateDecision::Warning => {
            "deployment has advisory pool safety warnings; review before continuing".to_string()
        }
        GateDecision::Critical => "deployment violates pool safety policy".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use poolsim_core::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange, TelemetryRecommendation},
        types::{EvaluationResult, SaturationLevel, SimulationReport},
    };

    use crate::gate::{build_gate_report, GatePolicy};

    use super::*;

    fn recommendation() -> TelemetryRecommendation {
        let evaluation = EvaluationResult {
            pool_size: 8,
            utilisation_rho: 0.90,
            mean_queue_wait_ms: 6.0,
            p99_queue_wait_ms: 60.0,
            saturation: SaturationLevel::Warning,
            warnings: Vec::new(),
        };
        let report = SimulationReport {
            optimal_pool_size: 10,
            confidence_interval: (9, 11),
            cold_start_min_pool_size: 9,
            utilisation_rho: 0.72,
            mean_queue_wait_ms: 3.0,
            p99_queue_wait_ms: 20.0,
            saturation: SaturationLevel::Ok,
            sensitivity: Vec::new(),
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        };
        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: None,
            diff: PoolRecommendationDiff {
                current_pool_size: 8,
                recommended_pool_size: 10,
                pool_size_delta: 2,
                change: PoolSizeChange::Increase,
                additional_connections_required: 2,
                removable_connections: 0,
                connection_change_percent: 25.0,
                current_evaluation: evaluation,
                recommended_report: report,
            },
        }
    }

    #[test]
    fn guard_report_maps_gate_status_to_deployment_fields() {
        let pass = build_guard_report(build_gate_report(recommendation(), &GatePolicy::default()));
        assert_eq!(pass.status, GateDecision::Pass);
        assert!(pass.deployment_safe);
        assert_eq!(pass.exit_code, 0);
        let _ = pass.exit_code();

        let warning_policy = GatePolicy {
            max_saturation: SaturationLevel::Ok,
            ..GatePolicy::default()
        };
        let warning = build_guard_report(build_gate_report(recommendation(), &warning_policy));
        assert_eq!(warning.status, GateDecision::Warning);
        assert!(!warning.deployment_safe);
        assert_eq!(warning.exit_code, 1);

        let critical_policy = GatePolicy {
            max_current_rho: Some(0.1),
            ..GatePolicy::default()
        };
        let critical = build_guard_report(build_gate_report(recommendation(), &critical_policy));
        assert_eq!(critical.status, GateDecision::Critical);
        assert!(!critical.deployment_safe);
        assert_eq!(critical.exit_code, 2);
        assert!(critical.reason.contains("violates"));
    }

    #[test]
    fn helper_outputs_are_stable() {
        assert_eq!(exit_code_value(GateDecision::Pass), 0);
        assert_eq!(exit_code_value(GateDecision::Warning), 1);
        assert_eq!(exit_code_value(GateDecision::Critical), 2);
        assert!(reason(GateDecision::Pass).contains("within"));
        assert!(reason(GateDecision::Warning).contains("warnings"));
        assert!(reason(GateDecision::Critical).contains("violates"));
    }
}
