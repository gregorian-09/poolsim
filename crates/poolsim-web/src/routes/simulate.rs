//! Full simulation endpoint.
//!
//! Route:
//!
//! - `POST /v1/simulate`
//!
//! Request payload:
//!
//! - [`crate::routes::simulate::SimulationRequest`]
//!
//! Response payload:
//!
//! - [`poolsim_core::types::SimulationReport`]

use axum::{extract::State, Json};
use poolsim_core::{simulate, types::SimulationReport};
use serde::Deserialize;

use crate::{error::AppError, state::AppState};

/// JSON payload for `/v1/simulate`.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulationRequest {
    /// Workload configuration.
    pub workload: poolsim_core::types::WorkloadConfig,
    /// Pool configuration.
    pub pool: poolsim_core::types::PoolConfig,
    /// Optional simulation options.
    #[serde(default)]
    pub options: poolsim_core::types::SimulationOptions,
}

/// Handles `POST /v1/simulate`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<SimulationReport>, AppError> {
    let req: SimulationRequest =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }
    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || simulate(&req.workload, &req.pool, &req.options)),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let report = join.map_err(AppError::from)??;
    Ok(Json(report))
}
