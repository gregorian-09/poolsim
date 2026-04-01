//! Error types and helpers for the public `poolsim-core` API.
//!
//! `PoolsimError` is designed to work for both library callers and service
//! adapters:
//!
//! - stable machine-readable codes via [`PoolsimError::code`]
//! - optional structured error details via [`PoolsimError::details`]
//! - human-readable messages via `Display`
//!
//! This makes the same error type suitable for direct Rust usage, CLI
//! rendering, and HTTP/WebSocket translation in `poolsim-web`.

use serde_json::Value;
use thiserror::Error;

/// Error type returned by `poolsim-core` public APIs.
#[derive(Debug, Error)]
pub enum PoolsimError {
    /// Invalid input payload or violated input constraints.
    #[error("{message}")]
    InvalidInput {
        /// Stable machine-readable error code.
        code: &'static str,
        /// Human-readable error message.
        message: String,
        /// Optional structured details for API responses.
        details: Option<Value>,
    },

    /// System is saturated (`rho >= 1.0`) for the requested configuration.
    #[error("system saturated at rho={rho:.4}")]
    Saturated {
        /// Utilization ratio (`rho`) at failure time.
        rho: f64,
    },

    /// Distribution fitting/sampling failure.
    #[error("distribution error: {0}")]
    Distribution(String),

    /// Simulation runtime failure.
    #[error("simulation error: {0}")]
    Simulation(String),
}

impl PoolsimError {
    /// Creates a standardized invalid-input error.
    pub fn invalid_input(code: &'static str, message: impl Into<String>, details: Option<Value>) -> Self {
        Self::InvalidInput {
            code,
            message: message.into(),
            details,
        }
    }

    /// Returns the stable machine-readable error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput { code, .. } => code,
            Self::Saturated { .. } => "SATURATED",
            Self::Distribution(_) => "DISTRIBUTION_ERROR",
            Self::Simulation(_) => "SIMULATION_ERROR",
        }
    }

    /// Returns optional structured details associated with this error.
    pub fn details(&self) -> Option<&Value> {
        match self {
            Self::InvalidInput { details, .. } => details.as_ref(),
            _ => None,
        }
    }
}
