use std::fs;

use anyhow::{anyhow, Context, Result};
use poolsim_core::{
    telemetry::TelemetrySnapshot,
    types::{PoolConfig, SimulationOptions, WorkloadConfig},
};
use serde_json::Value;

use crate::{args::OtlpImportArgs, config::TelemetryInput};

pub(crate) fn resolve_otlp_input(args: &OtlpImportArgs) -> Result<TelemetryInput> {
    let raw = fs::read_to_string(&args.config)
        .with_context(|| format!("failed to read OTLP file {}", args.config.display()))?;
    let root: Value = serde_json::from_str(&raw)
        .with_context(|| format!("invalid OTLP JSON file {}", args.config.display()))?;

    let workload = WorkloadConfig {
        requests_per_second: metric_value(&root, &args.rps_metric)?,
        latency_p50_ms: metric_value(&root, &args.p50_metric)?,
        latency_p95_ms: metric_value(&root, &args.p95_metric)?,
        latency_p99_ms: metric_value(&root, &args.p99_metric)?,
        raw_samples_ms: None,
        step_load_profile: None,
    };

    let defaults = SimulationOptions::default();

    Ok(TelemetryInput {
        snapshot: TelemetrySnapshot {
            service_name: args.service_name.clone(),
            window: args.window.clone(),
            observed_at: args.observed_at.clone(),
            current_pool_size: args.current_pool_size,
            workload,
            pool: PoolConfig {
                max_server_connections: args.max_server_connections,
                connection_overhead_ms: args.connection_overhead_ms,
                idle_timeout_ms: args.idle_timeout_ms,
                min_pool_size: args.min,
                max_pool_size: args.max,
            },
        },
        options: SimulationOptions {
            iterations: args.iterations.unwrap_or(defaults.iterations),
            seed: args.seed,
            distribution: args
                .distribution
                .map(Into::into)
                .unwrap_or(defaults.distribution),
            queue_model: args
                .queue_model
                .map(Into::into)
                .unwrap_or(defaults.queue_model),
            target_wait_p99_ms: args.target_wait_p99_ms.unwrap_or(45.0),
            max_acceptable_rho: args.max_acceptable_rho.unwrap_or(0.85),
        },
    })
}

fn metric_value(root: &Value, name: &str) -> Result<f64> {
    find_metric(root, name)
        .and_then(first_numeric)
        .ok_or_else(|| anyhow!("OTLP metric {name:?} was not found or had no numeric datapoint"))
}

fn find_metric<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            if map.get("name").and_then(Value::as_str) == Some(name) {
                return Some(value);
            }
            map.values().find_map(|child| find_metric(child, name))
        }
        Value::Array(items) => items.iter().find_map(|child| find_metric(child, name)),
        _ => None,
    }
}

fn first_numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse().ok(),
        Value::Object(map) => {
            for key in ["asDouble", "asInt", "doubleValue", "intValue", "value"] {
                if let Some(number) = map.get(key).and_then(first_numeric) {
                    return Some(number);
                }
            }
            for key in ["sum", "gauge", "histogram", "dataPoints"] {
                if let Some(number) = map.get(key).and_then(first_numeric) {
                    return Some(number);
                }
            }
            map.values().find_map(first_numeric)
        }
        Value::Array(items) => items.iter().find_map(first_numeric),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_otlp(content: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("poolsim_otlp_{ts}.json"));
        fs::write(&path, content).expect("OTLP fixture should write");
        path
    }

    fn args(path: &std::path::Path) -> OtlpImportArgs {
        OtlpImportArgs {
            config: path.to_path_buf(),
            rps_metric: "poolsim.rps".to_string(),
            p50_metric: "poolsim.latency.p50_ms".to_string(),
            p95_metric: "poolsim.latency.p95_ms".to_string(),
            p99_metric: "poolsim.latency.p99_ms".to_string(),
            service_name: Some("checkout-api".to_string()),
            window: Some("5m".to_string()),
            observed_at: None,
            current_pool_size: 8,
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

    #[test]
    fn resolves_otlp_metrics_into_telemetry_input() {
        let path = temp_otlp(
            r#"{
              "resourceMetrics": [{"scopeMetrics": [{"metrics": [
                {"name": "poolsim.rps", "sum": {"dataPoints": [{"asDouble": 180.0}]}},
                {"name": "poolsim.latency.p50_ms", "gauge": {"dataPoints": [{"asDouble": 8.0}]}},
                {"name": "poolsim.latency.p95_ms", "gauge": {"dataPoints": [{"asDouble": 30.0}]}},
                {"name": "poolsim.latency.p99_ms", "gauge": {"dataPoints": [{"asDouble": 70.0}]}}
              ]}]}]
            }"#,
        );
        let input = resolve_otlp_input(&args(&path)).expect("OTLP should resolve");
        assert_eq!(input.snapshot.service_name.as_deref(), Some("checkout-api"));
        assert_eq!(input.snapshot.workload.requests_per_second, 180.0);
        assert_eq!(input.snapshot.workload.latency_p99_ms, 70.0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_missing_otlp_metric() {
        let path = temp_otlp(r#"{"metrics": []}"#);
        let err = resolve_otlp_input(&args(&path)).expect_err("missing metric should fail");
        assert!(err.to_string().contains("poolsim.rps"));
        let _ = fs::remove_file(path);
    }
}
