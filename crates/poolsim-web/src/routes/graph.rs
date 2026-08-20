//! Connection graph routes.
//!
//! Route:
//!
//! - `POST /v1/graph/ownership`
//!
//! Request payload:
//!
//! - [`poolsim_core::ownership::ConnectionOwnershipInput`]
//!
//! Response payload:
//!
//! - [`poolsim_core::ownership::ConnectionOwnershipReport`]

use axum::Json;
use poolsim_core::ownership::{
    build_connection_ownership_graph, ConnectionOwnershipInput, ConnectionOwnershipReport,
};

use crate::error::AppError;

/// Handles `POST /v1/graph/ownership`.
pub async fn ownership_handler(body: String) -> Result<Json<ConnectionOwnershipReport>, AppError> {
    let req: ConnectionOwnershipInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(build_connection_ownership_graph(&req)?))
}
