# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses Semantic Versioning before `1.0` with minor releases for additive feature work.

## [Unreleased]

### Added

- OpenTelemetry OTLP JSON ingestion for recommendation workflows through `poolsim import otlp`, `poolsim gate otlp`, `poolsim guard otlp`, `poolsim doctor otlp`, and `poolsim generate-config otlp`.
- Shared `poolsim_core::otlp` helpers so Rust users, the CLI, and the web service use one documented OTLP metric extraction implementation.
- `POST /v1/otlp/recommend` in `poolsim-web` for direct OTLP-to-recommendation requests over HTTP.
- Kubernetes sidecar integration that exposes pool recommendations as Prometheus metrics from deployment annotations.
- Kubernetes controller integration that reads deployment annotations, computes recommendations with the stable CLI contract, and patches recommendation annotations only when explicitly enabled.
- Continuous recommendation worker for repeated telemetry recommendation diffs, local state tracking, and optional webhook delivery.
- Grafana panel package for displaying `poolsim-web` sensitivity rows as a heatmap with the current pool size overlaid.
- Benchmark result contract and summarizer for comparing Poolsim predictions against controlled real-pool benchmark runs.
- Opt-in deployed-pool survey payload generator for anonymous configuration-only statistics.
- Terraform/OpenTofu external-data adapter docs for managing pool sizing as infrastructure data.
- Python, TypeScript, and Go binding docs that delegate to the stable CLI JSON contract for non-Rust teams.
- PyPI publishing automation for the Python `poolsim` package using the `PYPI_API_TOKEN` repository secret.
- Homebrew formula template with a real GitHub release tarball checksum.
- GHCR Docker image and CI integration documentation for web deployment and capacity-gate adoption.

### Changed

- Removed the completed feature roadmap document from `docs/`; completed work now lives in the concrete feature guides and release notes.
- Expanded docs indexes and crate READMEs so users can discover completed adoption, observability, validation, packaging, and Kubernetes integrations directly.
- Kept all new workflows additive: no existing Rust APIs, CLI commands, REST routes, WebSocket endpoints, serialized fields, config keys, or exit-code contracts were intentionally removed or narrowed.

### Quality

- Added tests and validation for OTLP ingestion, Kubernetes controller behavior, continuous recommendation events, Grafana package metadata, benchmark summarization, survey consent handling, and Homebrew formula metadata.
- Verified public API compatibility with `cargo semver-checks check-release --workspace`.
- Re-verified docs coverage, public API documentation coverage, examples coverage, rustdoc warnings, doctests, Clippy, formatting, and workspace tests after the feature batch.

## [0.2.1] - 2026-05-18

### Fixed

- Restored the documented `wasm32-unknown-unknown` build for `poolsim-core` with `--no-default-features` by enabling `getrandom` WebAssembly JavaScript support for wasm targets.
- Kept the release additive and API-compatible with `0.2.0`; no public APIs were removed, renamed, or semantically narrowed.

### Quality

- Re-ran the release quality gates after the patch: workspace tests, docs coverage, executable examples, workspace line coverage, examples coverage, package verification, and the remote publish workflow.

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
