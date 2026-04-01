use anyhow::Result;
use csv::{Writer, WriterBuilder};
use poolsim_core::types::{EvaluationResult, SensitivityRow, SimulationReport};

pub fn simulation(report: &SimulationReport) -> Result<()> {
    let mut wtr = WriterBuilder::new().flexible(true).from_writer(std::io::stdout());

    wtr.write_record(["summary_field", "value"])?;
    wtr.write_record(["optimal_pool_size", &report.optimal_pool_size.to_string()])?;
    wtr.write_record([
        "confidence_interval",
        &format!("{}..{}", report.confidence_interval.0, report.confidence_interval.1),
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
    let mut wtr = WriterBuilder::new().flexible(true).from_writer(std::io::stdout());
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
    let mut wtr = WriterBuilder::new().flexible(true).from_writer(std::io::stdout());
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
    let mut wtr = WriterBuilder::new().flexible(true).from_writer(std::io::stdout());
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
    use poolsim_core::types::{
        EvaluationResult, RiskLevel, SaturationLevel, SensitivityRow, SimulationReport, StepLoadResult,
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

    #[test]
    fn csv_renderers_execute_for_all_output_types() {
        simulation(&sample_report()).expect("simulation CSV render should succeed");
        evaluation(&sample_evaluation()).expect("evaluation CSV render should succeed");
        sweep(&sample_rows()).expect("sweep CSV render should succeed");
        batch(&[sample_report(), sample_report()]).expect("batch CSV render should succeed");
    }
}
