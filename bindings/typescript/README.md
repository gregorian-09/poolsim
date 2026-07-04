# poolsim TypeScript Bindings

TypeScript bindings call the stable `poolsim` CLI JSON interface.

## Install

```bash
npm install poolsim
```

The package is a thin wrapper around the Rust CLI. Install the `poolsim`
executable separately and ensure it is available on `PATH`:

```bash
cargo install poolsim-cli
poolsim --version
```

Use `new PoolsimClient('/path/to/poolsim')` when the binary is not on `PATH`.

## Example

```ts
import { PoolsimClient } from 'poolsim';

const client = new PoolsimClient('poolsim');
const report = client.simulate('docs/fixtures/cli-config.json');
console.log(report.optimal_pool_size);
```

Available methods include `simulate`, `evaluate`, `sweep`, `batch`, `compare`,
`budget`, `telemetryRecommend`, `doctor`, `generateConfig`, and `gate`.
