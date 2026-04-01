//! Fixed pool-size evaluation endpoint.
//!
//! Route:
//!
//! - `POST /v1/evaluate`
//!
//! Request payload:
//!
//! - [`crate::routes::evaluate::EvaluateRequest`]
//!
//! Response payload:
//!
//! - [`poolsim_core::types::EvaluationResult`]

use axum::{extract::State, Json};
use poolsim_core::{evaluate, types::EvaluationResult};
use serde::Deserialize;

use crate::{error::AppError, state::AppState};

/// JSON payload for `/v1/evaluate`.
#[derive(Debug, Clone, Deserialize)]
pub struct EvaluateRequest {
    /// Workload configuration.
    pub workload: poolsim_core::types::WorkloadConfig,
    /// Pool size to evaluate.
    pub pool_size: u32,
    /// Optional simulation options.
    #[serde(default)]
    pub options: poolsim_core::types::SimulationOptions,
}

/// Handles `POST /v1/evaluate`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<EvaluationResult>, AppError> {
    let req: EvaluateRequest =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }
    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || evaluate(&req.workload, req.pool_size, &req.options)),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let result = join.map_err(AppError::from)??;
    Ok(Json(result))
}
