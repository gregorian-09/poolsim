use anyhow::{anyhow, Result};
use poolsim_core::{
    simulate,
    types::{SaturationLevel, SimulationReport},
};
use serde::Serialize;

use crate::config::ScenarioComparisonInput;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ScenarioComparisonReport {
    pub baseline: String,
    pub worst_saturation: SaturationLevel,
    pub rows: Vec<ScenarioComparisonRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ScenarioComparisonRow {
    pub name: String,
    pub is_baseline: bool,
    pub requests_per_second: f64,
    pub optimal_pool_size: u32,
    pub pool_size_delta: i32,
    pub p99_queue_wait_ms: f64,
    pub p99_queue_wait_delta_ms: f64,
    pub mean_queue_wait_ms: f64,
    pub mean_queue_wait_delta_ms: f64,
    pub utilisation_rho: f64,
    pub utilisation_rho_delta: f64,
    pub saturation: SaturationLevel,
    pub report: SimulationReport,
}

pub(crate) fn build_scenario_comparison_report(
    input: ScenarioComparisonInput,
) -> Result<ScenarioComparisonReport> {
    let mut reports = Vec::with_capacity(input.scenarios.len());
    for scenario in input.scenarios {
        let report = simulate(&scenario.workload, &scenario.pool, &scenario.options)?;
        reports.push((scenario.name, scenario.workload.requests_per_second, report));
    }

    let baseline = reports
        .iter()
        .find(|(name, _, _)| name == &input.baseline)
        .ok_or_else(|| anyhow!("baseline scenario '{}' was not found", input.baseline))?;
    let baseline_pool_size = baseline.2.optimal_pool_size;
    let baseline_p99_queue_wait_ms = baseline.2.p99_queue_wait_ms;
    let baseline_mean_queue_wait_ms = baseline.2.mean_queue_wait_ms;
    let baseline_utilisation_rho = baseline.2.utilisation_rho;

    let rows: Vec<ScenarioComparisonRow> = reports
        .into_iter()
        .map(
            |(name, requests_per_second, report)| ScenarioComparisonRow {
                is_baseline: name == input.baseline,
                pool_size_delta: report.optimal_pool_size as i32 - baseline_pool_size as i32,
                p99_queue_wait_delta_ms: report.p99_queue_wait_ms - baseline_p99_queue_wait_ms,
                mean_queue_wait_delta_ms: report.mean_queue_wait_ms - baseline_mean_queue_wait_ms,
                utilisation_rho_delta: report.utilisation_rho - baseline_utilisation_rho,
                requests_per_second,
                optimal_pool_size: report.optimal_pool_size,
                p99_queue_wait_ms: report.p99_queue_wait_ms,
                mean_queue_wait_ms: report.mean_queue_wait_ms,
                utilisation_rho: report.utilisation_rho,
                saturation: report.saturation,
                report,
                name,
            },
        )
        .collect();

    let worst_saturation = rows
        .iter()
        .map(|row| row.saturation)
        .max_by_key(|saturation| saturation_rank(*saturation))
        .unwrap_or(SaturationLevel::Ok);

    Ok(ScenarioComparisonReport {
        baseline: input.baseline,
        worst_saturation,
        rows,
    })
}

fn saturation_rank(saturation: SaturationLevel) -> u8 {
    match saturation {
        SaturationLevel::Ok => 0,
        SaturationLevel::Warning => 1,
        SaturationLevel::Critical => 2,
    }
}

#[cfg(test)]
mod tests {
    use poolsim_core::types::{PoolConfig, SimulationOptions, WorkloadConfig};

    use crate::config::{ScenarioComparisonInput, ScenarioInput};

    use super::*;

    fn scenario(name: &str, rps: f64) -> ScenarioInput {
        ScenarioInput {
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
                seed: Some(11),
                ..SimulationOptions::default()
            },
        }
    }

    #[test]
    fn scenario_comparison_builds_deltas_against_baseline() {
        let report = build_scenario_comparison_report(ScenarioComparisonInput {
            baseline: "normal".to_string(),
            scenarios: vec![scenario("normal", 180.0), scenario("peak", 260.0)],
        })
        .expect("comparison should build");

        assert_eq!(report.baseline, "normal");
        assert_eq!(report.rows.len(), 2);
        assert!(report.rows[0].is_baseline);
        assert_eq!(report.rows[0].pool_size_delta, 0);
        assert!(report.rows[1].pool_size_delta >= 0);
        let _ = saturation_rank(report.worst_saturation);
    }

    #[test]
    fn scenario_comparison_rejects_missing_baseline() {
        let err = build_scenario_comparison_report(ScenarioComparisonInput {
            baseline: "missing".to_string(),
            scenarios: vec![scenario("normal", 180.0)],
        })
        .expect_err("missing baseline should fail");

        assert!(err.to_string().contains("baseline scenario"));
    }

    #[test]
    fn saturation_rank_orders_warning_and_critical_levels() {
        assert!(saturation_rank(SaturationLevel::Warning) > saturation_rank(SaturationLevel::Ok));
        assert!(
            saturation_rank(SaturationLevel::Critical) > saturation_rank(SaturationLevel::Warning)
        );
    }
}
