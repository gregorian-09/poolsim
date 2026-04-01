#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/poolsim-cli/0.1.0")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#![deny(missing_docs)]

mod args;
mod config;
mod render;

use std::process::ExitCode;

use anyhow::{Context, Result};
use args::{Cli, Commands, OutputFormat};
use clap::Parser;
use poolsim_core::{
    evaluate, simulate, sweep_with_options,
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
        let mut args: Vec<String> =
            serde_json::from_str(&raw).context("POOLSIM_TEST_ARGS_JSON must be a JSON array of strings")?;
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
                Ok(exit_code_for_worst_risk(worst_risk(&rows), cli.warn_exit))
            } else {
                let input = config::resolve_simulation_input(&args.common)?;
                if let Some(pool_size) = args.pool_size {
                    let result = evaluate(&input.workload, pool_size, &input.options)?;
                    render_evaluation(&result, cli.format)?;
                    Ok(exit_code_for_saturation(result.saturation, cli.warn_exit))
                } else {
                    let report = simulate(&input.workload, &input.pool, &input.options)?;
                    render_simulation(&report, cli.format)?;
                    Ok(exit_code_for_saturation(report.saturation, cli.warn_exit))
                }
            }
        }
        Commands::Evaluate(args) => {
            let input = config::resolve_evaluate_input(&args)?;
            let result = evaluate(&input.workload, args.pool_size, &input.options)?;
            render_evaluation(&result, cli.format)?;
            Ok(exit_code_for_saturation(result.saturation, cli.warn_exit))
        }
        Commands::Sweep(args) => {
            let input = config::resolve_sweep_input(&args)?;
            let rows = sweep_with_options(&input.workload, &input.pool, &input.options)?;
            render_sweep(&rows, cli.format)?;
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
    }
}

fn render_simulation(report: &SimulationReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::simulation(report),
        OutputFormat::Json => render::json::print(report),
        OutputFormat::Csv => render::csv::simulation(report),
    }
}

fn render_evaluation(result: &EvaluationResult, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::evaluation(result),
        OutputFormat::Json => render::json::print(result),
        OutputFormat::Csv => render::csv::evaluation(result),
    }
}

fn render_sweep(rows: &[SensitivityRow], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::sweep(rows),
        OutputFormat::Json => render::json::print(rows),
        OutputFormat::Csv => render::csv::sweep(rows),
    }
}

fn render_batch(reports: &[SimulationReport], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => render::table::batch(reports),
        OutputFormat::Json => render::json::print(reports),
        OutputFormat::Csv => render::csv::batch(reports),
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
    rows.iter().map(|row| risk_severity(row.risk)).max().unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    use poolsim_core::types::{RiskLevel, StepLoadResult};

    use super::*;
    use crate::args::{BatchArgs, CommonArgs, EvaluateArgs, SimulateArgs};

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

    fn unique_temp_path(name: &str, ext: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("poolsim_cli_main_{name}_{}_{}.{}", std::process::id(), ts, ext))
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
            idle_timeout_ms: None,
            min: None,
            max: None,
            iterations: None,
            seed: None,
            distribution: None,
            queue_model: None,
            target_wait_p99_ms: None,
            max_acceptable_rho: None,
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

        render_evaluation(&evaluation, OutputFormat::Json).expect("json evaluation should render");
        render_evaluation(&evaluation, OutputFormat::Csv).expect("csv evaluation should render");
        render_evaluation(&evaluation, OutputFormat::Table).expect("table evaluation should render");

        render_sweep(&rows, OutputFormat::Json).expect("json sweep should render");
        render_sweep(&rows, OutputFormat::Csv).expect("csv sweep should render");
        render_sweep(&rows, OutputFormat::Table).expect("table sweep should render");

        render_batch(&reports, OutputFormat::Json).expect("json batch should render");
        render_batch(&reports, OutputFormat::Csv).expect("csv batch should render");
        render_batch(&reports, OutputFormat::Table).expect("table batch should render");
    }

    #[test]
    fn run_with_cli_covers_command_paths() {
        let cfg = write_temp_file("main_cfg", "json", &sample_config_json());

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: common_with_config(&cfg),
                pool_size: None,
                sweep: false,
            }),
            format: OutputFormat::Json,
            warn_exit: false,
        };
        let _ = run_with_cli(cli).expect("simulate should execute");

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: common_with_config(&cfg),
                pool_size: Some(8),
                sweep: false,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("simulate --pool-size should execute");

        let cli = Cli {
            command: Commands::Simulate(SimulateArgs {
                common: common_with_config(&cfg),
                pool_size: None,
                sweep: true,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("simulate --sweep should execute");

        let cli = Cli {
            command: Commands::Evaluate(EvaluateArgs {
                common: common_with_config(&cfg),
                pool_size: 9,
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("evaluate should execute");

        let cli = Cli {
            command: Commands::Sweep(common_with_config(&cfg)),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("sweep should execute");

        let batch_cfg = write_temp_file("main_batch", "json", &format!("[{}]", sample_config_json()));
        let cli = Cli {
            command: Commands::Batch(BatchArgs { config: batch_cfg.clone() }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(cli).expect("batch should execute");

        remove_if_exists(&cfg);
        remove_if_exists(&batch_cfg);
    }

    #[test]
    fn batch_command_exit_codes_cover_warning_and_critical_paths() {
        let warning_cfg = write_temp_file("main_batch_warning", "json", &batch_config_json(260.0, 4));
        let warning_cli = Cli {
            command: Commands::Batch(BatchArgs {
                config: warning_cfg.clone(),
            }),
            format: OutputFormat::Json,
            warn_exit: true,
        };
        let _ = run_with_cli(warning_cli).expect("warning batch should execute");

        let critical_cfg = write_temp_file("main_batch_critical", "json", &batch_config_json(2_000.0, 2));
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
