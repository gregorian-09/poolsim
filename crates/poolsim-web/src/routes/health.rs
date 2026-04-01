//! Health endpoint for service liveness and version reporting.
//!
//! Route:
//!
//! - `GET /v1/health`
//!
//! Response payload:
//!
//! - [`crate::routes::health::HealthResponse`]

use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

/// JSON response for `/v1/health`.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service health status.
    pub status: &'static str,
    /// Service version.
    pub version: &'static str,
}

/// Handles `GET /v1/health`.
pub async fn handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: state.version,
    })
}
