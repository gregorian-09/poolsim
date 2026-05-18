# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses Semantic Versioning before `1.0` with minor releases for additive feature work.

## [Unreleased]

- No unreleased entries yet.

## [0.2.0] - 2026-05-18

### Added

- `poolsim-cli budget` database connection budget planner for allocating a global database `max_connections` limit across multiple services and replicas.
- Budget planner JSON and TOML input fixtures under `docs/fixtures/budget.json` and `docs/fixtures/budget.toml`.
- Budget planner JSON, table, and CSV output renderers with stable status semantics: `Pass`, `Warning`, and `Critical`.
- Budget planner exit-code behavior: critical plans return `2`; warning plans return `3` when `--warn-exit` is enabled.
- Named scenario comparison for normal, peak, and incident traffic through `poolsim-cli compare`.
- Telemetry import and recommendation diff workflows for file-based telemetry and Prometheus-compatible response payloads.
- CI gate and deployment guard workflows for rejecting unsafe traffic or latency assumptions before release.
- Pool doctor workflow for diagnosing pools that are too small, too large, close to saturation, or healthy.
- Runtime configuration snippet generator for HikariCP, Spring Boot, SQLAlchemy, Prisma, node-postgres, sqlx, and deadpool.
- Comprehensive CLI reference documentation for every command, flag, input shape, output shape, and exit-code path.
- Crate README documentation intended for crates.io and docs.rs pages.

### Changed

- Expanded validation and executable docs fixtures so CLI examples are tested as part of CI.
- Expanded public API inventory and documentation coverage checks for new CLI and config surfaces.
- Strengthened release documentation around version sync, coverage, docs coverage, examples coverage, and publish workflow behavior.

### Quality

- Workspace line coverage is enforced at `100%` by `tools/check_coverage_threshold.py`.
- `poolsim-core/src` line coverage is enforced at `100%`.
- Example file coverage is enforced at `100%` by `tools/check_examples_coverage.py`.
- Public API documentation coverage is enforced by `tools/check_docs_api_coverage.py`.
- CI runs `cargo check`, `cargo test`, docs coverage checks, executable docs fixtures, examples, tarpaulin coverage, and WASM build checks.

### Compatibility

- This release is additive and does not intentionally remove or rename any public APIs from `0.1.0`.
- The sizing calculator remains a recommender and analysis tool; it does not enforce runtime pool settings in production.

## [0.1.0] - 2026-04-01

### Added

- `poolsim-core` sizing engine with workload validation, latency fitting, Erlang-C helpers, Monte Carlo simulation, optimization, sensitivity analysis, and step-load analysis.
- `poolsim-cli` with `simulate`, `evaluate`, `sweep`, and `batch` workflows plus table, JSON, and CSV rendering.
- `poolsim-web` with REST endpoints for health, models, simulate, evaluate, sensitivity, and batch execution.
- WebSocket live-stream endpoint for progress ticks and batch streaming.
- Full public API documentation and checked-in runnable documentation fixtures.
- CI gates for docs coverage, docs fixtures, traceability, example coverage, and workspace coverage.
