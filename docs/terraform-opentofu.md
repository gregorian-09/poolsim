# Terraform And OpenTofu

Poolsim supports Terraform and OpenTofu through an external-data adapter at [`../integrations/terraform/external/poolsim_sizing.py`](../integrations/terraform/external/poolsim_sizing.py). This provides a `poolsim_sizing`-style data source without requiring a custom provider binary.

## Usage

```hcl
data "external" "poolsim_sizing" {
  program = ["python3", "integrations/terraform/external/poolsim_sizing.py"]

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

The adapter returns strings because Terraform external data sources expect a flat string map. Numeric values should be converted with `tonumber` in Terraform or OpenTofu configuration.

## Example Module

A complete example lives in [`../integrations/terraform/examples/sizing`](../integrations/terraform/examples/sizing). It exposes recommended pool size, p99 queue wait, and saturation as outputs.

## Safety

The adapter only reads a Poolsim config and runs the existing CLI. It does not mutate cloud resources, databases, or runtime pool settings.
