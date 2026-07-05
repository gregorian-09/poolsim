use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::args::{CliConfigFramework, CliDatabaseKind, InitArgs};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct InitReport {
    pub framework: String,
    pub database: String,
    pub config_path: String,
    pub policy_path: String,
    pub expected_rps: f64,
    pub max_server_connections: u32,
    pub min_pool_size: u32,
    pub max_pool_size: u32,
    pub files_written: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SimulationConfigFile {
    workload: WorkloadConfigFile,
    pool: PoolConfigFile,
    options: OptionsConfigFile,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct WorkloadConfigFile {
    requests_per_second: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_p99_ms: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PoolConfigFile {
    max_server_connections: u32,
    connection_overhead_ms: f64,
    idle_timeout_ms: Option<u64>,
    min_pool_size: u32,
    max_pool_size: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct OptionsConfigFile {
    iterations: u32,
    seed: Option<u64>,
    distribution: &'static str,
    queue_model: &'static str,
    target_wait_p99_ms: f64,
    max_acceptable_rho: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct GatePolicyFile {
    max_saturation: &'static str,
    max_recommended_p99_queue_wait_ms: f64,
    max_recommended_rho: f64,
    max_current_p99_queue_wait_ms: f64,
    max_current_rho: f64,
}

pub(crate) fn run(args: &InitArgs) -> Result<InitReport> {
    validate_init_args(args)?;

    let config = SimulationConfigFile {
        workload: WorkloadConfigFile {
            requests_per_second: args.expected_rps,
            latency_p50_ms: args.p50,
            latency_p95_ms: args.p95,
            latency_p99_ms: args.p99,
        },
        pool: PoolConfigFile {
            max_server_connections: args.max_server_connections,
            connection_overhead_ms: args.connection_overhead_ms,
            idle_timeout_ms: args.idle_timeout_ms,
            min_pool_size: args.min,
            max_pool_size: args.max,
        },
        options: OptionsConfigFile {
            iterations: args.iterations,
            seed: args.seed,
            distribution: "LogNormal",
            queue_model: "MMC",
            target_wait_p99_ms: args.target_wait_p99_ms,
            max_acceptable_rho: args.max_acceptable_rho,
        },
    };
    let policy = GatePolicyFile {
        max_saturation: "Warning",
        max_recommended_p99_queue_wait_ms: args.target_wait_p99_ms,
        max_recommended_rho: args.max_acceptable_rho,
        max_current_p99_queue_wait_ms: args.target_wait_p99_ms * 1.5,
        max_current_rho: (args.max_acceptable_rho + 0.05).min(0.99),
    };

    write_new_file(
        &args.output,
        &serde_json::to_string_pretty(&config).context("failed to serialize initial config")?,
        args.force,
    )?;
    write_new_file(
        &args.policy_output,
        &toml::to_string_pretty(&policy).context("failed to serialize initial gate policy")?,
        args.force,
    )?;

    Ok(InitReport {
        framework: framework_name(args.framework).to_string(),
        database: database_name(args.database).to_string(),
        config_path: args.output.display().to_string(),
        policy_path: args.policy_output.display().to_string(),
        expected_rps: args.expected_rps,
        max_server_connections: args.max_server_connections,
        min_pool_size: args.min,
        max_pool_size: args.max,
        files_written: vec![
            args.output.display().to_string(),
            args.policy_output.display().to_string(),
        ],
    })
}

fn validate_init_args(args: &InitArgs) -> Result<()> {
    if args.expected_rps <= 0.0 {
        bail!("--expected-rps must be greater than 0");
    }
    if !(0.0..=args.p95).contains(&args.p50) || args.p95 > args.p99 {
        bail!("latency flags must satisfy 0 <= --p50 <= --p95 <= --p99");
    }
    if args.max_server_connections == 0 {
        bail!("--max-server-connections must be greater than 0");
    }
    if args.min > args.max {
        bail!("--min cannot be greater than --max");
    }
    if args.max > args.max_server_connections {
        bail!("--max cannot be greater than --max-server-connections");
    }
    if args.iterations == 0 {
        bail!("--iterations must be greater than 0");
    }
    if !(0.0..1.0).contains(&args.max_acceptable_rho) {
        bail!("--max-acceptable-rho must be greater than 0 and less than 1");
    }
    Ok(())
}

fn write_new_file(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "refusing to overwrite {}; pass --force to replace it",
            path.display()
        );
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn framework_name(value: CliConfigFramework) -> &'static str {
    match value {
        CliConfigFramework::Hikaricp => "hikaricp",
        CliConfigFramework::SpringBoot => "spring-boot",
        CliConfigFramework::Sqlalchemy => "sqlalchemy",
        CliConfigFramework::Prisma => "prisma",
        CliConfigFramework::NodePg => "node-pg",
        CliConfigFramework::Sqlx => "sqlx",
        CliConfigFramework::Deadpool => "deadpool",
    }
}

fn database_name(value: CliDatabaseKind) -> &'static str {
    match value {
        CliDatabaseKind::Postgres => "postgres",
        CliDatabaseKind::Mysql => "mysql",
        CliDatabaseKind::Sqlite => "sqlite",
        CliDatabaseKind::SqlServer => "sql-server",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str, ext: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("poolsim_init_{name}_{ts}.{ext}"))
    }

    fn args() -> InitArgs {
        InitArgs {
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
            output: temp_path("config", "json"),
            policy_output: temp_path("policy", "toml"),
            force: false,
        }
    }

    #[test]
    fn init_writes_config_and_policy_files() {
        let args = args();
        let report = run(&args).expect("init should write files");
        assert_eq!(report.framework, "sqlx");
        assert_eq!(report.database, "postgres");

        let config_text = fs::read_to_string(&args.output).expect("config should be readable");
        let config: serde_json::Value =
            serde_json::from_str(&config_text).expect("config should be JSON");
        assert_eq!(config["workload"]["requests_per_second"], 180.0);
        assert_eq!(config["pool"]["max_pool_size"], 20);

        let policy_text = fs::read_to_string(&args.policy_output).expect("policy should read");
        let policy: toml::Value = policy_text.parse().expect("policy should be TOML");
        assert_eq!(policy["max_saturation"].as_str(), Some("Warning"));

        let _ = fs::remove_file(args.output);
        let _ = fs::remove_file(args.policy_output);
    }

    #[test]
    fn init_refuses_to_overwrite_without_force() {
        let args = args();
        fs::write(&args.output, "existing").expect("temp file should write");
        let err = run(&args).expect_err("existing file should fail without force");
        assert!(err.to_string().contains("refusing to overwrite"));
        let _ = fs::remove_file(args.output);
        let _ = fs::remove_file(args.policy_output);
    }

    #[test]
    fn init_validates_bounds() {
        let mut args = args();
        args.max = 200;
        let err = run(&args).expect_err("max above server cap should fail");
        assert!(err.to_string().contains("--max cannot be greater"));
    }

    #[test]
    fn init_validation_covers_all_error_branches_and_names() {
        let mut invalid_rps = args();
        invalid_rps.expected_rps = 0.0;
        assert!(run(&invalid_rps)
            .expect_err("zero rps should fail")
            .to_string()
            .contains("--expected-rps"));

        let mut invalid_latency = args();
        invalid_latency.p50 = 31.0;
        assert!(run(&invalid_latency)
            .expect_err("latency ordering should fail")
            .to_string()
            .contains("latency flags"));

        let mut invalid_connections = args();
        invalid_connections.max_server_connections = 0;
        assert!(run(&invalid_connections)
            .expect_err("zero max connections should fail")
            .to_string()
            .contains("--max-server-connections"));

        let mut invalid_bounds = args();
        invalid_bounds.min = 30;
        invalid_bounds.max = 20;
        assert!(run(&invalid_bounds)
            .expect_err("min greater than max should fail")
            .to_string()
            .contains("--min cannot be greater"));

        let mut invalid_iterations = args();
        invalid_iterations.iterations = 0;
        assert!(run(&invalid_iterations)
            .expect_err("zero iterations should fail")
            .to_string()
            .contains("--iterations"));

        let mut invalid_rho = args();
        invalid_rho.max_acceptable_rho = 1.0;
        assert!(run(&invalid_rho)
            .expect_err("invalid rho should fail")
            .to_string()
            .contains("--max-acceptable-rho"));

        assert_eq!(framework_name(CliConfigFramework::Hikaricp), "hikaricp");
        assert_eq!(
            framework_name(CliConfigFramework::SpringBoot),
            "spring-boot"
        );
        assert_eq!(framework_name(CliConfigFramework::Sqlalchemy), "sqlalchemy");
        assert_eq!(framework_name(CliConfigFramework::Prisma), "prisma");
        assert_eq!(framework_name(CliConfigFramework::NodePg), "node-pg");
        assert_eq!(framework_name(CliConfigFramework::Deadpool), "deadpool");
        assert_eq!(database_name(CliDatabaseKind::Mysql), "mysql");
        assert_eq!(database_name(CliDatabaseKind::Sqlite), "sqlite");
        assert_eq!(database_name(CliDatabaseKind::SqlServer), "sql-server");
    }
}
