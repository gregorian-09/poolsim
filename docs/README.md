# Poolsim Documentation

This folder contains user-facing documentation for the current Poolsim capabilities.

## Contents

- [`sizing-calculator.md`](sizing-calculator.md): End-to-end guide for what the sizing calculator does, required inputs, calculation pipeline, output interpretation, and usage through library/CLI/web targets.
- [`library-api.md`](library-api.md): Exhaustive `poolsim-core` reference with examples for all exported functions, constants, modules, enums, and structs.
- [`cli-reference.md`](cli-reference.md): Exhaustive command-line reference covering every subcommand, flag, config shape, output format, and exit-code path.
- [`web-api.md`](web-api.md): Exhaustive REST/WebSocket reference plus embedding examples for `build_app`, `AppState`, and `RateLimitState`.
- [`fixtures/README.md`](fixtures/README.md): Checked-in sample inputs used by the docs and docs-validation tests.

## Scope for Current Version

This documentation covers the sizing calculator implemented today in:

- `crates/poolsim-core`
- `crates/poolsim-cli`
- `crates/poolsim-web`

Future runtime-enforcement documentation is intentionally out of scope for now.
