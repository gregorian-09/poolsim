use std::{fs, path::Path, process::ExitCode};

use anyhow::{anyhow, bail, Context, Result};
use poolsim_core::{
    telemetry::TelemetryRecommendation,
    types::SaturationLevel,
};
use serde::{Deserialize, Serialize};

use crate::args::{GateArgs, GuardArgs};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct GatePolicy {
    #[serde(default = "default_max_saturation")]
    pub max_saturation: SaturationLevel,
    #[serde(default)]
    pub max_pool_increase_percent: Option<f64>,
    #[serde(default)]
    pub max_additional_connections: Option<u32>,
    #[serde(default)]
    pub max_recommended_pool_size: Option<u32>,
    #[serde(default)]
    pub max_recommended_p99_queue_wait_ms: Option<f64>,
    #[serde(default)]
    pub max_recommended_mean_queue_wait_ms: Option<f64>,
    #[serde(default)]
    pub max_recommended_rho: Option<f64>,
    #[serde(default)]
    pub max_current_p99_queue_wait_ms: Option<f64>,
    #[serde(default)]
    pub max_current_mean_queue_wait_ms: Option<f64>,
    #[serde(default)]
    pub max_current_rho: Option<f64>,
    #[serde(default)]
    pub expected_pool_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum GateDecision {
    Pass,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct GateCheck {
    pub name: String,
    pub passed: bool,
    pub severity: GateDecision,
    pub observed: String,
    pub threshold: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct GateReport {
    pub status: GateDecision,
    pub service_name: Option<String>,
    pub window: Option<String>,
    pub observed_at: Option<String>,
    pub worst_saturation: SaturationLevel,
    pub checks: Vec<GateCheck>,
    pub recommendation: TelemetryRecommendation,
}

impl Default for GatePolicy {
    fn default() -> Self {
        Self {
            max_saturation: default_max_saturation(),
            max_pool_increase_percent: None,
            max_additional_connections: None,
            max_recommended_pool_size: None,
            max_recommended_p99_queue_wait_ms: None,
            max_recommended_mean_queue_wait_ms: None,
            max_recommended_rho: None,
            max_current_p99_queue_wait_ms: None,
            max_current_mean_queue_wait_ms: None,
            max_current_rho: None,
            expected_pool_size: None,
        }
    }
}

impl GateDecision {
    pub(crate) fn exit_code(self) -> ExitCode {
        match self {
            Self::Pass => ExitCode::from(0),
            Self::Warning => ExitCode::from(1),
            Self::Critical => ExitCode::from(2),
        }
    }
}

pub(crate) fn policy_from_args(args: &GateArgs) -> Result<GatePolicy> {
    let overrides = PolicyOverrides {
        max_saturation: args.max_saturation,
        max_pool_increase_percent: args.max_pool_increase_percent,
        max_additional_connections: args.max_additional_connections,
        max_recommended_pool_size: args.max_recommended_pool_size,
        max_recommended_p99_queue_wait_ms: args.max_recommended_p99_queue_wait_ms,
        max_recommended_mean_queue_wait_ms: args.max_recommended_mean_queue_wait_ms,
        max_recommended_rho: args.max_recommended_rho,
        max_current_p99_queue_wait_ms: args.max_current_p99_queue_wait_ms,
        max_current_mean_queue_wait_ms: args.max_current_mean_queue_wait_ms,
        max_current_rho: args.max_current_rho,
        expected_pool_size: args.expected_pool_size,
    };
    policy_from_path_and_overrides(args.policy.as_deref(), overrides)
}

pub(crate) fn policy_from_guard_args(args: &GuardArgs) -> Result<GatePolicy> {
    let overrides = PolicyOverrides {
        max_saturation: args.max_saturation,
        max_pool_increase_percent: args.max_pool_increase_percent,
        max_additional_connections: args.max_additional_connections,
        max_recommended_pool_size: args.max_recommended_pool_size,
        max_recommended_p99_queue_wait_ms: args.max_recommended_p99_queue_wait_ms,
        max_recommended_mean_queue_wait_ms: args.max_recommended_mean_queue_wait_ms,
        max_recommended_rho: args.max_recommended_rho,
        max_current_p99_queue_wait_ms: args.max_current_p99_queue_wait_ms,
        max_current_mean_queue_wait_ms: args.max_current_mean_queue_wait_ms,
        max_current_rho: args.max_current_rho,
        expected_pool_size: args.expected_pool_size,
    };
    policy_from_path_and_overrides(args.policy.as_deref(), overrides)
}

#[derive(Debug, Clone, Copy)]
struct PolicyOverrides {
    max_saturation: Option<crate::args::CliSaturationLevel>,
    max_pool_increase_percent: Option<f64>,
    max_additional_connections: Option<u32>,
    max_recommended_pool_size: Option<u32>,
    max_recommended_p99_queue_wait_ms: Option<f64>,
    max_recommended_mean_queue_wait_ms: Option<f64>,
    max_recommended_rho: Option<f64>,
    max_current_p99_queue_wait_ms: Option<f64>,
    max_current_mean_queue_wait_ms: Option<f64>,
    max_current_rho: Option<f64>,
    expected_pool_size: Option<u32>,
}

fn policy_from_path_and_overrides(
    policy_path: Option<&Path>,
    overrides: PolicyOverrides,
) -> Result<GatePolicy> {
    let mut policy = match policy_path {
        Some(path) => load_policy(path)?,
        None => GatePolicy::default(),
    };

    if let Some(value) = overrides.max_saturation {
        policy.max_saturation = value.into();
    }
    if let Some(value) = overrides.max_pool_increase_percent {
        policy.max_pool_increase_percent = Some(value);
    }
    if let Some(value) = overrides.max_additional_connections {
        policy.max_additional_connections = Some(value);
    }
    if let Some(value) = overrides.max_recommended_pool_size {
        policy.max_recommended_pool_size = Some(value);
    }
    if let Some(value) = overrides.max_recommended_p99_queue_wait_ms {
        policy.max_recommended_p99_queue_wait_ms = Some(value);
    }
    if let Some(value) = overrides.max_recommended_mean_queue_wait_ms {
        policy.max_recommended_mean_queue_wait_ms = Some(value);
    }
    if let Some(value) = overrides.max_recommended_rho {
        policy.max_recommended_rho = Some(value);
    }
    if let Some(value) = overrides.max_current_p99_queue_wait_ms {
        policy.max_current_p99_queue_wait_ms = Some(value);
    }
    if let Some(value) = overrides.max_current_mean_queue_wait_ms {
        policy.max_current_mean_queue_wait_ms = Some(value);
    }
    if let Some(value) = overrides.max_current_rho {
        policy.max_current_rho = Some(value);
    }
    if let Some(value) = overrides.expected_pool_size {
        policy.expected_pool_size = Some(value);
    }

    validate_policy(&policy)?;
    Ok(policy)
}

pub(crate) fn build_gate_report(
    recommendation: TelemetryRecommendation,
    policy: &GatePolicy,
) -> GateReport {
    let diff = &recommendation.diff;
    let worst_saturation = diff.worst_saturation();
    let mut checks = Vec::new();

    checks.push(check_saturation(
        worst_saturation,
        policy.max_saturation,
    ));

    if let Some(threshold) = policy.max_pool_increase_percent {
        let observed = diff.connection_change_percent.max(0.0);
        checks.push(check_f64_max(
            "max_pool_increase_percent",
            observed,
            threshold,
            GateDecision::Critical,
            "%",
        ));
    }
    if let Some(threshold) = policy.max_additional_connections {
        checks.push(check_u32_max(
            "max_additional_connections",
            diff.additional_connections_required,
            threshold,
            GateDecision::Critical,
        ));
    }
    if let Some(threshold) = policy.max_recommended_pool_size {
        checks.push(check_u32_max(
            "max_recommended_pool_size",
            diff.recommended_pool_size,
            threshold,
            GateDecision::Critical,
        ));
    }
    if let Some(threshold) = policy.max_recommended_p99_queue_wait_ms {
        checks.push(check_f64_max(
            "max_recommended_p99_queue_wait_ms",
            diff.recommended_report.p99_queue_wait_ms,
            threshold,
            GateDecision::Critical,
            "ms",
        ));
    }
    if let Some(threshold) = policy.max_recommended_mean_queue_wait_ms {
        checks.push(check_f64_max(
            "max_recommended_mean_queue_wait_ms",
            diff.recommended_report.mean_queue_wait_ms,
            threshold,
            GateDecision::Critical,
            "ms",
        ));
    }
    if let Some(threshold) = policy.max_recommended_rho {
        checks.push(check_f64_max(
            "max_recommended_rho",
            diff.recommended_report.utilisation_rho,
            threshold,
            GateDecision::Critical,
            "",
        ));
    }
    if let Some(threshold) = policy.max_current_p99_queue_wait_ms {
        checks.push(check_f64_max(
            "max_current_p99_queue_wait_ms",
            diff.current_evaluation.p99_queue_wait_ms,
            threshold,
            GateDecision::Critical,
            "ms",
        ));
    }
    if let Some(threshold) = policy.max_current_mean_queue_wait_ms {
        checks.push(check_f64_max(
            "max_current_mean_queue_wait_ms",
            diff.current_evaluation.mean_queue_wait_ms,
            threshold,
            GateDecision::Critical,
            "ms",
        ));
    }
    if let Some(threshold) = policy.max_current_rho {
        checks.push(check_f64_max(
            "max_current_rho",
            diff.current_evaluation.utilisation_rho,
            threshold,
            GateDecision::Critical,
            "",
        ));
    }
    if let Some(expected) = policy.expected_pool_size {
        checks.push(check_expected_pool_size(
            diff.recommended_pool_size,
            expected,
        ));
    }

    let status = checks
        .iter()
        .filter(|check| !check.passed)
        .map(|check| check.severity)
        .max_by_key(|decision| decision_rank(*decision))
        .unwrap_or(GateDecision::Pass);

    GateReport {
        status,
        service_name: recommendation.service_name.clone(),
        window: recommendation.window.clone(),
        observed_at: recommendation.observed_at.clone(),
        worst_saturation,
        checks,
        recommendation,
    }
}

fn load_policy(path: &Path) -> Result<GatePolicy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read gate policy file {}", path.display()))?;

    let policy = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON gate policy file {}", path.display()))?,
        Some("toml") => toml::from_str(&raw)
            .with_context(|| format!("invalid TOML gate policy file {}", path.display()))?,
        _ => {
            return Err(anyhow!(
                "unsupported gate policy extension for {} (use .json or .toml)",
                path.display()
            ))
        }
    };

    validate_policy(&policy)?;
    Ok(policy)
}

fn validate_policy(policy: &GatePolicy) -> Result<()> {
    validate_non_negative("max_pool_increase_percent", policy.max_pool_increase_percent)?;
    validate_non_negative(
        "max_recommended_p99_queue_wait_ms",
        policy.max_recommended_p99_queue_wait_ms,
    )?;
    validate_non_negative(
        "max_recommended_mean_queue_wait_ms",
        policy.max_recommended_mean_queue_wait_ms,
    )?;
    validate_non_negative("max_recommended_rho", policy.max_recommended_rho)?;
    validate_non_negative(
        "max_current_p99_queue_wait_ms",
        policy.max_current_p99_queue_wait_ms,
    )?;
    validate_non_negative(
        "max_current_mean_queue_wait_ms",
        policy.max_current_mean_queue_wait_ms,
    )?;
    validate_non_negative("max_current_rho", policy.max_current_rho)?;
    Ok(())
}

fn validate_non_negative(name: &str, value: Option<f64>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || value < 0.0 {
        bail!("{name} must be a finite value greater than or equal to 0");
    }
    Ok(())
}

fn check_saturation(observed: SaturationLevel, threshold: SaturationLevel) -> GateCheck {
    let passed = saturation_rank(observed) <= saturation_rank(threshold);
    let severity = if observed == SaturationLevel::Critical {
        GateDecision::Critical
    } else {
        GateDecision::Warning
    };
    GateCheck {
        name: "max_saturation".to_string(),
        passed,
        severity,
        observed: format!("{observed:?}"),
        threshold: format!("{threshold:?}"),
        message: if passed {
            format!("worst saturation {observed:?} is within {threshold:?}")
        } else {
            format!("worst saturation {observed:?} exceeds {threshold:?}")
        },
    }
}

fn check_u32_max(
    name: &str,
    observed: u32,
    threshold: u32,
    severity: GateDecision,
) -> GateCheck {
    let passed = observed <= threshold;
    GateCheck {
        name: name.to_string(),
        passed,
        severity,
        observed: observed.to_string(),
        threshold: threshold.to_string(),
        message: limit_message(name, passed),
    }
}

fn check_f64_max(
    name: &str,
    observed: f64,
    threshold: f64,
    severity: GateDecision,
    suffix: &str,
) -> GateCheck {
    let passed = observed <= threshold;
    GateCheck {
        name: name.to_string(),
        passed,
        severity,
        observed: format_measurement(observed, suffix),
        threshold: format_measurement(threshold, suffix),
        message: limit_message(name, passed),
    }
}

fn check_expected_pool_size(observed: u32, expected: u32) -> GateCheck {
    let passed = observed == expected;
    GateCheck {
        name: "expected_pool_size".to_string(),
        passed,
        severity: GateDecision::Critical,
        observed: observed.to_string(),
        threshold: expected.to_string(),
        message: if passed {
            "recommended pool size matches expected pool size".to_string()
        } else {
            "recommended pool size differs from expected pool size".to_string()
        },
    }
}

fn limit_message(name: &str, passed: bool) -> String {
    if passed {
        format!("{name} is within policy")
    } else {
        format!("{name} exceeds policy")
    }
}

fn format_measurement(value: f64, suffix: &str) -> String {
    if suffix.is_empty() {
        format!("{value:.3}")
    } else {
        format!("{value:.3}{suffix}")
    }
}

fn default_max_saturation() -> SaturationLevel {
    SaturationLevel::Warning
}

fn saturation_rank(value: SaturationLevel) -> u8 {
    match value {
        SaturationLevel::Ok => 0,
        SaturationLevel::Warning => 1,
        SaturationLevel::Critical => 2,
    }
}

fn decision_rank(value: GateDecision) -> u8 {
    match value {
        GateDecision::Pass => 0,
        GateDecision::Warning => 1,
        GateDecision::Critical => 2,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use poolsim_core::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange},
        types::{EvaluationResult, SimulationReport},
    };

    use crate::args::{GateSourceCommands, TelemetryImportArgs};

    use super::*;

    fn evaluation(saturation: SaturationLevel) -> EvaluationResult {
        EvaluationResult {
            pool_size: 8,
            utilisation_rho: 0.70,
            mean_queue_wait_ms: 3.0,
            p99_queue_wait_ms: 12.0,
            saturation,
            warnings: Vec::new(),
        }
    }

    fn report(pool_size: u32, saturation: SaturationLevel) -> SimulationReport {
        SimulationReport {
            optimal_pool_size: pool_size,
            confidence_interval: (pool_size, pool_size),
            cold_start_min_pool_size: pool_size,
            utilisation_rho: 0.74,
            mean_queue_wait_ms: 4.0,
            p99_queue_wait_ms: 24.0,
            saturation,
            sensitivity: Vec::new(),
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn recommendation(recommended_pool_size: u32) -> TelemetryRecommendation {
        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: Some("2026-05-16T00:00:00Z".to_string()),
            diff: PoolRecommendationDiff {
                current_pool_size: 8,
                recommended_pool_size,
                pool_size_delta: i64::from(recommended_pool_size) - 8,
                change: PoolSizeChange::Increase,
                additional_connections_required: recommended_pool_size.saturating_sub(8),
                removable_connections: 0,
                connection_change_percent: ((recommended_pool_size as f64 - 8.0) / 8.0) * 100.0,
                current_evaluation: evaluation(SaturationLevel::Ok),
                recommended_report: report(recommended_pool_size, SaturationLevel::Ok),
            },
        }
    }

    fn gate_args(policy: Option<PathBuf>) -> GateArgs {
        GateArgs {
            policy,
            max_saturation: None,
            max_pool_increase_percent: None,
            max_additional_connections: None,
            max_recommended_pool_size: None,
            max_recommended_p99_queue_wait_ms: None,
            max_recommended_mean_queue_wait_ms: None,
            max_recommended_rho: None,
            max_current_p99_queue_wait_ms: None,
            max_current_mean_queue_wait_ms: None,
            max_current_rho: None,
            expected_pool_size: None,
            source: GateSourceCommands::Telemetry(TelemetryImportArgs {
                config: PathBuf::from("telemetry.json"),
                current_pool_size: None,
            }),
        }
    }

    #[test]
    fn default_policy_allows_warning_saturation() {
        let policy = GatePolicy::default();
        assert_eq!(policy.max_saturation, SaturationLevel::Warning);
        assert!(policy.max_pool_increase_percent.is_none());
    }

    #[test]
    fn policy_from_args_applies_overrides_and_validates() {
        let mut args = gate_args(None);
        args.max_saturation = Some(crate::args::CliSaturationLevel::Ok);
        args.max_pool_increase_percent = Some(10.0);
        args.max_additional_connections = Some(2);
        args.max_recommended_pool_size = Some(10);
        args.max_recommended_p99_queue_wait_ms = Some(30.0);
        args.max_recommended_mean_queue_wait_ms = Some(5.0);
        args.max_recommended_rho = Some(0.8);
        args.max_current_p99_queue_wait_ms = Some(40.0);
        args.max_current_mean_queue_wait_ms = Some(6.0);
        args.max_current_rho = Some(0.9);
        args.expected_pool_size = Some(9);

        let policy = policy_from_args(&args).expect("policy should resolve");
        assert_eq!(policy.max_saturation, SaturationLevel::Ok);
        assert_eq!(policy.max_pool_increase_percent, Some(10.0));
        assert_eq!(policy.max_additional_connections, Some(2));
        assert_eq!(policy.max_recommended_pool_size, Some(10));
        assert_eq!(policy.max_recommended_p99_queue_wait_ms, Some(30.0));
        assert_eq!(policy.max_recommended_mean_queue_wait_ms, Some(5.0));
        assert_eq!(policy.max_recommended_rho, Some(0.8));
        assert_eq!(policy.max_current_p99_queue_wait_ms, Some(40.0));
        assert_eq!(policy.max_current_mean_queue_wait_ms, Some(6.0));
        assert_eq!(policy.max_current_rho, Some(0.9));
        assert_eq!(policy.expected_pool_size, Some(9));

        args.max_recommended_rho = Some(f64::NAN);
        assert!(policy_from_args(&args).is_err());
        args.max_recommended_rho = None;
        args.max_current_rho = Some(f64::INFINITY);
        assert!(policy_from_args(&args).is_err());
    }

    #[test]
    fn policy_files_support_json_toml_and_errors() {
        let dir = std::env::temp_dir();
        let base = format!("poolsim_gate_policy_{}", std::process::id());
        let toml_path = dir.join(format!("{base}.toml"));
        let json_path = dir.join(format!("{base}.json"));
        let bad_path = dir.join(format!("{base}.txt"));
        fs::write(
            &toml_path,
            "max_saturation = \"Ok\"\nmax_pool_increase_percent = 15\n",
        )
        .expect("toml policy should write");
        fs::write(&json_path, r#"{"max_saturation":"Critical"}"#)
            .expect("json policy should write");
        fs::write(&bad_path, "{}").expect("bad extension policy should write");

        assert_eq!(
            load_policy(&toml_path)
                .expect("toml policy should parse")
                .max_saturation,
            SaturationLevel::Ok
        );
        assert_eq!(
            load_policy(&json_path)
                .expect("json policy should parse")
                .max_saturation,
            SaturationLevel::Critical
        );
        assert!(load_policy(&bad_path).is_err());
        assert!(load_policy(&dir.join(format!("{base}.missing"))).is_err());

        let _ = fs::remove_file(toml_path);
        let _ = fs::remove_file(json_path);
        let _ = fs::remove_file(bad_path);
    }

    #[test]
    fn policy_from_args_loads_policy_file() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "poolsim_gate_args_policy_{}.toml",
            std::process::id()
        ));
        fs::write(&path, "max_saturation = \"Critical\"\n")
            .expect("gate policy file should write");

        let policy = policy_from_args(&gate_args(Some(path.clone()))).expect("policy should load");
        assert_eq!(policy.max_saturation, SaturationLevel::Critical);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn gate_report_passes_when_all_checks_pass() {
        let policy = GatePolicy {
            max_saturation: SaturationLevel::Warning,
            max_pool_increase_percent: Some(25.0),
            max_additional_connections: Some(2),
            max_recommended_pool_size: Some(10),
            max_recommended_p99_queue_wait_ms: Some(30.0),
            max_recommended_mean_queue_wait_ms: Some(5.0),
            max_recommended_rho: Some(0.80),
            max_current_p99_queue_wait_ms: Some(20.0),
            max_current_mean_queue_wait_ms: Some(5.0),
            max_current_rho: Some(0.80),
            expected_pool_size: Some(10),
        };

        let report = build_gate_report(recommendation(10), &policy);
        assert_eq!(report.status, GateDecision::Pass);
        let _ = report.status.exit_code();
        assert!(report.checks.iter().all(|check| check.passed));
    }

    #[test]
    fn gate_report_warns_for_warning_saturation_above_ok_budget() {
        let policy = GatePolicy {
            max_saturation: SaturationLevel::Ok,
            ..GatePolicy::default()
        };
        let mut recommendation = recommendation(8);
        recommendation.diff.recommended_report.saturation = SaturationLevel::Warning;

        let report = build_gate_report(recommendation, &policy);
        assert_eq!(report.status, GateDecision::Warning);
        let _ = report.status.exit_code();
        assert!(report.checks[0].message.contains("exceeds"));
    }

    #[test]
    fn gate_report_fails_for_critical_saturation_above_budget() {
        let policy = GatePolicy {
            max_saturation: SaturationLevel::Warning,
            ..GatePolicy::default()
        };
        let mut recommendation = recommendation(8);
        recommendation.diff.recommended_report.saturation = SaturationLevel::Critical;

        let report = build_gate_report(recommendation, &policy);
        assert_eq!(report.status, GateDecision::Critical);
        assert_eq!(report.checks[0].severity, GateDecision::Critical);
    }

    #[test]
    fn gate_report_fails_for_critical_policy_breaches() {
        let policy = GatePolicy {
            max_saturation: SaturationLevel::Warning,
            max_pool_increase_percent: Some(5.0),
            max_additional_connections: Some(0),
            max_recommended_pool_size: Some(8),
            max_recommended_p99_queue_wait_ms: Some(1.0),
            max_recommended_mean_queue_wait_ms: Some(1.0),
            max_recommended_rho: Some(0.1),
            max_current_p99_queue_wait_ms: Some(1.0),
            max_current_mean_queue_wait_ms: Some(1.0),
            max_current_rho: Some(0.1),
            expected_pool_size: Some(8),
        };

        let report = build_gate_report(recommendation(10), &policy);
        assert_eq!(report.status, GateDecision::Critical);
        let _ = report.status.exit_code();
        assert!(report.checks.iter().any(|check| !check.passed));
    }

    #[test]
    fn helpers_cover_formatting_and_rank_paths() {
        assert_eq!(format_measurement(1.25, ""), "1.250");
        assert_eq!(format_measurement(1.25, "ms"), "1.250ms");
        assert_eq!(decision_rank(GateDecision::Pass), 0);
        assert_eq!(decision_rank(GateDecision::Warning), 1);
        assert_eq!(decision_rank(GateDecision::Critical), 2);
        let _ = GateDecision::Pass.exit_code();
        let _ = GateDecision::Warning.exit_code();
        let _ = GateDecision::Critical.exit_code();
        assert_eq!(saturation_rank(SaturationLevel::Ok), 0);
        assert_eq!(saturation_rank(SaturationLevel::Warning), 1);
        assert_eq!(saturation_rank(SaturationLevel::Critical), 2);
    }
}
