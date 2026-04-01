use std::{
    net::SocketAddr,
    sync::OnceLock,
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use poolsim_web::{build_app, middleware::rate_limit::RateLimitState, state::AppState};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;

fn app_with_rate_limit(rpm: u64) -> Router {
    app_with_rate_limit_and_timeout(rpm, Duration::from_secs(10))
}

fn app_with_rate_limit_and_timeout(rpm: u64, simulation_timeout: Duration) -> Router {
    let state = AppState {
        simulation_timeout,
        version: "test-version",
    };
    let rate_limit_state = RateLimitState::new(rpm, Duration::from_secs(60));
    build_app(state, rate_limit_state, "")
}

fn sample_request(iterations: u32) -> Value {
    json!({
        "workload": {
            "requests_per_second": 300.0,
            "latency_p50_ms": 10.0,
            "latency_p95_ms": 50.0,
            "latency_p99_ms": 120.0,
            "raw_samples_ms": null,
            "step_load_profile": [
                { "time_s": 0, "requests_per_second": 240.0 },
                { "time_s": 30, "requests_per_second": 380.0 }
            ]
        },
        "pool": {
            "max_server_connections": 100,
            "connection_overhead_ms": 2.0,
            "idle_timeout_ms": null,
            "min_pool_size": 4,
            "max_pool_size": 24
        },
        "options": {
            "iterations": iterations,
            "seed": 7,
            "distribution": "LogNormal",
            "queue_model": "MMC",
            "target_wait_p99_ms": 50.0,
            "max_acceptable_rho": 0.85
        }
    })
}

async fn json_request(app: Router, method: &str, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method(method)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("request should build");

    let resp = app.oneshot(req).await.expect("route should respond");
    let status = resp.status();
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let json = serde_json::from_slice(&body).expect("response should be valid JSON");
    (status, json)
}

async fn start_ws_server(app: Router) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let addr = listener.local_addr().expect("listener should expose local addr");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("test server should run");
    });

    (format!("ws://{addr}/v1/live"), shutdown_tx, server)
}

fn ws_test_semaphore() -> &'static tokio::sync::Semaphore {
    static SEM: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    SEM.get_or_init(|| tokio::sync::Semaphore::new(1))
}

#[tokio::test]
async fn rest_routes_work_and_return_structured_errors() {
    let app = app_with_rate_limit(100);

    let health_req = Request::builder()
        .uri("/v1/health")
        .body(Body::empty())
        .expect("health request should build");
    let health_resp = app
        .clone()
        .oneshot(health_req)
        .await
        .expect("health route should respond");
    assert_eq!(health_resp.status(), StatusCode::OK);
    let health_body = to_bytes(health_resp.into_body(), usize::MAX)
        .await
        .expect("health body should be readable");
    let health_json: Value =
        serde_json::from_slice(&health_body).expect("health response should be valid JSON");
    assert_eq!(health_json["status"], "ok");
    assert_eq!(health_json["version"], "test-version");

    let models_req = Request::builder()
        .uri("/v1/models")
        .body(Body::empty())
        .expect("models request should build");
    let models_resp = app
        .clone()
        .oneshot(models_req)
        .await
        .expect("models route should respond");
    assert_eq!(models_resp.status(), StatusCode::OK);
    let models_body = to_bytes(models_resp.into_body(), usize::MAX)
        .await
        .expect("models body should be readable");
    let models_json: Value =
        serde_json::from_slice(&models_body).expect("models response should be valid JSON");
    assert!(models_json["distribution_models"].is_array());
    assert!(models_json["queue_models"].is_array());

    let (simulate_status, simulate_json) =
        json_request(app.clone(), "POST", "/v1/simulate", sample_request(1_200)).await;
    assert_eq!(simulate_status, StatusCode::OK);
    assert!(simulate_json["optimal_pool_size"].is_number());
    assert!(simulate_json["cold_start_min_pool_size"].is_number());
    assert_eq!(
        simulate_json["step_load_analysis"]
            .as_array()
            .expect("step_load_analysis should be an array")
            .len(),
        2
    );

    let eval_payload = json!({
        "workload": sample_request(1200)["workload"],
        "pool_size": 8,
        "options": sample_request(1200)["options"],
    });
    let (evaluate_status, evaluate_json) =
        json_request(app.clone(), "POST", "/v1/evaluate", eval_payload).await;
    assert_eq!(evaluate_status, StatusCode::OK);
    assert_eq!(evaluate_json["pool_size"], 8);

    let sensitivity_payload = json!({
        "workload": sample_request(1200)["workload"],
        "pool": sample_request(1200)["pool"],
        "options": sample_request(1200)["options"],
    });
    let (sensitivity_status, sensitivity_json) =
        json_request(app.clone(), "POST", "/v1/sensitivity", sensitivity_payload).await;
    assert_eq!(sensitivity_status, StatusCode::OK);
    assert!(sensitivity_json.is_array());
    assert!(!sensitivity_json.as_array().expect("sensitivity array").is_empty());

    let batch_payload = json!([sample_request(1200), sample_request(1300)]);
    let (batch_status, batch_json) = json_request(app.clone(), "POST", "/v1/batch", batch_payload).await;
    assert_eq!(batch_status, StatusCode::OK);
    assert_eq!(batch_json.as_array().expect("batch array").len(), 2);

    let invalid = json!({
        "workload": {
            "requests_per_second": 300.0,
            "latency_p50_ms": 100.0,
            "latency_p95_ms": 50.0,
            "latency_p99_ms": 120.0
        },
        "pool": sample_request(1200)["pool"],
        "options": sample_request(1200)["options"]
    });
    let (invalid_status, invalid_json) =
        json_request(app.clone(), "POST", "/v1/simulate", invalid).await;
    assert_eq!(invalid_status, StatusCode::BAD_REQUEST);
    assert!(invalid_json["error"].is_string());
    assert!(invalid_json["code"].is_string());
    assert!(invalid_json.get("details").is_some());
}

#[tokio::test]
async fn rest_routes_return_408_when_simulation_deadline_is_zero() {
    let app = app_with_rate_limit_and_timeout(100, Duration::from_millis(0));

    let (simulate_status, simulate_json) =
        json_request(app.clone(), "POST", "/v1/simulate", sample_request(1_200)).await;
    assert_eq!(simulate_status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(simulate_json["code"], "SIMULATION_TIMEOUT");

    let eval_payload = json!({
        "workload": sample_request(1200)["workload"],
        "pool_size": 8,
        "options": sample_request(1200)["options"],
    });
    let (evaluate_status, evaluate_json) =
        json_request(app.clone(), "POST", "/v1/evaluate", eval_payload).await;
    assert_eq!(evaluate_status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(evaluate_json["code"], "SIMULATION_TIMEOUT");

    let sensitivity_payload = json!({
        "workload": sample_request(1200)["workload"],
        "pool": sample_request(1200)["pool"],
        "options": sample_request(1200)["options"],
    });
    let (sensitivity_status, sensitivity_json) =
        json_request(app.clone(), "POST", "/v1/sensitivity", sensitivity_payload).await;
    assert_eq!(sensitivity_status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(sensitivity_json["code"], "SIMULATION_TIMEOUT");

    let batch_payload = json!([sample_request(1200), sample_request(1300)]);
    let (batch_status, batch_json) = json_request(app.clone(), "POST", "/v1/batch", batch_payload).await;
    assert_eq!(batch_status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(batch_json["code"], "SIMULATION_TIMEOUT");
}

#[tokio::test]
async fn rate_limit_returns_429_with_retry_after() {
    let app = app_with_rate_limit(1);

    let first = Request::builder()
        .uri("/v1/health")
        .header("x-forwarded-for", "203.0.113.20")
        .body(Body::empty())
        .expect("first request should build");
    let first_resp = app
        .clone()
        .oneshot(first)
        .await
        .expect("first request should complete");
    assert_eq!(first_resp.status(), StatusCode::OK);

    let second = Request::builder()
        .uri("/v1/health")
        .header("x-forwarded-for", "203.0.113.20")
        .body(Body::empty())
        .expect("second request should build");
    let second_resp = app
        .clone()
        .oneshot(second)
        .await
        .expect("second request should complete");
    assert_eq!(second_resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(second_resp.headers().get("retry-after").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_batch_stream_emits_ndjson_ticks_and_batch_done() {
    let _permit = ws_test_semaphore()
        .acquire()
        .await
        .expect("ws test semaphore should acquire");
    let app = app_with_rate_limit(100);
    let (url, shutdown_tx, server) = start_ws_server(app).await;
    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");

    let payload = json!([sample_request(2200), sample_request(2200)]).to_string();
    socket
        .send(Message::Text(payload.into()))
        .await
        .expect("batch payload should be sent");

    let mut saw_tick = false;
    let mut done_indices = Vec::new();
    let mut saw_batch_done = false;

    for _ in 0..80 {
        let maybe_msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .expect("websocket read should not time out");

        let Some(result) = maybe_msg else {
            break;
        };
        let msg = result.expect("websocket frame should be valid");
        let text = match msg {
            Message::Text(text) => text.to_string(),
            _ => continue,
        };

        assert!(text.ends_with('\n'), "websocket frame should be newline terminated NDJSON");
        let frame: Value =
            serde_json::from_str(text.trim_end()).expect("frame should be valid JSON object");
        if frame.get("tick").is_some() {
            saw_tick = true;
        }

        if frame.get("done").and_then(Value::as_bool) == Some(true) {
            if let Some(index) = frame.get("batch_index").and_then(Value::as_u64) {
                done_indices.push(index);
            }
        }

        if frame.get("batch_done").and_then(Value::as_bool) == Some(true) {
            saw_batch_done = true;
            assert_eq!(frame["batch_total"], 2);
            let reports = frame["reports"]
                .as_array()
                .expect("batch_done frame should include reports array");
            assert_eq!(reports.len(), 2);
            break;
        }
    }

    let _ = shutdown_tx.send(());
    let _ = server.await;

    done_indices.sort_unstable();
    assert!(saw_tick, "expected at least one tick frame");
    assert_eq!(done_indices, vec![0, 1], "expected done frames for both batch items");
    assert!(saw_batch_done, "expected final batch_done frame");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_accepts_binary_json_initial_frame() {
    let _permit = ws_test_semaphore()
        .acquire()
        .await
        .expect("ws test semaphore should acquire");
    let app = app_with_rate_limit(100);
    let (url, shutdown_tx, server) = start_ws_server(app).await;
    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");

    let payload = sample_request(1800).to_string().into_bytes();
    socket
        .send(Message::Binary(payload.into()))
        .await
        .expect("binary json payload should be sent");

    let mut saw_done = false;
    for _ in 0..60 {
        let maybe_msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .expect("websocket read should not time out");
        let Some(result) = maybe_msg else {
            break;
        };
        let msg = result.expect("websocket frame should be valid");
        let text = match msg {
            Message::Text(text) => text.to_string(),
            _ => continue,
        };
        let frame: Value =
            serde_json::from_str(text.trim_end()).expect("frame should be valid JSON");
        if frame.get("done").and_then(Value::as_bool) == Some(true) {
            saw_done = true;
            assert!(
                frame.get("batch_index").is_none(),
                "single-item run should not include batch_index"
            );
            break;
        }
    }

    let _ = shutdown_tx.send(());
    let _ = server.await;
    assert!(saw_done, "expected done frame for binary-JSON request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_reports_invalid_utf8_and_invalid_json_and_invalid_frame() {
    let _permit = ws_test_semaphore()
        .acquire()
        .await
        .expect("ws test semaphore should acquire");
    let app = app_with_rate_limit(100);
    let (url, shutdown_tx, server) = start_ws_server(app).await;

    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");
    socket
        .send(Message::Binary(vec![0xff, 0xfe, 0xfd].into()))
        .await
        .expect("invalid utf8 payload should be sent");
    let msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
        .await
        .expect("utf8 error should be returned")
        .expect("server should respond")
        .expect("frame should be valid");
    let text = match msg {
        Message::Text(text) => text.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    };
    let frame: Value = serde_json::from_str(text.trim_end()).expect("error frame should be JSON");
    assert_eq!(frame["code"], "INVALID_UTF8");
    drop(socket);

    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("second websocket connection should succeed");
    socket
        .send(Message::Ping(vec![1, 2].into()))
        .await
        .expect("ping frame should be sent");
    let msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
        .await
        .expect("invalid frame error should be returned")
        .expect("server should respond")
        .expect("frame should be valid");
    let text = match msg {
        Message::Text(text) => text.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    };
    let frame: Value = serde_json::from_str(text.trim_end()).expect("error frame should be JSON");
    assert_eq!(frame["code"], "INVALID_FRAME");
    drop(socket);

    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("third websocket connection should succeed");
    socket
        .send(Message::Text("{ this is invalid json".into()))
        .await
        .expect("invalid json text should be sent");
    let msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
        .await
        .expect("invalid json error should be returned")
        .expect("server should respond")
        .expect("frame should be valid");
    let text = match msg {
        Message::Text(text) => text.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    };
    let frame: Value = serde_json::from_str(text.trim_end()).expect("error frame should be JSON");
    assert_eq!(frame["code"], "INVALID_JSON");

    let _ = shutdown_tx.send(());
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_batch_emits_error_frame_for_invalid_item() {
    let _permit = ws_test_semaphore()
        .acquire()
        .await
        .expect("ws test semaphore should acquire");
    let app = app_with_rate_limit(100);
    let (url, shutdown_tx, server) = start_ws_server(app).await;
    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");

    let mut invalid = sample_request(1500);
    invalid["workload"]["latency_p50_ms"] = json!(100.0);
    invalid["workload"]["latency_p95_ms"] = json!(50.0);

    let payload = json!([sample_request(1500), invalid]).to_string();
    socket
        .send(Message::Text(payload.into()))
        .await
        .expect("batch payload should be sent");

    let mut saw_error = false;
    let mut saw_batch_done = false;
    for _ in 0..80 {
        let maybe_msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .expect("websocket read should not time out");
        let Some(result) = maybe_msg else {
            break;
        };
        let msg = result.expect("websocket frame should be valid");
        let text = match msg {
            Message::Text(text) => text.to_string(),
            _ => continue,
        };
        let frame: Value =
            serde_json::from_str(text.trim_end()).expect("frame should be valid JSON");

        if frame.get("code").is_some() {
            saw_error = true;
            assert!(frame.get("batch_index").is_some());
        }
        if frame.get("batch_done").and_then(Value::as_bool) == Some(true) {
            saw_batch_done = true;
            break;
        }
    }

    let _ = shutdown_tx.send(());
    let _ = server.await;
    assert!(saw_error, "expected per-item error frame for invalid batch item");
    assert!(saw_batch_done, "expected final batch_done frame");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_can_recover_when_client_disconnects_before_first_frame() {
    let _permit = ws_test_semaphore()
        .acquire()
        .await
        .expect("ws test semaphore should acquire");
    let app = app_with_rate_limit(100);
    let (url, shutdown_tx, server) = start_ws_server(app).await;

    let (socket, _response) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");
    drop(socket);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("second websocket connection should succeed");
    socket
        .send(Message::Text(sample_request(1500).to_string().into()))
        .await
        .expect("payload should be sent");

    let msg = tokio::time::timeout(Duration::from_secs(10), socket.next())
        .await
        .expect("websocket read should not time out")
        .expect("server should respond")
        .expect("frame should be valid");
    let text = match msg {
        Message::Text(text) => text.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    };
    let frame: Value = serde_json::from_str(text.trim_end()).expect("frame should be valid JSON");
    assert!(frame.get("tick").is_some() || frame.get("done").and_then(Value::as_bool) == Some(true));

    let _ = shutdown_tx.send(());
    let _ = server.await;
}
