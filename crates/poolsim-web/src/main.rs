//! Service binary entrypoint for the `poolsim-web` HTTP/WebSocket API.

use std::{env, net::SocketAddr, time::Duration};

use poolsim_web::{build_app, middleware::rate_limit::RateLimitState, state::AppState};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_tracing();
    configure_rayon();

    let host = env::var("POOLSIM_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = parse_port(env::var("POOLSIM_PORT").ok());
    let timeout_secs = parse_timeout_secs(env::var("POOLSIM_SIMULATION_TIMEOUT_SECS").ok());
    let rpm = parse_rate_limit_rpm(
        env::var("POOLSIM_RATE_LIMIT").ok(),
        env::var("POOLSIM_RATE_LIMIT_RPM").ok(),
    );

    let state = AppState {
        simulation_timeout: Duration::from_secs(timeout_secs),
        version: env!("CARGO_PKG_VERSION"),
    };

    let rate_limit_state = RateLimitState::new(rpm, Duration::from_secs(60));

    let cors_origins = env::var("POOLSIM_CORS_ORIGINS").unwrap_or_default();
    let app = build_app(state, rate_limit_state, &cors_origins).layer(TraceLayer::new_for_http());
    let shutdown_ms = parse_shutdown_ms(env::var("POOLSIM_TEST_SHUTDOWN_MS").ok());

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("failed to parse bind address");

    tracing::info!(%addr, "starting poolsim-web");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    let shutdown = wait_for_shutdown(shutdown_ms);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
    .expect("server failed");
}

fn init_tracing() {
    let (log_format, log_level) = log_settings(
        env::var("POOLSIM_LOG_FORMAT").ok(),
        env::var("POOLSIM_LOG_LEVEL").ok(),
    );

    let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    if log_format.eq_ignore_ascii_case("pretty") {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .compact()
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .with_current_span(true)
            .with_target(false)
            .try_init();
    }
}

fn configure_rayon() {
    if let Some(threads) = parse_rayon_threads(env::var("POOLSIM_RAYON_THREADS").ok()) {
        let _ = rayon::ThreadPoolBuilder::new().num_threads(threads).build_global();
        return;
    }

    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build_global();
}

fn parse_port(raw: Option<String>) -> u16 {
    raw.and_then(|v| v.parse::<u16>().ok()).unwrap_or(8080)
}

fn parse_timeout_secs(raw: Option<String>) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(30)
}

fn parse_rate_limit_rpm(primary: Option<String>, fallback: Option<String>) -> u64 {
    primary
        .or(fallback)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60)
}

fn parse_rayon_threads(raw: Option<String>) -> Option<usize> {
    raw.and_then(|v| v.parse::<usize>().ok())
}

fn log_settings(format: Option<String>, level: Option<String>) -> (String, String) {
    (
        format.unwrap_or_else(|| "json".to_string()),
        level.unwrap_or_else(|| "info".to_string()),
    )
}

fn parse_shutdown_ms(raw: Option<String>) -> Option<u64> {
    raw.and_then(|v| v.parse::<u64>().ok())
}

async fn wait_for_shutdown(shutdown_ms: Option<u64>) {
    if let Some(ms) = shutdown_ms {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    } else {
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
mod tests {
    use std::{panic::catch_unwind, sync::{Mutex, OnceLock}};

    use super::*;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn parse_port_and_timeout_use_defaults_on_invalid_values() {
        assert_eq!(parse_port(None), 8080);
        assert_eq!(parse_port(Some("not-a-port".to_string())), 8080);
        assert_eq!(parse_port(Some("9000".to_string())), 9000);

        assert_eq!(parse_timeout_secs(None), 30);
        assert_eq!(parse_timeout_secs(Some("x".to_string())), 30);
        assert_eq!(parse_timeout_secs(Some("12".to_string())), 12);
    }

    #[test]
    fn parse_rate_limit_honors_primary_then_fallback_then_default() {
        assert_eq!(parse_rate_limit_rpm(Some("99".to_string()), Some("12".to_string())), 99);
        assert_eq!(parse_rate_limit_rpm(None, Some("77".to_string())), 77);
        assert_eq!(parse_rate_limit_rpm(Some("bad".to_string()), Some("65".to_string())), 60);
        assert_eq!(parse_rate_limit_rpm(None, None), 60);
    }

    #[test]
    fn parse_rayon_threads_and_log_settings_cover_branches() {
        assert_eq!(parse_rayon_threads(None), None);
        assert_eq!(parse_rayon_threads(Some("x".to_string())), None);
        assert_eq!(parse_rayon_threads(Some("4".to_string())), Some(4));

        assert_eq!(
            log_settings(None, None),
            ("json".to_string(), "info".to_string())
        );
        assert_eq!(
            log_settings(Some("pretty".to_string()), Some("debug".to_string())),
            ("pretty".to_string(), "debug".to_string())
        );

        assert_eq!(parse_shutdown_ms(None), None);
        assert_eq!(parse_shutdown_ms(Some("5".to_string())), Some(5));
        assert_eq!(parse_shutdown_ms(Some("x".to_string())), None);
    }

    #[test]
    fn startup_helpers_are_callable_with_env_overrides() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        std::env::set_var("POOLSIM_LOG_FORMAT", "pretty");
        std::env::set_var("POOLSIM_LOG_LEVEL", "not-a-level");
        let _ = catch_unwind(init_tracing);

        std::env::set_var("POOLSIM_LOG_FORMAT", "json");
        std::env::set_var("POOLSIM_LOG_LEVEL", "info");
        let _ = catch_unwind(init_tracing);

        std::env::set_var("POOLSIM_RAYON_THREADS", "2");
        configure_rayon();
        std::env::set_var("POOLSIM_RAYON_THREADS", "invalid");
        configure_rayon();
        std::env::remove_var("POOLSIM_RAYON_THREADS");
    }

    #[test]
    fn main_panics_on_invalid_bind_address() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        std::env::set_var("POOLSIM_HOST", "[invalid-host");
        std::env::set_var("POOLSIM_PORT", "8080");
        std::env::set_var("POOLSIM_SIMULATION_TIMEOUT_SECS", "1");
        std::env::set_var("POOLSIM_RATE_LIMIT", "60");
        std::env::set_var("POOLSIM_CORS_ORIGINS", "");

        let panicked = catch_unwind(main).is_err();
        assert!(panicked, "invalid bind address should panic");
    }

    #[test]
    fn main_panics_when_port_is_already_in_use() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("prebind should succeed");
        let port = listener.local_addr().expect("prebind addr should exist").port();

        std::env::set_var("POOLSIM_HOST", "127.0.0.1");
        std::env::set_var("POOLSIM_PORT", port.to_string());
        std::env::set_var("POOLSIM_SIMULATION_TIMEOUT_SECS", "1");
        std::env::set_var("POOLSIM_RATE_LIMIT", "60");
        std::env::set_var("POOLSIM_CORS_ORIGINS", "");

        let panicked = catch_unwind(main).is_err();
        assert!(panicked, "occupied port should panic on bind");
        drop(listener);
    }

    #[test]
    fn main_can_boot_and_shutdown_in_test_mode() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        std::env::set_var("POOLSIM_HOST", "127.0.0.1");
        std::env::set_var("POOLSIM_PORT", "0");
        std::env::set_var("POOLSIM_SIMULATION_TIMEOUT_SECS", "1");
        std::env::set_var("POOLSIM_RATE_LIMIT", "60");
        std::env::set_var("POOLSIM_CORS_ORIGINS", "");
        std::env::set_var("POOLSIM_TEST_SHUTDOWN_MS", "5");

        let panicked = catch_unwind(main).is_err();
        assert!(!panicked, "test-mode shutdown should let main return cleanly");

        std::env::remove_var("POOLSIM_TEST_SHUTDOWN_MS");
    }

    #[test]
    fn wait_for_shutdown_covers_pending_and_sleep_paths() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime should build");

        let timed_out = runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(5), wait_for_shutdown(None)).await
        });
        assert!(
            timed_out.is_err(),
            "without override, shutdown future should remain pending"
        );

        let completed = runtime.block_on(async {
            tokio::time::timeout(Duration::from_millis(25), wait_for_shutdown(Some(0))).await
        });
        assert!(
            completed.is_ok(),
            "with explicit override, shutdown future should complete"
        );
    }
}
