use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use poolsim_core::types::{DistributionModel, QueueModel};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
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
}
