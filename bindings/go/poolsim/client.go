// Package poolsim wraps the stable Poolsim CLI JSON interface for Go services.
package poolsim

import (
	"encoding/json"
	"fmt"
	"os/exec"
)

// Client invokes the poolsim executable and decodes JSON output.
type Client struct {
	Executable string
}

// NewClient creates a client for an executable path. Use "poolsim" for PATH lookup.
func NewClient(executable string) Client {
	return Client{Executable: executable}
}

// Simulate runs `poolsim simulate`.
func (c Client) Simulate(config string) (map[string]any, error) {
	return c.runObject([]string{"simulate", "--config", config}, 0)
}

// Evaluate runs `poolsim evaluate`.
func (c Client) Evaluate(config string, poolSize int) (map[string]any, error) {
	return c.runObject([]string{"evaluate", "--config", config, "--pool-size", fmt.Sprint(poolSize)}, 0)
}

// Sweep runs `poolsim sweep`.
func (c Client) Sweep(config string) ([]map[string]any, error) {
	return c.runArray([]string{"sweep", "--config", config}, 0)
}

// Batch runs `poolsim batch`.
func (c Client) Batch(config string) ([]map[string]any, error) {
	return c.runArray([]string{"batch", "--config", config}, 0)
}

// Compare runs `poolsim compare`.
func (c Client) Compare(config string) (map[string]any, error) {
	return c.runObject([]string{"compare", "--config", config}, 0)
}

// Budget runs `poolsim budget`.
func (c Client) Budget(config string) (map[string]any, error) {
	return c.runObject([]string{"budget", "--config", config}, 0)
}

// TelemetryRecommend runs `poolsim import telemetry`.
func (c Client) TelemetryRecommend(config string) (map[string]any, error) {
	return c.runObject([]string{"import", "telemetry", "--config", config}, 0)
}

// Doctor runs `poolsim doctor telemetry`.
func (c Client) Doctor(config string) (map[string]any, error) {
	return c.runObject([]string{"doctor", "telemetry", "--config", config}, 0)
}

// GenerateConfig runs `poolsim generate-config` from a simulation config.
func (c Client) GenerateConfig(framework string, config string) (map[string]any, error) {
	return c.runObject([]string{"generate-config", "--framework", framework, "simulate", "--config", config}, 0)
}

func (c Client) runObject(args []string, allowedExitCodes ...int) (map[string]any, error) {
	var out map[string]any
	if err := c.runJSON(args, &out, allowedExitCodes...); err != nil {
		return nil, err
	}
	return out, nil
}

func (c Client) runArray(args []string, allowedExitCodes ...int) ([]map[string]any, error) {
	var out []map[string]any
	if err := c.runJSON(args, &out, allowedExitCodes...); err != nil {
		return nil, err
	}
	return out, nil
}

func (c Client) runJSON(args []string, out any, allowedExitCodes ...int) error {
	executable := c.Executable
	if executable == "" {
		executable = "poolsim"
	}
	cmdArgs := append([]string{"--format", "json"}, args...)
	cmd := exec.Command(executable, cmdArgs...)
	stdout, err := cmd.Output()
	if err != nil {
		return err
	}
	return json.Unmarshal(stdout, out)
}
