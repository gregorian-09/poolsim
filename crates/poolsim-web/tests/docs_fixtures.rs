use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    time::Duration,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use poolsim_web::{build_app, middleware::rate_limit::RateLimitState, state::AppState};
use serde_json::Value;
use tokio::{net::TcpListener, sync::oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tower::ServiceExt;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve")
}

fn fixture_text(path: &str) -> String {
    fs::read_to_string(workspace_root().join(path)).expect("fixture should be readable")
}

fn fixture_json(path: &str) -> Value {
    serde_json::from_str(&fixture_text(path)).expect("fixture JSON should deserialize")
}

fn docs_app() -> Router {
    let state = AppState {
        simulation_timeout: Duration::from_secs(10),
        version: "docs-test",
    };
    let rate_limit_state = RateLimitState::new(1_000, Duration::from_secs(60));
    build_app(state, rate_limit_state, "")
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

#[tokio::test]
async fn docs_rest_fixtures_round_trip() {
    let app = docs_app();

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
    assert_eq!(health_json["version"], "docs-test");

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

    let (simulate_status, simulate_json) = json_request(
        app.clone(),
        "POST",
        "/v1/simulate",
        fixture_json("docs/fixtures/web-simulate.json"),
    )
    .await;
    assert_eq!(simulate_status, StatusCode::OK);
    assert!(simulate_json["optimal_pool_size"].is_number());
    assert!(simulate_json["step_load_analysis"].is_array());

    let (evaluate_status, evaluate_json) = json_request(
        app.clone(),
        "POST",
        "/v1/evaluate",
        fixture_json("docs/fixtures/web-evaluate.json"),
    )
    .await;
    assert_eq!(evaluate_status, StatusCode::OK);
    assert_eq!(evaluate_json["pool_size"], 10);

    let (sensitivity_status, sensitivity_json) = json_request(
        app.clone(),
        "POST",
        "/v1/sensitivity",
        fixture_json("docs/fixtures/web-sensitivity.json"),
    )
    .await;
    assert_eq!(sensitivity_status, StatusCode::OK);
    assert!(sensitivity_json.is_array());
    assert!(!sensitivity_json.as_array().expect("sensitivity output").is_empty());

    let (batch_status, batch_json) = json_request(
        app.clone(),
        "POST",
        "/v1/batch",
        fixture_json("docs/fixtures/batch.json"),
    )
    .await;
    assert_eq!(batch_status, StatusCode::OK);
    assert_eq!(batch_json.as_array().expect("batch output should be an array").len(), 2);
}

#[tokio::test]
async fn docs_websocket_fixture_round_trip() {
    let app = docs_app();
    let (url, shutdown_tx, server) = start_ws_server(app).await;

    let (mut socket, _) = connect_async(&url)
        .await
        .expect("websocket connection should succeed");
    socket
        .send(Message::Text(
            fixture_text("docs/fixtures/web-ws-request.json").into(),
        ))
        .await
        .expect("docs websocket fixture should send");

    let mut saw_tick = false;
    let mut saw_done = false;

    while !saw_done {
        let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("websocket read should not time out")
            .expect("websocket should continue streaming")
            .expect("websocket frame should be valid");

        let text = match frame {
            Message::Text(text) => text.to_string(),
            other => panic!("unexpected websocket frame: {other:?}"),
        };
        assert!(text.ends_with('\n'), "websocket frame should be newline-delimited");

        let payload: Value = serde_json::from_str(text.trim_end())
            .expect("websocket frame payload should be valid JSON");
        saw_tick |= payload.get("tick").is_some();
        saw_done |= payload.get("done") == Some(&Value::Bool(true));
    }

    assert!(saw_tick, "docs websocket example should emit at least one progress tick");

    let _ = shutdown_tx.send(());
    server.await.expect("server task should shut down cleanly");
}
