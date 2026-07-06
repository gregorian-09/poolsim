# poolsim TypeScript Bindings

`@gregorian09/poolsim` is the TypeScript and Node.js binding package for the Poolsim connection-pool sizing toolkit.

The package intentionally delegates all sizing work to the Rust `poolsim` CLI JSON contract. It does not reimplement queueing formulas in TypeScript. That keeps JavaScript and TypeScript services aligned with the same model used by the Rust library, CLI, REST API, WebSocket API, CI gates, docs fixtures, and release tests.

Use this package when a Node.js service, script, dashboard job, or CI tool wants to call Poolsim without manually spawning the CLI and parsing JSON.

## What You Can Do

- Run a full pool-size recommendation from a simulation config.
- Evaluate an existing production pool size.
- Generate sensitivity rows across candidate pool sizes.
- Run batch sizing jobs.
- Compare normal, peak, and incident traffic scenarios.
- Allocate a shared database `max_connections` budget across services.
- Import telemetry snapshots and compute recommendation diffs.
- Diagnose whether a live pool is too small, too large, or close to saturation.
- Generate framework-specific pool configuration snippets.
- Run a CI capacity gate and consume the gate report as JavaScript data.

## Architecture

The TypeScript package calls this command shape internally:

```bash
poolsim --format json <command> ...
```

The returned value is decoded from the CLI JSON output. The wrapper deliberately keeps the payloads as JSON-like objects so new fields added by the CLI remain available without requiring a wrapper release.

This is a compatibility choice: existing methods stay stable, while the Rust CLI remains the single source of truth for simulation behavior.

## Install

Install the Node package:

```bash
npm install @gregorian09/poolsim
```

Install the Rust CLI executable separately:

```bash
cargo install poolsim-cli
poolsim --version
```

The TypeScript package expects the `poolsim` executable to be available on `PATH` by default. If your binary is installed somewhere else, pass the absolute path to `PoolsimClient`:

```ts
import { PoolsimClient } from '@gregorian09/poolsim';

const client = new PoolsimClient('/opt/tools/poolsim');
```

## Runtime Requirements

- Node.js with ECMAScript module support.
- The `poolsim` executable from `poolsim-cli`.
- Poolsim config, telemetry, scenario, or policy files that match the documented CLI schemas.

This package has no runtime npm dependencies. `typescript` and `@types/node` are development dependencies used to build the published `dist` files.

## Importing

The package is ESM-only:

```ts
import { PoolsimClient, PoolsimError } from '@gregorian09/poolsim';
```

CommonJS `require('@gregorian09/poolsim')` is not supported because the package publishes ESM output.

## Quick Start

```ts
import { PoolsimClient } from '@gregorian09/poolsim';

const client = new PoolsimClient();
const report = client.simulate('docs/fixtures/cli-config.json');

console.log('recommended pool size:', report.optimal_pool_size);
console.log('p99 queue wait ms:', report.p99_queue_wait_ms);
console.log('saturation:', report.saturation);
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

## Minimal Simulation Config

A typical simulation config contains workload assumptions, pool limits, and sizing options:

```json
{
  "workload": {
    "requests_per_second": 220.0,
    "latency_p50_ms": 8.0,
    "latency_p95_ms": 32.0,
    "latency_p99_ms": 85.0
  },
  "pool": {
    "max_server_connections": 120,
    "connection_overhead_ms": 2.0,
    "idle_timeout_ms": 120000,
    "min_pool_size": 3,
    "max_pool_size": 24
  },
  "options": {
    "iterations": 12000,
    "seed": 7,
    "distribution": "LogNormal",
    "queue_model": "MMC",
    "target_wait_p99_ms": 45.0,
    "max_acceptable_rho": 0.85
  }
}
```

Keep these files in source control when you want sizing assumptions to be reviewable.

## Type Model

The package exports a recursive JSON value type:

```ts
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
```

Most methods return either `Record<string, JsonValue>` or `JsonValue[]`. This keeps the wrapper forward-compatible when the CLI adds fields. You can narrow the shape in your application if you want stricter local types:

```ts
type SimulationReport = {
  optimal_pool_size?: number;
  p99_queue_wait_ms?: number;
  saturation?: string;
};

const report = client.simulate('docs/fixtures/cli-config.json') as SimulationReport;
```

## API Reference

### `new PoolsimClient(executable = 'poolsim')`

Creates a client that invokes the given executable.

Use the default when `poolsim` is on `PATH`:

```ts
const client = new PoolsimClient();
```

Use an explicit path when automation installs the binary in a fixed tool directory:

```ts
const client = new PoolsimClient('/usr/local/bin/poolsim');
```

### `simulate(config)`

Runs a full sizing simulation from a config file and returns a recommendation report.

```ts
const report = client.simulate('docs/fixtures/cli-config.json');

console.log(report.optimal_pool_size);
console.log(report.confidence_interval);
console.log(report.p99_queue_wait_ms);
console.log(report.saturation);
```

Equivalent CLI command:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

Use `simulate` when you are choosing the pool size to deploy.

### `evaluate(config, poolSize)`

Scores a fixed pool size against a workload config.

```ts
const evaluation = client.evaluate('docs/fixtures/cli-config.json', 8);

console.log(evaluation.pool_size);
console.log(evaluation.utilisation_rho);
console.log(evaluation.p99_queue_wait_ms);
console.log(evaluation.saturation);
```

Equivalent CLI command:

```bash
poolsim --format json evaluate --config docs/fixtures/cli-config.json --pool-size 8
```

Use `evaluate` when production already has a configured pool and you need to know whether that exact setting is safe.

### `sweep(config)`

Returns sensitivity rows for candidate pool sizes.

```ts
const rows = client.sweep('docs/fixtures/cli-config.json');

for (const row of rows) {
  if (typeof row === 'object' && row !== null && !Array.isArray(row)) {
    console.log(row.pool_size, row.utilisation_rho, row.risk);
  }
}
```

Equivalent CLI command:

```bash
poolsim --format json sweep --config docs/fixtures/cli-config.json
```

Use `sweep` to explain tradeoffs around nearby pool sizes in code review or capacity planning.

### `batch(config)`

Runs multiple simulation requests from one batch config.

```ts
const reports = client.batch('docs/fixtures/batch.json');

for (const report of reports) {
  if (typeof report === 'object' && report !== null && !Array.isArray(report)) {
    console.log(report.optimal_pool_size, report.saturation);
  }
}
```

Equivalent CLI command:

```bash
poolsim --format json batch --config docs/fixtures/batch.json
```

Use `batch` when a platform team wants to size several services in one job.

### `compare(config)`

Compares named traffic scenarios such as normal, peak, and incident load.

```ts
const comparison = client.compare('docs/fixtures/scenarios.json');

console.log(comparison.baseline);
console.log(comparison.worst_saturation);
console.log(comparison.rows);
```

Equivalent CLI command:

```bash
poolsim --format json compare --config docs/fixtures/scenarios.json
```

Use `compare` when one pool setting must survive several traffic assumptions.

### `budget(config)`

Plans allocation of a database connection budget across services and replicas.

```ts
const plan = client.budget('docs/fixtures/budget.json');

console.log(plan.status);
console.log(plan.available_connections);
console.log(plan.allocated_total_connections);
console.log(plan.services);
```

Equivalent CLI command:

```bash
poolsim --format json budget --config docs/fixtures/budget.json
```

Use `budget` when the database has a shared `max_connections` limit and several services compete for it.

### `telemetryRecommend(config)`

Imports a telemetry snapshot and returns a recommendation diff from the current pool setting.

```ts
const recommendation = client.telemetryRecommend('docs/fixtures/telemetry.json');
const diff = recommendation.diff as Record<string, unknown> | undefined;

console.log(recommendation.service_name);
console.log(diff?.current_pool_size);
console.log(diff?.recommended_pool_size);
console.log(diff?.change);
```

Equivalent CLI command:

```bash
poolsim --format json import telemetry --config docs/fixtures/telemetry.json
```

Use `telemetryRecommend` when you have observed production traffic and want to compare the configured pool with the recommended pool.

### `doctor(config)`

Diagnoses a telemetry snapshot and classifies risk.

```ts
const diagnosis = client.doctor('docs/fixtures/telemetry.json');

console.log(diagnosis.status);
for (const finding of (diagnosis.findings as unknown[] | undefined) ?? []) {
  console.log(finding);
}
```

Equivalent CLI command:

```bash
poolsim --format json doctor telemetry --config docs/fixtures/telemetry.json
```

Use `doctor` when an engineer asks: is this pool too small, too large, or close to saturation?

### `generateConfig(framework, config)`

Generates a framework-specific runtime configuration snippet from a simulation recommendation.

```ts
const sqlx = client.generateConfig('sqlx', 'docs/fixtures/cli-config.json');

console.log(sqlx.framework);
console.log(sqlx.recommended_pool_size);
console.log(sqlx.snippet);
```

Equivalent CLI command:

```bash
poolsim --format json generate-config --framework sqlx simulate --config docs/fixtures/cli-config.json
```

Supported framework names follow the CLI:

- `hikaricp`
- `spring-boot`
- `sqlalchemy`
- `prisma`
- `node-pg`
- `sqlx`
- `deadpool`

Use `generateConfig` after `simulate` when you want a copy-pasteable starting point for a real runtime pool.

### `gate(policy, telemetryConfig)`

Runs a capacity gate against a telemetry snapshot and policy file.

```ts
const gate = client.gate('docs/fixtures/gate-policy.toml', 'docs/fixtures/telemetry.json');

console.log(gate.status);
console.log(gate.deployment_safe);
console.log(gate.checks);
```

Equivalent CLI command:

```bash
poolsim --format json gate --policy docs/fixtures/gate-policy.toml telemetry --config docs/fixtures/telemetry.json
```

`gate` treats CLI exit codes `0` and `2` as valid machine-readable outcomes. Exit code `2` means the capacity gate failed, not that the TypeScript wrapper failed. Other non-zero exit codes throw `PoolsimError`.

## CI Usage

A minimal GitHub Actions step can install both packages and fail the job when your policy says the deployment is unsafe:

```yaml
- name: Install poolsim
  run: |
    cargo install poolsim-cli
    npm ci

- name: Run capacity gate
  run: |
    node --input-type=module <<'JS'
    import { PoolsimClient } from '@gregorian09/poolsim';

    const client = new PoolsimClient();
    const report = client.gate('capacity-policy.toml', 'telemetry.json');
    console.log(report);
    if (report.deployment_safe !== true) {
      process.exit(1);
    }
    JS
```

## Error Handling

The wrapper throws `PoolsimError` when:

- the `poolsim` executable cannot be started
- the CLI exits with an unexpected exit code
- the CLI does not emit valid JSON

```ts
import { PoolsimClient, PoolsimError } from '@gregorian09/poolsim';

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

The error message includes CLI stderr when the CLI provides it.

## Working With Returned Data

Return types intentionally use generic JSON values:

- JSON objects become JavaScript objects.
- JSON arrays become JavaScript arrays.
- JSON strings, numbers, booleans, and `null` become normal JavaScript values.

This keeps the binding forward-compatible when the CLI adds new output fields. For production code, define local TypeScript types around the fields your application actually consumes.

## Troubleshooting

### `spawnSync poolsim ENOENT`

The Rust CLI is not installed or is not on `PATH`.

```bash
cargo install poolsim-cli
poolsim --version
```

Or pass an explicit executable path:

```ts
const client = new PoolsimClient('/absolute/path/to/poolsim');
```

### `poolsim did not emit valid JSON`

The wrapper always passes `--format json`. This error usually means the executable path points to a different program, an older incompatible binary, or a command failed before producing JSON.

Check the CLI directly:

```bash
poolsim --format json simulate --config docs/fixtures/cli-config.json
```

### Gate returns a failed policy report

That is expected behavior when capacity assumptions are unsafe. Inspect `status`, `deployment_safe`, and `checks` in the returned object. The wrapper will still return JSON for CLI exit code `2`.

## Compatibility Notes

- The TypeScript wrapper does not change CLI semantics.
- The Rust CLI remains the source of truth for formulas, validation, and output fields.
- Existing method names are intended to remain stable.
- New CLI output fields can appear in returned objects without a TypeScript package change.

## Support

- Documentation: <https://github.com/gregorian-09/poolsim/tree/main/docs>
- Issues: <https://github.com/gregorian-09/poolsim/issues>
- Repository: <https://github.com/gregorian-09/poolsim>
