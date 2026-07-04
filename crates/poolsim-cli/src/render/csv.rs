use crate::budget::BudgetPlanReport;
use crate::compare::ScenarioComparisonReport;
use crate::config_gen::ConfigSnippetReport;
use crate::doctor::DoctorReport;
use crate::gate::GateReport;
use crate::guard::GuardReport;
use anyhow::Result;
use csv::{Writer, WriterBuilder};
use poolsim_core::telemetry::TelemetryRecommendation;
use poolsim_core::types::{EvaluationResult, SensitivityRow, SimulationReport};

pub fn simulation(report: &SimulationReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());

    wtr.write_record(["summary_field", "value"])?;
    wtr.write_record(["optimal_pool_size", &report.optimal_pool_size.to_string()])?;
    wtr.write_record([
        "confidence_interval",
        &format!(
            "{}..{}",
            report.confidence_interval.0, report.confidence_interval.1
        ),
    ])?;
    wtr.write_record([
        "cold_start_min_pool_size",
        &report.cold_start_min_pool_size.to_string(),
    ])?;
    wtr.write_record(["utilisation_rho", &report.utilisation_rho.to_string()])?;
    wtr.write_record(["mean_queue_wait_ms", &report.mean_queue_wait_ms.to_string()])?;
    wtr.write_record(["p99_queue_wait_ms", &report.p99_queue_wait_ms.to_string()])?;
    wtr.write_record(["saturation", &format!("{:?}", report.saturation)])?;
    wtr.write_record(["", ""])?;

    wtr.write_record([
        "pool_size",
        "utilisation_rho",
        "mean_queue_wait_ms",
        "p99_queue_wait_ms",
        "risk",
    ])?;
    for row in &report.sensitivity {
        write_sensitivity_row(&mut wtr, row)?;
    }

    if !report.step_load_analysis.is_empty() {
        wtr.write_record(["", ""])?;
        wtr.write_record([
            "time_s",
            "requests_per_second",
            "utilisation_rho",
            "p99_queue_wait_ms",
            "saturation",
        ])?;
        for row in &report.step_load_analysis {
            wtr.write_record([
                row.time_s.to_string(),
                row.requests_per_second.to_string(),
                row.utilisation_rho.to_string(),
                row.p99_queue_wait_ms.to_string(),
                format!("{:?}", row.saturation),
            ])?;
        }
    }

    wtr.flush()?;
    Ok(())
}

pub fn evaluation(result: &EvaluationResult) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record([
        "pool_size",
        "utilisation_rho",
        "mean_queue_wait_ms",
        "p99_queue_wait_ms",
        "saturation",
    ])?;
    wtr.write_record([
        result.pool_size.to_string(),
        result.utilisation_rho.to_string(),
        result.mean_queue_wait_ms.to_string(),
        result.p99_queue_wait_ms.to_string(),
        format!("{:?}", result.saturation),
    ])?;
    wtr.flush()?;
    Ok(())
}

pub fn sweep(rows: &[SensitivityRow]) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record([
        "pool_size",
        "utilisation_rho",
        "mean_queue_wait_ms",
        "p99_queue_wait_ms",
        "risk",
    ])?;

    for row in rows {
        write_sensitivity_row(&mut wtr, row)?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn batch(reports: &[SimulationReport]) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record([
        "request_index",
        "optimal_pool_size",
        "utilisation_rho",
        "mean_queue_wait_ms",
        "p99_queue_wait_ms",
        "saturation",
    ])?;

    for (idx, report) in reports.iter().enumerate() {
        wtr.write_record([
            idx.to_string(),
            report.optimal_pool_size.to_string(),
            report.utilisation_rho.to_string(),
            report.mean_queue_wait_ms.to_string(),
            report.p99_queue_wait_ms.to_string(),
            format!("{:?}", report.saturation),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn compare(report: &ScenarioComparisonReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record([
        "scenario",
        "baseline",
        "requests_per_second",
        "optimal_pool_size",
        "pool_size_delta",
        "p99_queue_wait_ms",
        "p99_queue_wait_delta_ms",
        "mean_queue_wait_ms",
        "mean_queue_wait_delta_ms",
        "utilisation_rho",
        "utilisation_rho_delta",
        "saturation",
    ])?;

    for row in &report.rows {
        wtr.write_record([
            row.name.clone(),
            row.is_baseline.to_string(),
            row.requests_per_second.to_string(),
            row.optimal_pool_size.to_string(),
            row.pool_size_delta.to_string(),
            row.p99_queue_wait_ms.to_string(),
            row.p99_queue_wait_delta_ms.to_string(),
            row.mean_queue_wait_ms.to_string(),
            row.mean_queue_wait_delta_ms.to_string(),
            row.utilisation_rho.to_string(),
            row.utilisation_rho_delta.to_string(),
            format!("{:?}", row.saturation),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn budget(report: &BudgetPlanReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record(["status", &format!("{:?}", report.status)])?;
    wtr.write_record(["max_connections", &report.max_connections.to_string()])?;
    wtr.write_record([
        "reserved_connections",
        &report.reserved_connections.to_string(),
    ])?;
    wtr.write_record([
        "safety_margin_connections",
        &report.safety_margin_connections.to_string(),
    ])?;
    wtr.write_record([
        "available_connections",
        &report.available_connections.to_string(),
    ])?;
    wtr.write_record([
        "current_total_connections",
        &report
            .current_total_connections
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
    ])?;
    wtr.write_record([
        "requested_total_connections",
        &report.requested_total_connections.to_string(),
    ])?;
    wtr.write_record([
        "min_required_connections",
        &report.min_required_connections.to_string(),
    ])?;
    wtr.write_record([
        "allocated_total_connections",
        &report.allocated_total_connections.to_string(),
    ])?;
    wtr.write_record(["unused_connections", &report.unused_connections.to_string()])?;
    wtr.write_record([
        "over_budget_connections",
        &report.over_budget_connections.to_string(),
    ])?;
    wtr.write_record(["", ""])?;
    wtr.write_record([
        "service",
        "replicas",
        "priority",
        "current_pool_size",
        "min_pool_size",
        "max_pool_size",
        "recommended_pool_size",
        "desired_pool_size",
        "allocated_pool_size",
        "current_total_connections",
        "requested_total_connections",
        "allocated_total_connections",
        "pool_size_delta_from_current",
        "reduction_from_recommended",
        "capped_by_service_max",
        "meets_minimum",
    ])?;
    for service in &report.services {
        wtr.write_record([
            service.name.clone(),
            service.replicas.to_string(),
            service.priority.to_string(),
            optional_u32(service.current_pool_size),
            service.min_pool_size.to_string(),
            optional_u32(service.max_pool_size),
            service.recommended_pool_size.to_string(),
            service.desired_pool_size.to_string(),
            service.allocated_pool_size.to_string(),
            optional_u32(service.current_total_connections),
            service.requested_total_connections.to_string(),
            service.allocated_total_connections.to_string(),
            service
                .pool_size_delta_from_current
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            service.reduction_from_recommended.to_string(),
            service.capped_by_service_max.to_string(),
            service.meets_minimum.to_string(),
        ])?;
    }
    for warning in &report.warnings {
        wtr.write_record(["warning".to_string(), warning.clone()])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn telemetry(recommendation: &TelemetryRecommendation) -> Result<()> {
    let diff = &recommendation.diff;
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record([
        "service_name",
        recommendation.service_name.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["window", recommendation.window.as_deref().unwrap_or("-")])?;
    wtr.write_record([
        "observed_at",
        recommendation.observed_at.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["current_pool_size", &diff.current_pool_size.to_string()])?;
    wtr.write_record([
        "recommended_pool_size",
        &diff.recommended_pool_size.to_string(),
    ])?;
    wtr.write_record(["pool_size_delta", &diff.pool_size_delta.to_string()])?;
    wtr.write_record(["change", &format!("{:?}", diff.change)])?;
    wtr.write_record([
        "additional_connections_required",
        &diff.additional_connections_required.to_string(),
    ])?;
    wtr.write_record([
        "removable_connections",
        &diff.removable_connections.to_string(),
    ])?;
    wtr.write_record([
        "connection_change_percent",
        &diff.connection_change_percent.to_string(),
    ])?;
    wtr.write_record([
        "current_saturation",
        &format!("{:?}", diff.current_evaluation.saturation),
    ])?;
    wtr.write_record([
        "recommended_saturation",
        &format!("{:?}", diff.recommended_report.saturation),
    ])?;
    wtr.flush()?;
    Ok(())
}

pub fn gate(report: &GateReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record(["status", &format!("{:?}", report.status)])?;
    wtr.write_record([
        "service_name",
        report.service_name.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["window", report.window.as_deref().unwrap_or("-")])?;
    wtr.write_record(["observed_at", report.observed_at.as_deref().unwrap_or("-")])?;
    wtr.write_record([
        "worst_saturation",
        &format!("{:?}", report.worst_saturation),
    ])?;
    wtr.write_record(["", ""])?;
    wtr.write_record([
        "check",
        "passed",
        "severity",
        "observed",
        "threshold",
        "message",
    ])?;
    for check in &report.checks {
        wtr.write_record([
            check.name.clone(),
            check.passed.to_string(),
            format!("{:?}", check.severity),
            check.observed.clone(),
            check.threshold.clone(),
            check.message.clone(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn guard(report: &GuardReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record(["status", &format!("{:?}", report.status)])?;
    wtr.write_record(["deployment_safe", &report.deployment_safe.to_string()])?;
    wtr.write_record(["exit_code", &report.exit_code.to_string()])?;
    wtr.write_record(["reason", &report.reason])?;
    wtr.write_record([
        "service_name",
        report.gate.service_name.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["window", report.gate.window.as_deref().unwrap_or("-")])?;
    wtr.write_record([
        "observed_at",
        report.gate.observed_at.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record([
        "worst_saturation",
        &format!("{:?}", report.gate.worst_saturation),
    ])?;
    wtr.write_record(["", ""])?;
    wtr.write_record([
        "check",
        "passed",
        "severity",
        "observed",
        "threshold",
        "message",
    ])?;
    for check in &report.gate.checks {
        wtr.write_record([
            check.name.clone(),
            check.passed.to_string(),
            format!("{:?}", check.severity),
            check.observed.clone(),
            check.threshold.clone(),
            check.message.clone(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn doctor(report: &DoctorReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record(["status", &format!("{:?}", report.status)])?;
    wtr.write_record([
        "service_name",
        report.service_name.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["window", report.window.as_deref().unwrap_or("-")])?;
    wtr.write_record(["observed_at", report.observed_at.as_deref().unwrap_or("-")])?;
    wtr.write_record(["current_pool_size", &report.current_pool_size.to_string()])?;
    wtr.write_record([
        "recommended_pool_size",
        &report.recommended_pool_size.to_string(),
    ])?;
    wtr.write_record(["pool_size_delta", &report.pool_size_delta.to_string()])?;
    wtr.write_record(["current_rho", &report.current_rho.to_string()])?;
    wtr.write_record([
        "current_p99_queue_wait_ms",
        &report.current_p99_queue_wait_ms.to_string(),
    ])?;
    wtr.write_record([
        "current_saturation",
        &format!("{:?}", report.current_saturation),
    ])?;
    wtr.write_record([
        "recommended_saturation",
        &format!("{:?}", report.recommended_saturation),
    ])?;
    wtr.write_record(["", ""])?;
    wtr.write_record(["finding", "severity", "message", "action"])?;
    for finding in &report.findings {
        wtr.write_record([
            finding.name.clone(),
            format!("{:?}", finding.severity),
            finding.message.clone(),
            finding.action.clone(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn config_snippet(report: &ConfigSnippetReport) -> Result<()> {
    let mut wtr = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());
    wtr.write_record(["field", "value"])?;
    wtr.write_record(["framework", report.framework.as_str()])?;
    wtr.write_record(["source", report.source.as_str()])?;
    wtr.write_record([
        "service_name",
        report.service_name.as_deref().unwrap_or("-"),
    ])?;
    wtr.write_record(["window", report.window.as_deref().unwrap_or("-")])?;
    wtr.write_record(["observed_at", report.observed_at.as_deref().unwrap_or("-")])?;
    wtr.write_record([
        "recommended_pool_size",
        &report.recommended_pool_size.to_string(),
    ])?;
    wtr.write_record(["min_idle", &report.min_idle.to_string()])?;
    wtr.write_record([
        "connection_timeout_ms",
        &report.connection_timeout_ms.to_string(),
    ])?;
    wtr.write_record(["idle_timeout_ms", &report.idle_timeout_ms.to_string()])?;
    wtr.write_record(["database_url_env", &report.database_url_env])?;
    wtr.write_record(["pool_name", &report.pool_name])?;
    wtr.write_record([
        "max_server_connections",
        &report.max_server_connections.to_string(),
    ])?;
    wtr.write_record(["utilisation_rho", &report.utilisation_rho.to_string()])?;
    wtr.write_record(["mean_queue_wait_ms", &report.mean_queue_wait_ms.to_string()])?;
    wtr.write_record(["p99_queue_wait_ms", &report.p99_queue_wait_ms.to_string()])?;
    wtr.write_record(["snippet", &report.snippet])?;
    for note in &report.notes {
        wtr.write_record(["note", note])?;
    }
    for reference in &report.references {
        wtr.write_record([
            "reference",
            &format!("{}: {}", reference.title, reference.url),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn write_sensitivity_row(wtr: &mut Writer<std::io::Stdout>, row: &SensitivityRow) -> Result<()> {
    wtr.write_record([
        row.pool_size.to_string(),
        row.utilisation_rho.to_string(),
        row.mean_queue_wait_ms.to_string(),
        row.p99_queue_wait_ms.to_string(),
        format!("{:?}", row.risk),
    ])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use poolsim_core::telemetry::{PoolRecommendationDiff, PoolSizeChange};
    use poolsim_core::types::{
        EvaluationResult, PoolConfig, RiskLevel, SaturationLevel, SensitivityRow,
        SimulationOptions, SimulationReport, StepLoadResult, WorkloadConfig,
    };

    use super::*;

    fn sample_rows() -> Vec<SensitivityRow> {
        vec![
            SensitivityRow {
                pool_size: 4,
                utilisation_rho: 0.72,
                mean_queue_wait_ms: 5.0,
                p99_queue_wait_ms: 20.0,
                risk: RiskLevel::Low,
            },
            SensitivityRow {
                pool_size: 5,
                utilisation_rho: 0.89,
                mean_queue_wait_ms: 7.5,
                p99_queue_wait_ms: 35.0,
                risk: RiskLevel::High,
            },
        ]
    }

    fn sample_report() -> SimulationReport {
        SimulationReport {
            optimal_pool_size: 5,
            confidence_interval: (4, 6),
            cold_start_min_pool_size: 4,
            utilisation_rho: 0.81,
            mean_queue_wait_ms: 6.2,
            p99_queue_wait_ms: 32.0,
            saturation: SaturationLevel::Warning,
            sensitivity: sample_rows(),
            step_load_analysis: vec![StepLoadResult {
                time_s: 0,
                requests_per_second: 180.0,
                utilisation_rho: 0.8,
                p99_queue_wait_ms: 30.0,
                saturation: SaturationLevel::Warning,
            }],
            warnings: vec!["test warning".to_string()],
        }
    }

    fn sample_evaluation() -> EvaluationResult {
        EvaluationResult {
            pool_size: 6,
            utilisation_rho: 0.75,
            mean_queue_wait_ms: 4.0,
            p99_queue_wait_ms: 18.0,
            saturation: SaturationLevel::Ok,
            warnings: Vec::new(),
        }
    }

    fn sample_recommendation() -> TelemetryRecommendation {
        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: None,
            diff: PoolRecommendationDiff {
                current_pool_size: 6,
                recommended_pool_size: 8,
                pool_size_delta: 2,
                change: PoolSizeChange::Increase,
                additional_connections_required: 2,
                removable_connections: 0,
                connection_change_percent: 33.333,
                current_evaluation: sample_evaluation(),
                recommended_report: sample_report(),
            },
        }
    }

    fn sample_budget_report() -> crate::budget::BudgetPlanReport {
        crate::budget::build_budget_plan_report(crate::config::BudgetPlanInput {
            max_connections: 120,
            reserved_connections: 20,
            safety_margin_connections: 10,
            services: vec![
                crate::config::BudgetServiceInput {
                    name: "checkout-api".to_string(),
                    replicas: 6,
                    current_pool_size: Some(8),
                    min_pool_size: 4,
                    max_pool_size: Some(12),
                    recommended_pool_size: 10,
                    priority: Some(5),
                },
                crate::config::BudgetServiceInput {
                    name: "billing-api".to_string(),
                    replicas: 4,
                    current_pool_size: Some(6),
                    min_pool_size: 3,
                    max_pool_size: Some(10),
                    recommended_pool_size: 8,
                    priority: Some(3),
                },
            ],
        })
        .expect("sample budget report should build")
    }

    fn sample_scenario(name: &str, rps: f64) -> crate::config::ScenarioInput {
        crate::config::ScenarioInput {
            name: name.to_string(),
            workload: WorkloadConfig {
                requests_per_second: rps,
                latency_p50_ms: 8.0,
                latency_p95_ms: 30.0,
                latency_p99_ms: 70.0,
                raw_samples_ms: None,
                step_load_profile: None,
            },
            pool: PoolConfig {
                max_server_connections: 100,
                connection_overhead_ms: 2.0,
                idle_timeout_ms: None,
                min_pool_size: 2,
                max_pool_size: 20,
            },
            options: SimulationOptions {
                iterations: 1_200,
                seed: Some(5),
                ..SimulationOptions::default()
            },
        }
    }

    #[test]
    fn csv_renderers_execute_for_all_output_types() {
        simulation(&sample_report()).expect("simulation CSV render should succeed");
        evaluation(&sample_evaluation()).expect("evaluation CSV render should succeed");
        sweep(&sample_rows()).expect("sweep CSV render should succeed");
        batch(&[sample_report(), sample_report()]).expect("batch CSV render should succeed");
        compare(
            &crate::compare::build_scenario_comparison_report(
                crate::config::ScenarioComparisonInput {
                    baseline: "normal".to_string(),
                    scenarios: vec![
                        sample_scenario("normal", 180.0),
                        sample_scenario("peak", 260.0),
                    ],
                },
            )
            .expect("comparison report should build"),
        )
        .expect("compare CSV render should succeed");
        budget(&sample_budget_report()).expect("budget CSV render should succeed");
        telemetry(&sample_recommendation()).expect("telemetry CSV render should succeed");
        gate(&crate::gate::build_gate_report(
            sample_recommendation(),
            &crate::gate::GatePolicy::default(),
        ))
        .expect("gate CSV render should succeed");
        guard(&crate::guard::build_guard_report(
            crate::gate::build_gate_report(
                sample_recommendation(),
                &crate::gate::GatePolicy::default(),
            ),
        ))
        .expect("guard CSV render should succeed");
        doctor(&crate::doctor::build_doctor_report(sample_recommendation()))
            .expect("doctor CSV render should succeed");

        let config_report = crate::config_gen::build_config_snippet(
            &crate::args::GenerateConfigArgs {
                framework: crate::args::CliConfigFramework::NodePg,
                min_idle: Some(3),
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: crate::args::GenerateConfigSourceCommands::Simulate(
                    crate::args::CommonArgs {
                        config: None,
                        rps: None,
                        p50: None,
                        p95: None,
                        p99: None,
                        samples_file: None,
                        max_server_connections: None,
                        connection_overhead_ms: None,
                        connection_profile: None,
                        idle_timeout_ms: None,
                        min: None,
                        max: None,
                        iterations: None,
                        seed: None,
                        distribution: None,
                        queue_model: None,
                        target_wait_p99_ms: None,
                        max_acceptable_rho: None,
                        explain: false,
                    },
                ),
            },
            crate::config_gen::ConfigRecommendation {
                source: crate::config_gen::ConfigSourceKind::Simulate,
                service_name: Some("checkout-api".to_string()),
                window: Some("1h".to_string()),
                observed_at: None,
                recommended_pool_size: 8,
                cold_start_min_pool_size: 4,
                max_server_connections: 100,
                utilisation_rho: 0.72,
                mean_queue_wait_ms: 3.0,
                p99_queue_wait_ms: 12.0,
            },
        );
        config_snippet(&config_report).expect("config snippet CSV render should succeed");
    }
}
