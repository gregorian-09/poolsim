#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/poolsim-web/0.1.0")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#![deny(missing_docs)]

/// Error mapping and HTTP response conversion.
pub mod error;
/// Middleware modules (rate-limiting, etc.).
pub mod middleware;
/// Route handlers and request/response models.
pub mod routes;
/// Shared application state.
pub mod state;

use axum::{
    http::Method,
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use middleware::rate_limit::RateLimitState;
use state::AppState;
use tower_http::cors::CorsLayer;

/// Builds the application router with routes, rate-limit middleware, and CORS policy.
pub fn build_app(state: AppState, rate_limit_state: RateLimitState, cors_origins: &str) -> Router {
    Router::new()
        .route("/v1/health", get(routes::health::handler))
        .route("/v1/models", get(routes::models::handler))
        .route("/v1/simulate", post(routes::simulate::handler))
        .route("/v1/sensitivity", post(routes::sensitivity::handler))
        .route("/v1/evaluate", post(routes::evaluate::handler))
        .route("/v1/batch", post(routes::batch::handler))
        .route("/v1/live", get(routes::live::handler))
        .with_state(state)
        .layer(from_fn_with_state(
            rate_limit_state,
            middleware::rate_limit::enforce_rate_limit,
        ))
        .layer(build_cors_layer(cors_origins))
}

fn build_cors_layer(cors_origins: &str) -> CorsLayer {
    let base = CorsLayer::new().allow_methods([Method::GET, Method::POST]);
    let origins = cors_origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect::<Vec<_>>();

    if origins.is_empty() {
        base
    } else {
        base.allow_origin(origins)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn cors_layer_builder_accepts_empty_and_non_empty_origins() {
        let _ = build_cors_layer("");
        let _ = build_cors_layer("https://example.com, http://localhost:3000");
        let _ = build_cors_layer("not-a-valid-origin");
    }

    #[test]
    fn build_app_constructs_router_with_state_and_rate_limiter() {
        let state = AppState {
            simulation_timeout: Duration::from_secs(5),
            version: "test",
        };
        let limiter = RateLimitState::new(10, Duration::from_secs(60));
        let _app = build_app(state, limiter, "https://example.com");
    }
}
