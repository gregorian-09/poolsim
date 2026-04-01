//! Sensitivity-analysis endpoint.
//!
//! Route:
//!
//! - `POST /v1/sensitivity`
//!
//! Request payload:
//!
//! - [`crate::routes::sensitivity::SensitivityRequest`]
//!
//! Response payload:
//!
//! - `Vec<`[`poolsim_core::types::SensitivityRow`]`>`

use axum::{extract::State, Json};
use poolsim_core::{sweep_with_options, types::SensitivityRow};
use serde::Deserialize;

use crate::{error::AppError, state::AppState};

/// JSON payload for `/v1/sensitivity`.
#[derive(Debug, Clone, Deserialize)]
pub struct SensitivityRequest {
    /// Workload configuration.
    pub workload: poolsim_core::types::WorkloadConfig,
    /// Pool configuration.
    pub pool: poolsim_core::types::PoolConfig,
    /// Optional simulation options.
    #[serde(default)]
    pub options: poolsim_core::types::SimulationOptions,
}

/// Handles `POST /v1/sensitivity`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Vec<SensitivityRow>>, AppError> {
    let req: SensitivityRequest =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }
    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || {
            sweep_with_options(&req.workload, &req.pool, &req.options)
        }),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let rows = join.map_err(AppError::from)??;
    Ok(Json(rows))
}
