//! Planning routes for deployment-topology capacity checks.
//!
//! Route:
//!
//! - `POST /v1/plan/serverless`
//!
//! Request payload:
//!
//! - [`poolsim_core::serverless::ServerlessConcurrencyInput`]
//!
//! Response payload:
//!
//! - [`poolsim_core::serverless::ServerlessConcurrencyReport`]

use axum::Json;
use poolsim_core::serverless::{
    plan_serverless_concurrency, ServerlessConcurrencyInput, ServerlessConcurrencyReport,
};

use crate::error::AppError;

/// Handles `POST /v1/plan/serverless`.
pub async fn serverless_handler(
    body: String,
) -> Result<Json<ServerlessConcurrencyReport>, AppError> {
    let req: ServerlessConcurrencyInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(plan_serverless_concurrency(&req)?))
}
