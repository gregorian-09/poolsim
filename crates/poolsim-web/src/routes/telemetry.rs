//! Telemetry recommendation endpoint.
//!
//! Route:
//!
//! - `POST /v1/telemetry/recommend`
//!
//! Request payload:
//!
//! - [`crate::routes::telemetry::TelemetryRecommendationRequest`]
//!
//! Response payload:
//!
//! - [`poolsim_core::telemetry::TelemetryRecommendation`]

use axum::{extract::State, Json};
use poolsim_core::{
    telemetry::{recommend_from_telemetry, TelemetryRecommendation, TelemetrySnapshot},
    types::SimulationOptions,
};
use serde::Deserialize;

use crate::{error::AppError, state::AppState};

/// JSON payload for `/v1/telemetry/recommend`.
#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryRecommendationRequest {
    /// Imported telemetry snapshot.
    pub telemetry: TelemetrySnapshot,
    /// Optional simulation options for the recommendation run.
    #[serde(default)]
    pub options: SimulationOptions,
}

/// Handles `POST /v1/telemetry/recommend`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<TelemetryRecommendation>, AppError> {
    let req: TelemetryRecommendationRequest =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }
    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || recommend_from_telemetry(&req.telemetry, &req.options)),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let recommendation = join.map_err(AppError::from)??;
    Ok(Json(recommendation))
}
