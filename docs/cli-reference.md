# Poolsim CLI Reference

## Purpose

This is the exhaustive user guide for the `poolsim` command-line interface.

It covers:

- Every subcommand
- Every global flag
- Every subcommand-specific flag
- JSON and TOML config formats
- Batch input formats
- Sample-file input
- Output formats
- Exit-code behavior

The CLI binary is `poolsim`.

Checked-in runnable fixture files live under `docs/fixtures/`:

- `docs/fixtures/cli-config.json`
- `docs/fixtures/cli-config.toml`
- `docs/fixtures/batch.json`
- `docs/fixtures/batch.toml`
- `docs/fixtures/latencies.txt`

## Command Summary

Available subcommands:

- `simulate`
- `evaluate`
- `sweep`
- `batch`

Global flags:

- `--format <table|json|csv>`
- `--warn-exit`

## Global Options

### `--format`

Controls output format.

Supported values:

- `table`
- `json`
- `csv`

Examples:

```bash
poolsim --format table simulate --config docs/fixtures/cli-config.toml
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 12
poolsim --format csv sweep --config docs/fixtures/cli-config.json
```

### `--warn-exit`

When enabled, warning-level outcomes can return exit code `3` instead of `0`.

Example:

```bash
poolsim --warn-exit simulate --config docs/fixtures/cli-config.json
```

## `simulate`

### Purpose

Runs the full recommendation workflow.

This is the highest-level CLI command and usually the first one users should try.

### Forms

Config-driven:

```bash
poolsim simulate --config docs/fixtures/cli-config.json
```

Flags-only:

```bash
poolsim simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24
```

Config plus overrides:

```bash
poolsim simulate \
  --config docs/fixtures/cli-config.toml \
  --rps 260 \
  --iterations 20000 \
  --distribution gamma \
  --queue-model mdc
```

### `simulate`-specific flags

#### `--pool-size`

Evaluates a single pool size from within the `simulate` command path.

Example:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --pool-size 10 --format json
```

#### `--sweep`

Generates the full sensitivity surface from the `simulate` command path.

Example:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --sweep --format csv
```

Conflict rule:

- `--pool-size` and `--sweep` cannot be used together.

## `evaluate`

### Purpose

Scores one fixed pool size against the workload.

### Example

```bash
poolsim evaluate --config docs/fixtures/cli-config.json --pool-size 12
```

With explicit flags:

```bash
poolsim evaluate \
  --pool-size 12 \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --iterations 10000 \
  --distribution log-normal \
  --queue-model mmc
```

## `sweep`

### Purpose

Returns all candidate pool sizes in the configured range together with queue-wait and risk metrics.

### Example

```bash
poolsim sweep --config docs/fixtures/cli-config.json
```

Tighter range override:

```bash
poolsim sweep --config docs/fixtures/cli-config.json --min 4 --max 18 --format json
```

## `batch`

### Purpose

Runs multiple simulation requests from a single batch file.

### Example

```bash
poolsim batch --config docs/fixtures/batch.json --format json
```

## Common Input Flags

These flags are shared by `simulate`, `evaluate`, and `sweep` through the common CLI argument surface.

### `--config <path>`

Loads a JSON or TOML config file.

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json
poolsim simulate --config docs/fixtures/cli-config.toml
```

### `--rps`

Override `workload.requests_per_second`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --rps 275
```

### `--p50`, `--p95`, `--p99`

Override percentile latencies.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --p50 9 --p95 35 --p99 90
```

### `--samples-file`

Load empirical latency samples from a file. The checked-in sample file is `docs/fixtures/latencies.txt`.

The parser accepts values separated by:

- commas
- spaces
- tabs
- newlines

Example:

```bash
poolsim simulate \
  --rps 180 \
  --p50 6 \
  --p95 25 \
  --p99 60 \
  --samples-file latencies.txt \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20
```

Sample file content:

```text
5.5
6.0
6.8
7.4
8.1
9.9
12.0
18.2
```

Or comma-separated:

```text
5.5,6.0,6.8,7.4,8.1,9.9,12.0,18.2
```

### `--max-server-connections`

Override `pool.max_server_connections`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --max-server-connections 150
```

### `--connection-overhead-ms`

Override `pool.connection_overhead_ms`.

The CLI also supports the alias:

- `--connection-establishment-overhead-ms`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --connection-overhead-ms 2.5
poolsim simulate --config docs/fixtures/cli-config.json --connection-establishment-overhead-ms 2.5
```

### `--idle-timeout-ms`

Override `pool.idle_timeout_ms`.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --idle-timeout-ms 120000
```

### `--min`, `--max`

Override `pool.min_pool_size` and `pool.max_pool_size`.

```bash
poolsim sweep --config docs/fixtures/cli-config.json --min 4 --max 24
```

### `--iterations`

Override Monte Carlo iteration count.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --iterations 20000
```

### `--seed`

Set deterministic RNG seed.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --seed 42
```

### `--distribution`

Supported values:

- `log-normal`
- `exponential`
- `empirical`
- `gamma`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --distribution log-normal
poolsim simulate --config docs/fixtures/cli-config.json --distribution gamma
```

### `--queue-model`

Supported values:

- `mmc`
- `mdc`

Examples:

```bash
poolsim simulate --config docs/fixtures/cli-config.json --queue-model mmc
poolsim simulate --config docs/fixtures/cli-config.json --queue-model mdc
```

### `--target-wait-p99-ms`

Override the acceptance and risk threshold for p99 queue wait.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --target-wait-p99-ms 40
```

### `--max-acceptable-rho`

Override the utilization ceiling for candidate acceptance.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --max-acceptable-rho 0.80
```

## Config File Formats

## JSON config

```json
{
  "workload": {
    "requests_per_second": 220.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0,
    "raw_samples_ms": null,
    "step_load_profile": [
      { "time_s": 0, "requests_per_second": 180.0 },
      { "time_s": 30, "requests_per_second": 260.0 }
    ]
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

## TOML config

```toml
[workload]
requests_per_second = 220.0
latency_p50_ms = 8.0
latency_p95_ms = 32.0
latency_p99_ms = 85.0

[[workload.step_load_profile]]
time_s = 0
requests_per_second = 180.0

[[workload.step_load_profile]]
time_s = 30
requests_per_second = 260.0

[pool]
max_server_connections = 120
connection_overhead_ms = 2.0
idle_timeout_ms = 120000
min_pool_size = 3
max_pool_size = 24

[options]
iterations = 12000
seed = 7
distribution = "LogNormal"
queue_model = "MMC"
target_wait_p99_ms = 45.0
max_acceptable_rho = 0.85
```

## Batch File Formats

## JSON array batch

```json
[
  {
    "workload": {
      "requests_per_second": 180.0,
      "latency_p50_ms": 7.0,
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
      "iterations": 10000
    }
  },
  {
    "workload": {
      "requests_per_second": 260.0,
      "latency_p50_ms": 8.0,
      "latency_p95_ms": 30.0,
      "latency_p99_ms": 70.0
    },
    "pool": {
      "max_server_connections": 120,
      "connection_overhead_ms": 2.0,
      "min_pool_size": 3,
      "max_pool_size": 24
    },
    "options": {
      "iterations": 10000
    }
  }
]
```

## JSON object batch

```json
{
  "requests": [
    {
      "workload": {
        "requests_per_second": 180.0,
        "latency_p50_ms": 7.0,
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
        "iterations": 10000
      }
    }
  ]
}
```

## TOML batch

```toml
[[requests]]
[requests.workload]
requests_per_second = 180.0
latency_p50_ms = 7.0
latency_p95_ms = 25.0
latency_p99_ms = 60.0

[requests.pool]
max_server_connections = 100
connection_overhead_ms = 2.0
min_pool_size = 2
max_pool_size = 20

[requests.options]
iterations = 10000
```

## Output Formats

### Table output

Best for humans in terminals.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --format table
```

### JSON output

Best for automation, CI, scripts, and downstream APIs.

```bash
poolsim simulate --config docs/fixtures/cli-config.json --format json
```

### CSV output

Best for spreadsheets and pipeline export.

```bash
poolsim sweep --config docs/fixtures/cli-config.json --format csv
```

## Exit Codes

- `0`: successful run with non-warning/non-critical outcome
- `1`: parse error, validation error, config error, I/O error, or execution failure
- `2`: critical saturation
- `3`: warning exit when `--warn-exit` is enabled

Practical CI pattern:

```bash
poolsim --warn-exit simulate --config docs/fixtures/cli-config.json --format json
status=$?
if [ "$status" -eq 2 ]; then
  echo "critical saturation"
elif [ "$status" -eq 3 ]; then
  echo "warning saturation"
fi
```

## End-to-End Examples

### Example: quick first run

```bash
poolsim simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24 \
  --format json
```

### Example: evaluate one candidate pool

```bash
poolsim evaluate \
  --config docs/fixtures/cli-config.json \
  --pool-size 12 \
  --format json
```

### Example: export sensitivity analysis

```bash
poolsim sweep \
  --config docs/fixtures/cli-config.json \
  --format csv > sensitivity.csv
```

### Example: sample-driven simulation

```bash
poolsim simulate \
  --rps 180 \
  --p50 6 \
  --p95 25 \
  --p99 60 \
  --samples-file latencies.txt \
  --max-server-connections 100 \
  --connection-overhead-ms 2 \
  --min 2 \
  --max 20 \
  --distribution empirical \
  --format json
```

### Example: deterministic-service approximation

```bash
poolsim simulate \
  --config docs/fixtures/cli-config.json \
  --queue-model mdc \
  --format json
```

### Example: batch execution

```bash
poolsim batch --config docs/fixtures/batch.json --format json
```

## Notes

- CLI flags override config-file values.
- `simulate --pool-size` is an evaluation shortcut through the `simulate` command path.
- `simulate --sweep` is a sweep shortcut through the `simulate` command path.
- If `raw_samples_ms` is present, the library uses empirical fitting regardless of the requested distribution model.
