//! Live WebSocket streaming endpoint.
//!
//! Route:
//!
//! - `GET /v1/live`
//!
//! Behavior:
//!
//! - accepts a single simulation request or an array of requests
//! - emits newline-delimited JSON progress ticks
//! - emits final `done` or `batch_done` frames
//! - emits structured error frames for invalid requests and runtime failures

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use poolsim_core::{simulate, types::SimulationReport, MIN_FULL_SIMULATION_ITERATIONS};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::{routes::simulate::SimulationRequest, state::AppState};

/// Handles `GET /v1/live` and upgrades to a streaming WebSocket session.
pub async fn handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    let request_message = match receiver.next().await {
        Some(Ok(Message::Text(text))) => text.to_string(),
        Some(Ok(Message::Binary(bytes))) => match String::from_utf8(bytes.to_vec()) {
            Ok(text) => text,
            Err(_) => {
                let _ = sender
                    .send(Message::Text(
                        ws_error("INVALID_UTF8", "initial websocket frame must be valid UTF-8", None).into(),
                    ))
                    .await;
                return;
            }
        },
        Some(Ok(_)) => {
            let _ = sender
                .send(Message::Text(
                    ws_error(
                        "INVALID_FRAME",
                        "initial websocket frame must be text or binary JSON",
                        None,
                    )
                    .into(),
                ))
                .await;
            return;
        }
        Some(Err(_)) | None => return,
    };

    let requests = match parse_requests(&request_message) {
        Ok(reqs) => reqs,
        Err(error) => {
            let _ = sender
                .send(Message::Text(
                    ws_error("INVALID_JSON", &format!("invalid request JSON: {error}"), None).into(),
                ))
                .await;
            return;
        }
    };

    let (tx, mut rx) = mpsc::channel::<String>(256);
    let send_task = tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            let mut payload = line;
            if !payload.ends_with('\n') {
                payload.push('\n');
            }
            if sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    let total_requests = requests.len();
    let mut reports = Vec::with_capacity(total_requests);

    for (index, req) in requests.into_iter().enumerate() {
        let batch_index = if total_requests > 1 {
            Some(index as u32)
        } else {
            None
        };

        match simulate_with_ticks(req, &state, &tx, batch_index, total_requests as u32).await {
            Ok(report) => {
                let mut done = json!({
                    "done": true,
                    "report": report,
                });
                if let Some(i) = batch_index {
                    done["batch_index"] = json!(i);
                    done["batch_total"] = json!(total_requests);
                }
                let _ = tx.try_send(done.to_string());
                reports.push(report);
            }
            Err(error) => {
                let _ = tx.try_send(ws_error(error.code, &error.message, batch_index));
            }
        }
    }

    if total_requests > 1 {
        let _ = tx.try_send(
            json!({
                "batch_done": true,
                "batch_total": total_requests,
                "reports": reports,
            })
            .to_string(),
        );
    }

    drop(tx);
    let _ = send_task.await;
}

async fn simulate_with_ticks(
    req: SimulationRequest,
    state: &AppState,
    tx: &mpsc::Sender<String>,
    batch_index: Option<u32>,
    batch_total: u32,
) -> Result<SimulationReport, WsRunError> {
    let total_iterations = req
        .options
        .iterations
        .max(MIN_FULL_SIMULATION_ITERATIONS)
        .max(1);
    let tx_ticks = tx.clone();
    let timeout = state.simulation_timeout;
    if timeout.is_zero() {
        return Err(WsRunError {
            code: "SIMULATION_TIMEOUT",
            message: "simulation timed out".to_string(),
        });
    }

    let blocking = tokio::task::spawn_blocking(move || {
        #[cfg(test)]
        if req.options.seed == Some(u64::MAX) {
            panic!("forced websocket worker panic for join-error coverage");
        }

        let step = 1_000u32;
        let mut iteration = step;

        while iteration < total_iterations {
            let mut options = req.options.clone();
            options.iterations = iteration;

            if let Ok(report) = simulate(&req.workload, &req.pool, &options) {
                let mut tick = json!({
                    "tick": iteration,
                    "of": total_iterations,
                    "p99_so_far_ms": report.p99_queue_wait_ms,
                    "optimal_so_far": report.optimal_pool_size,
                });
                if let Some(i) = batch_index {
                    tick["batch_index"] = json!(i);
                    tick["batch_total"] = json!(batch_total);
                }
                let _ = tx_ticks.try_send(tick.to_string());
            }

            iteration = iteration.saturating_add(step);
        }

        simulate(&req.workload, &req.pool, &req.options)
    });

    match tokio::time::timeout(timeout, blocking).await {
        Ok(join_result) => match join_result {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(error)) => Err(WsRunError {
                code: error.code(),
                message: error.to_string(),
            }),
            Err(error) => Err(WsRunError {
                code: "INTERNAL_JOIN_ERROR",
                message: error.to_string(),
            }),
        },
        Err(_) => Err(WsRunError {
            code: "SIMULATION_TIMEOUT",
            message: "simulation timed out".to_string(),
        }),
    }
}

fn parse_requests(input: &str) -> Result<Vec<SimulationRequest>, serde_json::Error> {
    let value: Value = serde_json::from_str(input)?;
    if value.is_array() {
        serde_json::from_value(value)
    } else {
        let request: SimulationRequest = serde_json::from_value(value)?;
        Ok(vec![request])
    }
}

fn ws_error(code: &str, message: &str, batch_index: Option<u32>) -> String {
    let mut obj = json!({
        "error": message,
        "code": code,
        "details": Value::Null,
    });
    if let Some(i) = batch_index {
        obj["batch_index"] = json!(i);
    }
    format!("{obj}\n")
}

#[derive(Debug)]
struct WsRunError {
    code: &'static str,
    message: String,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use poolsim_core::types::{PoolConfig, SimulationOptions, WorkloadConfig};

    use super::*;

    fn sample_request(iterations: u32) -> SimulationRequest {
        SimulationRequest {
            workload: WorkloadConfig {
                requests_per_second: 180.0,
                latency_p50_ms: 6.0,
                latency_p95_ms: 25.0,
                latency_p99_ms: 60.0,
                raw_samples_ms: None,
                step_load_profile: None,
            },
            pool: PoolConfig {
                max_server_connections: 100,
                connection_overhead_ms: 2.0,
                idle_timeout_ms: None,
                min_pool_size: 2,
                max_pool_size: 20,
            },
            options: SimulationOptions {
                iterations,
                ..SimulationOptions::default()
            },
        }
    }

    #[test]
    fn parse_requests_supports_single_and_array_payloads() {
        let single = serde_json::json!({
            "workload": {
                "requests_per_second": 180.0,
                "latency_p50_ms": 6.0,
                "latency_p95_ms": 25.0,
                "latency_p99_ms": 60.0
            },
            "pool": {
                "max_server_connections": 100,
                "connection_overhead_ms": 2.0,
                "min_pool_size": 2,
                "max_pool_size": 20
            },
            "options": {
                "iterations": 1500
            }
        })
        .to_string();
        let parsed = parse_requests(&single).expect("single request should parse");
        assert_eq!(parsed.len(), 1);

        let array = format!("[{single},{single}]");
        let parsed = parse_requests(&array).expect("request array should parse");
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn ws_error_always_emits_newline_and_optional_batch_index() {
        let plain = ws_error("X", "oops", None);
        assert!(plain.ends_with('\n'));
        let value: serde_json::Value =
            serde_json::from_str(plain.trim_end()).expect("error payload should be valid JSON");
        assert_eq!(value["code"], "X");
        assert!(value.get("batch_index").is_none());

        let with_idx = ws_error("Y", "oops2", Some(4));
        assert!(with_idx.ends_with('\n'));
        let value: serde_json::Value =
            serde_json::from_str(with_idx.trim_end()).expect("error payload should be valid JSON");
        assert_eq!(value["batch_index"], 4);
    }

    #[tokio::test]
    async fn simulate_with_ticks_returns_core_error_for_invalid_request() {
        let mut bad = sample_request(1_200);
        bad.workload.latency_p50_ms = 100.0;
        bad.workload.latency_p95_ms = 50.0;
        bad.workload.latency_p99_ms = 120.0;

        let state = AppState {
            simulation_timeout: Duration::from_secs(2),
            version: "test",
        };
        let (tx, _rx) = mpsc::channel(16);

        let err = simulate_with_ticks(bad, &state, &tx, Some(0), 1)
            .await
            .expect_err("invalid request should fail");
        assert_eq!(err.code, "INVALID_LATENCY_ORDER");
    }

    #[tokio::test]
    async fn simulate_with_ticks_times_out_when_deadline_is_zero() {
        let req = sample_request(1_200);
        let state = AppState {
            simulation_timeout: Duration::from_millis(0),
            version: "test",
        };
        let (tx, _rx) = mpsc::channel(16);

        let err = simulate_with_ticks(req, &state, &tx, None, 1)
            .await
            .expect_err("zero timeout should fail quickly");
        assert_eq!(err.code, "SIMULATION_TIMEOUT");
    }

    #[tokio::test]
    async fn simulate_with_ticks_returns_report_and_emits_progress() {
        let req = sample_request(1_200);
        let state = AppState {
            simulation_timeout: Duration::from_secs(3),
            version: "test",
        };
        let (tx, mut rx) = mpsc::channel(64);

        let report = simulate_with_ticks(req, &state, &tx, Some(1), 3)
            .await
            .expect("simulation should succeed");
        assert!(report.optimal_pool_size >= 1);

        let mut saw_tick = false;
        while let Ok(line) = rx.try_recv() {
            let frame: serde_json::Value =
                serde_json::from_str(&line).expect("tick should be valid JSON");
            if frame.get("tick").is_some() {
                saw_tick = true;
                assert_eq!(frame["batch_index"], 1);
                assert_eq!(frame["batch_total"], 3);
                break;
            }
        }
        assert!(saw_tick, "expected at least one tick frame");
    }

    #[tokio::test]
    async fn simulate_with_ticks_maps_join_error_from_panicing_worker() {
        let mut req = sample_request(1_200);
        req.options.seed = Some(u64::MAX);
        let state = AppState {
            simulation_timeout: Duration::from_secs(2),
            version: "test",
        };
        let (tx, _rx) = mpsc::channel(16);

        let err = simulate_with_ticks(req, &state, &tx, None, 1)
            .await
            .expect_err("forced panic should map to join error");
        assert_eq!(err.code, "INTERNAL_JOIN_ERROR");
    }
}
