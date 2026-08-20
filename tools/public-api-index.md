# Poolsim Public API Index

This is maintainer-facing inventory data for documentation coverage checks.

It is intentionally not part of the user-facing `docs/` set. The coverage checker uses it to keep public API documentation exhaustive without mixing process artifacts into user documentation.

## `poolsim_core`

### Crate Modules and Reexports

- `poolsim_core::advanced`: optional advanced sizing helpers for acquisition waits, transaction mixes, and connection leaks.
- `poolsim_core::distribution`: public distribution-fitting module; detailed usage in `docs/library-api.md`.
- `poolsim_core::erlang`: public Erlang-C helper module; detailed usage in `docs/library-api.md`.
- `poolsim_core::error`: public error module; detailed usage in `docs/library-api.md`.
- `poolsim_core::monte_carlo`: public Monte Carlo module; detailed usage in `docs/library-api.md`.
- `poolsim_core::optimizer`: public optimization module; detailed usage in `docs/library-api.md`.
- `poolsim_core::otlp`: public OpenTelemetry metric payload module; detailed usage in `docs/library-api.md`.
- `poolsim_core::sensitivity`: public sensitivity-analysis module; detailed usage in `docs/library-api.md`.
- `poolsim_core::telemetry`: public telemetry import and recommendation-diff module; detailed usage in `docs/library-api.md`.
- `poolsim_core::types`: public model/types module; detailed usage in `docs/library-api.md`.
- `poolsim_core::DistributionModel`: crate-root reexport of `poolsim_core::types::DistributionModel`.
- `poolsim_core::QueueModel`: crate-root reexport of `poolsim_core::types::QueueModel`.
- `poolsim_core::ConnectionOverheadProfile`: crate-root reexport of `poolsim_core::types::ConnectionOverheadProfile`.
- `poolsim_core::RiskLevel`: crate-root reexport of `poolsim_core::types::RiskLevel`.

### Crate-Level Constants and Functions

- `poolsim_core::MIN_FULL_SIMULATION_ITERATIONS`: iteration floor used by full simulation.
- `poolsim_core::PERFORMANCE_CONTRACT_WARNING`: warning text for the performance contract helper.
- `poolsim_core::emit_performance_contract_warning`: helper that emits `poolsim_core::PERFORMANCE_CONTRACT_WARNING` when a threshold is exceeded.
- `poolsim_core::simulate`: full sizing workflow; primary top-level API for recommendations.
- `poolsim_core::evaluate`: fixed-pool scoring workflow.
- `poolsim_core::sweep`: default sensitivity-table generator.
- `poolsim_core::sweep_with_options`: sensitivity-table generator with explicit options.


### `poolsim_core::advanced`

- `poolsim_core::advanced::AcquisitionEstimate`: acquisition wait estimate returned by `estimate_acquisition_wait`.
- `poolsim_core::advanced::AcquisitionEstimate::pool_size`: fixed pool size used by the estimate.
- `poolsim_core::advanced::AcquisitionEstimate::utilisation_rho`: estimated utilization ratio.
- `poolsim_core::advanced::AcquisitionEstimate::mean_acquisition_wait_ms`: estimated mean acquisition wait in milliseconds.
- `poolsim_core::advanced::AcquisitionEstimate::p99_acquisition_wait_ms`: estimated p99 acquisition wait in milliseconds.
- `poolsim_core::advanced::AcquisitionEstimate::acquisition_timeout_ms`: configured acquisition timeout in milliseconds.
- `poolsim_core::advanced::AcquisitionEstimate::timeout_risk`: whether the p99 acquisition wait reaches the timeout.
- `poolsim_core::advanced::estimate_acquisition_wait`: estimates pool-slot acquisition wait before database service time.
- `poolsim_core::advanced::TransactionClass`: one traffic class in a transaction-level workload mix.
- `poolsim_core::advanced::TransactionClass::new`: constructs a transaction class.
- `poolsim_core::advanced::TransactionClass::name`: returns the transaction class name.
- `poolsim_core::advanced::TransactionClass::requests_per_second`: returns the transaction class request rate.
- `poolsim_core::advanced::TransactionMix`: transaction-level workload mix.
- `poolsim_core::advanced::TransactionMix::new`: validates and constructs a transaction mix.
- `poolsim_core::advanced::TransactionMix::classes`: returns transaction classes in the mix.
- `poolsim_core::advanced::TransactionMix::aggregate_workload`: aggregates transaction classes into a `WorkloadConfig`.
- `poolsim_core::advanced::LeakSimulation`: connection leak modeling result.
- `poolsim_core::advanced::LeakSimulation::initial_pool_size`: initial pool size before leakage.
- `poolsim_core::advanced::LeakSimulation::final_available_connections`: available connections after modeled leakage.
- `poolsim_core::advanced::LeakSimulation::leaked_connections`: total leaked connections.
- `poolsim_core::advanced::LeakSimulation::minutes_to_exhaustion`: first minute where all slots are leaked.
- `poolsim_core::advanced::simulate_connection_leak`: simulates gradual connection leakage over time.

### `poolsim_core::distribution`

- `poolsim_core::distribution::EmpiricalCdf`: empirical CDF type returned through `LatencyDistribution::Empirical`.
- `poolsim_core::distribution::LatencyDistribution`: service-time distribution enum used during fitting and simulation.
- `poolsim_core::distribution::LatencyDistribution::fit`: fits a latency distribution from `WorkloadConfig` and `DistributionModel`.
- `poolsim_core::distribution::LatencyDistribution::sample_ms`: draws one latency sample in milliseconds.
- `poolsim_core::distribution::LatencyDistribution::percentile_ms`: evaluates a percentile from the fitted distribution.
- `poolsim_core::distribution::LatencyDistribution::mean_ms`: returns the fitted mean service time in milliseconds.

### `poolsim_core::erlang`

- `poolsim_core::erlang::utilisation`: computes queue utilization `rho`.
- `poolsim_core::erlang::erlang_c`: computes Erlang-C wait probability.
- `poolsim_core::erlang::mean_queue_wait_ms`: computes mean queue wait under the M/M/c model.
- `poolsim_core::erlang::queue_wait_percentile_ms`: computes a queue-wait percentile under the M/M/c model.

### `poolsim_core::error`

- `poolsim_core::error::PoolsimError`: core error enum returned by public APIs.
- `poolsim_core::error::PoolsimError::invalid_input`: standardized invalid-input constructor.
- `poolsim_core::error::PoolsimError::code`: stable machine-readable error-code accessor.
- `poolsim_core::error::PoolsimError::details`: optional structured-details accessor.

### `poolsim_core::monte_carlo`

- `poolsim_core::monte_carlo::MonteCarloResult`: raw Monte Carlo queueing output.
- `poolsim_core::monte_carlo::run`: public Monte Carlo execution entrypoint.

### `poolsim_core::optimizer`

- `poolsim_core::optimizer::OptimalResult`: optimizer output describing the chosen pool candidate.
- `poolsim_core::optimizer::find_optimal`: optimizer entrypoint for selecting the recommended pool size.

### `poolsim_core::otlp`

- `poolsim_core::otlp::DEFAULT_RPS_METRIC`: default OTLP request-rate metric name.
- `poolsim_core::otlp::DEFAULT_P50_METRIC`: default OTLP p50 latency metric name.
- `poolsim_core::otlp::DEFAULT_P95_METRIC`: default OTLP p95 latency metric name.
- `poolsim_core::otlp::DEFAULT_P99_METRIC`: default OTLP p99 latency metric name.
- `poolsim_core::otlp::OtlpMetricNames`: OTLP metric-name mapping.
- `poolsim_core::otlp::metric_value`: extracts one named numeric OTLP metric.
- `poolsim_core::otlp::workload_from_otlp_json`: converts OTLP JSON metrics into a `WorkloadConfig`.

### `poolsim_core::sensitivity`

- `poolsim_core::sensitivity::sweep`: default sensitivity sweep.
- `poolsim_core::sensitivity::sweep_with_target`: sensitivity sweep with an explicit wait target.
- `poolsim_core::sensitivity::sweep_with_target_and_model`: sensitivity sweep with explicit wait target and queue model.
- `poolsim_core::sensitivity::sweep_with_options`: full sensitivity sweep with explicit `SimulationOptions`.

### `poolsim_core::telemetry`

- `poolsim_core::telemetry::TelemetrySnapshot`: imported production telemetry for recommendation diffing.
- `poolsim_core::telemetry::TelemetrySnapshot::validate`: validates imported telemetry before recommendation.
- `poolsim_core::telemetry::PoolSizeChange`: direction enum for recommendation changes.
- `poolsim_core::telemetry::PoolRecommendationDiff`: current-vs-recommended pool-size diff.
- `poolsim_core::telemetry::PoolRecommendationDiff::worst_saturation`: returns the worst current/recommended saturation level.
- `poolsim_core::telemetry::TelemetryRecommendation`: top-level telemetry recommendation output.
- `poolsim_core::telemetry::recommend_from_telemetry`: computes a recommendation and diff from imported telemetry.

### `poolsim_core::types`

- `poolsim_core::types::DistributionModel`: distribution model enum.
- `poolsim_core::types::QueueModel`: queue model enum.
- `poolsim_core::types::ConnectionOverheadProfile`: named database/proxy connection-overhead profile enum.
- `poolsim_core::types::ConnectionOverheadProfile::connection_overhead_ms`: returns the profile overhead assumption in milliseconds.
- `poolsim_core::types::ConnectionOverheadProfile::apply_to_pool`: returns a copy of a pool with the profile overhead applied.
- `poolsim_core::types::RiskLevel`: risk classification enum.
- `poolsim_core::types::SaturationLevel`: saturation classification enum.
- `poolsim_core::types::SaturationLevel::from_rho`: maps utilization `rho` to a saturation label.
- `poolsim_core::types::WorkloadConfig`: workload input model.
- `poolsim_core::types::WorkloadConfig::validate`: validates workload inputs and ordering constraints.
- `poolsim_core::types::StepLoadPoint`: one step-load point in a burst profile.
- `poolsim_core::types::StepLoadPoint::validate`: validates a step-load point.
- `poolsim_core::types::PoolConfig`: pool sizing and server-capacity model.
- `poolsim_core::types::PoolConfig::validate`: validates pool sizing constraints.
- `poolsim_core::types::SimulationOptions`: simulation and optimization options.
- `poolsim_core::types::SimulationOptions::validate`: validates simulation options.
- `poolsim_core::types::SensitivityRow`: one sensitivity row for a candidate pool size.
- `poolsim_core::types::SimulationReport`: full simulation output.
- `poolsim_core::types::EvaluationResult`: fixed-pool evaluation output.
- `poolsim_core::types::StepLoadResult`: one row of step-load output.

## `poolsim_web`

### Crate Modules and Router Construction

- `poolsim_web::error`: HTTP/WebSocket error translation module.
- `poolsim_web::middleware`: middleware module namespace.
- `poolsim_web::routes`: route-handler module namespace.
- `poolsim_web::state`: shared application-state module.
- `poolsim_web::build_app`: Axum router builder for the documented REST and WebSocket surface.

### `poolsim_web::error`

- `poolsim_web::error::AppError`: web-layer error enum mapped into structured HTTP responses.

### `poolsim_web::middleware`

- `poolsim_web::middleware::rate_limit`: in-memory per-IP rate-limit module.
- `poolsim_web::middleware::rate_limit::RateLimitState`: rate-limit state container.
- `poolsim_web::middleware::rate_limit::RateLimitState::new`: rate-limit state constructor.
- `poolsim_web::middleware::rate_limit::enforce_rate_limit`: Axum middleware function for rate limiting.

### `poolsim_web::routes`

- `poolsim_web::routes::batch`: module for `POST /v1/batch`.
- `poolsim_web::routes::batch::handler`: handler for `POST /v1/batch`.
- `poolsim_web::routes::evaluate`: module for `POST /v1/evaluate`.
- `poolsim_web::routes::evaluate::handler`: handler for `POST /v1/evaluate`.
- `poolsim_web::routes::health`: module for `GET /v1/health`.
- `poolsim_web::routes::health::handler`: handler for `GET /v1/health`.
- `poolsim_web::routes::live`: module for `GET /v1/live`.
- `poolsim_web::routes::live::handler`: WebSocket upgrade handler for `GET /v1/live`.
- `poolsim_web::routes::models`: module containing shared HTTP and WebSocket request/response models.
- `poolsim_web::routes::models::BatchRequest`: batch endpoint request body.
- `poolsim_web::routes::models::EvaluateRequest`: fixed-pool evaluation request body.
- `poolsim_web::routes::models::HealthResponse`: health endpoint response body.
- `poolsim_web::routes::models::LiveRequest`: initial WebSocket request body.
- `poolsim_web::routes::models::LiveResponse`: WebSocket response envelope.
- `poolsim_web::routes::models::ModelsResponse`: supported-models endpoint response body.
- `poolsim_web::routes::models::SensitivityRequest`: sensitivity endpoint request body.
- `poolsim_web::routes::models::SimulationRequest`: simulation endpoint request body.
- `poolsim_web::routes::models::WebErrorBody`: stable REST error payload.
- `poolsim_web::routes::simulate`: module for `POST /v1/simulate`.
- `poolsim_web::routes::simulate::handler`: handler for `POST /v1/simulate`.
- `poolsim_web::routes::otlp`: module for `POST /v1/otlp/recommend`.
- `poolsim_web::routes::otlp::OtlpRecommendationRequest`: request body for OTLP recommendation.
- `poolsim_web::routes::otlp::handler`: handler for `POST /v1/otlp/recommend`.
- `poolsim_web::routes::sensitivity`: module for `POST /v1/sensitivity`.
- `poolsim_web::routes::sensitivity::handler`: handler for `POST /v1/sensitivity`.
- `poolsim_web::routes::telemetry`: module for `POST /v1/telemetry/recommend`.
- `poolsim_web::routes::ui`: module for `GET /` browser UI.
- `poolsim_web::routes::ui::handler`: handler for `GET /` browser UI.
- `poolsim_web::routes::telemetry::TelemetryRecommendationRequest`: request body for telemetry recommendation.
- `poolsim_web::routes::telemetry::handler`: handler for `POST /v1/telemetry/recommend`.

### `poolsim_web::state`

- `poolsim_web::state::AppState`: shared application state used by handlers and middleware.
- `poolsim_web::state::AppState::simulation_timeout`: configured per-request simulation timeout.
- `poolsim_web::state::AppState::version`: version string exposed by `GET /v1/health`.

## `poolsim_cli`

### Binary Surface

- `poolsim_cli::Cli`: top-level clap parser type for the `poolsim` binary.
- `poolsim_cli::OutputFormat`: output-format enum for `table`, `json`, `csv`, and `html`.
- `poolsim_cli::Command`: CLI subcommand enum.
- `poolsim_cli::ConfigFormat`: config-format enum for `json` and `toml`.
- `poolsim_cli::main`: binary entrypoint.

### `poolsim_cli::args`

- `poolsim_cli::args::OutputFormat`: output-format enum used by the clap parser.
- `poolsim_cli::args::CliDistributionModel`: CLI-facing distribution-model enum.
- `poolsim_cli::args::CliConnectionOverheadProfile`: CLI-facing connection-overhead profile enum.
- `poolsim_cli::args::CliQueueModel`: CLI-facing queue-model enum.
- `poolsim_cli::args::CliSaturationLevel`: CLI-facing saturation-level enum.
- `poolsim_cli::args::Cli`: top-level clap parser type defined in the `args` module.
- `poolsim_cli::args::Commands`: subcommand enum for `simulate`, `evaluate`, `sweep`, `batch`, `compare`, `budget`, `import`, `gate`, `guard`, `doctor`, `generate-config`, and `init`.
- `poolsim_cli::args::SimulateArgs`: argument model for the `simulate` subcommand.
- `poolsim_cli::args::CommonArgs`: shared argument model for simulation-style commands.
- `poolsim_cli::args::EvaluateArgs`: argument model for the `evaluate` subcommand.
- `poolsim_cli::args::BatchArgs`: argument model for the `batch` subcommand.
- `poolsim_cli::args::CompareArgs`: argument model for the `compare` subcommand.
- `poolsim_cli::args::BudgetArgs`: argument model for the `budget` subcommand.
- `poolsim_cli::args::ImportArgs`: argument model for the `import` subcommand.
- `poolsim_cli::args::ImportCommands`: nested import subcommand enum.
- `poolsim_cli::args::GateArgs`: argument model for the `gate` subcommand.
- `poolsim_cli::args::GateSourceCommands`: nested gate source enum for telemetry, Prometheus, and OTLP inputs.
- `poolsim_cli::args::GuardArgs`: argument model for the `guard` subcommand.
- `poolsim_cli::args::DoctorArgs`: argument model for the `doctor` subcommand.
- `poolsim_cli::args::DoctorSourceCommands`: nested doctor source enum for telemetry, Prometheus, and OTLP inputs.
- `poolsim_cli::args::CliConfigFramework`: CLI-facing target-framework enum for generated configuration snippets.
- `poolsim_cli::args::CliDatabaseKind`: CLI-facing database-kind enum for `poolsim init`.
- `poolsim_cli::args::GenerateConfigArgs`: argument model for the `generate-config` subcommand.
- `poolsim_cli::args::InitArgs`: argument model for the `init` subcommand.
- `poolsim_cli::args::GenerateConfigSourceCommands`: nested config-generator source enum for telemetry, Prometheus, OTLP, and simulation inputs.
- `poolsim_cli::args::TelemetryImportArgs`: argument model for `import telemetry`.
- `poolsim_cli::args::PrometheusImportArgs`: argument model for `import prometheus`.
- `poolsim_cli::args::OtlpImportArgs`: argument model for `import otlp`.

### `poolsim_cli::config`

- `poolsim_cli::config::SimulationInput`: resolved input bundle for simulation execution.
- `poolsim_cli::config::EvaluateInput`: resolved input bundle for fixed-pool evaluation execution.
- `poolsim_cli::config::SweepInput`: resolved input bundle for sensitivity sweep execution.
- `poolsim_cli::config::BatchSimulationInput`: resolved batch input bundle.
- `poolsim_cli::config::ScenarioInput`: one named scenario input for scenario comparison.
- `poolsim_cli::config::ScenarioComparisonInput`: resolved named-scenario comparison input bundle.
- `poolsim_cli::config::BudgetPlanInput`: resolved database connection budget input bundle.
- `poolsim_cli::config::BudgetServiceInput`: one service entry in a database connection budget plan.
- `poolsim_cli::config::TelemetryInput`: resolved telemetry import input bundle.
- `poolsim_cli::config::resolve_simulation_input`: builds a simulation input from config files and CLI overrides.
- `poolsim_cli::config::resolve_evaluate_input`: builds an evaluation input from config files and CLI overrides.
- `poolsim_cli::config::resolve_sweep_input`: builds a sweep input from config files and CLI overrides.
- `poolsim_cli::config::resolve_batch_input`: builds a batch input from JSON or TOML batch files.
- `poolsim_cli::config::resolve_scenario_comparison_input`: builds named scenario comparison input from JSON or TOML files.
- `poolsim_cli::config::resolve_budget_plan_input`: builds a database connection budget plan input from JSON or TOML files.
- `poolsim_cli::config::resolve_telemetry_input`: builds a telemetry input from JSON or TOML telemetry files.

### `poolsim_cli::explain`

- `poolsim_cli::explain`: internal explainable-output module for CLI prose generated by `--explain`.

### `poolsim_cli::render`

- `poolsim_cli::render::csv`: CSV rendering module.
- `poolsim_cli::render::html`: HTML rendering module.
- `poolsim_cli::render::html::print`: renders any serializable CLI payload as a self-contained HTML report.
- `poolsim_cli::render::csv::simulation`: renders a simulation result as CSV.
- `poolsim_cli::render::csv::evaluation`: renders an evaluation result as CSV.
- `poolsim_cli::render::csv::sweep`: renders a sensitivity sweep as CSV.
- `poolsim_cli::render::csv::batch`: renders a batch result as CSV.
- `poolsim_cli::render::csv::compare`: renders a scenario comparison report as CSV.
- `poolsim_cli::render::csv::budget`: renders a database connection budget report as CSV.
- `poolsim_cli::render::csv::telemetry`: renders telemetry recommendation diff as CSV.
- `poolsim_cli::render::csv::gate`: renders a capacity gate report as CSV.
- `poolsim_cli::render::csv::guard`: renders a deployment guard report as CSV.
- `poolsim_cli::render::csv::doctor`: renders a pool doctor report as CSV.
- `poolsim_cli::render::csv::config_snippet`: renders a generated framework configuration snippet report as CSV.
- `poolsim_cli::render::json`: JSON rendering module.
- `poolsim_cli::render::json::print`: renders a serializable value as JSON.
- `poolsim_cli::render::table`: table rendering module.
- `poolsim_cli::render::table::simulation`: renders a simulation result as a table.
- `poolsim_cli::render::table::evaluation`: renders an evaluation result as a table.
- `poolsim_cli::render::table::sweep`: renders a sensitivity sweep as a table.
- `poolsim_cli::render::table::batch`: renders a batch result as a table.
- `poolsim_cli::render::table::compare`: renders a scenario comparison report as a table.
- `poolsim_cli::render::table::budget`: renders a database connection budget report as a table.
- `poolsim_cli::render::table::telemetry`: renders telemetry recommendation diff as a table.
- `poolsim_cli::render::table::gate`: renders a capacity gate report as a table.
- `poolsim_cli::render::table::guard`: renders a deployment guard report as a table.
- `poolsim_cli::render::table::doctor`: renders a pool doctor report as a table.
- `poolsim_cli::render::table::config_snippet`: renders a generated framework configuration snippet report as a table.

### Route-Local Web Models

- `poolsim_web::routes::evaluate::EvaluateRequest`: request body local to the evaluation route.
- `poolsim_web::routes::health::HealthResponse`: response body local to the health route.
- `poolsim_web::routes::models::handler`: handler for `GET /v1/models`.
- `poolsim_web::routes::sensitivity::SensitivityRequest`: request body local to the sensitivity route.
- `poolsim_web::routes::simulate::SimulationRequest`: request body local to the simulation route.
- `poolsim_web::routes::otlp::OtlpRecommendationRequest`: request body local to the OTLP recommendation route.
- `poolsim_web::routes::telemetry::TelemetryRecommendationRequest`: request body local to the telemetry recommendation route.

## Generated Exact Symbol Inventory

This section is generated from the current public Rust symbol scan so documentation coverage can match exact fully qualified names.

- `poolsim_cli::args::BatchArgs`
- `poolsim_cli::args::BudgetArgs`
- `poolsim_cli::args::CheckArgs`
- `poolsim_cli::args::CheckCommands`
- `poolsim_cli::args::ClassifyArgs`
- `poolsim_cli::args::ClassifyCommands`
- `poolsim_cli::args::Cli`
- `poolsim_cli::args::CliClientLibraryKind`
- `poolsim_cli::args::CliConfigFramework`
- `poolsim_cli::args::CliConnectionOverheadProfile`
- `poolsim_cli::args::CliDatabaseKind`
- `poolsim_cli::args::CliDatabaseWorkflowKind`
- `poolsim_cli::args::CliDistributionModel`
- `poolsim_cli::args::CliEndpointConnectionKind`
- `poolsim_cli::args::CliEndpointProviderKind`
- `poolsim_cli::args::CliExternalPoolerKind`
- `poolsim_cli::args::CliMultiplexingMode`
- `poolsim_cli::args::CliQueueModel`
- `poolsim_cli::args::CliRiskLevel`
- `poolsim_cli::args::CliSaturationLevel`
- `poolsim_cli::args::CliServerlessPlatformKind`
- `poolsim_cli::args::CliSessionSemanticFeature`
- `poolsim_cli::args::Commands`
- `poolsim_cli::args::CommonArgs`
- `poolsim_cli::args::CompareArgs`
- `poolsim_cli::args::ConnectionOwnershipArgs`
- `poolsim_cli::args::DoctorArgs`
- `poolsim_cli::args::DoctorSourceCommands`
- `poolsim_cli::args::EndpointClassifyArgs`
- `poolsim_cli::args::EvaluateArgs`
- `poolsim_cli::args::GateArgs`
- `poolsim_cli::args::GateSourceCommands`
- `poolsim_cli::args::GenerateConfigArgs`
- `poolsim_cli::args::GenerateConfigSourceCommands`
- `poolsim_cli::args::GraphArgs`
- `poolsim_cli::args::GraphCommands`
- `poolsim_cli::args::GuardArgs`
- `poolsim_cli::args::ImportArgs`
- `poolsim_cli::args::ImportCommands`
- `poolsim_cli::args::InitArgs`
- `poolsim_cli::args::OtlpImportArgs`
- `poolsim_cli::args::OutputFormat`
- `poolsim_cli::args::PgbouncerPoolsImportArgs`
- `poolsim_cli::args::PlanArgs`
- `poolsim_cli::args::PlanCommands`
- `poolsim_cli::args::PoolerCheckArgs`
- `poolsim_cli::args::PoolerEvidenceImportArgs`
- `poolsim_cli::args::PrometheusImportArgs`
- `poolsim_cli::args::ServerlessPlanArgs`
- `poolsim_cli::args::SessionStateCheckArgs`
- `poolsim_cli::args::SimulateArgs`
- `poolsim_cli::args::TelemetryImportArgs`
- `poolsim_cli::config::BatchSimulationInput`
- `poolsim_cli::config::BudgetPlanInput`
- `poolsim_cli::config::BudgetServiceInput`
- `poolsim_cli::config::EvaluateInput`
- `poolsim_cli::config::ScenarioComparisonInput`
- `poolsim_cli::config::ScenarioInput`
- `poolsim_cli::config::SimulationInput`
- `poolsim_cli::config::SweepInput`
- `poolsim_cli::config::TelemetryInput`
- `poolsim_cli::config::resolve_batch_input`
- `poolsim_cli::config::resolve_budget_plan_input`
- `poolsim_cli::config::resolve_evaluate_input`
- `poolsim_cli::config::resolve_scenario_comparison_input`
- `poolsim_cli::config::resolve_simulation_input`
- `poolsim_cli::config::resolve_sweep_input`
- `poolsim_cli::config::resolve_telemetry_input`
- `poolsim_cli::render::csv`
- `poolsim_cli::render::csv::batch`
- `poolsim_cli::render::csv::budget`
- `poolsim_cli::render::csv::compare`
- `poolsim_cli::render::csv::config_snippet`
- `poolsim_cli::render::csv::doctor`
- `poolsim_cli::render::csv::evaluation`
- `poolsim_cli::render::csv::gate`
- `poolsim_cli::render::csv::guard`
- `poolsim_cli::render::csv::simulation`
- `poolsim_cli::render::csv::sweep`
- `poolsim_cli::render::csv::telemetry`
- `poolsim_cli::render::html`
- `poolsim_cli::render::html::print`
- `poolsim_cli::render::json`
- `poolsim_cli::render::json::print`
- `poolsim_cli::render::table`
- `poolsim_cli::render::table::batch`
- `poolsim_cli::render::table::budget`
- `poolsim_cli::render::table::compare`
- `poolsim_cli::render::table::config_snippet`
- `poolsim_cli::render::table::doctor`
- `poolsim_cli::render::table::evaluation`
- `poolsim_cli::render::table::gate`
- `poolsim_cli::render::table::guard`
- `poolsim_cli::render::table::simulation`
- `poolsim_cli::render::table::sweep`
- `poolsim_cli::render::table::telemetry`
- `poolsim_core::ConnectionOverheadProfile`
- `poolsim_core::DistributionModel`
- `poolsim_core::MIN_FULL_SIMULATION_ITERATIONS`
- `poolsim_core::PERFORMANCE_CONTRACT_WARNING`
- `poolsim_core::QueueModel`
- `poolsim_core::RiskLevel`
- `poolsim_core::advanced`
- `poolsim_core::advanced::AcquisitionEstimate`
- `poolsim_core::advanced::AcquisitionEstimate::acquisition_timeout_ms`
- `poolsim_core::advanced::AcquisitionEstimate::mean_acquisition_wait_ms`
- `poolsim_core::advanced::AcquisitionEstimate::p99_acquisition_wait_ms`
- `poolsim_core::advanced::AcquisitionEstimate::pool_size`
- `poolsim_core::advanced::AcquisitionEstimate::timeout_risk`
- `poolsim_core::advanced::AcquisitionEstimate::utilisation_rho`
- `poolsim_core::advanced::LeakSimulation`
- `poolsim_core::advanced::LeakSimulation::final_available_connections`
- `poolsim_core::advanced::LeakSimulation::initial_pool_size`
- `poolsim_core::advanced::LeakSimulation::leaked_connections`
- `poolsim_core::advanced::LeakSimulation::minutes_to_exhaustion`
- `poolsim_core::advanced::TransactionClass`
- `poolsim_core::advanced::TransactionClass::name`
- `poolsim_core::advanced::TransactionClass::new`
- `poolsim_core::advanced::TransactionClass::requests_per_second`
- `poolsim_core::advanced::TransactionMix`
- `poolsim_core::advanced::TransactionMix::aggregate_workload`
- `poolsim_core::advanced::TransactionMix::classes`
- `poolsim_core::advanced::TransactionMix::new`
- `poolsim_core::advanced::estimate_acquisition_wait`
- `poolsim_core::advanced::simulate_connection_leak`
- `poolsim_core::distribution`
- `poolsim_core::distribution::EmpiricalCdf`
- `poolsim_core::distribution::LatencyDistribution`
- `poolsim_core::distribution::LatencyDistribution::fit`
- `poolsim_core::distribution::LatencyDistribution::mean_ms`
- `poolsim_core::distribution::LatencyDistribution::percentile_ms`
- `poolsim_core::distribution::LatencyDistribution::sample_ms`
- `poolsim_core::emit_performance_contract_warning`
- `poolsim_core::erlang`
- `poolsim_core::erlang::erlang_c`
- `poolsim_core::erlang::mean_queue_wait_ms`
- `poolsim_core::erlang::queue_wait_percentile_ms`
- `poolsim_core::erlang::utilisation`
- `poolsim_core::error`
- `poolsim_core::error::PoolsimError`
- `poolsim_core::error::PoolsimError::code`
- `poolsim_core::error::PoolsimError::details`
- `poolsim_core::error::PoolsimError::invalid_input`
- `poolsim_core::evaluate`
- `poolsim_core::monte_carlo`
- `poolsim_core::monte_carlo::MonteCarloResult`
- `poolsim_core::monte_carlo::run`
- `poolsim_core::optimizer`
- `poolsim_core::optimizer::OptimalResult`
- `poolsim_core::optimizer::find_optimal`
- `poolsim_core::otlp`
- `poolsim_core::otlp::DEFAULT_P50_METRIC`
- `poolsim_core::otlp::DEFAULT_P95_METRIC`
- `poolsim_core::otlp::DEFAULT_P99_METRIC`
- `poolsim_core::otlp::DEFAULT_RPS_METRIC`
- `poolsim_core::otlp::OtlpMetricNames`
- `poolsim_core::otlp::metric_value`
- `poolsim_core::otlp::workload_from_otlp_json`
- `poolsim_core::ownership`
- `poolsim_core::ownership::ConnectionLayerKind`
- `poolsim_core::ownership::ConnectionOwnershipEdge`
- `poolsim_core::ownership::ConnectionOwnershipInput`
- `poolsim_core::ownership::ConnectionOwnershipInput::new`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_app_pool_size_per_runtime_unit`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_database_backend_limit`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_endpoint_kind`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_external_pooler`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_pooler_backend_limit`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_pooler_client_limit`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_runtime_units`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_service_name`
- `poolsim_core::ownership::ConnectionOwnershipInput::with_session_pinning_risk`
- `poolsim_core::ownership::ConnectionOwnershipNode`
- `poolsim_core::ownership::ConnectionOwnershipReport`
- `poolsim_core::ownership::ConnectionOwnershipStatus`
- `poolsim_core::ownership::ConnectionRelationshipKind`
- `poolsim_core::ownership::build_connection_ownership_graph`
- `poolsim_core::pooler`
- `poolsim_core::pooler::ClientCompatibilityGuidance`
- `poolsim_core::pooler::ClientLibraryKind`
- `poolsim_core::pooler::CompatibilityDecision`
- `poolsim_core::pooler::DatabaseWorkflowKind`
- `poolsim_core::pooler::EndpointClassificationInput`
- `poolsim_core::pooler::EndpointClassificationInput::new`
- `poolsim_core::pooler::EndpointClassificationInput::with_provider`
- `poolsim_core::pooler::EndpointClassificationInput::with_workflow`
- `poolsim_core::pooler::EndpointClassificationReport`
- `poolsim_core::pooler::EndpointConnectionKind`
- `poolsim_core::pooler::EndpointProviderKind`
- `poolsim_core::pooler::EvidenceConfidence`
- `poolsim_core::pooler::ExternalPoolerKind`
- `poolsim_core::pooler::MultiplexingMode`
- `poolsim_core::pooler::PgbouncerPoolRow`
- `poolsim_core::pooler::PgbouncerPoolRow::new`
- `poolsim_core::pooler::PgbouncerPoolRow::with_database`
- `poolsim_core::pooler::PgbouncerPoolRow::with_pool_mode`
- `poolsim_core::pooler::PgbouncerPoolRow::with_user`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot::new`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot::with_label`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot::with_mode`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot::with_pooler_backend_limit`
- `poolsim_core::pooler::PgbouncerShowPoolsSnapshot::with_pooler_client_limit`
- `poolsim_core::pooler::PoolerCompatibilityInput`
- `poolsim_core::pooler::PoolerCompatibilityInput::new`
- `poolsim_core::pooler::PoolerCompatibilityInput::with_features`
- `poolsim_core::pooler::PoolerCompatibilityInput::with_pooler_config`
- `poolsim_core::pooler::PoolerCompatibilityInput::with_workflow`
- `poolsim_core::pooler::PoolerCompatibilityReport`
- `poolsim_core::pooler::PoolerConfigSnapshot`
- `poolsim_core::pooler::PoolerConfigSnapshot::new`
- `poolsim_core::pooler::PoolerConfigSnapshot::with_max_prepared_statements`
- `poolsim_core::pooler::PoolerConfigSnapshot::with_resets_session_state`
- `poolsim_core::pooler::PoolerEvidenceReport`
- `poolsim_core::pooler::PoolerEvidenceSnapshot`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::new`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_client_active`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_client_waiting`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_label`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_pooler_backend_limit`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_pooler_client_limit`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_server_active`
- `poolsim_core::pooler::PoolerEvidenceSnapshot::with_server_idle`
- `poolsim_core::pooler::PoolerEvidenceStatus`
- `poolsim_core::pooler::PoolerFinding`
- `poolsim_core::pooler::SessionSemanticFeature`
- `poolsim_core::pooler::SessionStateCompatibilityInput`
- `poolsim_core::pooler::SessionStateCompatibilityInput::new`
- `poolsim_core::pooler::SessionStateCompatibilityInput::with_features`
- `poolsim_core::pooler::SessionStateCompatibilityInput::with_pooler_config`
- `poolsim_core::pooler::SessionStateCompatibilityInput::with_workflow`
- `poolsim_core::pooler::SessionStateCompatibilityReport`
- `poolsim_core::pooler::analyze_session_state_compatibility`
- `poolsim_core::pooler::check_pooler_compatibility`
- `poolsim_core::pooler::classify_endpoint`
- `poolsim_core::pooler::parse_pgbouncer_show_pools`
- `poolsim_core::pooler::redact_endpoint`
- `poolsim_core::pooler::summarize_pgbouncer_show_pools`
- `poolsim_core::pooler::summarize_pooler_evidence`
- `poolsim_core::sensitivity`
- `poolsim_core::sensitivity::sweep`
- `poolsim_core::sensitivity::sweep_with_options`
- `poolsim_core::sensitivity::sweep_with_target`
- `poolsim_core::sensitivity::sweep_with_target_and_model`
- `poolsim_core::serverless`
- `poolsim_core::serverless::ServerlessConcurrencyInput`
- `poolsim_core::serverless::ServerlessConcurrencyInput::new`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_app_pool_size_per_environment`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_database_backend_limit`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_external_pooler`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_max_concurrent_invocations`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_reserved_concurrency`
- `poolsim_core::serverless::ServerlessConcurrencyInput::with_warm_reuse_ratio`
- `poolsim_core::serverless::ServerlessConcurrencyReport`
- `poolsim_core::serverless::ServerlessConcurrencyStatus`
- `poolsim_core::serverless::ServerlessPlatformKind`
- `poolsim_core::serverless::plan_serverless_concurrency`
- `poolsim_core::simulate`
- `poolsim_core::sweep`
- `poolsim_core::sweep_with_options`
- `poolsim_core::telemetry`
- `poolsim_core::telemetry::PoolRecommendationDiff`
- `poolsim_core::telemetry::PoolRecommendationDiff::worst_saturation`
- `poolsim_core::telemetry::PoolSizeChange`
- `poolsim_core::telemetry::TelemetryRecommendation`
- `poolsim_core::telemetry::TelemetrySnapshot`
- `poolsim_core::telemetry::TelemetrySnapshot::validate`
- `poolsim_core::telemetry::recommend_from_telemetry`
- `poolsim_core::types`
- `poolsim_core::types::ConnectionOverheadProfile`
- `poolsim_core::types::ConnectionOverheadProfile::apply_to_pool`
- `poolsim_core::types::ConnectionOverheadProfile::connection_overhead_ms`
- `poolsim_core::types::DistributionModel`
- `poolsim_core::types::EvaluationResult`
- `poolsim_core::types::PoolConfig`
- `poolsim_core::types::PoolConfig::validate`
- `poolsim_core::types::QueueModel`
- `poolsim_core::types::RiskLevel`
- `poolsim_core::types::SaturationLevel`
- `poolsim_core::types::SaturationLevel::from_rho`
- `poolsim_core::types::SensitivityRow`
- `poolsim_core::types::SimulationOptions`
- `poolsim_core::types::SimulationOptions::validate`
- `poolsim_core::types::SimulationReport`
- `poolsim_core::types::StepLoadPoint`
- `poolsim_core::types::StepLoadPoint::validate`
- `poolsim_core::types::StepLoadResult`
- `poolsim_core::types::WorkloadConfig`
- `poolsim_core::types::WorkloadConfig::validate`
- `poolsim_web::build_app`
- `poolsim_web::error`
- `poolsim_web::error::AppError`
- `poolsim_web::middleware`
- `poolsim_web::middleware::rate_limit`
- `poolsim_web::middleware::rate_limit::RateLimitState`
- `poolsim_web::middleware::rate_limit::RateLimitState::new`
- `poolsim_web::middleware::rate_limit::enforce_rate_limit`
- `poolsim_web::routes`
- `poolsim_web::routes::batch`
- `poolsim_web::routes::batch::handler`
- `poolsim_web::routes::check`
- `poolsim_web::routes::check::pooler_handler`
- `poolsim_web::routes::check::session_state_handler`
- `poolsim_web::routes::classify`
- `poolsim_web::routes::classify::endpoint_handler`
- `poolsim_web::routes::evaluate`
- `poolsim_web::routes::evaluate::EvaluateRequest`
- `poolsim_web::routes::evaluate::handler`
- `poolsim_web::routes::graph`
- `poolsim_web::routes::graph::ownership_handler`
- `poolsim_web::routes::health`
- `poolsim_web::routes::health::HealthResponse`
- `poolsim_web::routes::health::handler`
- `poolsim_web::routes::live`
- `poolsim_web::routes::live::handler`
- `poolsim_web::routes::models`
- `poolsim_web::routes::models::ModelsResponse`
- `poolsim_web::routes::models::handler`
- `poolsim_web::routes::otlp`
- `poolsim_web::routes::otlp::OtlpRecommendationRequest`
- `poolsim_web::routes::otlp::handler`
- `poolsim_web::routes::plan`
- `poolsim_web::routes::plan::serverless_handler`
- `poolsim_web::routes::sensitivity`
- `poolsim_web::routes::sensitivity::SensitivityRequest`
- `poolsim_web::routes::sensitivity::handler`
- `poolsim_web::routes::simulate`
- `poolsim_web::routes::simulate::SimulationRequest`
- `poolsim_web::routes::simulate::handler`
- `poolsim_web::routes::telemetry`
- `poolsim_web::routes::telemetry::TelemetryRecommendationRequest`
- `poolsim_web::routes::telemetry::handler`
- `poolsim_web::routes::ui`
- `poolsim_web::routes::ui::handler`
- `poolsim_web::state`
- `poolsim_web::state::AppState`
