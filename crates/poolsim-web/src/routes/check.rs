//! Compatibility-check routes.
//!
//! Route:
//!
//! - `POST /v1/check/pooler`
//!
//! Request payload:
//!
//! - [`poolsim_core::pooler::PoolerCompatibilityInput`]
//!
//! Response payload:
//!
//! - [`poolsim_core::pooler::PoolerCompatibilityReport`]

use axum::Json;
use poolsim_core::pooler::{
    check_pooler_compatibility, PoolerCompatibilityInput, PoolerCompatibilityReport,
};

use crate::error::AppError;

/// Handles `POST /v1/check/pooler`.
pub async fn pooler_handler(body: String) -> Result<Json<PoolerCompatibilityReport>, AppError> {
    let req: PoolerCompatibilityInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(check_pooler_compatibility(&req)))
}
