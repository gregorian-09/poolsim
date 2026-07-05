# poolsim TypeScript Bindings

`poolsim` is the TypeScript/Node.js binding package for the Poolsim connection-pool sizing toolkit.

The package intentionally delegates all sizing work to the Rust `poolsim` CLI JSON contract. It does not reimplement queueing formulas in TypeScript. That keeps JavaScript and TypeScript services aligned with the same model used by the Rust library, CLI, REST API, CI gates, docs fixtures, and release tests.

Use this package when a Node.js service, script, dashboard job, or CI tool wants to call Poolsim from TypeScript without manually spawning the CLI and parsing JSON.

## Install

Install the Node package:

```bash
npm install poolsim
```

Install the Rust CLI executable separately:

```bash
cargo install poolsim-cli
poolsim --version
```

The TypeScript package expects the `poolsim` executable to be available on `PATH` by default. If your binary is installed somewhere else, pass the absolute path to `PoolsimClient`:

```ts
import { PoolsimClient } from 'poolsim';

const client = new PoolsimClient('/opt/tools/poolsim');
```

## Runtime Requirements

- Node.js with ECMAScript module support.
- The `poolsim` executable from `poolsim-cli`.
- Poolsim config files or telemetry files that match the documented CLI schemas.

This package has no runtime npm dependencies. `typescript` and `@types/node` are development dependencies used to build the published `dist` files.

## Importing

The package is ESM-only:

```ts
import { PoolsimClient, PoolsimError } from 'poolsim';
```

CommonJS `require('poolsim')` is not supported because the package publishes ESM output.

## Basic Simulation

Run a full recommendation workflow from a checked-in simulation config:

```ts
import { PoolsimClient } from 'poolsim';

const client = new PoolsimClient('poolsim');
const report = client.simulate('docs/fixtures/cli-config.json');

console.log(report.optimal_pool_size);
console.log(report.confidence_interval);
console.log(report.p99_queue_wait_ms);
console.log(report.saturation);
```

This calls:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

The returned value is the parsed JSON object emitted by the CLI.

## Evaluate A Fixed Pool Size

Use `evaluate` when production already has a configured pool and you want to score that exact size:

```ts
const evaluation = client.evaluate('docs/fixtures/cli-config.json', 8);

console.log(evaluation.pool_size);
console.log(evaluation.utilisation_rho);
console.log(evaluation.p99_queue_wait_ms);
console.log(evaluation.saturation);
```

This calls:

```bash
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 8
```

## Generate Sensitivity Rows

Use `sweep` to inspect nearby candidate pool sizes:

```ts
const rows = client.sweep('docs/fixtures/cli-config.json');

for (const row of rows) {
  console.log(row.pool_size, row.utilisation_rho, row.risk);
}
```

This calls:

```bash
poolsim --format json sweep --config docs/fixtures/cli-config.json
```

## Run A Batch File

Use `batch` when a file contains multiple simulation requests:

```ts
const reports = client.batch('docs/fixtures/batch.json');

for (const report of reports) {
  console.log(report.optimal_pool_size, report.saturation);
}
```

This calls:

```bash
poolsim --format json batch --config docs/fixtures/batch.json
```

## Compare Scenarios

Use `compare` for normal, peak, and incident traffic assumptions:

```ts
const comparison = client.compare('docs/fixtures/scenarios.json');

console.log(comparison.baseline);
console.log(comparison.worst_saturation);
console.log(comparison.rows);
```

This calls:

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

## Plan A Database Connection Budget

Use `budget` to allocate a global database connection budget across services and replicas:

```ts
const plan = client.budget('docs/fixtures/budget.json');

console.log(plan.status);
console.log(plan.available_connections);
console.log(plan.allocated_total_connections);
console.log(plan.services);
```

This calls:

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

## Import Telemetry And Get A Recommendation Diff

Use `telemetryRecommend` when you have a Poolsim telemetry snapshot and a current production pool size:

```ts
const recommendation = client.telemetryRecommend('docs/fixtures/telemetry.json');

console.log(recommendation.service_name);
console.log(recommendation.diff.current_pool_size);
console.log(recommendation.diff.recommended_pool_size);
console.log(recommendation.diff.change);
```

This calls:

```bash
poolsim --format json import telemetry --config docs/fixtures/telemetry.json
```

## Diagnose A Pool With Doctor

Use `doctor` to classify the current pool as healthy, too small, too large, or close to saturation:

```ts
const diagnosis = client.doctor('docs/fixtures/telemetry.json');

console.log(diagnosis.status);
for (const finding of diagnosis.findings ?? []) {
  console.log(finding.severity, finding.message, finding.action);
}
```

This calls:

```bash
poolsim --format json doctor telemetry --config docs/fixtures/telemetry.json
```

## Generate Framework Configuration

Use `generateConfig` to create runtime pool configuration snippets from the recommendation:

```ts
const sqlx = client.generateConfig('sqlx', 'docs/fixtures/cli-config.json');
console.log(sqlx.framework);
console.log(sqlx.recommended_pool_size);
console.log(sqlx.snippet);
```

This calls:

```bash
poolsim --format json generate-config --framework sqlx simulate --config docs/fixtures/cli-config.json
```

Supported framework names are the same as the CLI, including:

- `hikaricp`
- `spring-boot`
- `sqlalchemy`
- `prisma`
- `node-pg`
- `sqlx`
- `deadpool`

## Run A CI Gate

Use `gate` to enforce a safety policy in Node-based automation:

```ts
const gate = client.gate('docs/fixtures/gate-policy.toml', 'docs/fixtures/telemetry.json');

console.log(gate.status);
console.log(gate.deployment_safe);
console.log(gate.checks);
```

This calls:

```bash
poolsim --format json gate --policy docs/fixtures/gate-policy.toml telemetry --config docs/fixtures/telemetry.json
```

`gate` allows CLI exit codes `0` and `2` because a failed capacity gate is a valid machine-readable result, not a wrapper error. Other exit codes still throw `PoolsimError`.

## Error Handling

The wrapper throws `PoolsimError` when:

- the `poolsim` executable cannot be started
- the CLI exits with an unexpected exit code
- the CLI does not emit valid JSON

```ts
import { PoolsimClient, PoolsimError } from 'poolsim';

const client = new PoolsimClient('poolsim');

try {
  const report = client.simulate('missing.json');
  console.log(report);
} catch (error) {
  if (error instanceof PoolsimError) {
    console.error('Poolsim failed:', error.message);
  } else {
    throw error;
  }
}
```

## API Reference

### `new PoolsimClient(executable = 'poolsim')`

Creates a client that invokes the given executable. Use the default when `poolsim` is on `PATH`; pass an absolute path otherwise.

### `simulate(config)`

Runs `poolsim simulate --config <config>` and returns the recommendation report object.

### `evaluate(config, poolSize)`

Runs `poolsim evaluate --config <config> --pool-size <poolSize>` and returns a fixed-size evaluation object.

### `sweep(config)`

Runs `poolsim sweep --config <config>` and returns sensitivity rows.

### `batch(config)`

Runs `poolsim batch --config <config>` and returns an array of simulation reports.

### `compare(config)`

Runs `poolsim compare --config <config>` and returns a scenario comparison report.

### `budget(config)`

Runs `poolsim budget --config <config>` and returns a database budget plan.

### `telemetryRecommend(config)`

Runs `poolsim import telemetry --config <config>` and returns a recommendation diff.

### `doctor(config)`

Runs `poolsim doctor telemetry --config <config>` and returns a pool diagnosis report.

### `generateConfig(framework, config)`

Runs `poolsim generate-config --framework <framework> simulate --config <config>` and returns a framework config snippet report.

### `gate(policy, telemetryConfig)`

Runs `poolsim gate --policy <policy> telemetry --config <telemetryConfig>` and returns a gate report.

## Output Types

The wrapper exposes `JsonValue` as a broad JSON type:

```ts
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
```

Most methods return `Record<string, JsonValue>` because the exact JSON shape is owned by the stable CLI contract and documented in the main Poolsim docs. `sweep` and `batch` return arrays.

## Compatibility

This package is a binding layer over the CLI. It does not change Rust APIs, REST routes, WebSocket events, config schemas, or serialized CLI output. Existing CLI behavior remains the source of truth.

## Troubleshooting

### `spawnSync poolsim ENOENT`

The executable is not on `PATH`. Install it with `cargo install poolsim-cli`, or pass an absolute executable path:

```ts
const client = new PoolsimClient('/home/me/.cargo/bin/poolsim');
```

### `poolsim did not emit valid JSON`

The wrapper always passes `--format json`. This error usually means a different executable named `poolsim` is being invoked, or the binary is too old for the command being called.

### Gate failures throw in my script

`gate` accepts exit codes `0` and `2`. If you need warning exit code `3` behavior, call the CLI directly for now or add a wrapper method in your own code with the exit-code policy you need.

## Issues

Report bugs or documentation gaps at:

<https://github.com/gregorian-09/poolsim/issues>
