package poolsim

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func fakePoolsim(t *testing.T, payload string) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "poolsim")
	script := "#!/bin/sh\nprintf '%s' '" + payload + "'\n"
	if runtime.GOOS == "windows" {
		t.Skip("shell fake is unix-only")
	}
	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestClientSimulateDelegatesToCLI(t *testing.T) {
	client := NewClient(fakePoolsim(t, `{"optimal_pool_size":8}`))
	report, err := client.Simulate("config.json")
	if err != nil {
		t.Fatal(err)
	}
	if report["optimal_pool_size"].(float64) != 8 {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestClientMethodsExposeWorkflows(t *testing.T) {
	client := NewClient(fakePoolsim(t, `{"status":"ok"}`))
	calls := []func() (map[string]any, error){
		func() (map[string]any, error) { return client.Evaluate("c.json", 8) },
		func() (map[string]any, error) { return client.Compare("c.json") },
		func() (map[string]any, error) { return client.Budget("c.json") },
		func() (map[string]any, error) { return client.TelemetryRecommend("t.json") },
		func() (map[string]any, error) { return client.Doctor("t.json") },
		func() (map[string]any, error) { return client.GenerateConfig("sqlx", "c.json") },
	}
	for _, call := range calls {
		report, err := call()
		if err != nil {
			t.Fatal(err)
		}
		if report["status"] != "ok" {
			t.Fatalf("unexpected report: %#v", report)
		}
	}
}
