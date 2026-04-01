//! Shared immutable application state for `poolsim-web`.
//!
//! The web crate keeps runtime configuration intentionally small and explicit.
//! [`AppState`] currently carries:
//!
//! - per-request simulation timeout
//! - service version string exposed by `/v1/health`

use std::time::Duration;

/// Shared immutable state injected into route handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Per-request simulation timeout.
    pub simulation_timeout: Duration,
    /// Service version string exposed by health endpoints.
    pub version: &'static str,
}
