use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn bin_path() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_poolsim-cli")
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_poolsim_cli"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest
                .parent()
                .and_then(|p| p.parent())
                .map(|workspace| workspace.join("target/debug/poolsim-cli"))
                .expect("workspace root should exist")
        })
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    path.push(format!("poolsim_cli_{name}_{}_{}.json", std::process::id(), ts));
    fs::write(&path, contents).expect("temp config should be writable");
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("CLI process should start")
}

fn sample_simulation_config() -> String {
    r#"
    {
      "workload": {
        "requests_per_second": 220.0,
        "latency_p50_ms": 8.0,
        "latency_p95_ms": 35.0,
        "latency_p99_ms": 90.0,
        "raw_samples_ms": null,
        "step_load_profile": [
          { "time_s": 0, "requests_per_second": 180.0 },
          { "time_s": 30, "requests_per_second": 260.0 }
        ]
      },
      "pool": {
        "max_server_connections": 100,
        "connection_overhead_ms": 2.0,
        "idle_timeout_ms": null,
        "min_pool_size": 3,
        "max_pool_size": 28
      },
      "options": {
        "iterations": 1200,
        "seed": 7,
        "distribution": "LogNormal",
        "queue_model": "MMC",
        "target_wait_p99_ms": 45.0,
        "max_acceptable_rho": 0.85
      }
    }
    "#
    .to_string()
}

#[test]
fn simulate_json_contains_cold_start_and_step_load_analysis() {
    let cfg = temp_file("simulate", &sample_simulation_config());
    let output = run(&["--format", "json", "simulate", "--config", cfg.to_string_lossy().as_ref()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("simulate JSON output should parse");
    assert!(payload["optimal_pool_size"].is_number());
    assert!(payload["cold_start_min_pool_size"].is_number());
    assert_eq!(
        payload["step_load_analysis"]
            .as_array()
            .expect("step_load_analysis should be an array")
            .len(),
        2
    );
}

#[test]
fn simulate_sweep_flag_outputs_sensitivity_rows() {
    let cfg = temp_file("sweep", &sample_simulation_config());
    let output = run(&[
        "--format",
        "json",
        "simulate",
        "--config",
        cfg.to_string_lossy().as_ref(),
        "--sweep",
    ]);
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 2 || code == 3,
        "unexpected exit code {code}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("sweep JSON output should parse");
    let rows = payload
        .as_array()
        .expect("simulate --sweep should output JSON array");
    assert!(!rows.is_empty());
    assert!(rows[0]["pool_size"].is_number());
}

#[test]
fn batch_and_evaluate_commands_emit_json() {
    let base = sample_simulation_config();
    let batch = format!("[{base},{base}]");
    let batch_cfg = temp_file("batch", &batch);
    let batch_out = run(&[
        "--format",
        "json",
        "batch",
        "--config",
        batch_cfg.to_string_lossy().as_ref(),
    ]);
    assert!(
        batch_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&batch_out.stderr)
    );
    let batch_payload: serde_json::Value =
        serde_json::from_slice(&batch_out.stdout).expect("batch JSON output should parse");
    assert_eq!(
        batch_payload
            .as_array()
            .expect("batch output should be array")
            .len(),
        2
    );

    let eval_cfg = temp_file("evaluate", &sample_simulation_config());
    let eval_out = run(&[
        "--format",
        "json",
        "evaluate",
        "--config",
        eval_cfg.to_string_lossy().as_ref(),
        "--pool-size",
        "8",
    ]);
    assert!(
        eval_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&eval_out.stderr)
    );
    let eval_payload: serde_json::Value =
        serde_json::from_slice(&eval_out.stdout).expect("evaluate JSON output should parse");
    assert_eq!(eval_payload["pool_size"], 8);
}
