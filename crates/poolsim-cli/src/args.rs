use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use poolsim_core::types::{DistributionModel, QueueModel, SaturationLevel};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Html,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliDistributionModel {
    LogNormal,
    Exponential,
    Empirical,
    Gamma,
}

impl From<CliDistributionModel> for DistributionModel {
    fn from(value: CliDistributionModel) -> Self {
        match value {
            CliDistributionModel::LogNormal => DistributionModel::LogNormal,
            CliDistributionModel::Exponential => DistributionModel::Exponential,
            CliDistributionModel::Empirical => DistributionModel::Empirical,
            CliDistributionModel::Gamma => DistributionModel::Gamma,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliQueueModel {
    Mmc,
    Mdc,
}

impl From<CliQueueModel> for QueueModel {
    fn from(value: CliQueueModel) -> Self {
        match value {
            CliQueueModel::Mmc => QueueModel::MMC,
            CliQueueModel::Mdc => QueueModel::MDC,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "poolsim", version, about = "Connection pool sizing simulator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, value_enum, default_value = "table")]
    pub format: OutputFormat,

    #[arg(long, global = true)]
    pub warn_exit: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Simulate(SimulateArgs),
    Evaluate(EvaluateArgs),
    Sweep(CommonArgs),
    Batch(BatchArgs),
    Compare(CompareArgs),
    Budget(BudgetArgs),
    Import(ImportArgs),
    Gate(GateArgs),
    Guard(GuardArgs),
    Doctor(DoctorArgs),
    GenerateConfig(GenerateConfigArgs),
    Init(InitArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliConfigFramework {
    Hikaricp,
    SpringBoot,
    Sqlalchemy,
    Prisma,
    NodePg,
    Sqlx,
    Deadpool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliDatabaseKind {
    Postgres,
    Mysql,
    Sqlite,
    SqlServer,
}

#[derive(Debug, Clone, Args)]
pub struct InitArgs {
    #[arg(long, value_enum, default_value = "sqlx")]
    pub framework: CliConfigFramework,

    #[arg(long, value_enum, default_value = "postgres")]
    pub database: CliDatabaseKind,

    #[arg(long, default_value_t = 180.0)]
    pub expected_rps: f64,
    #[arg(long, default_value_t = 8.0)]
    pub p50: f64,
    #[arg(long, default_value_t = 30.0)]
    pub p95: f64,
    #[arg(long, default_value_t = 70.0)]
    pub p99: f64,

    #[arg(long, default_value_t = 100)]
    pub max_server_connections: u32,
    #[arg(
        long,
        alias = "connection-establishment-overhead-ms",
        default_value_t = 0.0
    )]
    pub connection_overhead_ms: f64,
    #[arg(long)]
    pub idle_timeout_ms: Option<u64>,
    #[arg(long, default_value_t = 2)]
    pub min: u32,
    #[arg(long, default_value_t = 20)]
    pub max: u32,

    #[arg(long, default_value_t = 10_000)]
    pub iterations: u32,
    #[arg(long)]
    pub seed: Option<u64>,
    #[arg(long, default_value_t = 45.0)]
    pub target_wait_p99_ms: f64,
    #[arg(long, default_value_t = 0.85)]
    pub max_acceptable_rho: f64,

    #[arg(long, default_value = "poolsim.json")]
    pub output: PathBuf,
    #[arg(long, default_value = "poolsim-gate-policy.toml")]
    pub policy_output: PathBuf,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct SimulateArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long)]
    pub pool_size: Option<u32>,

    #[arg(long, conflicts_with = "pool_size")]
    pub sweep: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliConnectionOverheadProfile {
    Postgres,
    Mysql,
    SqlServer,
    Sqlite,
    PgBouncer,
    RdsProxy,
}

impl From<CliConnectionOverheadProfile> for poolsim_core::ConnectionOverheadProfile {
    fn from(value: CliConnectionOverheadProfile) -> Self {
        match value {
            CliConnectionOverheadProfile::Postgres => Self::Postgres,
            CliConnectionOverheadProfile::Mysql => Self::Mysql,
            CliConnectionOverheadProfile::SqlServer => Self::SqlServer,
            CliConnectionOverheadProfile::Sqlite => Self::Sqlite,
            CliConnectionOverheadProfile::PgBouncer => Self::PgBouncer,
            CliConnectionOverheadProfile::RdsProxy => Self::RdsProxy,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct CommonArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub rps: Option<f64>,
    #[arg(long)]
    pub p50: Option<f64>,
    #[arg(long)]
    pub p95: Option<f64>,
    #[arg(long)]
    pub p99: Option<f64>,
    #[arg(long)]
    pub samples_file: Option<PathBuf>,

    #[arg(long)]
    pub max_server_connections: Option<u32>,
    #[arg(long, alias = "connection-establishment-overhead-ms")]
    pub connection_overhead_ms: Option<f64>,
    #[arg(long, value_enum)]
    pub connection_profile: Option<CliConnectionOverheadProfile>,
    #[arg(long)]
    pub idle_timeout_ms: Option<u64>,
    #[arg(long)]
    pub min: Option<u32>,
    #[arg(long)]
    pub max: Option<u32>,

    #[arg(long)]
    pub iterations: Option<u32>,
    #[arg(long)]
    pub seed: Option<u64>,
    #[arg(long, value_enum)]
    pub distribution: Option<CliDistributionModel>,
    #[arg(long, value_enum)]
    pub queue_model: Option<CliQueueModel>,
    #[arg(long)]
    pub target_wait_p99_ms: Option<f64>,
    #[arg(long)]
    pub max_acceptable_rho: Option<f64>,

    #[arg(long)]
    pub explain: bool,
}

#[derive(Debug, Clone, Args)]
pub struct EvaluateArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    #[arg(long)]
    pub pool_size: u32,
}

#[derive(Debug, Clone, Args)]
pub struct BatchArgs {
    #[arg(long)]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct CompareArgs {
    #[arg(long)]
    pub config: PathBuf,

    #[arg(long)]
    pub baseline: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct BudgetArgs {
    #[arg(long)]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(subcommand)]
    pub command: ImportCommands,
}

#[derive(Debug, Clone, Args)]
pub struct GateArgs {
    #[arg(long)]
    pub policy: Option<PathBuf>,

    #[arg(long, value_enum)]
    pub max_saturation: Option<CliSaturationLevel>,
    #[arg(long)]
    pub max_pool_increase_percent: Option<f64>,
    #[arg(long)]
    pub max_additional_connections: Option<u32>,
    #[arg(long)]
    pub max_recommended_pool_size: Option<u32>,
    #[arg(long)]
    pub max_recommended_p99_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_recommended_mean_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_recommended_rho: Option<f64>,
    #[arg(long)]
    pub max_current_p99_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_current_mean_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_current_rho: Option<f64>,
    #[arg(long)]
    pub expected_pool_size: Option<u32>,

    #[command(subcommand)]
    pub source: GateSourceCommands,
}

#[derive(Debug, Clone, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum GateSourceCommands {
    Telemetry(TelemetryImportArgs),
    Prometheus(PrometheusImportArgs),
}

#[derive(Debug, Clone, Args)]
pub struct GuardArgs {
    #[arg(long)]
    pub policy: Option<PathBuf>,

    #[arg(long, value_enum)]
    pub max_saturation: Option<CliSaturationLevel>,
    #[arg(long)]
    pub max_pool_increase_percent: Option<f64>,
    #[arg(long)]
    pub max_additional_connections: Option<u32>,
    #[arg(long)]
    pub max_recommended_pool_size: Option<u32>,
    #[arg(long)]
    pub max_recommended_p99_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_recommended_mean_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_recommended_rho: Option<f64>,
    #[arg(long)]
    pub max_current_p99_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_current_mean_queue_wait_ms: Option<f64>,
    #[arg(long)]
    pub max_current_rho: Option<f64>,
    #[arg(long)]
    pub expected_pool_size: Option<u32>,

    #[command(subcommand)]
    pub source: GateSourceCommands,
}

#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub source: DoctorSourceCommands,
}

#[derive(Debug, Clone, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum DoctorSourceCommands {
    Telemetry(TelemetryImportArgs),
    Prometheus(PrometheusImportArgs),
}

#[derive(Debug, Clone, Args)]
pub struct GenerateConfigArgs {
    #[arg(long, value_enum)]
    pub framework: CliConfigFramework,

    #[arg(long)]
    pub min_idle: Option<u32>,

    #[arg(long, default_value_t = 30_000)]
    pub connection_timeout_ms: u64,

    #[arg(long, default_value_t = 600_000)]
    pub idle_timeout_ms: u64,

    #[arg(long, default_value = "DATABASE_URL")]
    pub database_url_env: String,

    #[arg(long, default_value = "poolsim-recommended-pool")]
    pub pool_name: String,

    #[command(subcommand)]
    pub source: GenerateConfigSourceCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GenerateConfigSourceCommands {
    Telemetry(TelemetryImportArgs),
    Prometheus(PrometheusImportArgs),
    Simulate(CommonArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliSaturationLevel {
    Ok,
    Warning,
    Critical,
}

impl From<CliSaturationLevel> for SaturationLevel {
    fn from(value: CliSaturationLevel) -> Self {
        match value {
            CliSaturationLevel::Ok => SaturationLevel::Ok,
            CliSaturationLevel::Warning => SaturationLevel::Warning,
            CliSaturationLevel::Critical => SaturationLevel::Critical,
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ImportCommands {
    Telemetry(TelemetryImportArgs),
    Prometheus(PrometheusImportArgs),
}

#[derive(Debug, Clone, Args)]
pub struct TelemetryImportArgs {
    #[arg(long)]
    pub config: PathBuf,

    #[arg(long)]
    pub current_pool_size: Option<u32>,
}

#[derive(Debug, Clone, Args)]
pub struct PrometheusImportArgs {
    #[arg(
        long,
        conflicts_with = "response_file",
        required_unless_present = "response_file"
    )]
    pub endpoint: Option<String>,

    #[arg(
        long,
        conflicts_with = "endpoint",
        required_unless_present = "endpoint"
    )]
    pub response_file: Option<PathBuf>,

    #[arg(long, required_unless_present = "response_file")]
    pub rps_query: Option<String>,
    #[arg(long, required_unless_present = "response_file")]
    pub p50_query: Option<String>,
    #[arg(long, required_unless_present = "response_file")]
    pub p95_query: Option<String>,
    #[arg(long, required_unless_present = "response_file")]
    pub p99_query: Option<String>,

    #[arg(long)]
    pub header: Vec<String>,

    #[arg(long)]
    pub service_name: Option<String>,
    #[arg(long)]
    pub window: Option<String>,
    #[arg(long)]
    pub observed_at: Option<String>,

    #[arg(long)]
    pub current_pool_size: u32,
    #[arg(long)]
    pub max_server_connections: u32,
    #[arg(
        long,
        alias = "connection-establishment-overhead-ms",
        default_value_t = 0.0
    )]
    pub connection_overhead_ms: f64,
    #[arg(long)]
    pub idle_timeout_ms: Option<u64>,
    #[arg(long)]
    pub min: u32,
    #[arg(long)]
    pub max: u32,

    #[arg(long)]
    pub iterations: Option<u32>,
    #[arg(long)]
    pub seed: Option<u64>,
    #[arg(long, value_enum)]
    pub distribution: Option<CliDistributionModel>,
    #[arg(long, value_enum)]
    pub queue_model: Option<CliQueueModel>,
    #[arg(long)]
    pub target_wait_p99_ms: Option<f64>,
    #[arg(long)]
    pub max_acceptable_rho: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_distribution_model_maps_to_core_enum() {
        assert_eq!(
            DistributionModel::from(CliDistributionModel::LogNormal),
            DistributionModel::LogNormal
        );
        assert_eq!(
            DistributionModel::from(CliDistributionModel::Exponential),
            DistributionModel::Exponential
        );
        assert_eq!(
            DistributionModel::from(CliDistributionModel::Empirical),
            DistributionModel::Empirical
        );
        assert_eq!(
            DistributionModel::from(CliDistributionModel::Gamma),
            DistributionModel::Gamma
        );
    }

    #[test]
    fn cli_queue_model_maps_to_core_enum() {
        assert_eq!(QueueModel::from(CliQueueModel::Mmc), QueueModel::MMC);
        assert_eq!(QueueModel::from(CliQueueModel::Mdc), QueueModel::MDC);
    }

    #[test]
    fn parser_handles_connection_profile_flag() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "simulate",
            "--rps",
            "180",
            "--p50",
            "8",
            "--p95",
            "30",
            "--p99",
            "70",
            "--max-server-connections",
            "100",
            "--connection-profile",
            "rds-proxy",
            "--min",
            "2",
            "--max",
            "20",
        ])
        .expect("connection profile flag should parse");
        match cli.command {
            Commands::Simulate(args) => {
                assert!(matches!(
                    args.common.connection_profile,
                    Some(CliConnectionOverheadProfile::RdsProxy)
                ));
            }
            _ => panic!("expected simulate command"),
        }
    }

    #[test]
    fn parser_handles_explain_flag() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "simulate",
            "--config",
            "poolsim.json",
            "--explain",
        ])
        .expect("explain flag should parse");
        match cli.command {
            Commands::Simulate(args) => assert!(args.common.explain),
            _ => panic!("expected simulate command"),
        }
    }

    #[test]
    fn parser_handles_html_output_format() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "html",
            "simulate",
            "--config",
            "poolsim.json",
        ])
        .expect("html output format should parse");
        assert!(matches!(cli.format, OutputFormat::Html));
    }

    #[test]
    fn cli_saturation_model_maps_to_core_enum() {
        assert_eq!(
            SaturationLevel::from(CliSaturationLevel::Ok),
            SaturationLevel::Ok
        );
        assert_eq!(
            SaturationLevel::from(CliSaturationLevel::Warning),
            SaturationLevel::Warning
        );
        assert_eq!(
            SaturationLevel::from(CliSaturationLevel::Critical),
            SaturationLevel::Critical
        );
    }

    #[test]
    fn parser_handles_init_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "init",
            "--framework",
            "spring-boot",
            "--database",
            "mysql",
            "--expected-rps",
            "250",
            "--output",
            "poolsim.json",
            "--policy-output",
            "gate.toml",
            "--force",
        ])
        .expect("init args should parse");

        match cli.command {
            Commands::Init(args) => {
                assert!(matches!(args.framework, CliConfigFramework::SpringBoot));
                assert!(matches!(args.database, CliDatabaseKind::Mysql));
                assert_eq!(args.expected_rps, 250.0);
                assert_eq!(args.output, PathBuf::from("poolsim.json"));
                assert_eq!(args.policy_output, PathBuf::from("gate.toml"));
                assert!(args.force);
            }
            _ => panic!("expected init command"),
        }
    }

    #[test]
    fn parser_handles_generate_config_sources() {
        let telemetry_cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "generate-config",
            "--framework",
            "sqlx",
            "--min-idle",
            "3",
            "--connection-timeout-ms",
            "4000",
            "--idle-timeout-ms",
            "120000",
            "--database-url-env",
            "APP_DATABASE_URL",
            "--pool-name",
            "checkout-pool",
            "telemetry",
            "--config",
            "telemetry.json",
        ])
        .expect("generate-config telemetry args should parse");
        match telemetry_cli.command {
            Commands::GenerateConfig(args) => {
                assert!(matches!(args.framework, CliConfigFramework::Sqlx));
                assert_eq!(args.min_idle, Some(3));
                assert_eq!(args.connection_timeout_ms, 4_000);
                assert_eq!(args.idle_timeout_ms, 120_000);
                assert_eq!(args.database_url_env, "APP_DATABASE_URL");
                assert_eq!(args.pool_name, "checkout-pool");
                match args.source {
                    GenerateConfigSourceCommands::Telemetry(telemetry) => {
                        assert_eq!(telemetry.config, PathBuf::from("telemetry.json"));
                    }
                    _ => panic!("expected telemetry source"),
                }
            }
            _ => panic!("expected generate-config command"),
        }

        let prometheus_cli = Cli::try_parse_from([
            "poolsim",
            "generate-config",
            "--framework",
            "spring-boot",
            "prometheus",
            "--response-file",
            "prometheus.json",
            "--current-pool-size",
            "8",
            "--max-server-connections",
            "100",
            "--min",
            "2",
            "--max",
            "20",
        ])
        .expect("generate-config prometheus args should parse");
        assert!(matches!(
            prometheus_cli.command,
            Commands::GenerateConfig(GenerateConfigArgs {
                source: GenerateConfigSourceCommands::Prometheus(_),
                ..
            })
        ));

        let simulate_cli = Cli::try_parse_from([
            "poolsim",
            "generate-config",
            "--framework",
            "node-pg",
            "simulate",
            "--config",
            "cli-config.json",
        ])
        .expect("generate-config simulate args should parse");
        assert!(matches!(
            simulate_cli.command,
            Commands::GenerateConfig(GenerateConfigArgs {
                source: GenerateConfigSourceCommands::Simulate(_),
                ..
            })
        ));
    }

    #[test]
    fn parser_supports_global_flags_and_aliases() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "--warn-exit",
            "simulate",
            "--rps",
            "120",
            "--p50",
            "5",
            "--p95",
            "15",
            "--p99",
            "30",
            "--max-server-connections",
            "80",
            "--connection-establishment-overhead-ms",
            "1.5",
            "--min",
            "2",
            "--max",
            "16",
            "--queue-model",
            "mdc",
            "--distribution",
            "gamma",
        ])
        .expect("CLI args should parse");

        assert!(cli.warn_exit);
        assert!(matches!(cli.format, OutputFormat::Json));
        assert!(matches!(cli.command, Commands::Simulate(_)));
    }

    #[test]
    fn parser_handles_batch_subcommand() {
        let cli = Cli::try_parse_from(["poolsim", "batch", "--config", "batch.json"])
            .expect("batch args should parse");
        match cli.command {
            Commands::Batch(args) => {
                assert_eq!(args.config, PathBuf::from("batch.json"));
            }
            _ => panic!("expected batch command"),
        }
    }

    #[test]
    fn parser_handles_compare_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "compare",
            "--config",
            "scenarios.json",
            "--baseline",
            "normal",
        ])
        .expect("compare args should parse");

        match cli.command {
            Commands::Compare(args) => {
                assert_eq!(args.config, PathBuf::from("scenarios.json"));
                assert_eq!(args.baseline.as_deref(), Some("normal"));
            }
            _ => panic!("expected compare command"),
        }
    }

    #[test]
    fn parser_handles_budget_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "csv",
            "budget",
            "--config",
            "budget.json",
        ])
        .expect("budget args should parse");

        match cli.command {
            Commands::Budget(args) => {
                assert_eq!(args.config, PathBuf::from("budget.json"));
            }
            _ => panic!("expected budget command"),
        }
    }

    #[test]
    fn parser_handles_import_telemetry_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "import",
            "telemetry",
            "--config",
            "telemetry.json",
            "--current-pool-size",
            "12",
        ])
        .expect("import telemetry args should parse");

        match cli.command {
            Commands::Import(args) => match args.command {
                ImportCommands::Telemetry(telemetry) => {
                    assert_eq!(telemetry.config, PathBuf::from("telemetry.json"));
                    assert_eq!(telemetry.current_pool_size, Some(12));
                }
                ImportCommands::Prometheus(_) => panic!("expected telemetry import"),
            },
            _ => panic!("expected import telemetry command"),
        }
    }

    #[test]
    fn parser_handles_import_prometheus_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "import",
            "prometheus",
            "--endpoint",
            "http://localhost:9090",
            "--rps-query",
            "sum(rate(http_requests_total[5m]))",
            "--p50-query",
            "histogram_quantile(0.50, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000",
            "--p95-query",
            "histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000",
            "--p99-query",
            "histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket[5m]))) * 1000",
            "--current-pool-size",
            "12",
            "--max-server-connections",
            "100",
            "--min",
            "2",
            "--max",
            "24",
            "--header",
            "Authorization: Bearer token",
        ])
        .expect("import prometheus args should parse");

        match cli.command {
            Commands::Import(args) => match args.command {
                ImportCommands::Prometheus(prometheus) => {
                    assert_eq!(
                        prometheus.endpoint.as_deref(),
                        Some("http://localhost:9090")
                    );
                    assert_eq!(prometheus.current_pool_size, 12);
                    assert_eq!(prometheus.max_server_connections, 100);
                    assert_eq!(prometheus.header.len(), 1);
                }
                ImportCommands::Telemetry(_) => panic!("expected prometheus import"),
            },
            _ => panic!("expected import prometheus command"),
        }
    }

    #[test]
    fn parser_handles_gate_telemetry_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "gate",
            "--policy",
            "gate.toml",
            "--max-saturation",
            "warning",
            "--max-pool-increase-percent",
            "25",
            "--max-additional-connections",
            "4",
            "--max-recommended-pool-size",
            "16",
            "--max-recommended-p99-queue-wait-ms",
            "50",
            "--max-recommended-mean-queue-wait-ms",
            "10",
            "--max-recommended-rho",
            "0.85",
            "--max-current-p99-queue-wait-ms",
            "75",
            "--max-current-mean-queue-wait-ms",
            "15",
            "--max-current-rho",
            "0.9",
            "--expected-pool-size",
            "8",
            "telemetry",
            "--config",
            "telemetry.json",
        ])
        .expect("gate telemetry args should parse");

        match cli.command {
            Commands::Gate(args) => {
                assert_eq!(args.policy, Some(PathBuf::from("gate.toml")));
                assert!(matches!(
                    args.max_saturation,
                    Some(CliSaturationLevel::Warning)
                ));
                assert_eq!(args.max_pool_increase_percent, Some(25.0));
                assert_eq!(args.max_additional_connections, Some(4));
                assert_eq!(args.max_recommended_pool_size, Some(16));
                assert_eq!(args.max_recommended_p99_queue_wait_ms, Some(50.0));
                assert_eq!(args.max_recommended_mean_queue_wait_ms, Some(10.0));
                assert_eq!(args.max_recommended_rho, Some(0.85));
                assert_eq!(args.max_current_p99_queue_wait_ms, Some(75.0));
                assert_eq!(args.max_current_mean_queue_wait_ms, Some(15.0));
                assert_eq!(args.max_current_rho, Some(0.9));
                assert_eq!(args.expected_pool_size, Some(8));
                match args.source {
                    GateSourceCommands::Telemetry(telemetry) => {
                        assert_eq!(telemetry.config, PathBuf::from("telemetry.json"));
                    }
                    GateSourceCommands::Prometheus(_) => panic!("expected telemetry source"),
                }
            }
            _ => panic!("expected gate command"),
        }
    }

    #[test]
    fn parser_handles_guard_prometheus_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "guard",
            "--policy",
            "guard.toml",
            "--max-current-rho",
            "0.85",
            "prometheus",
            "--response-file",
            "prometheus.json",
            "--current-pool-size",
            "12",
            "--max-server-connections",
            "100",
            "--min",
            "2",
            "--max",
            "24",
        ])
        .expect("guard prometheus args should parse");

        match cli.command {
            Commands::Guard(args) => {
                assert_eq!(args.policy, Some(PathBuf::from("guard.toml")));
                assert_eq!(args.max_current_rho, Some(0.85));
                match args.source {
                    GateSourceCommands::Prometheus(prometheus) => {
                        assert_eq!(
                            prometheus.response_file,
                            Some(PathBuf::from("prometheus.json"))
                        );
                        assert_eq!(prometheus.current_pool_size, 12);
                    }
                    GateSourceCommands::Telemetry(_) => panic!("expected prometheus source"),
                }
            }
            _ => panic!("expected guard command"),
        }
    }

    #[test]
    fn parser_handles_doctor_prometheus_subcommand() {
        let cli = Cli::try_parse_from([
            "poolsim",
            "--format",
            "json",
            "doctor",
            "prometheus",
            "--response-file",
            "prometheus.json",
            "--current-pool-size",
            "12",
            "--max-server-connections",
            "100",
            "--min",
            "2",
            "--max",
            "24",
        ])
        .expect("doctor prometheus args should parse");

        match cli.command {
            Commands::Doctor(args) => match args.source {
                DoctorSourceCommands::Prometheus(prometheus) => {
                    assert_eq!(
                        prometheus.response_file,
                        Some(PathBuf::from("prometheus.json"))
                    );
                    assert_eq!(prometheus.current_pool_size, 12);
                    assert_eq!(prometheus.max_server_connections, 100);
                }
                DoctorSourceCommands::Telemetry(_) => panic!("expected prometheus source"),
            },
            _ => panic!("expected doctor command"),
        }
    }
}
