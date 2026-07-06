# Language Bindings

Poolsim includes first-party binding packages for Python, TypeScript, and Go. The bindings intentionally call the stable `poolsim` CLI JSON contract instead of duplicating the Rust queueing model in each language.

## Python

Package source: [`../bindings/python`](../bindings/python)

Install:

```bash
pip install poolsim
cargo install poolsim-cli
```

The Python package delegates to the Rust `poolsim` executable. Keep the binary on
`PATH`, or pass its absolute path to `PoolsimClient`.

```python
from poolsim import PoolsimClient

client = PoolsimClient(executable="poolsim")
report = client.simulate("docs/fixtures/cli-config.json")
print(report["optimal_pool_size"])
```

Supported methods include `simulate`, `evaluate`, `sweep`, `batch`, `compare`, `budget`, `telemetry_recommend`, `doctor`, `generate_config`, and `gate`.

## TypeScript

Package source: [`../bindings/typescript`](../bindings/typescript)

Install:

```bash
npm install @gregorian09/poolsim
cargo install poolsim-cli
```

The TypeScript package delegates to the Rust `poolsim` executable. Keep the binary on
`PATH`, or pass its absolute path to `PoolsimClient`.

```ts
import { PoolsimClient } from '@gregorian09/poolsim';

const client = new PoolsimClient('poolsim');
const report = client.simulate('docs/fixtures/cli-config.json');
console.log(report.optimal_pool_size);
```

Supported methods include `simulate`, `evaluate`, `sweep`, `batch`, `compare`, `budget`, `telemetryRecommend`, `doctor`, `generateConfig`, and `gate`.

## Go

Package source: [`../bindings/go`](../bindings/go)

```go
client := poolsim.NewClient("poolsim")
report, err := client.Simulate("docs/fixtures/cli-config.json")
if err != nil {
    panic(err)
}
fmt.Println(report["optimal_pool_size"])
```

Supported methods include `Simulate`, `Evaluate`, `Sweep`, `Batch`, `Compare`, `Budget`, `TelemetryRecommend`, `Doctor`, and `GenerateConfig`.

## Compatibility

The bindings are additive packages. They do not change Rust APIs, CLI commands, REST routes, config files, or serialized output. Users must install or provide the `poolsim` executable because the bindings delegate all sizing to the Rust implementation.
