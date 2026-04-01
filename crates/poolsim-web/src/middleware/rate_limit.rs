//! In-memory per-IP rate limiting.
//!
//! This module provides:
//!
//! - [`crate::middleware::rate_limit::RateLimitState`] for shared limiter state
//! - [`crate::middleware::rate_limit::enforce_rate_limit`] for Axum middleware integration
//!
//! The implementation is intentionally simple and process-local. It is suitable
//! for single-instance deployments and local tooling; distributed rate limiting
//! is outside the scope of this crate.

use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, State},
    http::{header, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// In-memory per-IP rate-limiter state.
#[derive(Debug, Clone)]
pub struct RateLimitState {
    rpm: usize,
    window: Duration,
    buckets: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
}

impl RateLimitState {
    /// Creates a new rate limiter with requests-per-window constraints.
    pub fn new(rpm: u64, window: Duration) -> Self {
        Self {
            rpm: rpm.max(1) as usize,
            window,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_and_update(&self, ip: IpAddr, now: Instant) -> Option<u64> {
        let mut map = self
            .buckets
            .lock()
            .expect("rate-limit mutex poisoned unexpectedly");

        let entry = map.entry(ip).or_default();

        while let Some(front) = entry.front().copied() {
            if now.duration_since(front) >= self.window {
                entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() >= self.rpm {
            let retry_after = entry
                .front()
                .copied()
                .map(|front| {
                    self.window
                        .saturating_sub(now.saturating_duration_since(front))
                        .as_secs()
                        .max(1)
                })
                .unwrap_or(1);
            return Some(retry_after);
        }

        entry.push_back(now);
        None
    }
}

/// Axum middleware that enforces per-IP request rate limiting.
pub async fn enforce_rate_limit(
    State(state): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let ip = client_ip(&request);

    if let Some(ip) = ip {
        if let Some(retry_after) = state.check_and_update(ip, Instant::now()) {
            let body = Json(json!({
                "error": "rate limit exceeded",
                "code": "RATE_LIMITED",
                "details": {
                    "ip": ip,
                    "retry_after_secs": retry_after,
                }
            }));

            let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
            if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
            return response;
        }
    }

    next.run(request).await
}

fn client_ip(request: &Request<Body>) -> Option<IpAddr> {
    if let Some(raw) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = raw.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_blocks_after_rpm_with_retry_after() {
        let state = RateLimitState::new(2, Duration::from_secs(60));
        let ip: IpAddr = "203.0.113.9".parse().expect("valid test ip");
        let now = Instant::now();

        assert_eq!(state.check_and_update(ip, now), None);
        assert_eq!(state.check_and_update(ip, now + Duration::from_secs(1)), None);

        let retry_after = state
            .check_and_update(ip, now + Duration::from_secs(2))
            .expect("third request should be rate-limited");
        assert!(retry_after >= 1);
    }

    #[test]
    fn expired_entries_are_pruned_before_new_request() {
        let state = RateLimitState::new(1, Duration::from_secs(1));
        let ip: IpAddr = "203.0.113.10".parse().expect("valid test ip");
        let now = Instant::now();

        assert_eq!(state.check_and_update(ip, now), None);
        assert_eq!(
            state.check_and_update(ip, now + Duration::from_secs(2)),
            None,
            "expired entry should be popped and replaced"
        );
    }
}
