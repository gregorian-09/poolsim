//! Endpoint-classification routes.
//!
//! Route:
//!
//! - `POST /v1/classify/endpoint`
//!
//! Request payload:
//!
//! - [`poolsim_core::pooler::EndpointClassificationInput`]
//!
//! Response payload:
//!
//! - [`poolsim_core::pooler::EndpointClassificationReport`]

use axum::Json;
use poolsim_core::pooler::{
    classify_endpoint, EndpointClassificationInput, EndpointClassificationReport,
};

use crate::error::AppError;

/// Handles `POST /v1/classify/endpoint`.
pub async fn endpoint_handler(
    body: String,
) -> Result<Json<EndpointClassificationReport>, AppError> {
    let req: EndpointClassificationInput =
        serde_json::from_str(&body).map_err(|e| AppError::InvalidJson(e.to_string()))?;
    Ok(Json(classify_endpoint(&req)))
}
