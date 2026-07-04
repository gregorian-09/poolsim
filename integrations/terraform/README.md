# Terraform And OpenTofu Integration

This integration exposes Poolsim sizing through the Terraform/OpenTofu `external` provider. It behaves like a `poolsim_sizing` data source without requiring a custom provider binary.

The adapter follows the external provider protocol: JSON query on stdin, flat string map JSON on stdout.

## Example

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
}
```

Use [`examples/sizing`](examples/sizing) as a copyable module.

## Outputs

The adapter returns string values for Terraform compatibility:

- `optimal_pool_size`
- `pool_size` for fixed-size evaluation
- `confidence_interval_min`
- `confidence_interval_max`
- `cold_start_min_pool_size`
- `utilisation_rho`
- `mean_queue_wait_ms`
- `p99_queue_wait_ms`
- `saturation`
- `raw_json`

## Compatibility

This integration is additive. It shells out to the existing `poolsim --format json` command and does not change Rust APIs, CLI output schemas, REST routes, or config files.
