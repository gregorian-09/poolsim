# poolsim-cli

`poolsim-cli` is the command-line interface for the `poolsim` sizing calculator.

It is intended for operators, developers, and CI workflows that want pool-sizing output without embedding the library directly.

## Commands

It supports:

- full simulation
- fixed-size evaluation
- sensitivity sweeps
- batch execution
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

## Exit Codes

- `0`: success
- `2`: critical outcome
- `3`: warning/advisory outcome when `--warn-exit` is enabled

## See Also

- Workspace repository: <https://github.com/gregorian-09/poolsim>
- Detailed CLI guide: <https://github.com/gregorian-09/poolsim/blob/main/docs/cli-reference.md>

## Notes

- Use `poolsim-core` if you want to embed sizing directly into Rust code.
- Use `poolsim-web` if you want REST or WebSocket access.
