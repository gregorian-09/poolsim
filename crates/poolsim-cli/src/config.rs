use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use poolsim_core::{
    telemetry::TelemetrySnapshot,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};
use serde::Deserialize;

use crate::args::{
    BatchArgs, BudgetArgs, CommonArgs, CompareArgs, EvaluateArgs, TelemetryImportArgs,
};

#[derive(Debug, Clone)]
pub struct SimulationInput {
    pub workload: WorkloadConfig,
    pub pool: PoolConfig,
    pub options: SimulationOptions,
}

#[derive(Debug, Clone)]
pub struct EvaluateInput {
    pub workload: WorkloadConfig,
    pub options: SimulationOptions,
}

#[derive(Debug, Clone)]
pub struct SweepInput {
    pub workload: WorkloadConfig,
    pub pool: PoolConfig,
    pub options: SimulationOptions,
}

#[derive(Debug, Clone)]
pub struct BatchSimulationInput {
    pub requests: Vec<SimulationInput>,
}

#[derive(Debug, Clone)]
pub struct ScenarioInput {
    pub name: String,
    pub workload: WorkloadConfig,
    pub pool: PoolConfig,
    pub options: SimulationOptions,
}

#[derive(Debug, Clone)]
pub struct ScenarioComparisonInput {
    pub baseline: String,
    pub scenarios: Vec<ScenarioInput>,
}

#[derive(Debug, Clone)]
pub struct BudgetPlanInput {
    pub max_connections: u32,
    pub reserved_connections: u32,
    pub safety_margin_connections: u32,
    pub services: Vec<BudgetServiceInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BudgetServiceInput {
    pub name: String,
    pub replicas: u32,
    pub current_pool_size: Option<u32>,
    pub min_pool_size: u32,
    pub max_pool_size: Option<u32>,
    pub recommended_pool_size: u32,
    pub priority: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TelemetryInput {
    pub snapshot: TelemetrySnapshot,
    pub options: SimulationOptions,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FileConfig {
    workload: Option<WorkloadConfig>,
    pool: Option<PoolConfig>,
    options: Option<SimulationOptions>,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchRequest {
    workload: WorkloadConfig,
    pool: PoolConfig,
    #[serde(default)]
    options: SimulationOptions,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchFile {
    requests: Vec<BatchRequest>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioRequest {
    name: String,
    workload: WorkloadConfig,
    pool: PoolConfig,
    #[serde(default)]
    options: SimulationOptions,
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioFile {
    baseline: Option<String>,
    scenarios: Vec<ScenarioRequest>,
}

#[derive(Debug, Clone, Deserialize)]
struct BudgetFile {
    max_connections: u32,
    #[serde(default)]
    reserved_connections: u32,
    #[serde(default)]
    safety_margin_connections: u32,
    services: Vec<BudgetServiceInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum TelemetryConfigFile {
    Wrapped {
        telemetry: TelemetrySnapshot,
        #[serde(default)]
        options: SimulationOptions,
    },
    Direct(TelemetrySnapshot),
}

impl TelemetryConfigFile {
    fn into_input(self) -> TelemetryInput {
        match self {
            Self::Wrapped { telemetry, options } => TelemetryInput {
                snapshot: telemetry,
                options,
            },
            Self::Direct(snapshot) => TelemetryInput {
                snapshot,
                options: SimulationOptions::default(),
            },
        }
    }
}

pub fn resolve_simulation_input(args: &CommonArgs) -> Result<SimulationInput> {
    let mut cfg = load_config(args.config.as_deref())?;

    apply_workload_overrides(&mut cfg, args)?;
    apply_pool_overrides(&mut cfg, args);
    apply_options_overrides(&mut cfg, args);

    let workload = cfg
        .workload
        .ok_or_else(|| anyhow!("missing workload configuration (provide --config or CLI flags)"))?;
    let pool = cfg
        .pool
        .ok_or_else(|| anyhow!("missing pool configuration (provide --config or CLI flags)"))?;
    let options = cfg.options.unwrap_or_default();

    Ok(SimulationInput {
        workload,
        pool,
        options,
    })
}

pub fn resolve_evaluate_input(args: &EvaluateArgs) -> Result<EvaluateInput> {
    let mut cfg = load_config(args.common.config.as_deref())?;

    apply_workload_overrides(&mut cfg, &args.common)?;
    apply_options_overrides(&mut cfg, &args.common);

    let workload = cfg
        .workload
        .ok_or_else(|| anyhow!("missing workload configuration (provide --config or CLI flags)"))?;

    Ok(EvaluateInput {
        workload,
        options: cfg.options.unwrap_or_default(),
    })
}

pub fn resolve_sweep_input(args: &CommonArgs) -> Result<SweepInput> {
    let mut cfg = load_config(args.config.as_deref())?;

    apply_workload_overrides(&mut cfg, args)?;
    apply_pool_overrides(&mut cfg, args);
    apply_options_overrides(&mut cfg, args);

    let workload = cfg
        .workload
        .ok_or_else(|| anyhow!("missing workload configuration (provide --config or CLI flags)"))?;
    let pool = cfg
        .pool
        .ok_or_else(|| anyhow!("missing pool configuration (provide --config or CLI flags)"))?;

    Ok(SweepInput {
        workload,
        pool,
        options: cfg.options.unwrap_or_default(),
    })
}

pub fn resolve_batch_input(args: &BatchArgs) -> Result<BatchSimulationInput> {
    let path = &args.config;
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read batch config file {}", path.display()))?;

    let requests = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => parse_batch_json(&raw)
            .with_context(|| format!("invalid JSON batch config file {}", path.display()))?,
        Some("toml") => parse_batch_toml(&raw)
            .with_context(|| format!("invalid TOML batch config file {}", path.display()))?,
        _ => {
            return Err(anyhow!(
                "unsupported batch config extension for {} (use .json or .toml)",
                path.display()
            ))
        }
    };

    if requests.is_empty() {
        return Err(anyhow!(
            "batch config {} contains no simulation requests",
            path.display()
        ));
    }

    let mapped = requests
        .into_iter()
        .map(|item| SimulationInput {
            workload: item.workload,
            pool: item.pool,
            options: item.options,
        })
        .collect();

    Ok(BatchSimulationInput { requests: mapped })
}

pub fn resolve_scenario_comparison_input(args: &CompareArgs) -> Result<ScenarioComparisonInput> {
    let raw = fs::read_to_string(&args.config).with_context(|| {
        format!(
            "failed to read scenario comparison file {}",
            args.config.display()
        )
    })?;

    let parsed = match args.config.extension().and_then(|e| e.to_str()) {
        Some("json") => parse_scenario_json(&raw).with_context(|| {
            format!(
                "invalid JSON scenario comparison file {}",
                args.config.display()
            )
        })?,
        Some("toml") => toml::from_str::<ScenarioFile>(&raw).with_context(|| {
            format!(
                "invalid TOML scenario comparison file {}",
                args.config.display()
            )
        })?,
        _ => {
            return Err(anyhow!(
                "unsupported scenario comparison extension for {} (use .json or .toml)",
                args.config.display()
            ))
        }
    };

    let scenarios = map_scenarios(parsed.scenarios)?;
    let baseline = args
        .baseline
        .clone()
        .or(parsed.baseline)
        .unwrap_or_else(|| scenarios[0].name.clone());
    if !scenarios.iter().any(|scenario| scenario.name == baseline) {
        return Err(anyhow!(
            "baseline scenario '{}' was not found in {}",
            baseline,
            args.config.display()
        ));
    }

    Ok(ScenarioComparisonInput {
        baseline,
        scenarios,
    })
}

pub fn resolve_budget_plan_input(args: &BudgetArgs) -> Result<BudgetPlanInput> {
    let raw = fs::read_to_string(&args.config)
        .with_context(|| format!("failed to read budget plan file {}", args.config.display()))?;

    let parsed = match args.config.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::from_str::<BudgetFile>(&raw)
            .with_context(|| format!("invalid JSON budget plan file {}", args.config.display()))?,
        Some("toml") => toml::from_str::<BudgetFile>(&raw)
            .with_context(|| format!("invalid TOML budget plan file {}", args.config.display()))?,
        _ => {
            return Err(anyhow!(
                "unsupported budget plan extension for {} (use .json or .toml)",
                args.config.display()
            ))
        }
    };

    Ok(BudgetPlanInput {
        max_connections: parsed.max_connections,
        reserved_connections: parsed.reserved_connections,
        safety_margin_connections: parsed.safety_margin_connections,
        services: parsed.services,
    })
}

pub fn resolve_telemetry_input(args: &TelemetryImportArgs) -> Result<TelemetryInput> {
    let raw = fs::read_to_string(&args.config).with_context(|| {
        format!(
            "failed to read telemetry config file {}",
            args.config.display()
        )
    })?;

    let mut input = match args.config.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::from_str::<TelemetryConfigFile>(&raw)
            .with_context(|| {
                format!(
                    "invalid JSON telemetry config file {}",
                    args.config.display()
                )
            })?
            .into_input(),
        Some("toml") => toml::from_str::<TelemetryConfigFile>(&raw)
            .with_context(|| {
                format!(
                    "invalid TOML telemetry config file {}",
                    args.config.display()
                )
            })?
            .into_input(),
        _ => {
            return Err(anyhow!(
                "unsupported telemetry config extension for {} (use .json or .toml)",
                args.config.display()
            ))
        }
    };

    if let Some(current_pool_size) = args.current_pool_size {
        input.snapshot.current_pool_size = current_pool_size;
    }

    input.snapshot.validate()?;
    input.options.validate()?;
    Ok(input)
}

fn load_config(path: Option<&Path>) -> Result<FileConfig> {
    let Some(path) = path else {
        return Ok(FileConfig::default());
    };

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;

    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON config file {}", path.display())),
        Some("toml") => toml::from_str(&raw)
            .with_context(|| format!("invalid TOML config file {}", path.display())),
        _ => Err(anyhow!(
            "unsupported config extension for {} (use .json or .toml)",
            path.display()
        )),
    }
}

fn apply_workload_overrides(cfg: &mut FileConfig, args: &CommonArgs) -> Result<()> {
    if cfg.workload.is_none()
        && (args.rps.is_some() || args.p50.is_some() || args.p95.is_some() || args.p99.is_some())
    {
        cfg.workload = Some(WorkloadConfig {
            requests_per_second: args.rps.unwrap_or_default(),
            latency_p50_ms: args.p50.unwrap_or_default(),
            latency_p95_ms: args.p95.unwrap_or_default(),
            latency_p99_ms: args.p99.unwrap_or_default(),
            raw_samples_ms: None,
            step_load_profile: None,
        });
    }

    if let Some(workload) = cfg.workload.as_mut() {
        if let Some(v) = args.rps {
            workload.requests_per_second = v;
        }
        if let Some(v) = args.p50 {
            workload.latency_p50_ms = v;
        }
        if let Some(v) = args.p95 {
            workload.latency_p95_ms = v;
        }
        if let Some(v) = args.p99 {
            workload.latency_p99_ms = v;
        }
        if let Some(samples_file) = &args.samples_file {
            workload.raw_samples_ms = Some(load_samples_file(samples_file)?);
        }
    }

    Ok(())
}

fn apply_pool_overrides(cfg: &mut FileConfig, args: &CommonArgs) {
    if cfg.pool.is_none()
        && (args.max_server_connections.is_some()
            || args.connection_overhead_ms.is_some()
            || args.connection_profile.is_some()
            || args.min.is_some()
            || args.max.is_some())
    {
        cfg.pool = Some(PoolConfig {
            max_server_connections: args.max_server_connections.unwrap_or_default(),
            connection_overhead_ms: args
                .connection_overhead_ms
                .or_else(|| {
                    args.connection_profile.map(|profile| {
                        poolsim_core::ConnectionOverheadProfile::from(profile)
                            .connection_overhead_ms()
                    })
                })
                .unwrap_or(0.0),
            idle_timeout_ms: args.idle_timeout_ms,
            min_pool_size: args.min.unwrap_or_default(),
            max_pool_size: args.max.unwrap_or_default(),
        });
    }

    if let Some(pool) = cfg.pool.as_mut() {
        if let Some(v) = args.max_server_connections {
            pool.max_server_connections = v;
        }
        if let Some(profile) = args.connection_profile {
            pool.connection_overhead_ms =
                poolsim_core::ConnectionOverheadProfile::from(profile).connection_overhead_ms();
        }
        if let Some(v) = args.connection_overhead_ms {
            pool.connection_overhead_ms = v;
        }
        if let Some(v) = args.idle_timeout_ms {
            pool.idle_timeout_ms = Some(v);
        }
        if let Some(v) = args.min {
            pool.min_pool_size = v;
        }
        if let Some(v) = args.max {
            pool.max_pool_size = v;
        }
    }
}

fn apply_options_overrides(cfg: &mut FileConfig, args: &CommonArgs) {
    if cfg.options.is_none()
        && (args.iterations.is_some()
            || args.seed.is_some()
            || args.distribution.is_some()
            || args.queue_model.is_some()
            || args.target_wait_p99_ms.is_some()
            || args.max_acceptable_rho.is_some())
    {
        cfg.options = Some(SimulationOptions::default());
    }

    if let Some(options) = cfg.options.as_mut() {
        if let Some(v) = args.iterations {
            options.iterations = v;
        }
        if let Some(v) = args.seed {
            options.seed = Some(v);
        }
        if let Some(v) = args.distribution {
            options.distribution = v.into();
        }
        if let Some(v) = args.queue_model {
            options.queue_model = v.into();
        }
        if let Some(v) = args.target_wait_p99_ms {
            options.target_wait_p99_ms = v;
        }
        if let Some(v) = args.max_acceptable_rho {
            options.max_acceptable_rho = v;
        }
    }
}

fn load_samples_file(path: &Path) -> Result<Vec<f64>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read samples file {}", path.display()))?;

    let mut out = Vec::new();
    for token in raw.split([',', '\n', '\r', '\t', ' ']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let value = token
            .parse::<f64>()
            .with_context(|| format!("invalid sample value '{}' in {}", token, path.display()))?;
        out.push(value);
    }

    if out.is_empty() {
        return Err(anyhow!(
            "samples file {} contains no numeric values",
            path.display()
        ));
    }

    Ok(out)
}

fn parse_batch_json(raw: &str) -> Result<Vec<BatchRequest>> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.is_array() {
        Ok(serde_json::from_value(value)?)
    } else {
        Ok(serde_json::from_value::<BatchFile>(value)?.requests)
    }
}

fn parse_batch_toml(raw: &str) -> Result<Vec<BatchRequest>> {
    let wrapped: BatchFile = toml::from_str(raw)?;
    Ok(wrapped.requests)
}

fn parse_scenario_json(raw: &str) -> Result<ScenarioFile> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.is_array() {
        Ok(ScenarioFile {
            baseline: None,
            scenarios: serde_json::from_value(value)?,
        })
    } else {
        Ok(serde_json::from_value(value)?)
    }
}

fn map_scenarios(scenarios: Vec<ScenarioRequest>) -> Result<Vec<ScenarioInput>> {
    if scenarios.is_empty() {
        return Err(anyhow!("scenario comparison contains no scenarios"));
    }

    let mut out = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let name = scenario.name.trim().to_string();
        if name.is_empty() {
            return Err(anyhow!("scenario names must not be empty"));
        }
        if out
            .iter()
            .any(|existing: &ScenarioInput| existing.name == name)
        {
            return Err(anyhow!("duplicate scenario name '{}'", name));
        }
        out.push(ScenarioInput {
            name,
            workload: scenario.workload,
            pool: scenario.pool,
            options: scenario.options,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use poolsim_core::types::{DistributionModel, QueueModel};

    use super::*;
    use crate::args::{CliDistributionModel, CliQueueModel};

    fn empty_common_args() -> CommonArgs {
        CommonArgs {
            config: None,
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

    fn unique_temp_path(name: &str, ext: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "poolsim_cli_config_{name}_{}_{}.{}",
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

    fn sample_config_json() -> String {
        r#"
{
  "workload": {
    "requests_per_second": 200.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 30.0,
    "latency_p99_ms": 70.0
  },
  "pool": {
    "max_server_connections": 100,
    "connection_overhead_ms": 2.0,
    "idle_timeout_ms": 120000,
    "min_pool_size": 2,
    "max_pool_size": 20
  },
  "options": {
    "iterations": 1200,
    "seed": 9,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 40.0,
    "max_acceptable_rho": 0.85
  }
}
"#
        .to_string()
    }

    fn sample_telemetry_json() -> String {
        r#"
{
  "telemetry": {
    "service_name": "checkout-api",
    "window": "1h",
    "observed_at": "2026-05-15T10:00:00Z",
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
      "idle_timeout_ms": 120000,
      "min_pool_size": 2,
      "max_pool_size": 20
    }
  },
  "options": {
    "iterations": 1200,
    "seed": 9,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 40.0,
    "max_acceptable_rho": 0.85
  }
}
"#
        .to_string()
    }

    fn sample_budget_json() -> String {
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
    }
  ]
}
"#
        .to_string()
    }

    #[test]
    fn resolve_simulation_input_from_cli_overrides() {
        let mut args = empty_common_args();
        args.rps = Some(180.0);
        args.p50 = Some(5.0);
        args.p95 = Some(20.0);
        args.p99 = Some(45.0);
        args.max_server_connections = Some(80);
        args.connection_overhead_ms = Some(1.5);
        args.idle_timeout_ms = Some(90_000);
        args.min = Some(3);
        args.max = Some(16);
        args.iterations = Some(2_000);
        args.seed = Some(42);
        args.distribution = Some(CliDistributionModel::Gamma);
        args.queue_model = Some(CliQueueModel::Mdc);
        args.target_wait_p99_ms = Some(55.0);
        args.max_acceptable_rho = Some(0.8);

        let resolved = resolve_simulation_input(&args).expect("input should resolve from flags");
        assert_eq!(resolved.workload.requests_per_second, 180.0);
        assert_eq!(resolved.workload.latency_p99_ms, 45.0);
        assert_eq!(resolved.pool.max_server_connections, 80);
        assert_eq!(resolved.pool.max_pool_size, 16);
        assert_eq!(resolved.options.iterations, 2_000);
        assert_eq!(resolved.options.seed, Some(42));
        assert_eq!(resolved.options.distribution, DistributionModel::Gamma);
        assert_eq!(resolved.options.queue_model, QueueModel::MDC);
        assert_eq!(resolved.options.target_wait_p99_ms, 55.0);
        assert_eq!(resolved.options.max_acceptable_rho, 0.8);
    }

    #[test]
    fn resolve_simulation_input_errors_when_required_sections_missing() {
        let mut args = empty_common_args();
        args.rps = Some(100.0);
        args.p50 = Some(5.0);
        args.p95 = Some(10.0);
        args.p99 = Some(20.0);

        let err = resolve_simulation_input(&args).expect_err("pool config should be required");
        assert!(err.to_string().contains("missing pool configuration"));

        let mut pool_only = empty_common_args();
        pool_only.max_server_connections = Some(80);
        pool_only.connection_overhead_ms = Some(2.0);
        pool_only.min = Some(2);
        pool_only.max = Some(10);
        let err =
            resolve_simulation_input(&pool_only).expect_err("workload config should be required");
        assert!(err.to_string().contains("missing workload configuration"));
    }

    #[test]
    fn resolve_evaluate_input_and_config_file_loading() {
        let json_path = write_temp_file("eval_cfg", "json", &sample_config_json());

        let mut args = empty_common_args();
        args.config = Some(json_path.clone());
        args.distribution = Some(CliDistributionModel::Exponential);
        args.queue_model = Some(CliQueueModel::Mdc);
        args.iterations = Some(333);

        let evaluate = EvaluateArgs {
            common: args,
            pool_size: 6,
        };

        let resolved = resolve_evaluate_input(&evaluate).expect("evaluate input should resolve");
        assert_eq!(resolved.workload.requests_per_second, 200.0);
        assert_eq!(resolved.options.iterations, 333);
        assert_eq!(
            resolved.options.distribution,
            DistributionModel::Exponential
        );
        assert_eq!(resolved.options.queue_model, QueueModel::MDC);

        remove_if_exists(&json_path);
    }

    #[test]
    fn resolve_evaluate_and_sweep_report_missing_sections() {
        let evaluate = EvaluateArgs {
            common: empty_common_args(),
            pool_size: 6,
        };
        let err = resolve_evaluate_input(&evaluate).expect_err("missing workload should fail");
        assert!(err.to_string().contains("missing workload configuration"));

        let mut sweep_missing_workload = empty_common_args();
        sweep_missing_workload.max_server_connections = Some(100);
        sweep_missing_workload.connection_overhead_ms = Some(1.0);
        sweep_missing_workload.min = Some(2);
        sweep_missing_workload.max = Some(12);
        let err =
            resolve_sweep_input(&sweep_missing_workload).expect_err("missing workload should fail");
        assert!(err.to_string().contains("missing workload configuration"));

        let mut sweep_missing_pool = empty_common_args();
        sweep_missing_pool.rps = Some(150.0);
        sweep_missing_pool.p50 = Some(5.0);
        sweep_missing_pool.p95 = Some(20.0);
        sweep_missing_pool.p99 = Some(45.0);
        let err = resolve_sweep_input(&sweep_missing_pool).expect_err("missing pool should fail");
        assert!(err.to_string().contains("missing pool configuration"));
    }

    #[test]
    fn load_config_supports_toml_and_rejects_unknown_extension() {
        let toml = r#"
[workload]
requests_per_second = 210.0
latency_p50_ms = 7.0
latency_p95_ms = 22.0
latency_p99_ms = 60.0

[pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 18

[options]
iterations = 1000
"#;
        let toml_path = write_temp_file("cfg", "toml", toml);
        let cfg = load_config(Some(&toml_path)).expect("TOML config should parse");
        assert!(cfg.workload.is_some());
        assert!(cfg.pool.is_some());

        let txt_path = write_temp_file("cfg", "txt", "irrelevant");
        let err = load_config(Some(&txt_path)).expect_err("unsupported extension should fail");
        assert!(err.to_string().contains("unsupported config extension"));

        remove_if_exists(&toml_path);
        remove_if_exists(&txt_path);
    }

    #[test]
    fn samples_file_parsing_supports_mixed_delimiters_and_errors() {
        let samples_path = write_temp_file("samples_ok", "txt", "1.1, 2.2\n3.3\t4.4 5.5");
        let mut args = empty_common_args();
        args.rps = Some(140.0);
        args.p50 = Some(4.0);
        args.p95 = Some(15.0);
        args.p99 = Some(28.0);
        args.max_server_connections = Some(40);
        args.min = Some(2);
        args.max = Some(12);
        args.samples_file = Some(samples_path.clone());

        let resolved = resolve_simulation_input(&args).expect("samples should parse");
        assert_eq!(
            resolved
                .workload
                .raw_samples_ms
                .expect("raw samples should be present")
                .len(),
            5
        );

        let bad_samples_path = write_temp_file("samples_bad", "txt", "1.2, x");
        let err = load_samples_file(&bad_samples_path).expect_err("invalid sample should fail");
        assert!(err.to_string().contains("invalid sample value"));

        let empty_samples_path = write_temp_file("samples_empty", "txt", " , \n\t ");
        let err = load_samples_file(&empty_samples_path).expect_err("empty samples should fail");
        assert!(err.to_string().contains("contains no numeric values"));

        remove_if_exists(&samples_path);
        remove_if_exists(&bad_samples_path);
        remove_if_exists(&empty_samples_path);
    }

    #[test]
    fn resolve_batch_input_supports_json_and_toml_formats() {
        let json_array = format!("[{}]", sample_config_json());
        let json_array_path = write_temp_file("batch_arr", "json", &json_array);
        let input = resolve_batch_input(&BatchArgs {
            config: json_array_path.clone(),
        })
        .expect("json array batch should parse");
        assert_eq!(input.requests.len(), 1);

        let json_wrapped = format!("{{\"requests\": {json_array}}}");
        let json_wrapped_path = write_temp_file("batch_wrap", "json", &json_wrapped);
        let input = resolve_batch_input(&BatchArgs {
            config: json_wrapped_path.clone(),
        })
        .expect("wrapped json batch should parse");
        assert_eq!(input.requests.len(), 1);

        let toml = r#"
[[requests]]
[requests.workload]
requests_per_second = 180.0
latency_p50_ms = 6.0
latency_p95_ms = 20.0
latency_p99_ms = 55.0

[requests.pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 16
"#;
        let toml_path = write_temp_file("batch_toml", "toml", toml);
        let input = resolve_batch_input(&BatchArgs {
            config: toml_path.clone(),
        })
        .expect("toml batch should parse");
        assert_eq!(input.requests.len(), 1);

        remove_if_exists(&json_array_path);
        remove_if_exists(&json_wrapped_path);
        remove_if_exists(&toml_path);
    }

    #[test]
    fn resolve_batch_input_rejects_empty_or_unsupported_config() {
        let empty = write_temp_file("batch_empty", "json", "{\"requests\": []}");
        let err = resolve_batch_input(&BatchArgs {
            config: empty.clone(),
        })
        .expect_err("empty batch should fail");
        assert!(err.to_string().contains("contains no simulation requests"));

        let unknown = write_temp_file("batch_unknown", "yaml", "requests: []");
        let err = resolve_batch_input(&BatchArgs {
            config: unknown.clone(),
        })
        .expect_err("unsupported extension should fail");
        assert!(err
            .to_string()
            .contains("unsupported batch config extension"));

        remove_if_exists(&empty);
        remove_if_exists(&unknown);
    }

    #[test]
    fn resolve_scenario_comparison_input_supports_json_and_toml() {
        let json = format!(
            r#"{{
  "baseline": "normal",
  "scenarios": [
    {{
      "name": "normal",
      "workload": {{
        "requests_per_second": 180.0,
        "latency_p50_ms": 7.0,
        "latency_p95_ms": 25.0,
        "latency_p99_ms": 60.0
      }},
      "pool": {{
        "max_server_connections": 100,
        "connection_overhead_ms": 2.0,
        "min_pool_size": 2,
        "max_pool_size": 20
      }}
    }},
    {}
  ]
}}"#,
            sample_config_json().replace("\"workload\"", "\"name\": \"peak\", \"workload\"")
        );
        let json_path = write_temp_file("scenario_json", "json", &json);
        let input = resolve_scenario_comparison_input(&CompareArgs {
            config: json_path.clone(),
            baseline: None,
        })
        .expect("scenario JSON should parse");
        assert_eq!(input.baseline, "normal");
        assert_eq!(input.scenarios.len(), 2);

        let toml = r#"
baseline = "normal"

[[scenarios]]
name = "normal"
[scenarios.workload]
requests_per_second = 180.0
latency_p50_ms = 7.0
latency_p95_ms = 25.0
latency_p99_ms = 60.0
[scenarios.pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 20

[[scenarios]]
name = "incident"
[scenarios.workload]
requests_per_second = 320.0
latency_p50_ms = 10.0
latency_p95_ms = 45.0
latency_p99_ms = 120.0
[scenarios.pool]
max_server_connections = 120
connection_overhead_ms = 2.0
min_pool_size = 3
max_pool_size = 30
"#;
        let toml_path = write_temp_file("scenario_toml", "toml", toml);
        let input = resolve_scenario_comparison_input(&CompareArgs {
            config: toml_path.clone(),
            baseline: Some("incident".to_string()),
        })
        .expect("scenario TOML should parse");
        assert_eq!(input.baseline, "incident");
        assert_eq!(input.scenarios.len(), 2);

        remove_if_exists(&json_path);
        remove_if_exists(&toml_path);
    }

    #[test]
    fn resolve_scenario_comparison_input_rejects_invalid_files() {
        let missing = unique_temp_path("scenario_missing", "json");
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: missing,
            baseline: None,
        })
        .expect_err("missing scenario file should fail");
        assert!(err
            .to_string()
            .contains("failed to read scenario comparison file"));

        let invalid_json = write_temp_file("scenario_invalid", "json", "{");
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: invalid_json.clone(),
            baseline: None,
        })
        .expect_err("invalid scenario JSON should fail");
        assert!(err
            .to_string()
            .contains("invalid JSON scenario comparison file"));

        let invalid_toml = write_temp_file("scenario_invalid", "toml", "scenarios = [");
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: invalid_toml.clone(),
            baseline: None,
        })
        .expect_err("invalid scenario TOML should fail");
        assert!(err
            .to_string()
            .contains("invalid TOML scenario comparison file"));

        let empty = write_temp_file("scenario_empty", "json", r#"{"scenarios":[]}"#);
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: empty.clone(),
            baseline: None,
        })
        .expect_err("empty scenario list should fail");
        assert!(err.to_string().contains("contains no scenarios"));

        let duplicate = write_temp_file(
            "scenario_duplicate",
            "json",
            &format!(
                r#"[{}, {}]"#,
                sample_config_json().replace("\"workload\"", "\"name\": \"normal\", \"workload\""),
                sample_config_json().replace("\"workload\"", "\"name\": \"normal\", \"workload\"")
            ),
        );
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: duplicate.clone(),
            baseline: None,
        })
        .expect_err("duplicate scenario should fail");
        assert!(err.to_string().contains("duplicate scenario name"));

        let empty_name = write_temp_file(
            "scenario_empty_name",
            "json",
            &format!(
                r#"[{}]"#,
                sample_config_json().replace("\"workload\"", "\"name\": \" \", \"workload\"")
            ),
        );
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: empty_name.clone(),
            baseline: None,
        })
        .expect_err("empty scenario name should fail");
        assert!(err.to_string().contains("scenario names must not be empty"));

        let missing_baseline = write_temp_file(
            "scenario_missing_baseline",
            "json",
            &format!(
                r#"[{}]"#,
                sample_config_json().replace("\"workload\"", "\"name\": \"normal\", \"workload\"")
            ),
        );
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: missing_baseline.clone(),
            baseline: Some("peak".to_string()),
        })
        .expect_err("missing baseline should fail");
        assert!(err.to_string().contains("was not found"));

        let unsupported = write_temp_file("scenario_yaml", "yaml", "scenarios: []");
        let err = resolve_scenario_comparison_input(&CompareArgs {
            config: unsupported.clone(),
            baseline: None,
        })
        .expect_err("unsupported extension should fail");
        assert!(err
            .to_string()
            .contains("unsupported scenario comparison extension"));

        remove_if_exists(&invalid_json);
        remove_if_exists(&invalid_toml);
        remove_if_exists(&empty);
        remove_if_exists(&duplicate);
        remove_if_exists(&empty_name);
        remove_if_exists(&missing_baseline);
        remove_if_exists(&unsupported);
    }

    #[test]
    fn resolve_budget_plan_input_supports_json_and_toml() {
        let json_path = write_temp_file("budget_json", "json", &sample_budget_json());
        let json_input = resolve_budget_plan_input(&BudgetArgs {
            config: json_path.clone(),
        })
        .expect("budget JSON should parse");
        assert_eq!(json_input.max_connections, 120);
        assert_eq!(json_input.services.len(), 1);
        assert_eq!(json_input.services[0].name, "checkout-api");

        let toml = r#"
max_connections = 120
reserved_connections = 20
safety_margin_connections = 10

[[services]]
name = "checkout-api"
replicas = 6
current_pool_size = 8
min_pool_size = 4
max_pool_size = 12
recommended_pool_size = 10
priority = 5
"#;
        let toml_path = write_temp_file("budget_toml", "toml", toml);
        let toml_input = resolve_budget_plan_input(&BudgetArgs {
            config: toml_path.clone(),
        })
        .expect("budget TOML should parse");
        assert_eq!(toml_input.reserved_connections, 20);
        assert_eq!(toml_input.safety_margin_connections, 10);
        assert_eq!(toml_input.services[0].replicas, 6);

        remove_if_exists(&json_path);
        remove_if_exists(&toml_path);
    }

    #[test]
    fn resolve_budget_plan_input_rejects_invalid_files() {
        let missing = unique_temp_path("budget_missing", "json");
        let err = resolve_budget_plan_input(&BudgetArgs { config: missing })
            .expect_err("missing budget file should fail");
        assert!(err.to_string().contains("failed to read budget plan file"));

        let invalid_json = write_temp_file("budget_invalid", "json", "{");
        let err = resolve_budget_plan_input(&BudgetArgs {
            config: invalid_json.clone(),
        })
        .expect_err("invalid budget JSON should fail");
        assert!(err.to_string().contains("invalid JSON budget plan file"));

        let invalid_toml = write_temp_file("budget_invalid", "toml", "services = [");
        let err = resolve_budget_plan_input(&BudgetArgs {
            config: invalid_toml.clone(),
        })
        .expect_err("invalid budget TOML should fail");
        assert!(err.to_string().contains("invalid TOML budget plan file"));

        let unsupported = write_temp_file("budget_yaml", "yaml", "services: []");
        let err = resolve_budget_plan_input(&BudgetArgs {
            config: unsupported.clone(),
        })
        .expect_err("unsupported extension should fail");
        assert!(err
            .to_string()
            .contains("unsupported budget plan extension"));

        remove_if_exists(&invalid_json);
        remove_if_exists(&invalid_toml);
        remove_if_exists(&unsupported);
    }

    #[test]
    fn resolve_telemetry_input_supports_wrapped_and_direct_files() {
        let wrapped_path = write_temp_file("telemetry_wrapped", "json", &sample_telemetry_json());
        let input = resolve_telemetry_input(&TelemetryImportArgs {
            config: wrapped_path.clone(),
            current_pool_size: Some(10),
        })
        .expect("wrapped telemetry config should parse");
        assert_eq!(input.snapshot.service_name.as_deref(), Some("checkout-api"));
        assert_eq!(input.snapshot.current_pool_size, 10);
        assert_eq!(input.options.seed, Some(9));

        let direct_toml = r#"
service_name = "billing-api"
window = "15m"
current_pool_size = 5

[workload]
requests_per_second = 120.0
latency_p50_ms = 5.0
latency_p95_ms = 18.0
latency_p99_ms = 45.0

[pool]
max_server_connections = 80
connection_overhead_ms = 1.0
min_pool_size = 2
max_pool_size = 12
"#;
        let direct_path = write_temp_file("telemetry_direct", "toml", direct_toml);
        let input = resolve_telemetry_input(&TelemetryImportArgs {
            config: direct_path.clone(),
            current_pool_size: None,
        })
        .expect("direct telemetry config should parse");
        assert_eq!(input.snapshot.service_name.as_deref(), Some("billing-api"));
        assert_eq!(input.snapshot.current_pool_size, 5);
        assert_eq!(input.options, SimulationOptions::default());

        remove_if_exists(&wrapped_path);
        remove_if_exists(&direct_path);
    }

    #[test]
    fn resolve_telemetry_input_rejects_bad_files() {
        let missing = unique_temp_path("telemetry_missing", "json");
        let err = resolve_telemetry_input(&TelemetryImportArgs {
            config: missing,
            current_pool_size: None,
        })
        .expect_err("missing telemetry file should fail");
        assert!(err
            .to_string()
            .contains("failed to read telemetry config file"));

        let unsupported = write_temp_file("telemetry_bad", "yaml", "telemetry: {}");
        let err = resolve_telemetry_input(&TelemetryImportArgs {
            config: unsupported.clone(),
            current_pool_size: None,
        })
        .expect_err("unsupported telemetry extension should fail");
        assert!(err
            .to_string()
            .contains("unsupported telemetry config extension"));

        let invalid = write_temp_file("telemetry_invalid", "json", "{}");
        let err = resolve_telemetry_input(&TelemetryImportArgs {
            config: invalid.clone(),
            current_pool_size: None,
        })
        .expect_err("invalid telemetry config should fail");
        assert!(err
            .to_string()
            .contains("invalid JSON telemetry config file"));

        let invalid_toml = write_temp_file("telemetry_invalid", "toml", "telemetry = [");
        let err = resolve_telemetry_input(&TelemetryImportArgs {
            config: invalid_toml.clone(),
            current_pool_size: None,
        })
        .expect_err("invalid telemetry TOML should fail");
        assert!(err
            .to_string()
            .contains("invalid TOML telemetry config file"));

        remove_if_exists(&unsupported);
        remove_if_exists(&invalid);
        remove_if_exists(&invalid_toml);
    }

    #[test]
    fn parse_batch_helpers_cover_direct_paths() {
        let items = parse_batch_json(&format!("[{}]", sample_config_json()))
            .expect("parse_batch_json should support array");
        assert_eq!(items.len(), 1);

        let wrapped = format!("{{\"requests\": [{}]}}", sample_config_json());
        let items =
            parse_batch_json(&wrapped).expect("parse_batch_json should support wrapped object");
        assert_eq!(items.len(), 1);

        let toml = r#"
[[requests]]
[requests.workload]
requests_per_second = 120.0
latency_p50_ms = 5.0
latency_p95_ms = 16.0
latency_p99_ms = 40.0
[requests.pool]
max_server_connections = 80
connection_overhead_ms = 1.0
min_pool_size = 2
max_pool_size = 12
"#;
        let items = parse_batch_toml(toml).expect("parse_batch_toml should parse");
        assert_eq!(items.len(), 1);
    }
}
