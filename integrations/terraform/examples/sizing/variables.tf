variable "poolsim_executable" {
  type        = string
  description = "Poolsim executable path. Use poolsim when it is on PATH."
  default     = "poolsim"
}

variable "poolsim_config" {
  type        = string
  description = "Path to a Poolsim simulation config JSON or TOML file."
}
