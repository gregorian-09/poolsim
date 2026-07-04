use std::io::IsTerminal;

use crate::budget::BudgetPlanReport;
use crate::compare::ScenarioComparisonReport;
use crate::config_gen::ConfigSnippetReport;
use crate::doctor::DoctorReport;
use crate::gate::GateReport;
use crate::guard::GuardReport;
use anyhow::Result;
use poolsim_core::telemetry::TelemetryRecommendation;
use poolsim_core::types::{
    EvaluationResult, RiskLevel, SaturationLevel, SensitivityRow, SimulationReport,
};
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct SummaryRow {
    metric: String,
    value: String,
}

#[derive(Tabled)]
struct SensitivityTableRow {
    pool_size: String,
    utilisation_rho: String,
    mean_queue_wait_ms: String,
    p99_queue_wait_ms: String,
    risk: String,
}

#[derive(Tabled)]
struct BatchTableRow {
    request_index: usize,
    optimal_pool_size: u32,
    utilisation_rho: String,
    p99_queue_wait_ms: String,
    saturation: String,
}

#[derive(Tabled)]
struct ScenarioComparisonTableRow {
    scenario: String,
    baseline: String,
    requests_per_second: String,
    optimal_pool_size: u32,
    pool_size_delta: String,
    p99_queue_wait_ms: String,
    p99_queue_wait_delta_ms: String,
    utilisation_rho: String,
    saturation: String,
}

#[derive(Tabled)]
struct BudgetServiceTableRow {
    service: String,
    replicas: u32,
    priority: u32,
    current: String,
    recommended: u32,
    allocated: u32,
    total_connections: u32,
    delta: String,
    reduction: u32,
    minimum_ok: String,
}

#[derive(Tabled)]
struct StepLoadTableRow {
    time_s: u32,
    requests_per_second: String,
    utilisation_rho: String,
    p99_queue_wait_ms: String,
    saturation: String,
}

#[derive(Tabled)]
struct GateCheckTableRow {
    check: String,
    passed: String,
    severity: String,
    observed: String,
    threshold: String,
    message: String,
}

#[derive(Tabled)]
struct DoctorFindingTableRow {
    finding: String,
    severity: String,
    message: String,
    action: String,
}

pub fn simulation(report: &SimulationReport) -> Result<()> {
    let use_color = std::io::stdout().is_terminal();
    let summary = vec![
        SummaryRow {
            metric: "optimal_pool_size".to_string(),
            value: report.optimal_pool_size.to_string(),
        },
        SummaryRow {
            metric: "confidence_interval".to_string(),
            value: format!(
                "{}..{}",
                report.confidence_interval.0, report.confidence_interval.1
            ),
        },
        SummaryRow {
            metric: "cold_start_min_pool_size".to_string(),
            value: report.cold_start_min_pool_size.to_string(),
        },
        SummaryRow {
            metric: "utilisation_rho".to_string(),
            value: format!("{:.4}", report.utilisation_rho),
        },
        SummaryRow {
            metric: "mean_queue_wait_ms".to_string(),
            value: format!("{:.3}", report.mean_queue_wait_ms),
        },
        SummaryRow {
            metric: "p99_queue_wait_ms".to_string(),
            value: format!("{:.3}", report.p99_queue_wait_ms),
        },
        SummaryRow {
            metric: "saturation".to_string(),
            value: format!("{:?}", report.saturation),
        },
    ];

    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    let rows: Vec<SensitivityTableRow> = report
        .sensitivity
        .iter()
        .map(|row| SensitivityTableRow {
            pool_size: render_pool_size(
                row.pool_size,
                Some(report.optimal_pool_size),
                row.risk,
                use_color,
            ),
            utilisation_rho: format!("{:.4}", row.utilisation_rho),
            mean_queue_wait_ms: format!("{:.3}", row.mean_queue_wait_ms),
            p99_queue_wait_ms: format!("{:.3}", row.p99_queue_wait_ms),
            risk: render_risk(row.risk, use_color),
        })
        .collect();

    let mut detail_table = Table::new(rows);
    detail_table.with(Style::psql());
    println!("{detail_table}");

    if !report.step_load_analysis.is_empty() {
        let step_rows: Vec<StepLoadTableRow> = report
            .step_load_analysis
            .iter()
            .map(|row| StepLoadTableRow {
                time_s: row.time_s,
                requests_per_second: format!("{:.3}", row.requests_per_second),
                utilisation_rho: format!("{:.4}", row.utilisation_rho),
                p99_queue_wait_ms: format!("{:.3}", row.p99_queue_wait_ms),
                saturation: render_saturation(row.saturation, use_color),
            })
            .collect();

        let mut step_table = Table::new(step_rows);
        step_table.with(Style::modern());
        println!("{step_table}");
    }

    if !report.warnings.is_empty() {
        eprintln!("warnings:");
        for warning in &report.warnings {
            eprintln!("- {warning}");
        }
    }

    Ok(())
}

pub fn evaluation(result: &EvaluationResult) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "pool_size".to_string(),
            value: result.pool_size.to_string(),
        },
        SummaryRow {
            metric: "utilisation_rho".to_string(),
            value: format!("{:.4}", result.utilisation_rho),
        },
        SummaryRow {
            metric: "mean_queue_wait_ms".to_string(),
            value: format!("{:.3}", result.mean_queue_wait_ms),
        },
        SummaryRow {
            metric: "p99_queue_wait_ms".to_string(),
            value: format!("{:.3}", result.p99_queue_wait_ms),
        },
        SummaryRow {
            metric: "saturation".to_string(),
            value: format!("{:?}", result.saturation),
        },
    ];

    let mut table = Table::new(summary);
    table.with(Style::rounded());
    println!("{table}");

    if !result.warnings.is_empty() {
        eprintln!("warnings:");
        for warning in &result.warnings {
            eprintln!("- {warning}");
        }
    }

    Ok(())
}

pub fn sweep(rows: &[SensitivityRow]) -> Result<()> {
    let use_color = std::io::stdout().is_terminal();
    let table_rows: Vec<SensitivityTableRow> = rows
        .iter()
        .map(|row| SensitivityTableRow {
            pool_size: render_pool_size(row.pool_size, None, row.risk, use_color),
            utilisation_rho: format!("{:.4}", row.utilisation_rho),
            mean_queue_wait_ms: format!("{:.3}", row.mean_queue_wait_ms),
            p99_queue_wait_ms: format!("{:.3}", row.p99_queue_wait_ms),
            risk: render_risk(row.risk, use_color),
        })
        .collect();

    let mut table = Table::new(table_rows);
    table.with(Style::psql());
    println!("{table}");
    Ok(())
}

pub fn batch(reports: &[SimulationReport]) -> Result<()> {
    let table_rows: Vec<BatchTableRow> = reports
        .iter()
        .enumerate()
        .map(|(idx, report)| BatchTableRow {
            request_index: idx,
            optimal_pool_size: report.optimal_pool_size,
            utilisation_rho: format!("{:.4}", report.utilisation_rho),
            p99_queue_wait_ms: format!("{:.3}", report.p99_queue_wait_ms),
            saturation: format!("{:?}", report.saturation),
        })
        .collect();

    let mut table = Table::new(table_rows);
    table.with(Style::psql());
    println!("{table}");
    Ok(())
}

pub fn compare(report: &ScenarioComparisonReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "baseline".to_string(),
            value: report.baseline.clone(),
        },
        SummaryRow {
            metric: "scenario_count".to_string(),
            value: report.rows.len().to_string(),
        },
        SummaryRow {
            metric: "worst_saturation".to_string(),
            value: format!("{:?}", report.worst_saturation),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    let rows: Vec<ScenarioComparisonTableRow> = report
        .rows
        .iter()
        .map(|row| ScenarioComparisonTableRow {
            scenario: row.name.clone(),
            baseline: row.is_baseline.to_string(),
            requests_per_second: format!("{:.3}", row.requests_per_second),
            optimal_pool_size: row.optimal_pool_size,
            pool_size_delta: format!("{:+}", row.pool_size_delta),
            p99_queue_wait_ms: format!("{:.3}", row.p99_queue_wait_ms),
            p99_queue_wait_delta_ms: format!("{:+.3}", row.p99_queue_wait_delta_ms),
            utilisation_rho: format!("{:.4}", row.utilisation_rho),
            saturation: format!("{:?}", row.saturation),
        })
        .collect();
    let mut table = Table::new(rows);
    table.with(Style::psql());
    println!("{table}");
    Ok(())
}

pub fn budget(report: &BudgetPlanReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "status".to_string(),
            value: format!("{:?}", report.status),
        },
        SummaryRow {
            metric: "max_connections".to_string(),
            value: report.max_connections.to_string(),
        },
        SummaryRow {
            metric: "reserved_connections".to_string(),
            value: report.reserved_connections.to_string(),
        },
        SummaryRow {
            metric: "safety_margin_connections".to_string(),
            value: report.safety_margin_connections.to_string(),
        },
        SummaryRow {
            metric: "available_connections".to_string(),
            value: report.available_connections.to_string(),
        },
        SummaryRow {
            metric: "requested_total_connections".to_string(),
            value: report.requested_total_connections.to_string(),
        },
        SummaryRow {
            metric: "allocated_total_connections".to_string(),
            value: report.allocated_total_connections.to_string(),
        },
        SummaryRow {
            metric: "unused_connections".to_string(),
            value: report.unused_connections.to_string(),
        },
        SummaryRow {
            metric: "over_budget_connections".to_string(),
            value: report.over_budget_connections.to_string(),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    let rows: Vec<BudgetServiceTableRow> = report
        .services
        .iter()
        .map(|service| BudgetServiceTableRow {
            service: service.name.clone(),
            replicas: service.replicas,
            priority: service.priority,
            current: service
                .current_pool_size
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            recommended: service.recommended_pool_size,
            allocated: service.allocated_pool_size,
            total_connections: service.allocated_total_connections,
            delta: service
                .pool_size_delta_from_current
                .map(|value| format!("{value:+}"))
                .unwrap_or_else(|| "-".to_string()),
            reduction: service.reduction_from_recommended,
            minimum_ok: service.meets_minimum.to_string(),
        })
        .collect();
    let mut service_table = Table::new(rows);
    service_table.with(Style::psql());
    println!("{service_table}");

    if !report.warnings.is_empty() {
        eprintln!("warnings:");
        for warning in &report.warnings {
            eprintln!("- {warning}");
        }
    }

    Ok(())
}

pub fn telemetry(recommendation: &TelemetryRecommendation) -> Result<()> {
    let diff = &recommendation.diff;
    let summary = vec![
        SummaryRow {
            metric: "service_name".to_string(),
            value: recommendation
                .service_name
                .as_deref()
                .unwrap_or("-")
                .to_string(),
        },
        SummaryRow {
            metric: "window".to_string(),
            value: recommendation.window.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "observed_at".to_string(),
            value: recommendation
                .observed_at
                .as_deref()
                .unwrap_or("-")
                .to_string(),
        },
        SummaryRow {
            metric: "current_pool_size".to_string(),
            value: diff.current_pool_size.to_string(),
        },
        SummaryRow {
            metric: "recommended_pool_size".to_string(),
            value: diff.recommended_pool_size.to_string(),
        },
        SummaryRow {
            metric: "pool_size_delta".to_string(),
            value: format!("{:+}", diff.pool_size_delta),
        },
        SummaryRow {
            metric: "change".to_string(),
            value: format!("{:?}", diff.change),
        },
        SummaryRow {
            metric: "connection_change_percent".to_string(),
            value: format!("{:+.2}%", diff.connection_change_percent),
        },
        SummaryRow {
            metric: "current_saturation".to_string(),
            value: format!("{:?}", diff.current_evaluation.saturation),
        },
        SummaryRow {
            metric: "recommended_saturation".to_string(),
            value: format!("{:?}", diff.recommended_report.saturation),
        },
    ];

    let mut table = Table::new(summary);
    table.with(Style::rounded());
    println!("{table}");

    if !diff.current_evaluation.warnings.is_empty() {
        eprintln!("current pool warnings:");
        for warning in &diff.current_evaluation.warnings {
            eprintln!("- {warning}");
        }
    }
    if !diff.recommended_report.warnings.is_empty() {
        eprintln!("recommended pool warnings:");
        for warning in &diff.recommended_report.warnings {
            eprintln!("- {warning}");
        }
    }

    Ok(())
}

pub fn gate(report: &GateReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "status".to_string(),
            value: format!("{:?}", report.status),
        },
        SummaryRow {
            metric: "service_name".to_string(),
            value: report.service_name.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "window".to_string(),
            value: report.window.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "observed_at".to_string(),
            value: report.observed_at.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "worst_saturation".to_string(),
            value: format!("{:?}", report.worst_saturation),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    let rows: Vec<GateCheckTableRow> = report
        .checks
        .iter()
        .map(|check| GateCheckTableRow {
            check: check.name.clone(),
            passed: check.passed.to_string(),
            severity: format!("{:?}", check.severity),
            observed: check.observed.clone(),
            threshold: check.threshold.clone(),
            message: check.message.clone(),
        })
        .collect();
    let mut checks_table = Table::new(rows);
    checks_table.with(Style::psql());
    println!("{checks_table}");
    Ok(())
}

pub fn guard(report: &GuardReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "status".to_string(),
            value: format!("{:?}", report.status),
        },
        SummaryRow {
            metric: "deployment_safe".to_string(),
            value: report.deployment_safe.to_string(),
        },
        SummaryRow {
            metric: "exit_code".to_string(),
            value: report.exit_code.to_string(),
        },
        SummaryRow {
            metric: "reason".to_string(),
            value: report.reason.clone(),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    gate(&report.gate)?;
    Ok(())
}

pub fn doctor(report: &DoctorReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "status".to_string(),
            value: format!("{:?}", report.status),
        },
        SummaryRow {
            metric: "service_name".to_string(),
            value: report.service_name.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "window".to_string(),
            value: report.window.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "observed_at".to_string(),
            value: report.observed_at.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "current_pool_size".to_string(),
            value: report.current_pool_size.to_string(),
        },
        SummaryRow {
            metric: "recommended_pool_size".to_string(),
            value: report.recommended_pool_size.to_string(),
        },
        SummaryRow {
            metric: "pool_size_delta".to_string(),
            value: format!("{:+}", report.pool_size_delta),
        },
        SummaryRow {
            metric: "current_rho".to_string(),
            value: format!("{:.4}", report.current_rho),
        },
        SummaryRow {
            metric: "current_p99_queue_wait_ms".to_string(),
            value: format!("{:.3}", report.current_p99_queue_wait_ms),
        },
        SummaryRow {
            metric: "current_saturation".to_string(),
            value: format!("{:?}", report.current_saturation),
        },
        SummaryRow {
            metric: "recommended_saturation".to_string(),
            value: format!("{:?}", report.recommended_saturation),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    let rows: Vec<DoctorFindingTableRow> = report
        .findings
        .iter()
        .map(|finding| DoctorFindingTableRow {
            finding: finding.name.clone(),
            severity: format!("{:?}", finding.severity),
            message: finding.message.clone(),
            action: finding.action.clone(),
        })
        .collect();
    let mut findings_table = Table::new(rows);
    findings_table.with(Style::psql());
    println!("{findings_table}");
    Ok(())
}

pub fn config_snippet(report: &ConfigSnippetReport) -> Result<()> {
    let summary = vec![
        SummaryRow {
            metric: "framework".to_string(),
            value: report.framework.as_str().to_string(),
        },
        SummaryRow {
            metric: "source".to_string(),
            value: report.source.as_str().to_string(),
        },
        SummaryRow {
            metric: "service_name".to_string(),
            value: report.service_name.as_deref().unwrap_or("-").to_string(),
        },
        SummaryRow {
            metric: "recommended_pool_size".to_string(),
            value: report.recommended_pool_size.to_string(),
        },
        SummaryRow {
            metric: "min_idle".to_string(),
            value: report.min_idle.to_string(),
        },
        SummaryRow {
            metric: "connection_timeout_ms".to_string(),
            value: report.connection_timeout_ms.to_string(),
        },
        SummaryRow {
            metric: "idle_timeout_ms".to_string(),
            value: report.idle_timeout_ms.to_string(),
        },
        SummaryRow {
            metric: "max_server_connections".to_string(),
            value: report.max_server_connections.to_string(),
        },
        SummaryRow {
            metric: "utilisation_rho".to_string(),
            value: format!("{:.4}", report.utilisation_rho),
        },
        SummaryRow {
            metric: "p99_queue_wait_ms".to_string(),
            value: format!("{:.3}", report.p99_queue_wait_ms),
        },
    ];
    let mut summary_table = Table::new(summary);
    summary_table.with(Style::rounded());
    println!("{summary_table}");

    println!("\n{}", report.snippet);

    if !report.notes.is_empty() {
        println!("\nnotes:");
        for note in &report.notes {
            println!("- {note}");
        }
    }

    if !report.references.is_empty() {
        println!("\nreferences:");
        for reference in &report.references {
            println!("- {}: {}", reference.title, reference.url);
        }
    }

    Ok(())
}

fn render_pool_size(
    pool_size: u32,
    recommended: Option<u32>,
    risk: RiskLevel,
    use_color: bool,
) -> String {
    let text = pool_size.to_string();
    if !use_color {
        return text;
    }
    if recommended == Some(pool_size) {
        return paint(&text, "\x1b[32m");
    }
    if risk == RiskLevel::Critical {
        return paint(&text, "\x1b[31m");
    }
    text
}

fn render_risk(risk: RiskLevel, use_color: bool) -> String {
    let text = format!("{risk:?}");
    if !use_color {
        return text;
    }

    match risk {
        RiskLevel::Low => paint(&text, "\x1b[32m"),
        RiskLevel::Medium => paint(&text, "\x1b[33m"),
        RiskLevel::High => paint(&text, "\x1b[91m"),
        RiskLevel::Critical => paint(&text, "\x1b[31m"),
    }
}

fn paint(input: &str, color: &str) -> String {
    format!("{color}{input}\x1b[0m")
}

fn render_saturation(saturation: SaturationLevel, use_color: bool) -> String {
    let text = format!("{saturation:?}");
    if !use_color {
        return text;
    }

    match saturation {
        SaturationLevel::Ok => paint(&text, "\x1b[32m"),
        SaturationLevel::Warning => paint(&text, "\x1b[33m"),
        SaturationLevel::Critical => paint(&text, "\x1b[31m"),
    }
}

#[cfg(test)]
mod tests {
    use poolsim_core::telemetry::{PoolRecommendationDiff, PoolSizeChange};
    use poolsim_core::types::{
        PoolConfig, RiskLevel, SaturationLevel, SimulationOptions, StepLoadResult, WorkloadConfig,
    };

    use super::*;

    fn sample_rows() -> Vec<SensitivityRow> {
        vec![
            SensitivityRow {
                pool_size: 4,
                utilisation_rho: 0.70,
                mean_queue_wait_ms: 4.0,
                p99_queue_wait_ms: 16.0,
                risk: RiskLevel::Low,
            },
            SensitivityRow {
                pool_size: 5,
                utilisation_rho: 0.93,
                mean_queue_wait_ms: 9.0,
                p99_queue_wait_ms: 45.0,
                risk: RiskLevel::Critical,
            },
        ]
    }

    fn sample_report() -> SimulationReport {
        SimulationReport {
            optimal_pool_size: 5,
            confidence_interval: (4, 6),
            cold_start_min_pool_size: 4,
            utilisation_rho: 0.86,
            mean_queue_wait_ms: 7.0,
            p99_queue_wait_ms: 40.0,
            saturation: SaturationLevel::Warning,
            sensitivity: sample_rows(),
            step_load_analysis: vec![StepLoadResult {
                time_s: 30,
                requests_per_second: 240.0,
                utilisation_rho: 0.88,
                p99_queue_wait_ms: 42.0,
                saturation: SaturationLevel::Warning,
            }],
            warnings: vec!["warning line".to_string()],
        }
    }

    fn sample_evaluation() -> EvaluationResult {
        EvaluationResult {
            pool_size: 6,
            utilisation_rho: 0.79,
            mean_queue_wait_ms: 5.2,
            p99_queue_wait_ms: 22.0,
            saturation: SaturationLevel::Ok,
            warnings: vec!["eval warning".to_string()],
        }
    }

    fn sample_recommendation() -> TelemetryRecommendation {
        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: Some("2026-05-15T10:00:00Z".to_string()),
            diff: PoolRecommendationDiff {
                current_pool_size: 6,
                recommended_pool_size: 5,
                pool_size_delta: -1,
                change: PoolSizeChange::Decrease,
                additional_connections_required: 0,
                removable_connections: 1,
                connection_change_percent: -16.666,
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
    fn private_render_helpers_cover_colored_and_plain_paths() {
        assert_eq!(render_pool_size(4, None, RiskLevel::Low, false), "4");
        assert_eq!(render_risk(RiskLevel::Medium, false), "Medium");
        assert_eq!(render_saturation(SaturationLevel::Ok, false), "Ok");

        let highlighted = render_pool_size(8, Some(8), RiskLevel::Low, true);
        assert!(highlighted.contains("\x1b[32m"));
        let critical = render_pool_size(9, None, RiskLevel::Critical, true);
        assert!(critical.contains("\x1b[31m"));
        assert_eq!(render_pool_size(10, None, RiskLevel::Low, true), "10");

        assert!(render_risk(RiskLevel::Low, true).contains("\x1b[32m"));
        assert!(render_risk(RiskLevel::Medium, true).contains("\x1b[33m"));
        assert!(render_risk(RiskLevel::High, true).contains("\x1b[91m"));
        assert!(render_risk(RiskLevel::Critical, true).contains("\x1b[31m"));

        assert!(render_saturation(SaturationLevel::Ok, true).contains("\x1b[32m"));
        assert!(render_saturation(SaturationLevel::Warning, true).contains("\x1b[33m"));
        assert!(render_saturation(SaturationLevel::Critical, true).contains("\x1b[31m"));
        assert_eq!(paint("abc", "\x1b[32m"), "\x1b[32mabc\x1b[0m");
    }

    #[test]
    fn table_renderers_execute_for_all_output_types() {
        simulation(&sample_report()).expect("simulation table should render");
        evaluation(&sample_evaluation()).expect("evaluation table should render");
        sweep(&sample_rows()).expect("sweep table should render");
        batch(&[sample_report(), sample_report()]).expect("batch table should render");
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
        .expect("compare table should render");
        budget(&sample_budget_report()).expect("budget table should render");
        telemetry(&sample_recommendation()).expect("telemetry table should render");
        gate(&crate::gate::build_gate_report(
            sample_recommendation(),
            &crate::gate::GatePolicy::default(),
        ))
        .expect("gate table should render");
        guard(&crate::guard::build_guard_report(
            crate::gate::build_gate_report(
                sample_recommendation(),
                &crate::gate::GatePolicy::default(),
            ),
        ))
        .expect("guard table should render");
        doctor(&crate::doctor::build_doctor_report(sample_recommendation()))
            .expect("doctor table should render");

        let config_report = crate::config_gen::build_config_snippet(
            &crate::args::GenerateConfigArgs {
                framework: crate::args::CliConfigFramework::Sqlx,
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
        config_snippet(&config_report).expect("config snippet table should render");
    }
}
