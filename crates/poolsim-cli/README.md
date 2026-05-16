# poolsim-cli

`poolsim-cli` is the command-line interface for the `poolsim` sizing calculator.

It is intended for operators, developers, and CI workflows that want pool-sizing output without embedding the library directly.

## Commands

It supports:

- full simulation
- fixed-size evaluation
- sensitivity sweeps
- batch execution
- named scenario comparison
- telemetry import and recommendation diff
- Prometheus-compatible telemetry import
- CI capacity gates for telemetry-backed release checks
- pool diagnosis for configured production settings
- framework-specific runtime pool config generation
- table, JSON, and CSV output

## Install

```bash
cargo install poolsim-cli
```

## Main Commands

- `poolsim-cli simulate`
- `poolsim-cli evaluate`
- `poolsim-cli sweep`
- `poolsim-cli batch`
- `poolsim-cli compare`
- `poolsim-cli import telemetry`
- `poolsim-cli import prometheus`
- `poolsim-cli gate telemetry`
- `poolsim-cli gate prometheus`
- `poolsim-cli guard telemetry`
- `poolsim-cli guard prometheus`
- `poolsim-cli doctor telemetry`
- `poolsim-cli doctor prometheus`
- `poolsim-cli generate-config telemetry`
- `poolsim-cli generate-config prometheus`
- `poolsim-cli generate-config simulate`

Supported output formats:

- `table`
- `json`
- `csv`

## Example

```bash
poolsim-cli --format json simulate \
  --rps 220 \
  --p50 8 \
  --p95 32 \
  --p99 85 \
  --max-server-connections 120 \
  --connection-overhead-ms 2 \
  --min 3 \
  --max 24
```

Telemetry diff example:

```bash
poolsim-cli --format json import telemetry --config docs/fixtures/telemetry.json
```

Prometheus response-file example:

```bash
poolsim-cli --format json import prometheus \
  --response-file docs/fixtures/prometheus-responses.json \
  --current-pool-size 8 \
  --max-server-connections 100 \
  --min 2 \
  --max 20
```

Capacity gate example:

```bash
poolsim-cli --format json gate \
  --policy docs/fixtures/gate-policy.toml \
  telemetry \
  --config docs/fixtures/telemetry.json
```

Scenario comparison example:

```bash
poolsim-cli --format json compare --config docs/fixtures/scenarios.json
```

Deployment guard example:

```bash
poolsim-cli --format json guard \
  --policy docs/fixtures/gate-policy.toml \
  --max-current-rho 0.95 \
  telemetry \
  --config docs/fixtures/telemetry.json
```

Doctor example:

```bash
poolsim-cli --format json doctor telemetry --config docs/fixtures/telemetry.json
```

Config generator example:

```bash
poolsim-cli --format json generate-config \
  --framework sqlx \
  --pool-name checkout-pool \
  telemetry \
  --config docs/fixtures/telemetry.json
```

## Exit Codes

- `0`: success
- `1`: capacity-gate or deployment-guard warning policy failure
- `2`: critical outcome
- `3`: warning/advisory outcome when `--warn-exit` is enabled

## See Also

- Workspace repository: <https://github.com/gregorian-09/poolsim>
- Detailed CLI guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/cli-reference.md>

## Notes

- Use `poolsim-core` if you want to embed sizing directly into Rust code.
- Use `poolsim-web` if you want REST or WebSocket access.
