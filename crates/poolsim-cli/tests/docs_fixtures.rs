use std::{
    path::PathBuf,
    process::{Command, Output},
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

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

#[cfg(unix)]
#[test]
#[should_panic(expected = "intentional failure")]
fn assert_success_failure_message_is_covered() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1 << 8),
        stdout: b"stdout body".to_vec(),
        stderr: b"stderr body".to_vec(),
    };

    assert_success(&output, "intentional failure");
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
    assert!(csv
        .lines()
        .next()
        .expect("CSV should have header")
        .contains("pool_size"));
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
    assert_eq!(
        batch_json
            .as_array()
            .expect("batch output should be an array")
            .len(),
        2
    );

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
fn docs_compare_examples_work_for_json_csv_and_toml_inputs() {
    let json_output = run_cli(&[
        "--format",
        "json",
        "compare",
        "--config",
        &fixture("docs/fixtures/scenarios.json"),
    ]);
    assert_success(&json_output, "compare JSON docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&json_output))
        .expect("compare JSON output should deserialize");
    assert_eq!(report["baseline"], "normal");
    assert_eq!(
        report["rows"]
            .as_array()
            .expect("rows should be an array")
            .len(),
        3
    );
    assert!(report["rows"][1]["pool_size_delta"].is_number());

    let csv_output = run_cli(&[
        "--format",
        "csv",
        "compare",
        "--config",
        &fixture("docs/fixtures/scenarios.json"),
        "--baseline",
        "peak",
    ]);
    assert_success(&csv_output, "compare CSV docs example");
    let csv = stdout_utf8(&csv_output);
    assert!(csv.contains("scenario,baseline"));
    assert!(csv.contains("incident"));

    let toml_output = run_cli(&[
        "--format",
        "table",
        "compare",
        "--config",
        &fixture("docs/fixtures/scenarios.toml"),
    ]);
    assert_success(&toml_output, "compare TOML docs example");
    let table = stdout_utf8(&toml_output);
    assert!(table.contains("scenario_count"));
    assert!(table.contains("worst_saturation"));
}

#[test]
fn docs_budget_examples_work_for_json_csv_and_toml_inputs() {
    let json_output = run_cli(&[
        "--format",
        "json",
        "budget",
        "--config",
        &fixture("docs/fixtures/budget.json"),
    ]);
    assert_success(&json_output, "budget JSON docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&json_output))
        .expect("budget JSON output should deserialize");
    assert_eq!(report["status"], "Warning");
    assert!(
        report["services"]
            .as_array()
            .expect("services should be an array")
            .len()
            >= 3
    );

    let csv_output = run_cli(&[
        "--format",
        "csv",
        "budget",
        "--config",
        &fixture("docs/fixtures/budget.json"),
    ]);
    assert_success(&csv_output, "budget CSV docs example");
    let csv = stdout_utf8(&csv_output);
    assert!(csv.contains("allocated_total_connections"));
    assert!(csv.contains("checkout-api"));

    let toml_output = run_cli(&[
        "--format",
        "table",
        "budget",
        "--config",
        &fixture("docs/fixtures/budget.toml"),
    ]);
    assert_success(&toml_output, "budget TOML docs example");
    let table = stdout_utf8(&toml_output);
    assert!(table.contains("available_connections"));
    assert!(table.contains("billing-api"));
}

#[test]
fn docs_import_telemetry_example_works() {
    let json_output = run_cli(&[
        "--format",
        "json",
        "import",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_success(&json_output, "telemetry import docs example");
    let recommendation: Value = serde_json::from_str(&stdout_utf8(&json_output))
        .expect("telemetry recommendation output should deserialize");
    assert_eq!(recommendation["service_name"], "checkout-api");
    assert!(recommendation["diff"]["current_pool_size"].is_number());
    assert!(recommendation["diff"]["recommended_pool_size"].is_number());

    let csv_output = run_cli(&[
        "--format",
        "csv",
        "import",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
        "--current-pool-size",
        "10",
    ]);
    assert_success(&csv_output, "telemetry import CSV docs example");
    let csv = stdout_utf8(&csv_output);
    assert!(csv.contains("recommended_pool_size"));
    assert!(csv.contains("pool_size_delta"));
}

#[test]
fn docs_import_prometheus_example_works() {
    let output = run_cli(&[
        "--format",
        "json",
        "import",
        "prometheus",
        "--response-file",
        &fixture("docs/fixtures/prometheus-responses.json"),
        "--service-name",
        "checkout-api",
        "--window",
        "5m",
        "--current-pool-size",
        "8",
        "--max-server-connections",
        "100",
        "--connection-overhead-ms",
        "2",
        "--min",
        "2",
        "--max",
        "20",
    ]);
    assert_success(&output, "prometheus import docs example");
    let recommendation: Value = serde_json::from_str(&stdout_utf8(&output))
        .expect("prometheus recommendation output should deserialize");
    assert_eq!(recommendation["service_name"], "checkout-api");
    assert_eq!(recommendation["window"], "5m");
    assert!(recommendation["diff"]["recommended_pool_size"].is_number());
}

#[test]
fn docs_gate_examples_work() {
    let telemetry_output = run_cli(&[
        "--format",
        "json",
        "gate",
        "--policy",
        &fixture("docs/fixtures/gate-policy.toml"),
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_success(&telemetry_output, "gate telemetry docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&telemetry_output))
        .expect("gate output should deserialize");
    assert_eq!(report["status"], "Pass");
    assert!(report["checks"].is_array());
    assert!(report["recommendation"]["diff"]["recommended_pool_size"].is_number());

    let prometheus_output = run_cli(&[
        "--format",
        "json",
        "gate",
        "--policy",
        &fixture("docs/fixtures/gate-policy.toml"),
        "prometheus",
        "--response-file",
        &fixture("docs/fixtures/prometheus-responses.json"),
        "--service-name",
        "checkout-api",
        "--window",
        "5m",
        "--current-pool-size",
        "8",
        "--max-server-connections",
        "100",
        "--connection-overhead-ms",
        "2",
        "--min",
        "2",
        "--max",
        "20",
    ]);
    assert_success(&prometheus_output, "gate prometheus docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&prometheus_output))
        .expect("gate prometheus output should deserialize");
    assert_eq!(report["service_name"], "checkout-api");
    assert_eq!(report["window"], "5m");

    let failing_output = run_cli(&[
        "--format",
        "json",
        "gate",
        "--expected-pool-size",
        "999",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_eq!(failing_output.status.code(), Some(2));
    let report: Value = serde_json::from_str(&stdout_utf8(&failing_output))
        .expect("failing gate output should deserialize");
    assert_eq!(report["status"], "Critical");
}

#[test]
fn docs_guard_examples_work() {
    let telemetry_output = run_cli(&[
        "--format",
        "json",
        "guard",
        "--policy",
        &fixture("docs/fixtures/gate-policy.toml"),
        "--max-current-rho",
        "0.95",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_success(&telemetry_output, "guard telemetry docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&telemetry_output))
        .expect("guard output should deserialize");
    assert_eq!(report["status"], "Pass");
    assert_eq!(report["deployment_safe"], true);
    assert_eq!(report["exit_code"], 0);
    assert!(report["gate"]["checks"].is_array());

    let prometheus_output = run_cli(&[
        "--format",
        "json",
        "guard",
        "--max-current-p99-queue-wait-ms",
        "100",
        "--max-current-mean-queue-wait-ms",
        "20",
        "--max-current-rho",
        "0.95",
        "prometheus",
        "--response-file",
        &fixture("docs/fixtures/prometheus-responses.json"),
        "--service-name",
        "checkout-api",
        "--window",
        "5m",
        "--current-pool-size",
        "8",
        "--max-server-connections",
        "100",
        "--connection-overhead-ms",
        "2",
        "--min",
        "2",
        "--max",
        "20",
    ]);
    assert_success(&prometheus_output, "guard prometheus docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&prometheus_output))
        .expect("guard prometheus output should deserialize");
    assert_eq!(report["gate"]["service_name"], "checkout-api");
    assert_eq!(report["gate"]["window"], "5m");

    let failing_output = run_cli(&[
        "--format",
        "json",
        "guard",
        "--max-current-rho",
        "0.01",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_eq!(failing_output.status.code(), Some(2));
    let report: Value = serde_json::from_str(&stdout_utf8(&failing_output))
        .expect("failing guard output should deserialize");
    assert_eq!(report["status"], "Critical");
    assert_eq!(report["deployment_safe"], false);
}

#[test]
fn docs_doctor_examples_work() {
    let telemetry_output = run_cli(&[
        "--format",
        "json",
        "doctor",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_success(&telemetry_output, "doctor telemetry docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&telemetry_output))
        .expect("doctor output should deserialize");
    assert!(report["status"].is_string());
    assert!(report["findings"].is_array());
    assert!(report["recommendation"]["diff"]["recommended_pool_size"].is_number());

    let prometheus_output = run_cli(&[
        "--format",
        "json",
        "doctor",
        "prometheus",
        "--response-file",
        &fixture("docs/fixtures/prometheus-responses.json"),
        "--service-name",
        "checkout-api",
        "--window",
        "5m",
        "--current-pool-size",
        "8",
        "--max-server-connections",
        "100",
        "--connection-overhead-ms",
        "2",
        "--min",
        "2",
        "--max",
        "20",
    ]);
    assert_success(&prometheus_output, "doctor prometheus docs example");
    let report: Value = serde_json::from_str(&stdout_utf8(&prometheus_output))
        .expect("doctor prometheus output should deserialize");
    assert_eq!(report["service_name"], "checkout-api");
    assert_eq!(report["window"], "5m");
    assert!(report["current_pool_size"].is_number());
}

#[test]
fn docs_generate_config_examples_work() {
    let telemetry_output = run_cli(&[
        "--format",
        "json",
        "generate-config",
        "--framework",
        "sqlx",
        "--pool-name",
        "checkout-pool",
        "telemetry",
        "--config",
        &fixture("docs/fixtures/telemetry.json"),
    ]);
    assert_success(&telemetry_output, "generate-config telemetry docs example");
    let telemetry_report: Value = serde_json::from_str(&stdout_utf8(&telemetry_output))
        .expect("generate-config telemetry output should deserialize");
    assert_eq!(telemetry_report["framework"], "sqlx");
    assert_eq!(telemetry_report["source"], "telemetry");
    assert!(telemetry_report["recommended_pool_size"].is_number());
    assert!(telemetry_report["snippet"]
        .as_str()
        .unwrap_or_default()
        .contains(".max_connections("));

    let prometheus_output = run_cli(&[
        "--format",
        "json",
        "generate-config",
        "--framework",
        "spring-boot",
        "prometheus",
        "--response-file",
        &fixture("docs/fixtures/prometheus-responses.json"),
        "--service-name",
        "checkout-api",
        "--window",
        "5m",
        "--current-pool-size",
        "8",
        "--max-server-connections",
        "100",
        "--connection-overhead-ms",
        "2",
        "--min",
        "2",
        "--max",
        "20",
    ]);
    assert_success(
        &prometheus_output,
        "generate-config prometheus docs example",
    );
    let prometheus_report: Value = serde_json::from_str(&stdout_utf8(&prometheus_output))
        .expect("generate-config prometheus output should deserialize");
    assert_eq!(prometheus_report["framework"], "spring-boot");
    assert_eq!(prometheus_report["source"], "prometheus");
    assert!(prometheus_report["snippet"]
        .as_str()
        .unwrap_or_default()
        .contains("maximum-pool-size"));

    let simulate_output = run_cli(&[
        "--format",
        "csv",
        "generate-config",
        "--framework",
        "node-pg",
        "simulate",
        "--config",
        &fixture("docs/fixtures/cli-config.json"),
    ]);
    assert_success(&simulate_output, "generate-config simulate docs example");
    let csv = stdout_utf8(&simulate_output);
    assert!(csv.contains("framework,node-pg"));
    assert!(csv.contains("new Pool"));
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

#[test]
fn docs_json_schemas_are_valid_json_and_match_fixture_shapes() {
    let schema_paths = [
        "docs/schemas/poolsim-config.schema.json",
        "docs/schemas/batch.schema.json",
        "docs/schemas/scenarios.schema.json",
        "docs/schemas/budget.schema.json",
        "docs/schemas/telemetry.schema.json",
        "docs/schemas/gate-policy.schema.json",
    ];

    for path in schema_paths {
        let schema_text = std::fs::read_to_string(workspace_root().join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        let schema: Value = serde_json::from_str(&schema_text)
            .unwrap_or_else(|err| panic!("{path} should be valid JSON: {err}"));
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(schema["title"].is_string(), "{path} should have a title");
    }

    let simulation: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join("docs/fixtures/cli-config.json"))
            .expect("simulation fixture should be readable"),
    )
    .expect("simulation fixture should parse");
    assert!(simulation["workload"].is_object());
    assert!(simulation["pool"].is_object());

    let batch: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join("docs/fixtures/batch.json"))
            .expect("batch fixture should be readable"),
    )
    .expect("batch fixture should parse");
    assert!(batch.as_array().is_some_and(|items| !items.is_empty()));

    let scenarios: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join("docs/fixtures/scenarios.json"))
            .expect("scenario fixture should be readable"),
    )
    .expect("scenario fixture should parse");
    assert!(scenarios["scenarios"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let budget: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join("docs/fixtures/budget.json"))
            .expect("budget fixture should be readable"),
    )
    .expect("budget fixture should parse");
    assert!(budget["services"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let telemetry: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join("docs/fixtures/telemetry.json"))
            .expect("telemetry fixture should be readable"),
    )
    .expect("telemetry fixture should parse");
    assert!(telemetry["telemetry"].is_object());

    let gate_policy_text =
        std::fs::read_to_string(workspace_root().join("docs/fixtures/gate-policy.toml"))
            .expect("gate policy fixture should be readable");
    let gate_policy: toml::Value = gate_policy_text
        .parse()
        .expect("gate policy fixture should parse as TOML");
    assert!(gate_policy.get("max_saturation").is_some());
}
