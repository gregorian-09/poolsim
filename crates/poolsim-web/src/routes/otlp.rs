//! OTLP recommendation endpoint.
//!
//! Route:
//!
//! - `POST /v1/otlp/recommend`
//!
//! Request payload:
//!
//! - [`crate::routes::otlp::OtlpRecommendationRequest`]
//!
//! Response payload:
//!
//! - [`poolsim_core::telemetry::TelemetryRecommendation`]

use axum::{extract::State, Json};
use poolsim_core::{
    otlp::{workload_from_otlp_json, OtlpMetricNames},
    telemetry::{recommend_from_telemetry, TelemetryRecommendation, TelemetrySnapshot},
    types::{PoolConfig, SimulationOptions},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{error::AppError, state::AppState};

/// JSON payload for `/v1/otlp/recommend`.
#[derive(Debug, Clone, Deserialize)]
pub struct OtlpRecommendationRequest {
    /// OTLP JSON metric export payload.
    pub otlp: Value,
    /// Optional metric-name mapping. Defaults match the CLI `import otlp` command.
    #[serde(default)]
    pub metric_names: OtlpMetricNames,
    /// Optional service, application, or pool name.
    #[serde(default)]
    pub service_name: Option<String>,
    /// Optional telemetry window label, such as `5m` or `1h`.
    #[serde(default)]
    pub window: Option<String>,
    /// Optional timestamp or timestamp range indicating when the snapshot was observed.
    #[serde(default)]
    pub observed_at: Option<String>,
    /// Current production pool size to compare against the recommendation.
    pub current_pool_size: u32,
    /// Pool bounds and backend connection limits to use for recommendation.
    pub pool: PoolConfig,
    /// Optional simulation options for the recommendation run.
    #[serde(default)]
    pub options: SimulationOptions,
}

/// Handles `POST /v1/otlp/recommend`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<TelemetryRecommendation>, AppError> {
    let req: OtlpRecommendationRequest =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }

    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || {
            let workload = workload_from_otlp_json(&req.otlp, &req.metric_names)?;
            let snapshot = TelemetrySnapshot {
                service_name: req.service_name,
                window: req.window,
                observed_at: req.observed_at,
                current_pool_size: req.current_pool_size,
                workload,
                pool: req.pool,
            };
            recommend_from_telemetry(&snapshot, &req.options)
        }),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let recommendation = join.map_err(AppError::from)??;
    Ok(Json(recommendation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_deserializes_with_default_metric_names() {
        let request: OtlpRecommendationRequest = serde_json::from_value(serde_json::json!({
            "otlp": {"metrics": []},
            "current_pool_size": 8,
            "pool": {
                "max_server_connections": 100,
                "connection_overhead_ms": 2.0,
                "min_pool_size": 2,
                "max_pool_size": 20
            }
        }))
        .expect("request should deserialize");
        assert_eq!(request.metric_names.rps_metric, "poolsim.rps");
    }
}
