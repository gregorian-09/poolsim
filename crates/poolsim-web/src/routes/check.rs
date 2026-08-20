//! Compatibility-check routes.
//!
//! Route:
//!
//! - `POST /v1/check/pooler`
//! - `POST /v1/check/session-state`
//!
//! Request payload:
//!
//! - [`poolsim_core::pooler::PoolerCompatibilityInput`]
//! - [`poolsim_core::pooler::SessionStateCompatibilityInput`]
//!
//! Response payload:
//!
//! - [`poolsim_core::pooler::PoolerCompatibilityReport`]
//! - [`poolsim_core::pooler::SessionStateCompatibilityReport`]

use axum::Json;
use poolsim_core::pooler::{
    analyze_session_state_compatibility, check_pooler_compatibility, PoolerCompatibilityInput,
    PoolerCompatibilityReport, SessionStateCompatibilityInput, SessionStateCompatibilityReport,
};

use crate::error::AppError;

/// Handles `POST /v1/check/pooler`.
pub async fn pooler_handler(body: String) -> Result<Json<PoolerCompatibilityReport>, AppError> {
    let req: PoolerCompatibilityInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(check_pooler_compatibility(&req)))
}

/// Handles `POST /v1/check/session-state`.
pub async fn session_state_handler(
    body: String,
) -> Result<Json<SessionStateCompatibilityReport>, AppError> {
    let req: SessionStateCompatibilityInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(analyze_session_state_compatibility(&req)))
}
