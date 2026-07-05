use poolsim_core::types::{
    EvaluationResult, RiskLevel, SensitivityRow, SimulationReport, WorkloadConfig,
};

pub(crate) fn simulation(workload: &WorkloadConfig, report: &SimulationReport) -> String {
    let mut lines = vec![format!(
        "Poolsim recommends a pool of {} for {:.3} requests/sec.",
        report.optimal_pool_size, workload.requests_per_second
    )];
    lines.push(format!(
        "At that size, utilisation rho is {:.3}, mean queue wait is {:.3} ms, and p99 queue wait is {:.3} ms.",
        report.utilisation_rho, report.mean_queue_wait_ms, report.p99_queue_wait_ms
    ));
    lines.push(format!(
        "The confidence band is {}..{}, and the cold-start minimum is {}.",
        report.confidence_interval.0, report.confidence_interval.1, report.cold_start_min_pool_size
    ));
    if !report.step_load_analysis.is_empty() {
        let worst = report
            .step_load_analysis
            .iter()
            .max_by_key(|row| saturation_rank(row.saturation));
        if let Some(worst) = worst {
            lines.push(format!(
                "The highest step-load pressure occurs at {}s with {:.3} requests/sec, rho {:.3}, and p99 queue wait {:.3} ms.",
                worst.time_s, worst.requests_per_second, worst.utilisation_rho, worst.p99_queue_wait_ms
            ));
        }
    }
    lines.push(saturation_sentence(report.saturation));
    lines.join("\n")
}

pub(crate) fn evaluation(workload: &WorkloadConfig, result: &EvaluationResult) -> String {
    format!(
        "A pool of {} handling {:.3} requests/sec has rho {:.3}, mean queue wait {:.3} ms, and p99 queue wait {:.3} ms. {}",
        result.pool_size,
        workload.requests_per_second,
        result.utilisation_rho,
        result.mean_queue_wait_ms,
        result.p99_queue_wait_ms,
        saturation_sentence(result.saturation)
    )
}

pub(crate) fn sweep(rows: &[SensitivityRow]) -> String {
    if rows.is_empty() {
        return "No sensitivity rows were produced, so there is no pool-size pressure to explain."
            .to_string();
    }
    let safest = rows
        .iter()
        .find(|row| matches!(row.risk, RiskLevel::Low))
        .unwrap_or(&rows[0]);
    let worst = rows
        .iter()
        .max_by_key(|row| risk_rank(row.risk))
        .unwrap_or(&rows[0]);
    format!(
        "The sensitivity sweep first reaches low risk at pool size {} with rho {:.3} and p99 queue wait {:.3} ms. The highest-risk candidate is pool size {} with {:?} risk, rho {:.3}, and p99 queue wait {:.3} ms.",
        safest.pool_size,
        safest.utilisation_rho,
        safest.p99_queue_wait_ms,
        worst.pool_size,
        worst.risk,
        worst.utilisation_rho,
        worst.p99_queue_wait_ms
    )
}

fn saturation_sentence(saturation: poolsim_core::types::SaturationLevel) -> String {
    match saturation {
        poolsim_core::types::SaturationLevel::Ok => {
            "This is within the configured saturation target.".to_string()
        }
        poolsim_core::types::SaturationLevel::Warning => {
            "This is close to the configured saturation target; review the sensitivity table before deploying.".to_string()
        }
        poolsim_core::types::SaturationLevel::Critical => {
            "This is above the safe saturation target and should be treated as unsafe without more capacity or lower latency.".to_string()
        }
    }
}

fn saturation_rank(saturation: poolsim_core::types::SaturationLevel) -> u8 {
    match saturation {
        poolsim_core::types::SaturationLevel::Ok => 0,
        poolsim_core::types::SaturationLevel::Warning => 1,
        poolsim_core::types::SaturationLevel::Critical => 2,
    }
}

fn risk_rank(risk: RiskLevel) -> u8 {
    match risk {
        RiskLevel::Low => 0,
        RiskLevel::Medium => 1,
        RiskLevel::High => 2,
        RiskLevel::Critical => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poolsim_core::types::{SaturationLevel, StepLoadResult};

    fn workload() -> WorkloadConfig {
        WorkloadConfig {
            requests_per_second: 180.0,
            latency_p50_ms: 8.0,
            latency_p95_ms: 30.0,
            latency_p99_ms: 70.0,
            raw_samples_ms: None,
            step_load_profile: None,
        }
    }

    fn rows() -> Vec<SensitivityRow> {
        vec![
            SensitivityRow {
                pool_size: 4,
                utilisation_rho: 0.95,
                mean_queue_wait_ms: 10.0,
                p99_queue_wait_ms: 90.0,
                risk: RiskLevel::High,
            },
            SensitivityRow {
                pool_size: 8,
                utilisation_rho: 0.55,
                mean_queue_wait_ms: 1.0,
                p99_queue_wait_ms: 8.0,
                risk: RiskLevel::Low,
            },
        ]
    }

    #[test]
    fn simulation_explanation_mentions_recommendation_and_step_load() {
        let report = SimulationReport {
            optimal_pool_size: 8,
            confidence_interval: (7, 9),
            cold_start_min_pool_size: 4,
            utilisation_rho: 0.55,
            mean_queue_wait_ms: 1.0,
            p99_queue_wait_ms: 8.0,
            saturation: SaturationLevel::Ok,
            sensitivity: rows(),
            step_load_analysis: vec![StepLoadResult {
                time_s: 30,
                requests_per_second: 240.0,
                utilisation_rho: 0.72,
                p99_queue_wait_ms: 20.0,
                saturation: SaturationLevel::Warning,
            }],
            warnings: Vec::new(),
        };
        let text = simulation(&workload(), &report);
        assert!(text.contains("recommends a pool of 8"));
        assert!(text.contains("30s"));
    }

    #[test]
    fn evaluation_explanation_mentions_fixed_pool_size() {
        let result = EvaluationResult {
            pool_size: 6,
            utilisation_rho: 0.75,
            mean_queue_wait_ms: 3.0,
            p99_queue_wait_ms: 18.0,
            saturation: SaturationLevel::Warning,
            warnings: Vec::new(),
        };
        assert!(evaluation(&workload(), &result).contains("pool of 6"));
    }

    #[test]
    fn sweep_explanation_handles_empty_and_ranked_rows() {
        assert!(sweep(&[]).contains("No sensitivity rows"));
        let text = sweep(&rows());
        assert!(text.contains("first reaches low risk at pool size 8"));
        assert!(text.contains("highest-risk candidate"));
    }

    #[test]
    fn explanations_cover_remaining_saturation_and_risk_branches() {
        let critical = EvaluationResult {
            pool_size: 3,
            utilisation_rho: 1.05,
            mean_queue_wait_ms: 50.0,
            p99_queue_wait_ms: 200.0,
            saturation: SaturationLevel::Critical,
            warnings: Vec::new(),
        };
        assert!(evaluation(&workload(), &critical).contains("unsafe"));

        let medium_rows = vec![
            SensitivityRow {
                pool_size: 5,
                utilisation_rho: 0.70,
                mean_queue_wait_ms: 2.0,
                p99_queue_wait_ms: 10.0,
                risk: RiskLevel::Medium,
            },
            SensitivityRow {
                pool_size: 6,
                utilisation_rho: 0.60,
                mean_queue_wait_ms: 1.0,
                p99_queue_wait_ms: 5.0,
                risk: RiskLevel::Critical,
            },
        ];
        let text = sweep(&medium_rows);
        assert!(text.contains("first reaches low risk at pool size 5"));
        assert!(text.contains("Critical risk"));
        assert_eq!(saturation_rank(SaturationLevel::Critical), 2);
    }
}
