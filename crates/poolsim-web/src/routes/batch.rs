//! Batch simulation endpoint.
//!
//! Route:
//!
//! - `POST /v1/batch`
//!
//! Request payload:
//!
//! - `Vec<`[`crate::routes::simulate::SimulationRequest`]`>`
//!
//! Response payload:
//!
//! - `Vec<`[`poolsim_core::types::SimulationReport`]`>`

use axum::{extract::State, Json};
use poolsim_core::{simulate, types::SimulationReport};

use crate::{error::AppError, routes::simulate::SimulationRequest, state::AppState};

/// Handles `POST /v1/batch`.
pub async fn handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Vec<SimulationReport>>, AppError> {
    let requests: Vec<SimulationRequest> =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;

    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(AppError::Timeout);
    }
    let join = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || {
            requests
                .into_iter()
                .map(|req| simulate(&req.workload, &req.pool, &req.options))
                .collect::<Result<Vec<_>, _>>()
        }),
    )
    .await
    .map_err(|_| AppError::Timeout)?;

    let reports = join.map_err(AppError::from)??;
    Ok(Json(reports))
}
