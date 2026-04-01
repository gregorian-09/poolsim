//! Public route modules for the REST and WebSocket API.
//!
//! These route modules correspond directly to the published HTTP surface:
//!
//! - `/v1/health`
//! - `/v1/models`
//! - `/v1/simulate`
//! - `/v1/evaluate`
//! - `/v1/sensitivity`
//! - `/v1/batch`
//! - `/v1/live`
//!
//! Request and response types live in the child modules, including:
//!
//! - [`crate::routes::simulate::SimulationRequest`]
//! - [`crate::routes::evaluate::EvaluateRequest`]
//! - [`crate::routes::sensitivity::SensitivityRequest`]
//! - [`crate::routes::health::HealthResponse`]
//! - [`crate::routes::models::ModelsResponse`]

/// Batch simulation endpoint.
pub mod batch;
/// Fixed pool-size evaluation endpoint.
pub mod evaluate;
/// Health endpoint.
pub mod health;
/// Live WebSocket streaming endpoint.
pub mod live;
/// Supported-models metadata endpoint.
pub mod models;
/// Sensitivity analysis endpoint.
pub mod sensitivity;
/// Single simulation endpoint.
pub mod simulate;
