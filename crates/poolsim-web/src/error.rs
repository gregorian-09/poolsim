//! Web-layer error translation for `poolsim-web`.
//!
//! This module converts core-library failures and web-layer operational errors
//! into a stable JSON error shape with:
//!
//! - `error`
//! - `code`
//! - `details`
//!
//! It is used by every REST handler and by the WebSocket route when surfacing
//! request and simulation failures to clients.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use poolsim_core::error::PoolsimError;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

/// Web-layer error type mapped to consistent HTTP error responses.
#[derive(Debug, Error)]
pub enum AppError {
    /// Wrapped core-library error.
    #[error(transparent)]
    Core(#[from] PoolsimError),

    /// JSON payload parsing failure.
    #[error("invalid json payload: {0}")]
    InvalidJson(String),

    /// Simulation exceeded configured timeout.
    #[error("simulation timed out")]
    Timeout,

    /// Internal task join failure.
    #[error("internal join error: {0}")]
    Join(String),
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    code: String,
    details: Option<Value>,
}

impl From<tokio::task::JoinError> for AppError {
    fn from(value: tokio::task::JoinError) -> Self {
        Self::Join(value.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Core(err) => {
                let (status, details) = match &err {
                    PoolsimError::InvalidInput { details, .. } => (StatusCode::BAD_REQUEST, details.clone()),
                    PoolsimError::Saturated { rho } => {
                        (StatusCode::UNPROCESSABLE_ENTITY, Some(json!({ "rho": rho })))
                    }
                    PoolsimError::Distribution(_) | PoolsimError::Simulation(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, err.details().cloned())
                    }
                };

                let body = ErrorBody {
                    error: err.to_string(),
                    code: err.code().to_string(),
                    details,
                };
                (status, Json(body)).into_response()
            }
            AppError::InvalidJson(message) => {
                let body = ErrorBody {
                    error: message,
                    code: "INVALID_JSON".to_string(),
                    details: None,
                };
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            AppError::Timeout => {
                let body = ErrorBody {
                    error: "simulation timed out".to_string(),
                    code: "SIMULATION_TIMEOUT".to_string(),
                    details: None,
                };
                (StatusCode::REQUEST_TIMEOUT, Json(body)).into_response()
            }
            AppError::Join(message) => {
                let body = ErrorBody {
                    error: message,
                    code: "INTERNAL_JOIN_ERROR".to_string(),
                    details: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::StatusCode,
        response::IntoResponse,
    };
    use serde_json::{json, Value};

    use super::*;

    async fn response_parts(error: AppError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let json: Value = serde_json::from_slice(&body).expect("body should be valid JSON");
        (status, json)
    }

    #[tokio::test]
    async fn app_error_maps_core_invalid_input_and_saturated_statuses() {
        let invalid = AppError::Core(PoolsimError::invalid_input(
            "INVALID_FIELD",
            "bad input",
            Some(json!({"field": "rps"})),
        ));
        let (status, body) = response_parts(invalid).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "INVALID_FIELD");
        assert_eq!(body["details"]["field"], "rps");

        let saturated = AppError::Core(PoolsimError::Saturated { rho: 0.97 });
        let (status, body) = response_parts(saturated).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "SATURATED");
        assert_eq!(body["details"]["rho"], 0.97);
    }

    #[tokio::test]
    async fn app_error_maps_internal_core_errors_to_500() {
        let dist = AppError::Core(PoolsimError::Distribution("shape mismatch".to_string()));
        let (status, body) = response_parts(dist).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "DISTRIBUTION_ERROR");
        assert!(body["details"].is_null());

        let sim = AppError::Core(PoolsimError::Simulation("numerical issue".to_string()));
        let (status, body) = response_parts(sim).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "SIMULATION_ERROR");
        assert!(body["details"].is_null());
    }

    #[tokio::test]
    async fn app_error_maps_invalid_json_timeout_and_join_variants() {
        let (status, body) = response_parts(AppError::InvalidJson("bad json".to_string())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "INVALID_JSON");
        assert!(body["details"].is_null());

        let (status, body) = response_parts(AppError::Timeout).await;
        assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
        assert_eq!(body["code"], "SIMULATION_TIMEOUT");

        let join_err = tokio::spawn(async {
            panic!("boom");
        })
        .await
        .expect_err("task should panic and produce JoinError");
        let (status, body) = response_parts(AppError::from(join_err)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "INTERNAL_JOIN_ERROR");
        assert!(body["error"].is_string());
    }
}
