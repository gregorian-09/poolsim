//! Supported-model metadata endpoint.
//!
//! Route:
//!
//! - `GET /v1/models`
//!
//! Response payload:
//!
//! - [`crate::routes::models::ModelsResponse`]

use axum::Json;
use serde::Serialize;

/// JSON response listing supported simulation models.
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    /// Supported latency distribution models.
    pub distribution_models: Vec<&'static str>,
    /// Supported queue models.
    pub queue_models: Vec<&'static str>,
}

/// Handles `GET /v1/models`.
pub async fn handler() -> Json<ModelsResponse> {
    Json(ModelsResponse {
        distribution_models: vec!["LogNormal", "Exponential", "Empirical", "Gamma"],
        queue_models: vec!["MMC", "MDC"],
    })
}
