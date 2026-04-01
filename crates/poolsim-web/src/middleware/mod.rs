//! Middleware namespace for `poolsim-web`.
//!
//! The current release exposes only rate limiting, but the module exists as the
//! stable place for cross-cutting HTTP concerns around the router.

/// Per-IP request rate limiting middleware.
pub mod rate_limit;
