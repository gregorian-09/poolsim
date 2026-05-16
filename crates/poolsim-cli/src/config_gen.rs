use serde::Serialize;

use crate::args::{CliConfigFramework, GenerateConfigArgs};
use poolsim_core::{
    telemetry::TelemetryRecommendation,
    types::{PoolConfig, SimulationReport},
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConfigFramework {
    Hikaricp,
    SpringBoot,
    Sqlalchemy,
    Prisma,
    NodePg,
    Sqlx,
    Deadpool,
}

impl ConfigFramework {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Hikaricp => "hikaricp",
            Self::SpringBoot => "spring-boot",
            Self::Sqlalchemy => "sqlalchemy",
            Self::Prisma => "prisma",
            Self::NodePg => "node-pg",
            Self::Sqlx => "sqlx",
            Self::Deadpool => "deadpool",
        }
    }
}

impl From<CliConfigFramework> for ConfigFramework {
    fn from(value: CliConfigFramework) -> Self {
        match value {
            CliConfigFramework::Hikaricp => Self::Hikaricp,
            CliConfigFramework::SpringBoot => Self::SpringBoot,
            CliConfigFramework::Sqlalchemy => Self::Sqlalchemy,
            CliConfigFramework::Prisma => Self::Prisma,
            CliConfigFramework::NodePg => Self::NodePg,
            CliConfigFramework::Sqlx => Self::Sqlx,
            CliConfigFramework::Deadpool => Self::Deadpool,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConfigSourceKind {
    Telemetry,
    Prometheus,
    Simulate,
}

impl ConfigSourceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Telemetry => "telemetry",
            Self::Prometheus => "prometheus",
            Self::Simulate => "simulate",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ConfigReference {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ConfigRecommendation {
    pub source: ConfigSourceKind,
    pub service_name: Option<String>,
    pub window: Option<String>,
    pub observed_at: Option<String>,
    pub recommended_pool_size: u32,
    pub cold_start_min_pool_size: u32,
    pub max_server_connections: u32,
    pub utilisation_rho: f64,
    pub mean_queue_wait_ms: f64,
    pub p99_queue_wait_ms: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ConfigSnippetReport {
    pub framework: ConfigFramework,
    pub source: ConfigSourceKind,
    pub service_name: Option<String>,
    pub window: Option<String>,
    pub observed_at: Option<String>,
    pub recommended_pool_size: u32,
    pub min_idle: u32,
    pub connection_timeout_ms: u64,
    pub idle_timeout_ms: u64,
    pub database_url_env: String,
    pub pool_name: String,
    pub max_server_connections: u32,
    pub utilisation_rho: f64,
    pub mean_queue_wait_ms: f64,
    pub p99_queue_wait_ms: f64,
    pub snippet: String,
    pub notes: Vec<String>,
    pub references: Vec<ConfigReference>,
}

pub(crate) fn recommendation_from_telemetry(
    source: ConfigSourceKind,
    recommendation: &TelemetryRecommendation,
    max_server_connections: u32,
) -> ConfigRecommendation {
    ConfigRecommendation {
        source,
        service_name: recommendation.service_name.clone(),
        window: recommendation.window.clone(),
        observed_at: recommendation.observed_at.clone(),
        recommended_pool_size: recommendation.diff.recommended_pool_size,
        cold_start_min_pool_size: recommendation
            .diff
            .recommended_report
            .cold_start_min_pool_size,
        max_server_connections,
        utilisation_rho: recommendation.diff.recommended_report.utilisation_rho,
        mean_queue_wait_ms: recommendation.diff.recommended_report.mean_queue_wait_ms,
        p99_queue_wait_ms: recommendation.diff.recommended_report.p99_queue_wait_ms,
    }
}

pub(crate) fn recommendation_from_simulation(
    report: &SimulationReport,
    pool: &PoolConfig,
) -> ConfigRecommendation {
    ConfigRecommendation {
        source: ConfigSourceKind::Simulate,
        service_name: None,
        window: None,
        observed_at: None,
        recommended_pool_size: report.optimal_pool_size,
        cold_start_min_pool_size: report.cold_start_min_pool_size,
        max_server_connections: pool.max_server_connections,
        utilisation_rho: report.utilisation_rho,
        mean_queue_wait_ms: report.mean_queue_wait_ms,
        p99_queue_wait_ms: report.p99_queue_wait_ms,
    }
}

pub(crate) fn build_config_snippet(
    args: &GenerateConfigArgs,
    recommendation: ConfigRecommendation,
) -> ConfigSnippetReport {
    let framework = ConfigFramework::from(args.framework);
    let recommended = recommendation.recommended_pool_size;
    let min_idle = effective_min_idle(
        args.min_idle,
        recommendation.cold_start_min_pool_size,
        recommended,
    );
    let snippet_input = SnippetInput {
        recommended_pool_size: recommended,
        min_idle,
        connection_timeout_ms: args.connection_timeout_ms,
        idle_timeout_ms: args.idle_timeout_ms,
        database_url_env: &args.database_url_env,
        pool_name: &args.pool_name,
    };

    ConfigSnippetReport {
        framework,
        source: recommendation.source,
        service_name: recommendation.service_name,
        window: recommendation.window,
        observed_at: recommendation.observed_at,
        recommended_pool_size: recommended,
        min_idle,
        connection_timeout_ms: args.connection_timeout_ms,
        idle_timeout_ms: args.idle_timeout_ms,
        database_url_env: args.database_url_env.clone(),
        pool_name: args.pool_name.clone(),
        max_server_connections: recommendation.max_server_connections,
        utilisation_rho: recommendation.utilisation_rho,
        mean_queue_wait_ms: recommendation.mean_queue_wait_ms,
        p99_queue_wait_ms: recommendation.p99_queue_wait_ms,
        snippet: render_snippet(framework, &snippet_input),
        notes: notes(recommended, recommendation.max_server_connections),
        references: references(framework),
    }
}

fn effective_min_idle(
    explicit: Option<u32>,
    cold_start_min_pool_size: u32,
    recommended: u32,
) -> u32 {
    explicit
        .unwrap_or(cold_start_min_pool_size)
        .clamp(1, recommended)
}

struct SnippetInput<'a> {
    recommended_pool_size: u32,
    min_idle: u32,
    connection_timeout_ms: u64,
    idle_timeout_ms: u64,
    database_url_env: &'a str,
    pool_name: &'a str,
}

fn render_snippet(framework: ConfigFramework, input: &SnippetInput<'_>) -> String {
    match framework {
        ConfigFramework::Hikaricp => hikaricp_snippet(input),
        ConfigFramework::SpringBoot => spring_boot_snippet(input),
        ConfigFramework::Sqlalchemy => sqlalchemy_snippet(input),
        ConfigFramework::Prisma => prisma_snippet(input),
        ConfigFramework::NodePg => node_pg_snippet(input),
        ConfigFramework::Sqlx => sqlx_snippet(input),
        ConfigFramework::Deadpool => deadpool_snippet(input),
    }
}

fn hikaricp_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"HikariConfig config = new HikariConfig();
config.setPoolName("{pool_name}");
config.setMaximumPoolSize({max});
config.setMinimumIdle({min});
config.setConnectionTimeout({connection_timeout_ms});
config.setIdleTimeout({idle_timeout_ms});"#,
        pool_name = escape_double_quoted(input.pool_name),
        max = input.recommended_pool_size,
        min = input.min_idle,
        connection_timeout_ms = input.connection_timeout_ms,
        idle_timeout_ms = input.idle_timeout_ms,
    )
}

fn spring_boot_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"spring:
  datasource:
    hikari:
      pool-name: {pool_name}
      maximum-pool-size: {max}
      minimum-idle: {min}
      connection-timeout: {connection_timeout_ms}
      idle-timeout: {idle_timeout_ms}"#,
        pool_name = yaml_double_quoted(input.pool_name),
        max = input.recommended_pool_size,
        min = input.min_idle,
        connection_timeout_ms = input.connection_timeout_ms,
        idle_timeout_ms = input.idle_timeout_ms,
    )
}

fn sqlalchemy_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"import os
from sqlalchemy import create_engine

engine = create_engine(
    os.environ["{database_url_env}"],
    pool_size={max},
    max_overflow=0,
    pool_timeout={timeout_seconds},
    pool_pre_ping=True,
)"#,
        database_url_env = escape_double_quoted(input.database_url_env),
        max = input.recommended_pool_size,
        timeout_seconds = millis_to_seconds_ceil(input.connection_timeout_ms),
    )
}

fn prisma_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"# Prisma ORM v6 and earlier URL parameter:
{database_url_env}="postgresql://USER:PASSWORD@HOST:PORT/DATABASE?connection_limit={max}&pool_timeout={timeout_seconds}"

// Prisma ORM v7 pg adapter:
const adapter = new PrismaPg({{
  connectionString: {database_url_access},
  max: {max},
  connectionTimeoutMillis: {connection_timeout_ms},
  idleTimeoutMillis: {idle_timeout_ms},
}});"#,
        database_url_env = escape_double_quoted(input.database_url_env),
        database_url_access = js_env_access(input.database_url_env),
        max = input.recommended_pool_size,
        timeout_seconds = millis_to_seconds_ceil(input.connection_timeout_ms),
        connection_timeout_ms = input.connection_timeout_ms,
        idle_timeout_ms = input.idle_timeout_ms,
    )
}

fn node_pg_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"import {{ Pool }} from "pg";

export const pool = new Pool({{
  connectionString: {database_url_access},
  max: {max},
  min: {min},
  connectionTimeoutMillis: {connection_timeout_ms},
  idleTimeoutMillis: {idle_timeout_ms},
}});"#,
        database_url_access = js_env_access(input.database_url_env),
        max = input.recommended_pool_size,
        min = input.min_idle,
        connection_timeout_ms = input.connection_timeout_ms,
        idle_timeout_ms = input.idle_timeout_ms,
    )
}

fn sqlx_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"use std::{{env, time::Duration}};
use sqlx::postgres::PgPoolOptions;

let database_url = env::var("{database_url_env}")?;
let pool = PgPoolOptions::new()
    .max_connections({max})
    .min_connections({min})
    .acquire_timeout(Duration::from_millis({connection_timeout_ms}))
    .idle_timeout(Duration::from_millis({idle_timeout_ms}))
    .connect(&database_url)
    .await?;"#,
        database_url_env = escape_double_quoted(input.database_url_env),
        max = input.recommended_pool_size,
        min = input.min_idle,
        connection_timeout_ms = input.connection_timeout_ms,
        idle_timeout_ms = input.idle_timeout_ms,
    )
}

fn deadpool_snippet(input: &SnippetInput<'_>) -> String {
    format!(
        r#"PG__POOL__MAX_SIZE={max}
PG__POOL__TIMEOUTS__WAIT__SECS={timeout_seconds}
PG__POOL__TIMEOUTS__WAIT__NANOS=0"#,
        max = input.recommended_pool_size,
        timeout_seconds = millis_to_seconds_ceil(input.connection_timeout_ms),
    )
}

fn notes(recommended: u32, max_server_connections: u32) -> Vec<String> {
    vec![
        format!(
            "Recommended pool size {recommended} must be multiplied by service replica count before comparing with the database max_connections budget."
        ),
        format!(
            "Keep all application pools plus administrative connections within the database limit of {max_server_connections} connections."
        ),
        "Re-run poolsim after traffic, latency, replica count, database limits, or query behavior changes.".to_string(),
    ]
}

fn references(framework: ConfigFramework) -> Vec<ConfigReference> {
    match framework {
        ConfigFramework::Hikaricp => vec![reference(
            "HikariCP configuration",
            "https://github.com/brettwooldridge/HikariCP",
        )],
        ConfigFramework::SpringBoot => vec![reference(
            "Spring Boot data access and Hikari settings",
            "https://docs.spring.io/spring-boot/how-to/data-access.html",
        )],
        ConfigFramework::Sqlalchemy => vec![reference(
            "SQLAlchemy pooling and engine configuration",
            "https://docs.sqlalchemy.org/en/latest/core/pooling.html",
        )],
        ConfigFramework::Prisma => vec![reference(
            "Prisma ORM connection pool",
            "https://www.prisma.io/docs/orm/prisma-client/setup-and-configuration/databases-connections/connection-pool",
        )],
        ConfigFramework::NodePg => vec![reference(
            "node-postgres Pool API",
            "https://node-postgres.com/apis/pool",
        )],
        ConfigFramework::Sqlx => vec![reference(
            "sqlx PoolOptions",
            "https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html",
        )],
        ConfigFramework::Deadpool => vec![reference(
            "deadpool-postgres Config",
            "https://docs.rs/deadpool-postgres/latest/deadpool_postgres/struct.Config.html",
        )],
    }
}

fn reference(title: &str, url: &str) -> ConfigReference {
    ConfigReference {
        title: title.to_string(),
        url: url.to_string(),
    }
}

fn millis_to_seconds_ceil(ms: u64) -> u64 {
    ms.saturating_add(999) / 1_000
}

fn escape_double_quoted(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn yaml_double_quoted(input: &str) -> String {
    format!("\"{}\"", escape_double_quoted(input))
}

fn js_env_access(input: &str) -> String {
    if is_js_identifier(input) {
        format!("process.env.{input}")
    } else {
        format!("process.env[\"{}\"]", escape_double_quoted(input))
    }
}

fn is_js_identifier(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first == '$' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use poolsim_core::{
        telemetry::{PoolRecommendationDiff, PoolSizeChange},
        types::{EvaluationResult, RiskLevel, SaturationLevel, SensitivityRow},
    };

    use super::*;

    fn sample_report() -> SimulationReport {
        SimulationReport {
            optimal_pool_size: 8,
            confidence_interval: (7, 9),
            cold_start_min_pool_size: 5,
            utilisation_rho: 0.72,
            mean_queue_wait_ms: 3.2,
            p99_queue_wait_ms: 18.4,
            saturation: SaturationLevel::Ok,
            sensitivity: vec![SensitivityRow {
                pool_size: 20,
                utilisation_rho: 0.30,
                mean_queue_wait_ms: 0.2,
                p99_queue_wait_ms: 1.0,
                risk: RiskLevel::Low,
            }],
            step_load_analysis: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn sample_args(framework: CliConfigFramework) -> GenerateConfigArgs {
        GenerateConfigArgs {
            framework,
            min_idle: None,
            connection_timeout_ms: 30_001,
            idle_timeout_ms: 600_000,
            database_url_env: "DATABASE_URL".to_string(),
            pool_name: "checkout-pool".to_string(),
            source: crate::args::GenerateConfigSourceCommands::Simulate(crate::args::CommonArgs {
                config: None,
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
            }),
        }
    }

    fn sample_recommendation() -> ConfigRecommendation {
        ConfigRecommendation {
            source: ConfigSourceKind::Telemetry,
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: None,
            recommended_pool_size: 8,
            cold_start_min_pool_size: 5,
            max_server_connections: 100,
            utilisation_rho: 0.72,
            mean_queue_wait_ms: 3.2,
            p99_queue_wait_ms: 18.4,
        }
    }

    #[test]
    fn framework_mapping_and_labels_are_stable() {
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::Hikaricp).as_str(),
            "hikaricp"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::SpringBoot).as_str(),
            "spring-boot"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::Sqlalchemy).as_str(),
            "sqlalchemy"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::Prisma).as_str(),
            "prisma"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::NodePg).as_str(),
            "node-pg"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::Sqlx).as_str(),
            "sqlx"
        );
        assert_eq!(
            ConfigFramework::from(CliConfigFramework::Deadpool).as_str(),
            "deadpool"
        );
        assert_eq!(ConfigSourceKind::Telemetry.as_str(), "telemetry");
        assert_eq!(ConfigSourceKind::Prometheus.as_str(), "prometheus");
        assert_eq!(ConfigSourceKind::Simulate.as_str(), "simulate");
    }

    #[test]
    fn config_snippets_cover_supported_frameworks() {
        let cases = [
            (CliConfigFramework::Hikaricp, "setMaximumPoolSize(8)"),
            (CliConfigFramework::SpringBoot, "maximum-pool-size: 8"),
            (CliConfigFramework::Sqlalchemy, "pool_size=8"),
            (CliConfigFramework::Prisma, "connection_limit=8"),
            (CliConfigFramework::NodePg, "max: 8"),
            (CliConfigFramework::Sqlx, ".max_connections(8)"),
            (CliConfigFramework::Deadpool, "PG__POOL__MAX_SIZE=8"),
        ];

        for (framework, expected) in cases {
            let report = build_config_snippet(&sample_args(framework), sample_recommendation());
            assert!(report.snippet.contains(expected), "missing {expected}");
            assert_eq!(report.min_idle, 5);
            assert_eq!(report.references.len(), 1);
            assert_eq!(report.notes.len(), 3);
        }
    }

    #[test]
    fn explicit_min_idle_is_clamped_and_strings_are_escaped() {
        let mut args = sample_args(CliConfigFramework::Hikaricp);
        args.min_idle = Some(99);
        args.pool_name = "checkout \"primary\"".to_string();
        args.database_url_env = "DATABASE-URL".to_string();
        let report = build_config_snippet(&args, sample_recommendation());
        assert_eq!(report.min_idle, 8);
        assert!(report.snippet.contains("checkout \\\"primary\\\""));

        let node_report = build_config_snippet(
            &GenerateConfigArgs {
                framework: CliConfigFramework::NodePg,
                ..args
            },
            sample_recommendation(),
        );
        assert!(node_report
            .snippet
            .contains("process.env[\"DATABASE-URL\"]"));
    }

    #[test]
    fn recommendation_adapters_preserve_source_metrics() {
        let pool = PoolConfig {
            max_server_connections: 120,
            connection_overhead_ms: 2.0,
            idle_timeout_ms: None,
            min_pool_size: 2,
            max_pool_size: 20,
        };
        let simulation = recommendation_from_simulation(&sample_report(), &pool);
        assert_eq!(simulation.source, ConfigSourceKind::Simulate);
        assert_eq!(simulation.max_server_connections, 120);

        let recommendation = TelemetryRecommendation {
            service_name: Some("checkout-api".to_string()),
            window: Some("1h".to_string()),
            observed_at: Some("2026-05-16T00:00:00Z".to_string()),
            diff: PoolRecommendationDiff {
                current_pool_size: 6,
                recommended_pool_size: 8,
                pool_size_delta: 2,
                change: PoolSizeChange::Increase,
                additional_connections_required: 2,
                removable_connections: 0,
                connection_change_percent: 33.3,
                current_evaluation: EvaluationResult {
                    pool_size: 6,
                    utilisation_rho: 0.9,
                    mean_queue_wait_ms: 10.0,
                    p99_queue_wait_ms: 50.0,
                    saturation: SaturationLevel::Warning,
                    warnings: Vec::new(),
                },
                recommended_report: sample_report(),
            },
        };
        let telemetry =
            recommendation_from_telemetry(ConfigSourceKind::Prometheus, &recommendation, 100);
        assert_eq!(telemetry.source, ConfigSourceKind::Prometheus);
        assert_eq!(telemetry.service_name.as_deref(), Some("checkout-api"));
        assert_eq!(telemetry.max_server_connections, 100);
    }

    #[test]
    fn helper_edge_cases_are_stable() {
        assert_eq!(millis_to_seconds_ceil(0), 0);
        assert_eq!(millis_to_seconds_ceil(1), 1);
        assert_eq!(millis_to_seconds_ceil(1_001), 2);
        assert_eq!(js_env_access("DATABASE_URL"), "process.env.DATABASE_URL");
        assert_eq!(
            js_env_access("DATABASE-URL"),
            "process.env[\"DATABASE-URL\"]"
        );
        assert!(!is_js_identifier(""));
        assert!(!is_js_identifier("1DATABASE_URL"));
        assert_eq!(yaml_double_quoted("pool"), "\"pool\"");
    }
}
