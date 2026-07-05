# Terraform And OpenTofu Integration

This integration exposes Poolsim sizing through the Terraform/OpenTofu `external` provider. It behaves like a lightweight `poolsim_sizing` data source without requiring a custom provider binary.

Use it when connection-pool sizing should appear in infrastructure review, `terraform plan`, or module outputs.

## What It Does

- Reads a JSON query from Terraform's `external` data source on stdin.
- Runs `poolsim --format json simulate` or `poolsim --format json evaluate`.
- Flattens selected Poolsim JSON fields into Terraform-compatible string values.
- Preserves the full CLI payload in `raw_json` for consumers that need fields not flattened directly.

It does not manage database connections, mutate application configuration, or create cloud resources by itself.

## Runtime Requirements

- Terraform or OpenTofu with the `hashicorp/external` provider.
- Python 3.9 or newer.
- The Rust `poolsim` executable from `poolsim-cli`.
- A Poolsim config file committed or generated before `terraform plan`.

Install the CLI:

```bash
cargo install poolsim-cli
poolsim --version
```

## Files

- `external/poolsim_sizing.py`: external-provider adapter.
- `external/test_poolsim_sizing.py`: unit tests for adapter behavior.
- `examples/sizing/main.tf`: copyable Terraform example.
- `examples/sizing/variables.tf`: variables for the example module.

## Basic Simulation Data Source

```hcl
data "external" "poolsim_sizing" {
  program = ["python3", "${path.module}/external/poolsim_sizing.py"]

  query = {
    poolsim_executable = "poolsim"
    command            = "simulate"
    config             = "docs/fixtures/cli-config.json"
  }
}

locals {
  recommended_pool_size = tonumber(data.external.poolsim_sizing.result.optimal_pool_size)
  p99_queue_wait_ms     = tonumber(data.external.poolsim_sizing.result.p99_queue_wait_ms)
  saturation            = data.external.poolsim_sizing.result.saturation
}
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

## Fixed Pool Evaluation

Use `evaluate` when Terraform should score an already configured pool size:

```hcl
data "external" "current_pool" {
  program = ["python3", "${path.module}/external/poolsim_sizing.py"]

  query = {
    poolsim_executable = "poolsim"
    command            = "evaluate"
    config             = "docs/fixtures/cli-config.json"
    pool_size          = "8"
  }
}

locals {
  current_pool_size = tonumber(data.external.current_pool.result.pool_size)
  current_rho       = tonumber(data.external.current_pool.result.utilisation_rho)
  current_p99_wait  = tonumber(data.external.current_pool.result.p99_queue_wait_ms)
}
```

Equivalent CLI command:

```bash
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 8
```

## Query Fields

| Field | Required | Description |
| --- | --- | --- |
| `poolsim_executable` | No | Path to the `poolsim` CLI. Defaults to `poolsim`. |
| `command` | No | `simulate` or `evaluate`. Defaults to `simulate`. |
| `config` | Yes | Path to a Poolsim JSON or TOML config file. |
| `pool_size` | Only for `evaluate` | Fixed pool size to evaluate. |

Terraform external-provider values are strings, so pass numeric values as strings when practical.

## Result Fields

The adapter returns string values because Terraform's external provider expects a flat map of strings.

| Field | Present When | Meaning |
| --- | --- | --- |
| `optimal_pool_size` | `simulate` output includes it | Recommended pool size. |
| `pool_size` | `evaluate` output includes it | Evaluated pool size. |
| `confidence_interval_min` | Confidence interval exists | Lower recommendation confidence bound. |
| `confidence_interval_max` | Confidence interval exists | Upper recommendation confidence bound. |
| `cold_start_min_pool_size` | CLI output includes it | Suggested warm minimum pool size. |
| `utilisation_rho` | CLI output includes it | Queue utilization ratio. |
| `mean_queue_wait_ms` | CLI output includes it | Predicted mean queue wait in milliseconds. |
| `p99_queue_wait_ms` | CLI output includes it | Predicted p99 queue wait in milliseconds. |
| `saturation` | CLI output includes it | Saturation/risk classification. |
| `raw_json` | Always | Full Poolsim JSON payload encoded as a string. |

Convert numbers with `tonumber(...)` in Terraform.

## Using The Example Module

```bash
cd integrations/terraform/examples/sizing
terraform init
terraform plan -var='poolsim_config=../../../docs/fixtures/cli-config.json'
```

The example exposes:

- `recommended_pool_size`
- `p99_queue_wait_ms`
- `saturation`

## Feeding Pool Sizes Into Infrastructure

A typical pattern is to use the recommendation as an input to service configuration:

```hcl
locals {
  recommended_pool_size = tonumber(data.external.poolsim_sizing.result.optimal_pool_size)
}

resource "kubernetes_config_map" "checkout_pool" {
  metadata {
    name = "checkout-pool"
  }

  data = {
    DATABASE_POOL_MAX = tostring(local.recommended_pool_size)
  }
}
```

Treat this as a policy choice. Some teams prefer to expose the recommendation as an output only and require a human to approve changing the runtime setting.

## Error Handling

Terraform surfaces adapter stderr when the external program exits non-zero.

Common errors:

- `missing required query field: config`: the `config` query key is absent or empty.
- `command must be simulate or evaluate`: unsupported command value.
- `poolsim exited with <code>`: the CLI rejected the config or failed to run.
- `exec`/file-not-found errors: the configured `poolsim_executable` path is invalid.

Debug by running the CLI directly first:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

## Run Tests

```bash
python3 -m unittest integrations/terraform/external/test_poolsim_sizing.py
```

The tests validate payload flattening, `raw_json`, and command validation without requiring Terraform.

## Compatibility

This integration is additive. It shells out to the existing `poolsim --format json` command and does not change Rust APIs, CLI output schemas, REST routes, WebSocket routes, or config files.

## Support

- Example module: [`examples/sizing`](examples/sizing)
- Issues: <https://github.com/gregorian-09/poolsim/issues>
