use std::io::IsTerminal;

use anyhow::Result;
use poolsim_core::types::{EvaluationResult, RiskLevel, SaturationLevel, SensitivityRow, SimulationReport};
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
struct StepLoadTableRow {
    time_s: u32,
    requests_per_second: String,
    utilisation_rho: String,
    p99_queue_wait_ms: String,
    saturation: String,
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
            value: format!("{}..{}", report.confidence_interval.0, report.confidence_interval.1),
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
            pool_size: render_pool_size(row.pool_size, Some(report.optimal_pool_size), row.risk, use_color),
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

fn render_pool_size(pool_size: u32, recommended: Option<u32>, risk: RiskLevel, use_color: bool) -> String {
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
    use poolsim_core::types::{StepLoadResult, RiskLevel, SaturationLevel};

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
    }
}
