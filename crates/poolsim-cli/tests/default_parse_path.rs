use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn sample_config_json() -> &'static str {
    r#"
{
  "workload": {
    "requests_per_second": 220.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0
  },
  "pool": {
    "max_server_connections": 120,
    "connection_overhead_ms": 2.0,
    "min_pool_size": 3,
    "max_pool_size": 24
  },
  "options": {
    "iterations": 100,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
"#
}

#[test]
fn binary_exec_uses_default_process_args_parse_path() {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after UNIX epoch")
        .as_nanos();
    let cfg_path = std::env::temp_dir().join(format!(
        "poolsim_cli_default_parse_{}_{}.json",
        std::process::id(),
        ts
    ));

    fs::write(&cfg_path, sample_config_json()).expect("config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_poolsim-cli"))
        .arg("--format")
        .arg("json")
        .arg("simulate")
        .arg("--config")
        .arg(&cfg_path)
        .env_remove("POOLSIM_TEST_ARGS_JSON")
        .output()
        .expect("binary should run");

    fs::remove_file(&cfg_path).expect("temp config should be removable");

    assert!(
        output.status.success(),
        "cli should exit successfully, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("optimal_pool_size"),
        "simulation JSON should include optimal_pool_size, stdout: {stdout}"
    );
}
