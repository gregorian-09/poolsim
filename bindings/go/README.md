# poolsim Go Bindings

`github.com/gregorian-09/poolsim/bindings/go` is the Go binding module for the Poolsim connection-pool sizing toolkit.

The package intentionally delegates all sizing work to the Rust `poolsim` CLI JSON contract. It does not reimplement queueing formulas in Go. That keeps Go services, platform tooling, and CI checks aligned with the same model used by the Rust crates, CLI, REST API, WebSocket API, docs fixtures, and release tests.

Use this module when Go code wants to call Poolsim without manually constructing `exec.Command` calls or decoding JSON for each workflow.

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

The current Go wrapper does not expose a dedicated `Gate` method. Use the CLI directly for CI gate mode from Go projects until that wrapper is added in a future compatible release.

## Architecture

The Go package calls this command shape internally:

```bash
poolsim --format json <command> ...
```

The returned value is decoded into `map[string]any` or `[]map[string]any`. The wrapper deliberately keeps payloads as generic JSON-like Go values so new fields added by the CLI remain available without requiring a wrapper release.

This is a compatibility choice: existing methods stay stable, while the Rust CLI remains the single source of truth for simulation behavior.

## Install

Add the Go module:

```bash
go get github.com/gregorian-09/poolsim/bindings/go/poolsim
```

Install the Rust CLI executable separately:

```bash
cargo install poolsim-cli
poolsim --version
```

The Go client expects a `poolsim` executable on `PATH` when the executable field is empty or set to `"poolsim"`. If the binary is installed somewhere else, pass the absolute path:

```go
client := poolsim.NewClient("/opt/tools/poolsim")
```

## Runtime Requirements

- Go 1.22 or newer.
- The Rust `poolsim` executable from `poolsim-cli`.
- Config, telemetry, scenario, or policy files that match the documented Poolsim CLI schemas.

The Go module has no third-party runtime dependencies.

## Quick Start

```go
package main

import (
    "fmt"
    "log"

    poolsim "github.com/gregorian-09/poolsim/bindings/go/poolsim"
)

func main() {
    client := poolsim.NewClient("poolsim")

    report, err := client.Simulate("docs/fixtures/cli-config.json")
    if err != nil {
        log.Fatal(err)
    }

    fmt.Println("recommended pool size:", report["optimal_pool_size"])
    fmt.Println("p99 queue wait ms:", report["p99_queue_wait_ms"])
    fmt.Println("saturation:", report["saturation"])
}
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

### `NewClient(executable string) Client`

Creates a client that invokes the given executable.

Use `"poolsim"` when the binary is on `PATH`:

```go
client := poolsim.NewClient("poolsim")
```

Use an explicit path when automation installs the binary in a fixed tool directory:

```go
client := poolsim.NewClient("/usr/local/bin/poolsim")
```

The zero-value `Client{}` also falls back to `"poolsim"`.

### `Client.Simulate(config string) (map[string]any, error)`

Runs a full sizing simulation from a config file and returns a recommendation report.

```go
report, err := client.Simulate("docs/fixtures/cli-config.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(report["optimal_pool_size"])
fmt.Println(report["confidence_interval"])
fmt.Println(report["p99_queue_wait_ms"])
fmt.Println(report["saturation"])
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

Use `Simulate` when you are choosing the pool size to deploy.

### `Client.Evaluate(config string, poolSize int) (map[string]any, error)`

Scores a fixed pool size against a workload config.

```go
evaluation, err := client.Evaluate("docs/fixtures/cli-config.json", 8)
if err != nil {
    log.Fatal(err)
}

fmt.Println(evaluation["pool_size"])
fmt.Println(evaluation["utilisation_rho"])
fmt.Println(evaluation["p99_queue_wait_ms"])
fmt.Println(evaluation["saturation"])
```

Equivalent CLI command:

```bash
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 8
```

Use `Evaluate` when production already has a configured pool and you need to know whether that exact setting is safe.

### `Client.Sweep(config string) ([]map[string]any, error)`

Returns sensitivity rows for candidate pool sizes.

```go
rows, err := client.Sweep("docs/fixtures/cli-config.json")
if err != nil {
    log.Fatal(err)
}

for _, row := range rows {
    fmt.Println(row["pool_size"], row["utilisation_rho"], row["risk"])
}
```

Equivalent CLI command:

```bash
poolsim --format json sweep --config docs/fixtures/cli-config.json
```

Use `Sweep` to explain tradeoffs around nearby pool sizes in code review or capacity planning.

### `Client.Batch(config string) ([]map[string]any, error)`

Runs multiple simulation requests from one batch config.

```go
reports, err := client.Batch("docs/fixtures/batch.json")
if err != nil {
    log.Fatal(err)
}

for _, report := range reports {
    fmt.Println(report["optimal_pool_size"], report["saturation"])
}
```

Equivalent CLI command:

```bash
poolsim --format json batch --config docs/fixtures/batch.json
```

Use `Batch` when a platform team wants to size several services in one job.

### `Client.Compare(config string) (map[string]any, error)`

Compares named traffic scenarios such as normal, peak, and incident load.

```go
comparison, err := client.Compare("docs/fixtures/scenarios.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(comparison["baseline"])
fmt.Println(comparison["worst_saturation"])
fmt.Println(comparison["rows"])
```

Equivalent CLI command:

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

Use `Compare` when one pool setting must survive several traffic assumptions.

### `Client.Budget(config string) (map[string]any, error)`

Plans allocation of a database connection budget across services and replicas.

```go
plan, err := client.Budget("docs/fixtures/budget.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(plan["status"])
fmt.Println(plan["available_connections"])
fmt.Println(plan["allocated_total_connections"])
fmt.Println(plan["services"])
```

Equivalent CLI command:

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

Use `Budget` when the database has a shared `max_connections` limit and several services compete for it.

### `Client.TelemetryRecommend(config string) (map[string]any, error)`

Imports a telemetry snapshot and returns a recommendation diff from the current pool setting.

```go
recommendation, err := client.TelemetryRecommend("docs/fixtures/telemetry.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(recommendation["service_name"])
fmt.Println(recommendation["diff"])
```

Equivalent CLI command:

```bash
poolsim --format json import telemetry --config docs/fixtures/telemetry.json
```

Use `TelemetryRecommend` when you have observed production traffic and want to compare the configured pool with the recommended pool.

### `Client.Doctor(config string) (map[string]any, error)`

Diagnoses a telemetry snapshot and classifies risk.

```go
diagnosis, err := client.Doctor("docs/fixtures/telemetry.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(diagnosis["status"])
fmt.Println(diagnosis["findings"])
```

Equivalent CLI command:

```bash
poolsim --format json doctor telemetry --config docs/fixtures/telemetry.json
```

Use `Doctor` when an engineer asks: is this pool too small, too large, or close to saturation?

### `Client.GenerateConfig(framework string, config string) (map[string]any, error)`

Generates a framework-specific runtime configuration snippet from a simulation recommendation.

```go
snippet, err := client.GenerateConfig("sqlx", "docs/fixtures/cli-config.json")
if err != nil {
    log.Fatal(err)
}

fmt.Println(snippet["framework"])
fmt.Println(snippet["recommended_pool_size"])
fmt.Println(snippet["snippet"])
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

Use `GenerateConfig` after `Simulate` when you want a copy-pasteable starting point for a real runtime pool.

## CI Gate From Go Projects

The current Go wrapper does not expose a `Gate` helper. Use the CLI directly in CI:

```bash
poolsim --format json gate --policy capacity-policy.toml telemetry --config telemetry.json
```

If you need to call the gate from Go code today, use `os/exec` directly and allow CLI exit code `2` as a valid capacity-gate failure report. Exit code `2` means the policy failed; it is not the same as a crashed command.

## Error Handling

Every method returns `(result, error)`.

Errors can come from:

- starting the `poolsim` executable
- the CLI exiting unsuccessfully
- JSON decoding failing

```go
report, err := client.Simulate("missing.json")
if err != nil {
    log.Printf("Poolsim failed: %v", err)
    return
}
fmt.Println(report)
```

For CLI validation failures, Go's `exec.Cmd.Output` returns an `*exec.ExitError`. If you need stderr details for a custom workflow, call the CLI directly with `CombinedOutput` or `Output` plus stderr wiring.

## Working With Returned Data

Return types intentionally use ordinary Go JSON containers:

- JSON objects become `map[string]any`.
- JSON arrays become `[]map[string]any` for array-returning methods.
- JSON numbers decode as `float64` when accessed through `any`.
- JSON strings, booleans, and null decode as standard Go values.

Example numeric extraction:

```go
if value, ok := report["optimal_pool_size"].(float64); ok {
    fmt.Println("recommended:", int(value))
}
```

This generic shape keeps the binding forward-compatible when the CLI adds new output fields. For production code, define local structs around the fields your application actually consumes if you want stricter typing.

## Troubleshooting

### `exec: "poolsim": executable file not found in $PATH`

The Rust CLI is not installed or is not on `PATH`.

```bash
cargo install poolsim-cli
poolsim --version
```

Or pass an explicit executable path:

```go
client := poolsim.NewClient("/absolute/path/to/poolsim")
```

### JSON decoding fails

The wrapper always passes `--format json`. A decoding error usually means the executable path points to a different program, an older incompatible binary, or a command failed before producing JSON.

Check the CLI directly:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

### Need capacity gate behavior

Use the CLI directly for now:

```bash
poolsim --format json gate --policy docs/fixtures/gate-policy.toml telemetry --config docs/fixtures/telemetry.json
```

## Compatibility Notes

- The Go wrapper does not change CLI semantics.
- The Rust CLI remains the source of truth for formulas, validation, and output fields.
- Existing method names are intended to remain stable.
- New CLI output fields can appear in returned maps without a Go module change.

## Support

- Documentation: <https://github.com/gregorian-09/poolsim/tree/main/docs>
- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
