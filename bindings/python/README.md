# poolsim Python Bindings

`poolsim` is the Python binding package for the Poolsim connection-pool sizing toolkit.

The package is intentionally a thin wrapper around the stable Rust `poolsim` CLI JSON interface. It does not reimplement the sizing engine in Python. That design keeps Python services, notebooks, CI scripts, and automation jobs aligned with the same model used by the Rust crates, CLI, REST API, WebSocket API, examples, docs fixtures, and release tests.

Use this package when you want to call Poolsim from Python without manually spawning subprocesses or parsing JSON.

## What You Can Do

- Run a full pool-size recommendation from a simulation config.
- Evaluate an existing production pool size.
- Generate sensitivity rows across candidate pool sizes.
- Run batch sizing jobs.
- Compare normal, peak, and incident traffic scenarios.
- Allocate a shared database `max_connections` budget across services.
- Import telemetry snapshots and compute recommendation diffs.
- Diagnose whether a live pool is too small, too large, or close to saturation.
- Generate framework-specific pool configuration snippets.
- Run a CI capacity gate and consume the gate report as Python data.

## Architecture

The Python package calls this command shape internally:

```bash
poolsim --format json <command> ...
```

The returned Python value is decoded from the CLI JSON output. The wrapper deliberately keeps the payloads as normal Python dictionaries and lists so new fields added by the CLI remain available without requiring a wrapper release.

This is a compatibility choice: existing methods stay stable, while the Rust CLI remains the single source of truth for simulation behavior.

## Install

Install the Python package:

```bash
pip install poolsim
```

Install the Rust CLI executable separately:

```bash
cargo install poolsim-cli
poolsim --version
```

The default client expects a `poolsim` executable on `PATH`. If the binary is installed elsewhere, pass the absolute path:

```python
from poolsim import PoolsimClient

client = PoolsimClient(executable="/opt/tools/poolsim")
```

## Runtime Requirements

- Python 3.9 or newer.
- The Rust `poolsim` executable from `poolsim-cli`.
- Config, telemetry, scenario, or policy files that match the documented Poolsim CLI schemas.

The Python package has no runtime third-party dependencies.

## Quick Start

```python
from poolsim import PoolsimClient

client = PoolsimClient()
report = client.simulate("docs/fixtures/cli-config.json")

print("recommended pool size:", report["optimal_pool_size"])
print("p99 queue wait ms:", report.get("p99_queue_wait_ms"))
print("saturation:", report.get("saturation"))
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

## Minimal Simulation Config

A typical simulation config contains workload assumptions, pool limits, and sizing options:

```json
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
    "idle_timeout_ms": 120000,
    "min_pool_size": 3,
    "max_pool_size": 24
  },
  "options": {
    "iterations": 12000,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
```

Keep these files in source control when you want sizing assumptions to be reviewable.

## API Reference

### `PoolsimClient(executable="poolsim")`

Creates a client that invokes the given executable.

Use the default when `poolsim` is on `PATH`:

```python
from poolsim import PoolsimClient

client = PoolsimClient()
```

Use an explicit path when automation installs the binary in a fixed tool directory:

```python
client = PoolsimClient(executable="/usr/local/bin/poolsim")
```

### `simulate(config)`

Runs a full sizing simulation from a config file and returns a recommendation report.

```python
report = client.simulate("docs/fixtures/cli-config.json")

optimal = report["optimal_pool_size"]
interval = report.get("confidence_interval")
rho = report.get("rho") or report.get("utilisation_rho")

print(optimal, interval, rho)
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

Use `simulate` when you are choosing the pool size to deploy.

### `evaluate(config, pool_size)`

Scores a fixed pool size against a workload config.

```python
evaluation = client.evaluate("docs/fixtures/cli-config.json", pool_size=8)

print(evaluation.get("pool_size"))
print(evaluation.get("utilisation_rho"))
print(evaluation.get("p99_queue_wait_ms"))
print(evaluation.get("saturation"))
```

Equivalent CLI command:

```bash
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 8
```

Use `evaluate` when production already has a configured pool and you need to know whether that exact setting is safe.

### `sweep(config)`

Returns sensitivity rows for candidate pool sizes.

```python
rows = client.sweep("docs/fixtures/cli-config.json")

for row in rows:
    print(row.get("pool_size"), row.get("utilisation_rho"), row.get("risk"))
```

Equivalent CLI command:

```bash
poolsim --format json sweep --config docs/fixtures/cli-config.json
```

Use `sweep` to explain tradeoffs around nearby pool sizes in code review or capacity planning.

### `batch(config)`

Runs multiple simulation requests from one batch config.

```python
reports = client.batch("docs/fixtures/batch.json")

for index, report in enumerate(reports, start=1):
    print(index, report.get("optimal_pool_size"), report.get("saturation"))
```

Equivalent CLI command:

```bash
poolsim --format json batch --config docs/fixtures/batch.json
```

Use `batch` when a platform team wants to size several services in one job.

### `compare(config)`

Compares named traffic scenarios such as normal, peak, and incident load.

```python
comparison = client.compare("docs/fixtures/scenarios.json")

print(comparison.get("baseline"))
print(comparison.get("worst_saturation"))
for row in comparison.get("rows", []):
    print(row)
```

Equivalent CLI command:

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

Use `compare` when one pool setting must survive several traffic assumptions.

### `budget(config)`

Plans allocation of a database connection budget across services and replicas.

```python
plan = client.budget("docs/fixtures/budget.json")

print(plan.get("status"))
print(plan.get("available_connections"))
print(plan.get("allocated_total_connections"))
for service in plan.get("services", []):
    print(service)
```

Equivalent CLI command:

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

Use `budget` when the database has a shared `max_connections` limit and several services compete for it.

### `telemetry_recommend(config)`

Imports a telemetry snapshot and returns a recommendation diff from the current pool setting.

```python
recommendation = client.telemetry_recommend("docs/fixtures/telemetry.json")

diff = recommendation.get("diff", {})
print(recommendation.get("service_name"))
print("current:", diff.get("current_pool_size"))
print("recommended:", diff.get("recommended_pool_size"))
print("change:", diff.get("change"))
```

Equivalent CLI command:

```bash
poolsim --format json import telemetry --config docs/fixtures/telemetry.json
```

Use `telemetry_recommend` when you have observed production traffic and want to compare the configured pool with the recommended pool.

### `doctor(config)`

Diagnoses a telemetry snapshot and classifies risk.

```python
diagnosis = client.doctor("docs/fixtures/telemetry.json")

print(diagnosis.get("status"))
for finding in diagnosis.get("findings", []):
    print(finding.get("severity"), finding.get("message"), finding.get("action"))
```

Equivalent CLI command:

```bash
poolsim --format json doctor telemetry --config docs/fixtures/telemetry.json
```

Use `doctor` when an engineer asks: is this pool too small, too large, or close to saturation?

### `generate_config(framework, config)`

Generates a framework-specific runtime configuration snippet from a simulation recommendation.

```python
snippet = client.generate_config("sqlx", "docs/fixtures/cli-config.json")

print(snippet.get("framework"))
print(snippet.get("recommended_pool_size"))
print(snippet.get("snippet"))
```

Equivalent CLI command:

```bash
poolsim --format json generate-config --framework sqlx simulate --config docs/fixtures/cli-config.json
```

Supported framework names follow the CLI:

- `hikaricp`
- `spring-boot`
- `sqlalchemy`
- `prisma`
- `node-pg`
- `sqlx`
- `deadpool`

Use `generate_config` after `simulate` when you want a copy-pasteable starting point for a real runtime pool.

### `gate(policy, telemetry_config)`

Runs a capacity gate against a telemetry snapshot and policy file.

```python
gate = client.gate("docs/fixtures/gate-policy.toml", "docs/fixtures/telemetry.json")

print(gate.get("status"))
print(gate.get("deployment_safe"))
for check in gate.get("checks", []):
    print(check)
```

Equivalent CLI command:

```bash
poolsim --format json gate --policy docs/fixtures/gate-policy.toml telemetry --config docs/fixtures/telemetry.json
```

`gate` treats CLI exit codes `0` and `2` as valid machine-readable outcomes. Exit code `2` means the capacity gate failed, not that the Python wrapper failed. Other non-zero exit codes raise `PoolsimError`.

## CI Usage

A minimal GitHub Actions step can install both packages and fail the job when your policy says the deployment is unsafe:

```yaml
- name: Install poolsim
  run: |
    cargo install poolsim-cli
    python -m pip install poolsim

- name: Run capacity gate
  run: |
    python - <<'PY'
    from poolsim import PoolsimClient

    client = PoolsimClient()
    report = client.gate("capacity-policy.toml", "telemetry.json")
    print(report)
    if not report.get("deployment_safe", False):
        raise SystemExit(1)
    PY
```

## Error Handling

The wrapper raises `PoolsimError` when:

- the CLI exits with an unexpected exit code
- the CLI writes invalid JSON
- the executable cannot be started by Python's subprocess layer

```python
from poolsim import PoolsimClient, PoolsimError

client = PoolsimClient()

try:
    report = client.simulate("missing.json")
except PoolsimError as exc:
    print(f"Poolsim failed: {exc}")
else:
    print(report)
```

The error message includes CLI stderr when the CLI provides it.

## Working With Returned Data

Return types intentionally use ordinary Python containers:

- JSON objects become dictionaries.
- JSON arrays become lists.
- JSON strings, numbers, booleans, and `null` become standard Python values.

This keeps the binding forward-compatible when the CLI adds new output fields. Prefer `dict.get(...)` for optional fields when writing automation that should tolerate additional CLI versions.

## Troubleshooting

### `FileNotFoundError` or executable startup failure

The Rust CLI is not installed or is not on `PATH`.

```bash
cargo install poolsim-cli
poolsim --version
```

Or pass an explicit executable path:

```python
client = PoolsimClient(executable="/absolute/path/to/poolsim")
```

### `PoolsimError: poolsim did not emit valid JSON`

The wrapper always passes `--format json`. This error usually means the executable path points to a different program, an older incompatible binary, or a command failed before producing JSON.

Check the CLI directly:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

### Gate returns a failed policy report

That is expected behavior when capacity assumptions are unsafe. Inspect `status`, `deployment_safe`, and `checks` in the returned object. The wrapper will still return JSON for CLI exit code `2`.

## Compatibility Notes

- The Python wrapper does not change CLI semantics.
- The Rust CLI remains the source of truth for formulas, validation, and output fields.
- Existing method names are intended to remain stable.
- New CLI output fields can appear in returned dictionaries without a Python package change.

## Support

- Documentation: <https://github.com/gregorian-09/poolsim/tree/main/docs>
- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
