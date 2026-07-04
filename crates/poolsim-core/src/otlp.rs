//! OpenTelemetry metric payload helpers.
//!
//! This module converts OTLP-style JSON metric exports into Poolsim workload
//! inputs. It is shared by the CLI and web crates so OTLP metric-name handling,
//! numeric datapoint extraction, and error reporting stay consistent.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error::PoolsimError, types::WorkloadConfig};

/// Default request-rate metric name used by OTLP import helpers.
pub const DEFAULT_RPS_METRIC: &str = "poolsim.rps";
/// Default p50 latency metric name used by OTLP import helpers.
pub const DEFAULT_P50_METRIC: &str = "poolsim.latency.p50_ms";
/// Default p95 latency metric name used by OTLP import helpers.
pub const DEFAULT_P95_METRIC: &str = "poolsim.latency.p95_ms";
/// Default p99 latency metric name used by OTLP import helpers.
pub const DEFAULT_P99_METRIC: &str = "poolsim.latency.p99_ms";

fn default_rps_metric() -> String {
    DEFAULT_RPS_METRIC.to_string()
}

fn default_p50_metric() -> String {
    DEFAULT_P50_METRIC.to_string()
}

fn default_p95_metric() -> String {
    DEFAULT_P95_METRIC.to_string()
}

fn default_p99_metric() -> String {
    DEFAULT_P99_METRIC.to_string()
}

/// Metric-name mapping used to extract Poolsim workload values from OTLP JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OtlpMetricNames {
    /// Metric containing requests per second.
    #[serde(default = "default_rps_metric")]
    pub rps_metric: String,
    /// Metric containing p50 latency in milliseconds.
    #[serde(default = "default_p50_metric")]
    pub p50_metric: String,
    /// Metric containing p95 latency in milliseconds.
    #[serde(default = "default_p95_metric")]
    pub p95_metric: String,
    /// Metric containing p99 latency in milliseconds.
    #[serde(default = "default_p99_metric")]
    pub p99_metric: String,
}

impl Default for OtlpMetricNames {
    fn default() -> Self {
        Self {
            rps_metric: default_rps_metric(),
            p50_metric: default_p50_metric(),
            p95_metric: default_p95_metric(),
            p99_metric: default_p99_metric(),
        }
    }
}

/// Builds a workload from OTLP JSON metrics.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when any configured metric is missing
/// or does not contain a numeric datapoint.
pub fn workload_from_otlp_json(
    root: &Value,
    names: &OtlpMetricNames,
) -> Result<WorkloadConfig, PoolsimError> {
    Ok(WorkloadConfig {
        requests_per_second: metric_value(root, &names.rps_metric)?,
        latency_p50_ms: metric_value(root, &names.p50_metric)?,
        latency_p95_ms: metric_value(root, &names.p95_metric)?,
        latency_p99_ms: metric_value(root, &names.p99_metric)?,
        raw_samples_ms: None,
        step_load_profile: None,
    })
}

/// Extracts the first numeric datapoint from a named OTLP metric.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when the metric is absent or no
/// numeric datapoint can be found.
pub fn metric_value(root: &Value, name: &str) -> Result<f64, PoolsimError> {
    find_metric(root, name)
        .and_then(first_numeric)
        .ok_or_else(|| {
            PoolsimError::invalid_input(
                "OTLP_METRIC_NOT_FOUND",
                format!("OTLP metric {name:?} was not found or had no numeric datapoint"),
                Some(serde_json::json!({ "metric": name })),
            )
        })
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

    fn fixture() -> Value {
        serde_json::json!({
            "resourceMetrics": [{
                "scopeMetrics": [{
                    "metrics": [
                        {"name": "poolsim.rps", "sum": {"dataPoints": [{"asDouble": 180.0}]}},
                        {"name": "poolsim.latency.p50_ms", "gauge": {"dataPoints": [{"asDouble": 8.0}]}},
                        {"name": "poolsim.latency.p95_ms", "gauge": {"dataPoints": [{"asInt": "30"}]}},
                        {"name": "poolsim.latency.p99_ms", "gauge": {"dataPoints": [{"value": 70.0}]}}
                    ]
                }]
            }]
        })
    }

    #[test]
    fn extracts_workload_from_otlp_json() {
        let workload = workload_from_otlp_json(&fixture(), &OtlpMetricNames::default())
            .expect("OTLP workload should resolve");
        assert_eq!(workload.requests_per_second, 180.0);
        assert_eq!(workload.latency_p95_ms, 30.0);
        assert_eq!(workload.latency_p99_ms, 70.0);
    }

    #[test]
    fn reports_missing_metric_with_stable_error_code() {
        let err = metric_value(&serde_json::json!({ "metrics": [] }), "missing")
            .expect_err("missing metric should fail");
        assert_eq!(err.code(), "OTLP_METRIC_NOT_FOUND");
    }
}
