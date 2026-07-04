terraform {
  required_providers {
    external = {
      source  = "hashicorp/external"
      version = ">= 2.3.0"
    }
  }
}

data "external" "poolsim_sizing" {
  program = ["python3", "${path.module}/../../external/poolsim_sizing.py"]

  query = {
    poolsim_executable = var.poolsim_executable
    command            = "simulate"
    config             = var.poolsim_config
  }
}

locals {
  recommended_pool_size = tonumber(data.external.poolsim_sizing.result.optimal_pool_size)
  p99_queue_wait_ms     = tonumber(data.external.poolsim_sizing.result.p99_queue_wait_ms)
  saturation            = data.external.poolsim_sizing.result.saturation
}

output "recommended_pool_size" {
  value = local.recommended_pool_size
}

output "p99_queue_wait_ms" {
  value = local.p99_queue_wait_ms
}

output "saturation" {
  value = local.saturation
}
