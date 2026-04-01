use std::{
    path::PathBuf,
    process::Command,
};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve")
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_poolsim-cli")
}

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(bin_path())
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("CLI process should start")
}

fn fixture(path: &str) -> String {
    workspace_root().join(path).display().to_string()
}

fn assert_success(output: &std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn stdout_utf8(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid UTF-8")
}

#[test]
fn docs_simulate_examples_work_for_json_table_and_samples_file_paths() {
    let json_output = run_cli(&[
        "--format",
        "json",
        "simulate",
        "--config",
        &fixture("docs/fixtures/cli-config.json"),
    ]);
    assert_success(&json_output, "JSON simulate docs example");
    let json: Value = serde_json::from_str(&stdout_utf8(&json_output))
        .expect("JSON simulate output should deserialize");
    assert!(json["optimal_pool_size"].is_number());
    assert!(json["sensitivity"].is_array());

    let table_output = run_cli(&[
        "--format",
        "table",
        "simulate",
        "--config",
        &fixture("docs/fixtures/cli-config.toml"),
    ]);
    assert_success(&table_output, "table simulate docs example");
    let table_stdout = stdout_utf8(&table_output);
    assert!(table_stdout.contains("optimal_pool_size"));
    assert!(table_stdout.contains("confidence_interval"));

    let samples_output = run_cli(&[
        "--format",
        "json",
        "simulate",
        "--config",
        &fixture("docs/fixtures/cli-config.json"),
        "--samples-file",
        &fixture("docs/fixtures/latencies.txt"),
    ]);
    assert_success(&samples_output, "samples-file simulate docs example");
    let samples_json: Value = serde_json::from_str(&stdout_utf8(&samples_output))
        .expect("samples-file simulate output should deserialize");
    assert!(samples_json["optimal_pool_size"].is_number());
}

#[test]
fn docs_evaluate_and_sweep_examples_work() {
    let evaluate_output = run_cli(&[
        "--format",
        "json",
        "evaluate",
        "--config",
        &fixture("docs/fixtures/cli-config.toml"),
        "--pool-size",
        "10",
    ]);
    assert_success(&evaluate_output, "evaluate docs example");
    let evaluation: Value = serde_json::from_str(&stdout_utf8(&evaluate_output))
        .expect("evaluate output should deserialize");
    assert_eq!(evaluation["pool_size"], 10);

    let sweep_output = run_cli(&[
        "--format",
        "csv",
        "sweep",
        "--config",
        &fixture("docs/fixtures/cli-config.json"),
    ]);
    let sweep_code = sweep_output
        .status
        .code()
        .expect("sweep example should exit with an integer status code");
    assert!(
        sweep_code == 0 || sweep_code == 2 || sweep_code == 3,
        "sweep docs example should produce a documented exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&sweep_output.stdout),
        String::from_utf8_lossy(&sweep_output.stderr),
    );
    let csv = stdout_utf8(&sweep_output);
    assert!(csv.lines().next().expect("CSV should have header").contains("pool_size"));
    assert!(csv.contains("p99_queue_wait_ms"));
}

#[test]
fn docs_batch_examples_work_for_json_and_toml_inputs() {
    let batch_json_output = run_cli(&[
        "--format",
        "json",
        "batch",
        "--config",
        &fixture("docs/fixtures/batch.json"),
    ]);
    assert_success(&batch_json_output, "batch JSON docs example");
    let batch_json: Value = serde_json::from_str(&stdout_utf8(&batch_json_output))
        .expect("batch JSON output should deserialize");
    assert_eq!(batch_json.as_array().expect("batch output should be an array").len(), 2);

    let batch_toml_output = run_cli(&[
        "--format",
        "table",
        "batch",
        "--config",
        &fixture("docs/fixtures/batch.toml"),
    ]);
    assert_success(&batch_toml_output, "batch TOML docs example");
    let table = stdout_utf8(&batch_toml_output);
    assert!(table.contains("request_index"));
    assert!(table.contains("optimal_pool_size"));
}

#[test]
fn docs_warn_exit_example_is_stable() {
    let output = run_cli(&[
        "--warn-exit",
        "evaluate",
        "--config",
        &fixture("docs/fixtures/cli-config.json"),
        "--pool-size",
        "1",
    ]);
    let code = output
        .status
        .code()
        .expect("CLI should exit with an integer status code");
    assert!(
        code == 2 || code == 3,
        "warn-exit example should produce a non-zero advisory or critical exit\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
