#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/poolsim-cli/0.3.0")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

mod args;
mod budget;
mod compare;
mod config;
mod config_gen;
mod doctor;
mod explain;
mod gate;
mod guard;
mod init;
mod otlp;
mod prometheus;
mod render;

use std::process::ExitCode;

use anyhow::{Context, Result};
use args::{Cli, Commands, OutputFormat};
use clap::Parser;
use poolsim_core::{
    evaluate, simulate, sweep_with_options,
    telemetry::{recommend_from_telemetry, TelemetryRecommendation},
    types::{EvaluationResult, RiskLevel, SaturationLevel, SensitivityRow, SimulationReport},
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = parse_cli_from_env().context("failed to parse CLI arguments")?;
    run_with_cli(cli)
}

fn parse_cli_from_env() -> Result<Cli> {
    if let Ok(raw) = std::env::var("POOLSIM_TEST_ARGS_JSON") {
        let mut args: Vec<String> = serde_json::from_str(&raw)
            .context("POOLSIM_TEST_ARGS_JSON must be a JSON array of strings")?;
        args.insert(0, "poolsim-cli".to_string());
        return Cli::try_parse_from(args).map_err(|err| anyhow::anyhow!(err.to_string()));
    }

    parse_default_cli_from_process_args()
}

#[cfg(test)]
fn parse_default_cli_from_process_args() -> Result<Cli> {
    Cli::try_parse().map_err(|err| anyhow::anyhow!(err.to_string()))
}

#[cfg(not(test))]
fn parse_default_cli_from_process_args() -> Result<Cli> {
    Ok(Cli::parse())
}

fn run_with_cli(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Simulate(args) => {
            if args.sweep {
                let input = config::resolve_sweep_input(&args.common)?;
                let rows = sweep_with_options(&input.workload, &input.pool, &input.options)?;
                render_sweep(&rows, cli.format)?;
                if args.common.explain {
                    eprintln!("{}", explain::sweep(&rows));
                }
                Ok(exit_code_for_worst_risk(worst_risk(&rows), cli.warn_exit))
            } else {
                let input = config::resolve_simulation_input(&args.common)?;
                if let Some(pool_size) = args.pool_size {
                    let result = evaluate(&input.workload, pool_size, &input.options)?;
                    render_evaluation(&result, cli.format)?;
                    if args.common.explain {
                        eprintln!("{}", explain::evaluation(&input.workload, &result));
                    }
                    Ok(exit_code_for_saturation(result.saturation, cli.warn_exit))
                } else {
                    let report = simulate(&input.workload, &input.pool, &input.options)?;
                    render_simulation(&report, cli.format)?;
                    if args.common.explain {
                        eprintln!("{}", explain::simulation(&input.workload, &report));
                    }
                    Ok(exit_code_for_saturation(report.saturation, cli.warn_exit))
                }
            }
        }
        Commands::Evaluate(args) => {
            let input = config::resolve_evaluate_input(&args)?;
            let result = evaluate(&input.workload, args.pool_size, &input.options)?;
            render_evaluation(&result, cli.format)?;
            if args.common.explain {
                eprintln!("{}", explain::evaluation(&input.workload, &result));
            }
            Ok(exit_code_for_saturation(result.saturation, cli.warn_exit))
        }
        Commands::Sweep(args) => {
            let input = config::resolve_sweep_input(&args)?;
            let rows = sweep_with_options(&input.workload, &input.pool, &input.options)?;
            render_sweep(&rows, cli.format)?;
            if args.explain {
                eprintln!("{}", explain::sweep(&rows));
            }
            Ok(exit_code_for_worst_risk(worst_risk(&rows), cli.warn_exit))
        }
        Commands::Batch(args) => {
            let input = config::resolve_batch_input(&args)?;
            let mut reports = Vec::with_capacity(input.requests.len());
            for req in input.requests {
                let report = simulate(&req.workload, &req.pool, &req.options)?;
                reports.push(report);
            }

            render_batch(&reports, cli.format)?;
            let worst = reports
                .iter()
                .map(|report| exit_severity_for_saturation(report.saturation))
                .max()
                .unwrap_or(0);

            let code = if worst >= 2 {
                ExitCode::from(2)
            } else if cli.warn_exit && worst >= 1 {
                ExitCode::from(3)
            } else {
                ExitCode::from(0)
            };
            Ok(code)
        }
        Commands::Compare(args) => {
            let input = config::resolve_scenario_comparison_input(&args)?;
            let report = compare::build_scenario_comparison_report(input)?;
            render_compare(&report, cli.format)?;
            Ok(exit_code_for_saturation(
                report.worst_saturation,
                cli.warn_exit,
            ))
        }
        Commands::Budget(args) => {
            let input = config::resolve_budget_plan_input(&args)?;
            let report = budget::build_budget_plan_report(input)?;
            render_budget(&report, cli.format)?;
            Ok(exit_code_for_budget_status(report.status, cli.warn_exit))
        }
        Commands::Import(args) => match args.command {
            args::ImportCommands::Telemetry(args) => {
                let input = config::resolve_telemetry_input(&args)?;
                let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                render_telemetry(&recommendation, cli.format)?;
                Ok(exit_code_for_saturation(
                    recommendation.diff.worst_saturation(),
                    cli.warn_exit,
                ))
            }
            args::ImportCommands::Prometheus(args) => {
                let input = prometheus::resolve_prometheus_input(&args)?;
                let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                render_telemetry(&recommendation, cli.format)?;
                Ok(exit_code_for_saturation(
                    recommendation.diff.worst_saturation(),
                    cli.warn_exit,
                ))
            }
            args::ImportCommands::Otlp(args) => {
                let input = otlp::resolve_otlp_input(&args)?;
                let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                render_telemetry(&recommendation, cli.format)?;
                Ok(exit_code_for_saturation(
                    recommendation.diff.worst_saturation(),
                    cli.warn_exit,
                ))
            }
        },
        Commands::Gate(args) => {
            let policy = gate::policy_from_args(&args)?;
            let input = match args.source {
                args::GateSourceCommands::Telemetry(source) => {
                    config::resolve_telemetry_input(&source)?
                }
                args::GateSourceCommands::Prometheus(source) => {
                    prometheus::resolve_prometheus_input(&source)?
                }
                args::GateSourceCommands::Otlp(source) => otlp::resolve_otlp_input(&source)?,
            };
            let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
            let report = gate::build_gate_report(recommendation, &policy);
            render_gate(&report, cli.format)?;
            Ok(report.status.exit_code())
        }
        Commands::Guard(args) => {
            let policy = gate::policy_from_guard_args(&args)?;
            let input = match args.source {
                args::GateSourceCommands::Telemetry(source) => {
                    config::resolve_telemetry_input(&source)?
                }
                args::GateSourceCommands::Prometheus(source) => {
                    prometheus::resolve_prometheus_input(&source)?
                }
                args::GateSourceCommands::Otlp(source) => otlp::resolve_otlp_input(&source)?,
            };
            let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
            let gate_report = gate::build_gate_report(recommendation, &policy);
            let report = guard::build_guard_report(gate_report);
            render_guard(&report, cli.format)?;
            Ok(report.exit_code())
        }
        Commands::Doctor(args) => {
            let input = match args.source {
                args::DoctorSourceCommands::Telemetry(source) => {
                    config::resolve_telemetry_input(&source)?
                }
                args::DoctorSourceCommands::Prometheus(source) => {
                    prometheus::resolve_prometheus_input(&source)?
                }
                args::DoctorSourceCommands::Otlp(source) => otlp::resolve_otlp_input(&source)?,
            };
            let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
            let report = doctor::build_doctor_report(recommendation);
            render_doctor(&report, cli.format)?;
            Ok(report.status.exit_code(cli.warn_exit))
        }
        Commands::Init(args) => {
            let report = init::run(&args)?;
            render_init(&report, cli.format)?;
            Ok(ExitCode::from(0))
        }
        Commands::GenerateConfig(args) => {
            let recommendation = match &args.source {
                args::GenerateConfigSourceCommands::Telemetry(source) => {
                    let input = config::resolve_telemetry_input(source)?;
                    let max_server_connections = input.snapshot.pool.max_server_connections;
                    let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                    config_gen::recommendation_from_telemetry(
                        config_gen::ConfigSourceKind::Telemetry,
                        &recommendation,
                        max_server_connections,
                    )
                }
                args::GenerateConfigSourceCommands::Prometheus(source) => {
                    let input = prometheus::resolve_prometheus_input(source)?;
                    let max_server_connections = input.snapshot.pool.max_server_connections;
                    let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                    config_gen::recommendation_from_telemetry(
                        config_gen::ConfigSourceKind::Prometheus,
                        &recommendation,
                        max_server_connections,
                    )
                }
                args::GenerateConfigSourceCommands::Otlp(source) => {
                    let input = otlp::resolve_otlp_input(source)?;
                    let max_server_connections = input.snapshot.pool.max_server_connections;
                    let recommendation = recommend_from_telemetry(&input.snapshot, &input.options)?;
                    config_gen::recommendation_from_telemetry(
                        config_gen::ConfigSourceKind::Otlp,
                        &recommendation,
                        max_server_connections,
                    )
                }
                args::GenerateConfigSourceCommands::Simulate(source) => {
                    let input = config::resolve_simulation_input(source)?;
                    let report = simulate(&input.workload, &input.pool, &input.options)?;
                    config_gen::recommendation_from_simulation(&report, &input.pool)
                }
            };
            let report = config_gen::build_config_snippet(&args, recommendation);
            render_config_snippet(&report, cli.format)?;
            Ok(ExitCode::from(0))
        }
    }
}

fn render_simulation(report: &SimulationReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::simulation(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::simulation(report),
        OutputFormat::Html => render::html::print("Poolsim simulation report", report),
    }
}

fn render_evaluation(result: &EvaluationResult, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::evaluation(result),
        OutputFormat::Json => render::json::print(result),
        OutputFormat::Csv => render::csv::evaluation(result),
        OutputFormat::Html => render::html::print("Poolsim evaluation report", result),
    }
}

fn render_sweep(rows: &[SensitivityRow], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::sweep(rows),
        OutputFormat::Json => render::json::print(rows),
        OutputFormat::Csv => render::csv::sweep(rows),
        OutputFormat::Html => render::html::print("Poolsim sensitivity report", rows),
    }
}

fn render_batch(reports: &[SimulationReport], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::batch(reports),
        OutputFormat::Json => render::json::print(reports),
        OutputFormat::Csv => render::csv::batch(reports),
        OutputFormat::Html => render::html::print("Poolsim batch report", reports),
    }
}

fn render_compare(report: &compare::ScenarioComparisonReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::compare(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::compare(report),
        OutputFormat::Html => render::html::print("Poolsim scenario comparison report", report),
    }
}

fn render_budget(report: &budget::BudgetPlanReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::budget(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::budget(report),
        OutputFormat::Html => render::html::print("Poolsim database budget report", report),
    }
}

fn render_telemetry(recommendation: &TelemetryRecommendation, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::telemetry(recommendation),
        OutputFormat::Json => render::json::print(recommendation),
        OutputFormat::Csv => render::csv::telemetry(recommendation),
        OutputFormat::Html => {
            render::html::print("Poolsim telemetry recommendation report", recommendation)
        }
    }
}

fn render_gate(report: &gate::GateReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::gate(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::gate(report),
        OutputFormat::Html => render::html::print("Poolsim capacity gate report", report),
    }
}

fn render_guard(report: &guard::GuardReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::guard(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::guard(report),
        OutputFormat::Html => render::html::print("Poolsim deployment guard report", report),
    }
}

fn render_doctor(report: &doctor::DoctorReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::doctor(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::doctor(report),
        OutputFormat::Html => render::html::print("Poolsim doctor report", report),
    }
}

fn render_init(report: &init::InitReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("framework: {}", report.framework);
            println!("database: {}", report.database);
            println!("config_path: {}", report.config_path);
            println!("policy_path: {}", report.policy_path);
            Ok(())
        }
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => {
            println!("field,value");
            println!("framework,{}", report.framework);
            println!("database,{}", report.database);
            println!("config_path,{}", report.config_path);
            println!("policy_path,{}", report.policy_path);
            Ok(())
        }
        OutputFormat::Html => render::html::print("Poolsim init report", report),
    }
}

fn render_config_snippet(
    report: &config_gen::ConfigSnippetReport,
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::config_snippet(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::config_snippet(report),
        OutputFormat::Html => render::html::print("Poolsim generated config report", report),
    }
}

fn exit_code_for_saturation(saturation: SaturationLevel, warn_exit: bool) -> ExitCode {
    match saturation {
        SaturationLevel::Critical => ExitCode::from(2),
        SaturationLevel::Warning if warn_exit => ExitCode::from(3),
        _ => ExitCode::from(0),
    }
}

fn exit_severity_for_saturation(saturation: SaturationLevel) -> u8 {
    match saturation {
        SaturationLevel::Ok => 0,
        SaturationLevel::Warning => 1,
        SaturationLevel::Critical => 2,
    }
}

fn risk_severity(risk: RiskLevel) -> u8 {
    match risk {
        RiskLevel::Low => 0,
        RiskLevel::Medium => 1,
        RiskLevel::High => 2,
        RiskLevel::Critical => 3,
    }
}

fn worst_risk(rows: &[SensitivityRow]) -> u8 {
    rows.iter()
        .map(|row| risk_severity(row.risk))
        .max()
        .unwrap_or(0)
}

fn exit_code_for_worst_risk(worst: u8, warn_exit: bool) -> ExitCode {
    if worst >= 3 {
        ExitCode::from(2)
    } else if warn_exit && worst >= 2 {
        ExitCode::from(3)
    } else {
        ExitCode::from(0)
    }
}

fn exit_code_for_budget_status(status: budget::BudgetStatus, warn_exit: bool) -> ExitCode {
    match status {
        budget::BudgetStatus::Critical => ExitCode::from(2),
        budget::BudgetStatus::Warning if warn_exit => ExitCode::from(3),
        _ => ExitCode::from(0),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    use poolsim_core::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange, TelemetryRecommendation},
        types::{PoolConfig, RiskLevel, SimulationOptions, StepLoadResult, WorkloadConfig},
    };

    use super::*;
    use crate::args::{
        BatchArgs, BudgetArgs, CliConfigFramework, CliDatabaseKind, CommonArgs, CompareArgs,
        DoctorArgs, DoctorSourceCommands, EvaluateArgs, GateArgs, GateSourceCommands,
        GenerateConfigArgs, GenerateConfigSourceCommands, GuardArgs, ImportArgs, ImportCommands,
        InitArgs, OtlpImportArgs, PrometheusImportArgs, SimulateArgs, TelemetryImportArgs,
    };

    fn sample_config_json() -> String {
        r#"
{
  "workload": {
    "requests_per_second": 220.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0,
    "step_load_profile": [
      {"time_s": 0, "requests_per_second": 180.0},
      {"time_s": 30, "requests_per_second": 260.0}
    ]
  },
  "pool": {
    "max_server_connections": 120,
    "connection_overhead_ms": 2.0,
    "min_pool_size": 3,
    "max_pool_size": 24
  },
  "options": {
    "iterations": 1200,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
"#
        .to_string()
    }

    fn batch_config_json(requests_per_second: f64, fixed_pool_size: u32) -> String {
        format!(
            r#"[{{
  "workload": {{
    "requests_per_second": {requests_per_second},
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0
  }},
  "pool": {{
    "max_server_connections": {fixed_pool_size},
    "connection_overhead_ms": 2.0,
    "min_pool_size": {fixed_pool_size},
    "max_pool_size": {fixed_pool_size}
  }},
  "options": {{
    "iterations": 1200,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }}
}}]"#
        )
    }

    fn scenario_comparison_json() -> String {
        r#"
{
  "baseline": "normal",
  "scenarios": [
    {
      "name": "normal",
      "workload": {
        "requests_per_second": 180.0,
        "latency_p50_ms": 7.0,
        "latency_p95_ms": 25.0,
        "latency_p99_ms": 60.0
      },
      "pool": {
        "max_server_connections": 100,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 2,
        "max_pool_size": 20
      },
      "options": {
        "iterations": 1200,
        "seed": 3
      }
    },
    {
      "name": "peak",
      "workload": {
        "requests_per_second": 260.0,
        "latency_p50_ms": 8.0,
        "latency_p95_ms": 30.0,
        "latency_p99_ms": 70.0
      },
      "pool": {
        "max_server_connections": 120,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 3,
        "max_pool_size": 24
      },
      "options": {
        "iterations": 1200,
        "seed": 3
      }
    }
  ]
}
"#
        .to_string()
    }

    fn telemetry_config_json() -> String {
        r#"
{
  "telemetry": {
    "service_name": "checkout-api",
    "window": "1h",
    "current_pool_size": 8,
    "workload": {
      "requests_per_second": 180.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 30.0,
      "latency_p99_ms": 70.0
    },
    "pool": {
      "max_server_connections": 100,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 2,
      "max_pool_size": 20
    }
  },
  "options": {
    "iterations": 1200,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
"#
        .to_string()
    }

    fn budget_plan_json() -> String {
        r#"
{
  "max_connections": 120,
  "reserved_connections": 20,
  "safety_margin_connections": 10,
  "services": [
    {
      "name": "checkout-api",
      "replicas": 6,
      "current_pool_size": 8,
      "min_pool_size": 4,
      "max_pool_size": 12,
      "recommended_pool_size": 10,
      "priority": 5
    },
    {
      "name": "billing-api",
      "replicas": 4,
      "current_pool_size": 6,
      "min_pool_size": 3,
      "max_pool_size": 10,
      "recommended_pool_size": 8,
      "priority": 3
    },
    {
      "name": "admin-api",
      "replicas": 2,
      "current_pool_size": 4,
      "min_pool_size": 2,
      "max_pool_size": 6,
      "recommended_pool_size": 5,
      "priority": 1
    }
  ]
}
"#
        .to_string()
    }

    fn prometheus_response_json() -> String {
        let response = |value: &str| {
            serde_json::json!({
                "status": "success",
                "data": {
                    "resultType": "vector",
                    "result": [
                        {"metric": {}, "value": [1710000000.0, value]}
                    ]
                }
            })
        };
        serde_json::json!({
            "rps": response("180"),
            "p50": response("8"),
            "p95": response("30"),
            "p99": response("70")
        })
        .to_string()
    }

    fn otlp_metrics_json() -> String {
        r#"{
  "resourceMetrics": [{"scopeMetrics": [{"metrics": [
    {"name": "poolsim.rps", "sum": {"dataPoints": [{"asDouble": 180.0}]}},
    {"name": "poolsim.latency.p50_ms", "gauge": {"dataPoints": [{"asDouble": 8.0}]}},
    {"name": "poolsim.latency.p95_ms", "gauge": {"dataPoints": [{"asDouble": 30.0}]}},
    {"name": "poolsim.latency.p99_ms", "gauge": {"dataPoints": [{"asDouble": 70.0}]}}
  ]}]}]
}"#
        .to_string()
    }

    fn unique_temp_path(name: &str, ext: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "poolsim_cli_main_{name}_{}_{}.{}",
            std::process::id(),
            ts,
            ext
        ))
    }

    fn write_temp_file(name: &str, ext: &str, content: &str) -> PathBuf {
        let path = unique_temp_path(name, ext);
        fs::write(&path, content).expect("temp file should be writable");
        path
    }

    fn remove_if_exists(path: &Path) {
        let _ = fs::remove_file(path);
    }

    fn common_with_config(path: &Path) -> CommonArgs {
        CommonArgs {
            config: Some(path.to_path_buf()),
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
        }
    }

    fn explained_common_with_config(path: &Path) -> CommonArgs {
        let mut args = common_with_config(path);
        args.explain = true;
        args
    }

    fn otlp_args(path: &Path) -> OtlpImportArgs {
        OtlpImportArgs {
            config: path.to_path_buf(),
            rps_metric: "poolsim.rps".to_string(),
            p50_metric: "poolsim.latency.p50_ms".to_string(),
            p95_metric: "poolsim.latency.p95_ms".to_string(),
            p99_metric: "poolsim.latency.p99_ms".to_string(),
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: None,
            current_pool_size: 9,
            max_server_connections: 100,
            connection_overhead_ms: 2.0,
            idle_timeout_ms: None,
            min: 2,
            max: 20,
            iterations: Some(1_200),
            seed: Some(7),
            distribution: None,
            queue_model: None,
            target_wait_p99_ms: Some(45.0),
            max_acceptable_rho: Some(0.85),
        }
    }

    fn sample_rows() -> Vec<SensitivityRow> {
        vec![SensitivityRow {
            pool_size: 6,
            utilisation_rho: 0.84,
            mean_queue_wait_ms: 6.0,
            p99_queue_wait_ms: 30.0,
            risk: RiskLevel::Medium,
        }]
    }

    fn sample_report() -> SimulationReport {
        SimulationReport {
            optimal_pool_size: 6,
            confidence_interval: (5, 7),
            cold_start_min_pool_size: 5,
            utilisation_rho: 0.84,
            mean_queue_wait_ms: 6.0,
            p99_queue_wait_ms: 30.0,
            saturation: SaturationLevel::Warning,
            sensitivity: sample_rows(),
            step_load_analysis: vec![StepLoadResult {
                time_s: 0,
                requests_per_second: 210.0,
                utilisation_rho: 0.83,
                p99_queue_wait_ms: 28.0,
                saturation: SaturationLevel::Warning,
            }],
            warnings: Vec::new(),
        }
    }

    fn sample_scenario(name: &str, rps: f64) -> config::ScenarioInput {
        config::ScenarioInput {
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
                seed: Some(7),
                ..SimulationOptions::default()
            },
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn sample_evaluation() -> EvaluationResult {
        EvaluationResult {
            pool_size: 6,
            utilisation_rho: 0.84,
            mean_queue_wait_ms: 6.0,
            p99_queue_wait_ms: 30.0,
            saturation: SaturationLevel::Warning,
            warnings: Vec::new(),
        }
    }

    fn sample_recommendation() -> TelemetryRecommendation {
        TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: None,
            diff: PoolRecommendationDiff {
                current_pool_size: 8,
                recommended_pool_size: 6,
                pool_size_delta: -2,
                change: PoolSizeChange::Decrease,
                additional_connections_required: 0,
                removable_connections: 2,
                connection_change_percent: -25.0,
                current_evaluation: sample_evaluation(),
                recommended_report: sample_report(),
            },
        }
    }

    fn sample_budget_report() -> budget::BudgetPlanReport {
        budget::build_budget_plan_report(config::BudgetPlanInput {
            max_connections: 120,
            reserved_connections: 20,
            safety_margin_connections: 10,
            services: vec![
                config::BudgetServiceInput {
                    name: "checkout-api".to_string(),
                    replicas: 6,
                    current_pool_size: Some(8),
                    min_pool_size: 4,
                    max_pool_size: Some(12),
                    recommended_pool_size: 10,
                    priority: Some(5),
                },
                config::BudgetServiceInput {
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

    #[test]
    fn exit_code_helpers_map_expected_levels() {
        let _ = exit_code_for_saturation(SaturationLevel::Ok, false);
        let _ = exit_code_for_saturation(SaturationLevel::Warning, false);
        let _ = exit_code_for_saturation(SaturationLevel::Warning, true);
        let _ = exit_code_for_saturation(SaturationLevel::Critical, false);

        assert_eq!(exit_severity_for_saturation(SaturationLevel::Ok), 0);
        assert_eq!(exit_severity_for_saturation(SaturationLevel::Warning), 1);
        assert_eq!(exit_severity_for_saturation(SaturationLevel::Critical), 2);
        assert_eq!(risk_severity(RiskLevel::Low), 0);
        assert_eq!(risk_severity(RiskLevel::Medium), 1);
        assert_eq!(risk_severity(RiskLevel::High), 2);
        assert_eq!(risk_severity(RiskLevel::Critical), 3);
        assert_eq!(worst_risk(&[]), 0);
        assert_eq!(
            worst_risk(&[
                SensitivityRow {
                    pool_size: 1,
                    utilisation_rho: 0.5,
                    mean_queue_wait_ms: 1.0,
                    p99_queue_wait_ms: 2.0,
                    risk: RiskLevel::Medium,
                },
                SensitivityRow {
                    pool_size: 2,
                    utilisation_rho: 0.7,
                    mean_queue_wait_ms: 4.0,
                    p99_queue_wait_ms: 10.0,
                    risk: RiskLevel::High,
                },
            ]),
            2
        );
        let _ = exit_code_for_worst_risk(3, false);
        let _ = exit_code_for_worst_risk(2, true);
        let _ = exit_code_for_worst_risk(2, false);
        let _ = exit_code_for_budget_status(budget::BudgetStatus::Pass, false);
        let _ = exit_code_for_budget_status(budget::BudgetStatus::Warning, true);
        let _ = exit_code_for_budget_status(budget::BudgetStatus::Warning, false);
        let _ = exit_code_for_budget_status(budget::BudgetStatus::Critical, false);
    }

    #[test]
    fn render_wrappers_execute_for_all_formats() {
        let report = sample_report();
        let evaluation = sample_evaluation();
        let rows = sample_rows();
        let reports = vec![report.clone(), report.clone()];

        render_simulation(&report, OutputFormat::Json).expect("json simulation should render");
        render_simulation(&report, OutputFormat::Csv).expect("csv simulation should render");
        render_simulation(&report, OutputFormat::Table).expect("table simulation should render");
        render_simulation(&report, OutputFormat::Html).expect("html simulation should render");

        render_evaluation(&evaluation, OutputFormat::Json).expect("json evaluation should render");
        render_evaluation(&evaluation, OutputFormat::Csv).expect("csv evaluation should render");
        render_evaluation(&evaluation, OutputFormat::Table)
            .expect("table evaluation should render");
        render_evaluation(&evaluation, OutputFormat::Html).expect("html evaluation should render");

        render_sweep(&rows, OutputFormat::Json).expect("json sweep should render");
        render_sweep(&rows, OutputFormat::Csv).expect("csv sweep should render");
        render_sweep(&rows, OutputFormat::Table).expect("table sweep should render");
        render_sweep(&rows, OutputFormat::Html).expect("html sweep should render");

        render_batch(&reports, OutputFormat::Json).expect("json batch should render");
        render_batch(&reports, OutputFormat::Csv).expect("csv batch should render");
        render_batch(&reports, OutputFormat::Table).expect("table batch should render");
        render_batch(&reports, OutputFormat::Html).expect("html batch should render");

        let compare_report =
            compare::build_scenario_comparison_report(config::ScenarioComparisonInput {
                baseline: "normal".to_string(),
                scenarios: vec![
                    sample_scenario("normal", 180.0),
                    sample_scenario("peak", 260.0),
                ],
            })
            .expect("compare report should build");
        render_compare(&compare_report, OutputFormat::Json).expect("json compare should render");
        render_compare(&compare_report, OutputFormat::Csv).expect("csv compare should render");
        render_compare(&compare_report, OutputFormat::Table).expect("table compare should render");
        render_compare(&compare_report, OutputFormat::Html).expect("html compare should render");

        let budget_report = sample_budget_report();
        render_budget(&budget_report, OutputFormat::Json).expect("json budget should render");
        render_budget(&budget_report, OutputFormat::Csv).expect("csv budget should render");
        render_budget(&budget_report, OutputFormat::Table).expect("table budget should render");
        render_budget(&budget_report, OutputFormat::Html).expect("html budget should render");

        let recommendation = sample_recommendation();
        render_telemetry(&recommendation, OutputFormat::Json)
            .expect("json telemetry should render");
        render_telemetry(&recommendation, OutputFormat::Csv).expect("csv telemetry should render");
        render_telemetry(&recommendation, OutputFormat::Table)
            .expect("table telemetry should render");
        render_telemetry(&recommendation, OutputFormat::Html)
            .expect("html telemetry should render");

        let gate_report = gate::build_gate_report(recommendation, &gate::GatePolicy::default());
        render_gate(&gate_report, OutputFormat::Json).expect("json gate should render");
        render_gate(&gate_report, OutputFormat::Csv).expect("csv gate should render");
        render_gate(&gate_report, OutputFormat::Table).expect("table gate should render");
        render_gate(&gate_report, OutputFormat::Html).expect("html gate should render");

        let guard_report = guard::build_guard_report(gate_report);
        render_guard(&guard_report, OutputFormat::Json).expect("json guard should render");
        render_guard(&guard_report, OutputFormat::Csv).expect("csv guard should render");
        render_guard(&guard_report, OutputFormat::Table).expect("table guard should render");
        render_guard(&guard_report, OutputFormat::Html).expect("html guard should render");

        let doctor_report = doctor::build_doctor_report(sample_recommendation());
        render_doctor(&doctor_report, OutputFormat::Json).expect("json doctor should render");
        render_doctor(&doctor_report, OutputFormat::Csv).expect("csv doctor should render");
        render_doctor(&doctor_report, OutputFormat::Table).expect("table doctor should render");
        render_doctor(&doctor_report, OutputFormat::Html).expect("html doctor should render");

        let init_report = init::InitReport {
            framework: "sqlx".to_string(),
            database: "postgres".to_string(),
            config_path: "poolsim.json".to_string(),
            policy_path: "poolsim-gate-policy.toml".to_string(),
            expected_rps: 180.0,
            max_server_connections: 100,
            min_pool_size: 2,
            max_pool_size: 20,
            files_written: vec![
                "poolsim.json".to_string(),
                "poolsim-gate-policy.toml".to_string(),
            ],
        };
        render_init(&init_report, OutputFormat::Table).expect("table init should render");
        render_init(&init_report, OutputFormat::Json).expect("json init should render");
        render_init(&init_report, OutputFormat::Csv).expect("csv init should render");
        render_init(&init_report, OutputFormat::Html).expect("html init should render");

        let config_report = config_gen::build_config_snippet(
            &GenerateConfigArgs {
                framework: CliConfigFramework::Sqlalchemy,
                min_idle: Some(2),
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: GenerateConfigSourceCommands::Simulate(common_with_config(Path::new(
                    "unused.json",
                ))),
            },
            config_gen::ConfigRecommendation {
                source: config_gen::ConfigSourceKind::Simulate,
                service_name: Some("checkout-api".to_string()),
                window: Some("1h".to_string()),
                observed_at: None,
                recommended_pool_size: 8,
                cold_start_min_pool_size: 3,
                max_server_connections: 100,
                utilisation_rho: 0.72,
                mean_queue_wait_ms: 3.0,
                p99_queue_wait_ms: 12.0,
            },
        );
        render_config_snippet(&config_report, OutputFormat::Json)
            .expect("json config snippet should render");
        render_config_snippet(&config_report, OutputFormat::Csv)
            .expect("csv config snippet should render");
        render_config_snippet(&config_report, OutputFormat::Table)
            .expect("table config snippet should render");
        render_config_snippet(&config_report, OutputFormat::Html)
            .expect("html config snippet should render");
    }

    #[test]
    fn run_with_cli_covers_command_paths() {
        let cfg = write_temp_file("main_cfg", "json", &sample_config_json());

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: explained_common_with_config(&cfg),
                pool_size: None,
                sweep: false,
            }),
            format: OutputFormat::Json,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("simulate should execute");

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: explained_common_with_config(&cfg),
                pool_size: Some(8),
                sweep: false,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("simulate --pool-size should execute");

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: explained_common_with_config(&cfg),
                pool_size: None,
                sweep: true,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("simulate --sweep should execute");

        let cli = Cli {
            command: Commands::Evaluate(EvaluateArgs {
                common: explained_common_with_config(&cfg),
                pool_size: 9,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("evaluate should execute");

        let cli = Cli {
            command: Commands::Sweep(explained_common_with_config(&cfg)),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("sweep should execute");

        let batch_cfg =
            write_temp_file("main_batch", "json", &format!("[{}]", sample_config_json()));
        let cli = Cli {
            command: Commands::Batch(BatchArgs {
                config: batch_cfg.clone(),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("batch should execute");

        let compare_cfg = write_temp_file("main_compare", "json", &scenario_comparison_json());
        let cli = Cli {
            command: Commands::Compare(CompareArgs {
                config: compare_cfg.clone(),
                baseline: Some("normal".to_string()),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("compare should execute");

        let budget_cfg = write_temp_file("main_budget", "json", &budget_plan_json());
        let cli = Cli {
            command: Commands::Budget(BudgetArgs {
                config: budget_cfg.clone(),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("budget should execute");

        let telemetry_cfg = write_temp_file("main_telemetry", "json", &telemetry_config_json());
        let cli = Cli {
            command: Commands::Import(ImportArgs {
                command: ImportCommands::Telemetry(TelemetryImportArgs {
                    config: telemetry_cfg.clone(),
                    current_pool_size: Some(9),
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("telemetry import should execute");

        let prometheus_cfg =
            write_temp_file("main_prometheus", "json", &prometheus_response_json());
        let cli = Cli {
            command: Commands::Import(ImportArgs {
                command: ImportCommands::Prometheus(PrometheusImportArgs {
                    endpoint: None,
                    response_file: Some(prometheus_cfg.clone()),
                    rps_query: None,
                    p50_query: None,
                    p95_query: None,
                    p99_query: None,
                    header: Vec::new(),
                    service_name: Some("checkout-api".to_string()),
                    window: Some("5m".to_string()),
                    observed_at: None,
                    current_pool_size: 9,
                    max_server_connections: 100,
                    connection_overhead_ms: 2.0,
                    idle_timeout_ms: None,
                    min: 2,
                    max: 20,
                    iterations: Some(1_200),
                    seed: Some(7),
                    distribution: None,
                    queue_model: None,
                    target_wait_p99_ms: None,
                    max_acceptable_rho: None,
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("prometheus import should execute");

        let otlp_cfg = write_temp_file("main_otlp", "json", &otlp_metrics_json());
        let cli = Cli {
            command: Commands::Import(ImportArgs {
                command: ImportCommands::Otlp(otlp_args(&otlp_cfg)),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("otlp import should execute");

        let cli = Cli {
            command: Commands::Gate(GateArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Telemetry(TelemetryImportArgs {
                    config: telemetry_cfg.clone(),
                    current_pool_size: Some(9),
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("gate telemetry should execute");

        let cli = Cli {
            command: Commands::Gate(GateArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Prometheus(PrometheusImportArgs {
                    endpoint: None,
                    response_file: Some(prometheus_cfg.clone()),
                    rps_query: None,
                    p50_query: None,
                    p95_query: None,
                    p99_query: None,
                    header: Vec::new(),
                    service_name: Some("checkout-api".to_string()),
                    window: Some("5m".to_string()),
                    observed_at: None,
                    current_pool_size: 9,
                    max_server_connections: 100,
                    connection_overhead_ms: 2.0,
                    idle_timeout_ms: None,
                    min: 2,
                    max: 20,
                    iterations: Some(1_200),
                    seed: Some(7),
                    distribution: None,
                    queue_model: None,
                    target_wait_p99_ms: None,
                    max_acceptable_rho: None,
                }),
            }),
            format: OutputFormat::Csv,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("gate prometheus should execute");

        let cli = Cli {
            command: Commands::Gate(GateArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Otlp(otlp_args(&otlp_cfg)),
            }),
            format: OutputFormat::Html,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("gate otlp should execute");

        let cli = Cli {
            command: Commands::Guard(GuardArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Telemetry(TelemetryImportArgs {
                    config: telemetry_cfg.clone(),
                    current_pool_size: Some(9),
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("guard telemetry should execute");

        let cli = Cli {
            command: Commands::Guard(GuardArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Prometheus(PrometheusImportArgs {
                    endpoint: None,
                    response_file: Some(prometheus_cfg.clone()),
                    rps_query: None,
                    p50_query: None,
                    p95_query: None,
                    p99_query: None,
                    header: Vec::new(),
                    service_name: Some("checkout-api".to_string()),
                    window: Some("5m".to_string()),
                    observed_at: None,
                    current_pool_size: 9,
                    max_server_connections: 100,
                    connection_overhead_ms: 2.0,
                    idle_timeout_ms: None,
                    min: 2,
                    max: 20,
                    iterations: Some(1_200),
                    seed: Some(7),
                    distribution: None,
                    queue_model: None,
                    target_wait_p99_ms: None,
                    max_acceptable_rho: None,
                }),
            }),
            format: OutputFormat::Csv,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("guard prometheus should execute");

        let cli = Cli {
            command: Commands::Guard(GuardArgs {
                policy: None,
                max_saturation: None,
                max_pool_increase_percent: Some(100.0),
                max_additional_connections: Some(10),
                max_recommended_pool_size: Some(20),
                max_recommended_p99_queue_wait_ms: Some(100.0),
                max_recommended_mean_queue_wait_ms: Some(20.0),
                max_recommended_rho: Some(1.0),
                max_current_p99_queue_wait_ms: Some(100.0),
                max_current_mean_queue_wait_ms: Some(20.0),
                max_current_rho: Some(1.0),
                expected_pool_size: None,
                source: GateSourceCommands::Otlp(otlp_args(&otlp_cfg)),
            }),
            format: OutputFormat::Html,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("guard otlp should execute");

        let cli = Cli {
            command: Commands::Doctor(DoctorArgs {
                source: DoctorSourceCommands::Telemetry(TelemetryImportArgs {
                    config: telemetry_cfg.clone(),
                    current_pool_size: Some(9),
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("doctor telemetry should execute");

        let cli = Cli {
            command: Commands::Doctor(DoctorArgs {
                source: DoctorSourceCommands::Prometheus(PrometheusImportArgs {
                    endpoint: None,
                    response_file: Some(prometheus_cfg.clone()),
                    rps_query: None,
                    p50_query: None,
                    p95_query: None,
                    p99_query: None,
                    header: Vec::new(),
                    service_name: Some("checkout-api".to_string()),
                    window: Some("5m".to_string()),
                    observed_at: None,
                    current_pool_size: 9,
                    max_server_connections: 100,
                    connection_overhead_ms: 2.0,
                    idle_timeout_ms: None,
                    min: 2,
                    max: 20,
                    iterations: Some(1_200),
                    seed: Some(7),
                    distribution: None,
                    queue_model: None,
                    target_wait_p99_ms: None,
                    max_acceptable_rho: None,
                }),
            }),
            format: OutputFormat::Csv,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("doctor prometheus should execute");

        let cli = Cli {
            command: Commands::Doctor(DoctorArgs {
                source: DoctorSourceCommands::Otlp(otlp_args(&otlp_cfg)),
            }),
            format: OutputFormat::Html,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("doctor otlp should execute");

        let init_config = unique_temp_path("main_init_config", "json");
        let init_policy = unique_temp_path("main_init_policy", "toml");
        let cli = Cli {
            command: Commands::Init(InitArgs {
                framework: CliConfigFramework::Sqlx,
                database: CliDatabaseKind::Postgres,
                expected_rps: 180.0,
                p50: 8.0,
                p95: 30.0,
                p99: 70.0,
                max_server_connections: 100,
                connection_overhead_ms: 2.0,
                idle_timeout_ms: Some(120_000),
                min: 2,
                max: 20,
                iterations: 1_200,
                seed: Some(7),
                target_wait_p99_ms: 45.0,
                max_acceptable_rho: 0.85,
                output: init_config.clone(),
                policy_output: init_policy.clone(),
                force: true,
            }),
            format: OutputFormat::Html,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("init should execute");

        let cli = Cli {
            command: Commands::GenerateConfig(GenerateConfigArgs {
                framework: CliConfigFramework::Sqlx,
                min_idle: Some(3),
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: GenerateConfigSourceCommands::Telemetry(TelemetryImportArgs {
                    config: telemetry_cfg.clone(),
                    current_pool_size: Some(9),
                }),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("generate-config telemetry should execute");

        let cli = Cli {
            command: Commands::GenerateConfig(GenerateConfigArgs {
                framework: CliConfigFramework::SpringBoot,
                min_idle: None,
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: GenerateConfigSourceCommands::Prometheus(PrometheusImportArgs {
                    endpoint: None,
                    response_file: Some(prometheus_cfg.clone()),
                    rps_query: None,
                    p50_query: None,
                    p95_query: None,
                    p99_query: None,
                    header: Vec::new(),
                    service_name: Some("checkout-api".to_string()),
                    window: Some("5m".to_string()),
                    observed_at: None,
                    current_pool_size: 9,
                    max_server_connections: 100,
                    connection_overhead_ms: 2.0,
                    idle_timeout_ms: None,
                    min: 2,
                    max: 20,
                    iterations: Some(1_200),
                    seed: Some(7),
                    distribution: None,
                    queue_model: None,
                    target_wait_p99_ms: None,
                    max_acceptable_rho: None,
                }),
            }),
            format: OutputFormat::Csv,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("generate-config prometheus should execute");

        let cli = Cli {
            command: Commands::GenerateConfig(GenerateConfigArgs {
                framework: CliConfigFramework::Deadpool,
                min_idle: None,
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: GenerateConfigSourceCommands::Otlp(otlp_args(&otlp_cfg)),
            }),
            format: OutputFormat::Html,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("generate-config otlp should execute");

        let cli = Cli {
            command: Commands::GenerateConfig(GenerateConfigArgs {
                framework: CliConfigFramework::NodePg,
                min_idle: None,
                connection_timeout_ms: 30_000,
                idle_timeout_ms: 600_000,
                database_url_env: "DATABASE_URL".to_string(),
                pool_name: "checkout-pool".to_string(),
                source: GenerateConfigSourceCommands::Simulate(common_with_config(&cfg)),
            }),
            format: OutputFormat::Table,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("generate-config simulate should execute");

        remove_if_exists(&cfg);
        remove_if_exists(&batch_cfg);
        remove_if_exists(&compare_cfg);
        remove_if_exists(&budget_cfg);
        remove_if_exists(&telemetry_cfg);
        remove_if_exists(&prometheus_cfg);
        remove_if_exists(&otlp_cfg);
        remove_if_exists(&init_config);
        remove_if_exists(&init_policy);
    }

    #[test]
    fn batch_command_exit_codes_cover_warning_and_critical_paths() {
        let warning_cfg =
            write_temp_file("main_batch_warning", "json", &batch_config_json(260.0, 4));
        let warning_cli = Cli {
            command: Commands::Batch(BatchArgs {
                config: warning_cfg.clone(),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(warning_cli).expect("warning batch should execute");

        let critical_cfg = write_temp_file(
            "main_batch_critical",
            "json",
            &batch_config_json(2_000.0, 2),
        );
        let critical_cli = Cli {
            command: Commands::Batch(BatchArgs {
                config: critical_cfg.clone(),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(critical_cli).expect("critical batch should execute");

        remove_if_exists(&warning_cfg);
        remove_if_exists(&critical_cfg);
    }

    #[test]
    fn entrypoint_helpers_cover_parse_and_main_error_paths() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        let cfg = write_temp_file("main_entry_cfg", "json", &sample_config_json());
        let args = serde_json::json!([
            "--format",
            "json",
            "simulate",
            "--config",
            cfg.to_string_lossy()
        ]);
        std::env::set_var("POOLSIM_TEST_ARGS_JSON", args.to_string());
        let _ = run().expect("run should succeed with explicit test args");
        let _ = main();

        std::env::set_var("POOLSIM_TEST_ARGS_JSON", "{not-valid-json");
        let _ = main();

        std::env::remove_var("POOLSIM_TEST_ARGS_JSON");
        let _ = parse_cli_from_env();
        remove_if_exists(&cfg);
    }
}
