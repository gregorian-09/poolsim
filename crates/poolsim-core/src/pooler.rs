//! Endpoint classification and external-pooler compatibility checks.
//!
//! This module is intentionally conservative. It helps callers distinguish
//! direct database connections from pooled/proxied endpoints, then checks
//! whether application session features are compatible with the selected
//! pooling mode. Unknown provider behavior lowers confidence instead of being
//! treated as safe.

use serde::{Deserialize, Serialize};

use crate::{error::PoolsimError, types::RiskLevel};

/// Confidence assigned to topology and compatibility evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum EvidenceConfidence {
    /// The evidence is source-backed and enough for an actionable decision.
    High,
    /// The evidence is useful, but one or more assumptions should be reviewed.
    Medium,
    /// The evidence is incomplete or inferred from weak endpoint patterns.
    Low,
}

/// Provider hint used by endpoint classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum EndpointProviderKind {
    /// Supabase Postgres endpoints and pooler hostnames.
    Supabase,
    /// Neon Postgres direct and pooled endpoints.
    Neon,
    /// Prisma Postgres or Prisma Accelerate-style endpoint.
    PrismaPostgres,
    /// Amazon RDS direct database endpoint.
    AwsRds,
    /// Amazon RDS Proxy endpoint.
    AwsRdsProxy,
    /// Cloudflare Hyperdrive binding or endpoint.
    CloudflareHyperdrive,
    /// PgBouncer or PgBouncer-compatible endpoint.
    PgBouncer,
    /// Provider is not known to poolsim.
    Unknown,
}

/// Operational class of a connection endpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum EndpointConnectionKind {
    /// Direct connection to a database backend endpoint.
    DirectDatabase,
    /// Session-pooling endpoint where a server connection is held for a session.
    SessionPooler,
    /// Transaction-pooling endpoint where server connections are reused between transactions.
    TransactionPooler,
    /// Statement-pooling endpoint where server connections may change between statements.
    StatementPooler,
    /// Managed database proxy endpoint.
    DatabaseProxy,
    /// Edge/provider-managed pooler endpoint.
    EdgePooler,
    /// HTTP or Data API style endpoint rather than a normal database socket.
    HttpDataApi,
    /// Endpoint could not be classified safely.
    Unknown,
}

/// Database workflow that will use the endpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DatabaseWorkflowKind {
    /// User-facing application/API traffic.
    ApiTraffic,
    /// Background worker or queue consumer traffic.
    BackgroundWorker,
    /// Serverless function traffic.
    ServerlessFunction,
    /// Edge function or edge-worker traffic.
    EdgeFunction,
    /// Schema migration workflow.
    Migration,
    /// Backup or restore workflow.
    BackupRestore,
    /// Database GUI, shell, or interactive admin workflow.
    DatabaseGui,
    /// Replication or change-data-capture workflow.
    Replication,
    /// Long-running analytics or reporting workflow.
    LongRunningAnalytics,
    /// General administrative task.
    AdminTask,
    /// Workflow is not known to poolsim.
    Unknown,
}

/// External pooler or proxy family.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ExternalPoolerKind {
    /// PgBouncer.
    PgBouncer,
    /// Amazon RDS Proxy.
    RdsProxy,
    /// Supabase Supavisor/PgBouncer-compatible pooler.
    Supavisor,
    /// Prisma Postgres pooled endpoint.
    PrismaPostgresPooler,
    /// Neon pooled endpoint.
    NeonPooler,
    /// Cloudflare Hyperdrive.
    CloudflareHyperdrive,
    /// Pooler is not known to poolsim.
    Unknown,
}

/// Multiplexing mode used by a pooler or proxy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MultiplexingMode {
    /// No multiplexing is expected.
    None,
    /// Server connections are reused per client session.
    Session,
    /// Server connections are reused after each transaction.
    Transaction,
    /// Server connections are reused after each statement.
    Statement,
    /// Provider controls multiplexing details.
    ProviderManaged,
    /// Multiplexing mode is unknown.
    Unknown,
}

/// Session-level database feature that can affect pooler compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SessionSemanticFeature {
    /// SQL `SET` or equivalent session-setting behavior.
    SetStatement,
    /// Persistent `LISTEN`/notification listener behavior.
    ListenNotifyListener,
    /// Fire-and-forget `NOTIFY` behavior.
    NotifyOnly,
    /// Generic prepared-statement usage.
    PreparedStatements,
    /// Protocol-level prepared statements.
    ProtocolPreparedStatements,
    /// SQL `PREPARE`/named prepared statement usage.
    NamedPreparedStatements,
    /// Temporary table usage.
    TemporaryTables,
    /// Holdable cursors.
    HoldCursors,
    /// Session-level advisory locks.
    AdvisoryLocks,
    /// Session variables or state that must persist across transactions.
    SessionVariables,
    /// Schema migration behavior.
    Migrations,
    /// Long-running query or long transaction behavior.
    LongRunningQuery,
    /// Interactive transaction behavior.
    InteractiveTransaction,
    /// Copy protocol or bulk load behavior.
    CopyProtocol,
    /// Feature is not known to poolsim.
    Unknown,
}

/// Client library or framework whose database behavior influences pooler safety.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ClientLibraryKind {
    /// Generic PostgreSQL client with no framework-specific assumptions.
    GenericPostgres,
    /// Prisma Client or Prisma ORM runtime traffic.
    Prisma,
    /// node-postgres / `pg`.
    NodePostgres,
    /// Rust `sqlx`.
    Sqlx,
    /// SQLAlchemy using the asyncpg PostgreSQL dialect.
    SqlalchemyAsyncpg,
    /// PostgREST.
    Postgrest,
    /// PostgreSQL JDBC driver.
    PgJdbc,
    /// Client library is not known to poolsim.
    Unknown,
}

/// Compatibility decision for a pooler/workflow combination.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CompatibilityDecision {
    /// The combination appears compatible from the supplied evidence.
    Compatible,
    /// The combination is incompatible and should be changed before use.
    Incompatible,
    /// The combination may be safe, but missing evidence prevents a firm decision.
    NeedsReview,
}

/// Status of observed external pooler evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PoolerEvidenceStatus {
    /// Observed evidence does not show client waiting or backend saturation.
    Healthy,
    /// Clients are waiting for backend/server capacity.
    ClientWaiting,
    /// Backend/server usage is at or above the supplied backend limit.
    BackendSaturated,
    /// Evidence is incomplete or provider behavior needs review.
    NeedsReview,
}

/// Diagnosis status for an application pool and its downstream pooler.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DownstreamPoolerDiagnosisStatus {
    /// Both layers have healthy observed headroom.
    Healthy,
    /// The downstream pooler has queued clients waiting for backend capacity.
    DownstreamPoolerWaiting,
    /// The downstream pooler's observed backend connections reached its limit.
    DownstreamPoolerSaturated,
    /// The application pool reached its configured connection limit.
    ApplicationPoolSaturated,
    /// Evidence is incomplete or one or more layers are close to a limit.
    NeedsReview,
}

/// A compatibility or endpoint-classification finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolerFinding {
    /// Stable machine-readable finding code.
    pub code: String,
    /// Severity/risk associated with this finding.
    pub risk: RiskLevel,
    /// Human-readable explanation of the issue.
    pub message: String,
    /// Recommended remediation or next verification step.
    pub remediation: String,
}

impl PoolerFinding {
    fn new(
        code: impl Into<String>,
        risk: RiskLevel,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            risk,
            message: message.into(),
            remediation: remediation.into(),
        }
    }
}

/// Input for endpoint classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct EndpointClassificationInput {
    /// Connection string, hostname, DSN, binding name, or provider endpoint to classify.
    pub endpoint: String,
    /// Optional provider hint used to disambiguate similar hostnames.
    #[serde(default)]
    pub provider: Option<EndpointProviderKind>,
    /// Optional workflow hint used to check endpoint/workflow compatibility.
    #[serde(default)]
    pub workflow: Option<DatabaseWorkflowKind>,
}

impl EndpointClassificationInput {
    /// Creates endpoint-classification input from an endpoint string.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            provider: None,
            workflow: None,
        }
    }

    /// Sets the provider hint.
    #[must_use]
    pub fn with_provider(mut self, provider: EndpointProviderKind) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Sets the workflow hint.
    #[must_use]
    pub fn with_workflow(mut self, workflow: DatabaseWorkflowKind) -> Self {
        self.workflow = Some(workflow);
        self
    }
}

/// Result of endpoint classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct EndpointClassificationReport {
    /// Classified endpoint kind.
    pub endpoint_kind: EndpointConnectionKind,
    /// Provider inferred or supplied for the endpoint.
    pub provider: EndpointProviderKind,
    /// Whether the endpoint appears compatible with the supplied workflow.
    #[serde(default)]
    pub workflow_compatible: Option<bool>,
    /// Redacted endpoint safe for logs, reports, and CI output.
    pub redacted_endpoint: String,
    /// Findings explaining the classification and any risks.
    pub findings: Vec<PoolerFinding>,
    /// Evidence confidence for the classification.
    pub confidence: EvidenceConfidence,
}

/// Pooler configuration evidence used by compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PoolerConfigSnapshot {
    /// PgBouncer `max_prepared_statements`, when known.
    #[serde(default)]
    pub max_prepared_statements: Option<u32>,
    /// Whether provider documentation or live config confirms session-state reset behavior.
    #[serde(default)]
    pub resets_session_state: Option<bool>,
}

impl PoolerConfigSnapshot {
    /// Creates an empty pooler configuration snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets PgBouncer `max_prepared_statements` evidence.
    #[must_use]
    pub fn with_max_prepared_statements(mut self, value: u32) -> Self {
        self.max_prepared_statements = Some(value);
        self
    }

    /// Sets whether evidence confirms session-state reset behavior.
    #[must_use]
    pub fn with_resets_session_state(mut self, value: bool) -> Self {
        self.resets_session_state = Some(value);
        self
    }
}

/// Input for pooler compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolerCompatibilityInput {
    /// Pooler family to check.
    pub pooler: ExternalPoolerKind,
    /// Multiplexing mode used by the pooler.
    pub mode: MultiplexingMode,
    /// Application/database features used by this workload.
    #[serde(default)]
    pub features_used: Vec<SessionSemanticFeature>,
    /// Optional workflow that will run through the pooler.
    #[serde(default)]
    pub workflow: Option<DatabaseWorkflowKind>,
    /// Optional pooler configuration evidence.
    #[serde(default)]
    pub pooler_config: Option<PoolerConfigSnapshot>,
}

impl PoolerCompatibilityInput {
    /// Creates pooler compatibility input.
    pub fn new(pooler: ExternalPoolerKind, mode: MultiplexingMode) -> Self {
        Self {
            pooler,
            mode,
            features_used: Vec::new(),
            workflow: None,
            pooler_config: None,
        }
    }

    /// Sets the session features used by the workload.
    #[must_use]
    pub fn with_features(mut self, features: Vec<SessionSemanticFeature>) -> Self {
        self.features_used = features;
        self
    }

    /// Sets the workflow.
    #[must_use]
    pub fn with_workflow(mut self, workflow: DatabaseWorkflowKind) -> Self {
        self.workflow = Some(workflow);
        self
    }

    /// Sets pooler configuration evidence.
    #[must_use]
    pub fn with_pooler_config(mut self, config: PoolerConfigSnapshot) -> Self {
        self.pooler_config = Some(config);
        self
    }
}

/// Result of a pooler compatibility check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolerCompatibilityReport {
    /// Overall compatibility decision.
    pub compatible: CompatibilityDecision,
    /// Features that are incompatible or unsafe for the supplied pooler mode.
    pub incompatible_features: Vec<SessionSemanticFeature>,
    /// Whether migration/admin-style direct connections are recommended.
    pub migration_direct_connection_required: bool,
    /// Whether long-running work should avoid this pooled endpoint.
    pub long_running_direct_connection_required: bool,
    /// Findings explaining the decision.
    pub findings: Vec<PoolerFinding>,
    /// Evidence confidence for the decision.
    pub confidence: EvidenceConfidence,
}

/// Observed pooler evidence from admin output or provider metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PoolerEvidenceSnapshot {
    /// Pooler family that produced the evidence.
    pub pooler: ExternalPoolerKind,
    /// Multiplexing mode in effect for this evidence.
    pub mode: MultiplexingMode,
    /// Optional service, database, user, or pool label.
    #[serde(default)]
    pub label: Option<String>,
    /// Active/connected client-side connections.
    #[serde(default)]
    pub client_active: Option<u32>,
    /// Client-side connections waiting for a backend/server connection.
    #[serde(default)]
    pub client_waiting: Option<u32>,
    /// Active backend/server connections opened by the pooler.
    #[serde(default)]
    pub server_active: Option<u32>,
    /// Idle backend/server connections held by the pooler.
    #[serde(default)]
    pub server_idle: Option<u32>,
    /// Configured or provider-documented client connection cap.
    #[serde(default)]
    pub pooler_client_limit: Option<u32>,
    /// Configured or provider-documented backend/server connection cap.
    #[serde(default)]
    pub pooler_backend_limit: Option<u32>,
}

impl PoolerEvidenceSnapshot {
    /// Creates pooler evidence with the required pooler family and mode.
    pub fn new(pooler: ExternalPoolerKind, mode: MultiplexingMode) -> Self {
        Self {
            pooler,
            mode,
            label: None,
            client_active: None,
            client_waiting: None,
            server_active: None,
            server_idle: None,
            pooler_client_limit: None,
            pooler_backend_limit: None,
        }
    }

    /// Sets a report label.
    #[must_use]
    pub fn with_label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets observed active client-side connections.
    #[must_use]
    pub fn with_client_active(mut self, value: u32) -> Self {
        self.client_active = Some(value);
        self
    }

    /// Sets observed waiting client-side connections.
    #[must_use]
    pub fn with_client_waiting(mut self, value: u32) -> Self {
        self.client_waiting = Some(value);
        self
    }

    /// Sets observed active backend/server connections.
    #[must_use]
    pub fn with_server_active(mut self, value: u32) -> Self {
        self.server_active = Some(value);
        self
    }

    /// Sets observed idle backend/server connections.
    #[must_use]
    pub fn with_server_idle(mut self, value: u32) -> Self {
        self.server_idle = Some(value);
        self
    }

    /// Sets the pooler client connection limit.
    #[must_use]
    pub fn with_pooler_client_limit(mut self, value: u32) -> Self {
        self.pooler_client_limit = Some(value);
        self
    }

    /// Sets the pooler backend/server connection limit.
    #[must_use]
    pub fn with_pooler_backend_limit(mut self, value: u32) -> Self {
        self.pooler_backend_limit = Some(value);
        self
    }
}

/// One row parsed from PgBouncer `SHOW POOLS` output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PgbouncerPoolRow {
    /// PgBouncer database name for this `(database, user)` pool, when present.
    #[serde(default)]
    pub database: Option<String>,
    /// PgBouncer user name for this `(database, user)` pool, when present.
    #[serde(default)]
    pub user: Option<String>,
    /// PgBouncer `cl_active` count.
    pub cl_active: u32,
    /// PgBouncer `cl_waiting` count.
    pub cl_waiting: u32,
    /// PgBouncer `sv_active` count.
    pub sv_active: u32,
    /// PgBouncer `sv_idle` count.
    pub sv_idle: u32,
    /// PgBouncer `pool_mode` value for this row, when present.
    #[serde(default)]
    pub pool_mode: Option<MultiplexingMode>,
}

impl PgbouncerPoolRow {
    /// Creates a PgBouncer `SHOW POOLS` row from the capacity counters poolsim needs.
    pub fn new(cl_active: u32, cl_waiting: u32, sv_active: u32, sv_idle: u32) -> Self {
        Self {
            database: None,
            user: None,
            cl_active,
            cl_waiting,
            sv_active,
            sv_idle,
            pool_mode: None,
        }
    }

    /// Sets the PgBouncer database label for this row.
    #[must_use]
    pub fn with_database(mut self, value: impl Into<String>) -> Self {
        self.database = Some(value.into());
        self
    }

    /// Sets the PgBouncer user label for this row.
    #[must_use]
    pub fn with_user(mut self, value: impl Into<String>) -> Self {
        self.user = Some(value.into());
        self
    }

    /// Sets the PgBouncer pool mode for this row.
    #[must_use]
    pub fn with_pool_mode(mut self, value: MultiplexingMode) -> Self {
        self.pool_mode = Some(value);
        self
    }
}

/// Parsed PgBouncer `SHOW POOLS` snapshot plus optional capacity limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PgbouncerShowPoolsSnapshot {
    /// Parsed `SHOW POOLS` rows.
    pub rows: Vec<PgbouncerPoolRow>,
    /// Optional report label such as service, cluster, environment, or capture name.
    #[serde(default)]
    pub label: Option<String>,
    /// Override for the PgBouncer pooling mode when the capture does not include `pool_mode`.
    #[serde(default)]
    pub mode: Option<MultiplexingMode>,
    /// Configured PgBouncer client connection cap, when known.
    #[serde(default)]
    pub pooler_client_limit: Option<u32>,
    /// Configured PgBouncer backend/server pool cap, when known.
    #[serde(default)]
    pub pooler_backend_limit: Option<u32>,
}

impl PgbouncerShowPoolsSnapshot {
    /// Creates a PgBouncer `SHOW POOLS` snapshot from parsed rows.
    pub fn new(rows: Vec<PgbouncerPoolRow>) -> Self {
        Self {
            rows,
            label: None,
            mode: None,
            pooler_client_limit: None,
            pooler_backend_limit: None,
        }
    }

    /// Sets a report label.
    #[must_use]
    pub fn with_label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets or overrides the pooling mode for the summarized evidence.
    #[must_use]
    pub fn with_mode(mut self, value: MultiplexingMode) -> Self {
        self.mode = Some(value);
        self
    }

    /// Sets the PgBouncer client connection limit.
    #[must_use]
    pub fn with_pooler_client_limit(mut self, value: u32) -> Self {
        self.pooler_client_limit = Some(value);
        self
    }

    /// Sets the PgBouncer backend/server connection limit.
    #[must_use]
    pub fn with_pooler_backend_limit(mut self, value: u32) -> Self {
        self.pooler_backend_limit = Some(value);
        self
    }
}

/// One row parsed from PgBouncer `SHOW STATS` output.
///
/// PgBouncer reports the query and wait counters as cumulative values since
/// the process started or since the last reset. Callers should compare two
/// snapshots with [`diff_pgbouncer_time_series`] rather than interpreting a
/// single row as a rate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PgbouncerStatsRow {
    /// PgBouncer database name for this statistics row, when present.
    #[serde(default)]
    pub database: Option<String>,
    /// Cumulative SQL command count from `total_query_count`.
    pub total_query_count: u64,
    /// Cumulative client wait time in microseconds from `total_wait_time`.
    pub total_wait_time_us: u64,
    /// PgBouncer's reported average client wait time in microseconds, when present.
    #[serde(default)]
    pub avg_wait_time_us: Option<f64>,
}

impl PgbouncerStatsRow {
    /// Creates a statistics row from cumulative query and wait counters.
    pub fn new(total_query_count: u64, total_wait_time_us: u64) -> Self {
        Self {
            database: None,
            total_query_count,
            total_wait_time_us,
            avg_wait_time_us: None,
        }
    }

    /// Sets the PgBouncer database label for this row.
    #[must_use]
    pub fn with_database(mut self, value: impl Into<String>) -> Self {
        self.database = Some(value.into());
        self
    }

    /// Sets PgBouncer's reported average wait time in microseconds.
    #[must_use]
    pub fn with_avg_wait_time_us(mut self, value: f64) -> Self {
        self.avg_wait_time_us = Some(value);
        self
    }
}

/// A timestamped PgBouncer time-series capture.
///
/// The rows normally come from [`parse_pgbouncer_show_stats`]. The optional
/// `maxwait_seconds` and `client_waiting` values can come from the matching
/// `SHOW POOLS` capture or exporter gauges taken at the same timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PgbouncerShowStatsSnapshot {
    /// Capture timestamp in Unix seconds.
    pub timestamp_seconds: f64,
    /// Parsed `SHOW STATS` rows.
    pub rows: Vec<PgbouncerStatsRow>,
    /// Optional report label such as service, cluster, environment, or capture name.
    #[serde(default)]
    pub label: Option<String>,
    /// Age of the oldest waiting client in seconds from `SHOW POOLS.maxwait`.
    #[serde(default)]
    pub maxwait_seconds: Option<f64>,
    /// Number of clients waiting for a server connection from `SHOW POOLS.cl_waiting`.
    #[serde(default)]
    pub client_waiting: Option<u32>,
}

impl PgbouncerShowStatsSnapshot {
    /// Creates a timestamped `SHOW STATS` snapshot.
    pub fn new(timestamp_seconds: f64, rows: Vec<PgbouncerStatsRow>) -> Self {
        Self {
            timestamp_seconds,
            rows,
            label: None,
            maxwait_seconds: None,
            client_waiting: None,
        }
    }

    /// Sets a report label.
    #[must_use]
    pub fn with_label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets the matching `SHOW POOLS.maxwait` gauge in seconds.
    #[must_use]
    pub fn with_maxwait_seconds(mut self, value: f64) -> Self {
        self.maxwait_seconds = Some(value);
        self
    }

    /// Sets the matching `SHOW POOLS.cl_waiting` gauge.
    #[must_use]
    pub fn with_client_waiting(mut self, value: u32) -> Self {
        self.client_waiting = Some(value);
        self
    }
}

/// Aggregated cumulative PgBouncer counters at one timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PgbouncerTimeSeriesSample {
    /// Capture timestamp in Unix seconds.
    pub timestamp_seconds: f64,
    /// Optional report label copied from the source snapshot.
    #[serde(default)]
    pub label: Option<String>,
    /// Aggregated cumulative query count across all statistics rows.
    pub total_query_count: u64,
    /// Aggregated cumulative client wait time in microseconds.
    pub total_wait_time_us: u64,
    /// Age of the oldest waiting client in seconds, when supplied.
    #[serde(default)]
    pub maxwait_seconds: Option<f64>,
    /// Number of clients waiting for a server connection, when supplied.
    #[serde(default)]
    pub client_waiting: Option<u32>,
}

/// Cumulative counter that decreased between two captures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PgbouncerCounterKind {
    /// `total_query_count` decreased, indicating a reset or source change.
    TotalQueryCount,
    /// `total_wait_time` decreased, indicating a reset or source change.
    TotalWaitTime,
}

/// Operational interpretation of a PgBouncer time-series delta.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PgbouncerTimeSeriesStatus {
    /// No queue was observed and the counters were continuous.
    Healthy,
    /// A queue is present at the current capture, but it did not grow during the interval.
    QueuePresent,
    /// `maxwait` or the waiting-client gauge increased during the interval.
    QueueGrowing,
    /// A cumulative counter reset or source replacement was detected.
    CounterReset,
    /// Evidence is incomplete and should not be used for an automatic decision.
    NeedsReview,
}

/// Delta report for two timestamped PgBouncer captures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PgbouncerTimeSeriesDeltaReport {
    /// Previous capture timestamp in Unix seconds.
    pub previous_timestamp_seconds: f64,
    /// Current capture timestamp in Unix seconds.
    pub current_timestamp_seconds: f64,
    /// Positive interval between captures in seconds.
    pub interval_seconds: f64,
    /// Query throughput derived from the cumulative query counter.
    pub query_rate_per_second: f64,
    /// Client wait time accumulated per second, in microseconds per second.
    pub wait_time_rate_us_per_second: f64,
    /// Mean client wait per newly completed query, in milliseconds.
    #[serde(default)]
    pub average_wait_ms_per_query: Option<f64>,
    /// Current oldest-client wait age in seconds, when supplied.
    #[serde(default)]
    pub current_maxwait_seconds: Option<f64>,
    /// Change in oldest-client wait age in seconds, when both captures supplied it.
    #[serde(default)]
    pub maxwait_delta_seconds: Option<f64>,
    /// Current waiting-client count, when supplied.
    #[serde(default)]
    pub current_client_waiting: Option<u32>,
    /// Cumulative counters that reset during the interval.
    pub counter_resets: Vec<PgbouncerCounterKind>,
    /// Overall time-series interpretation.
    pub status: PgbouncerTimeSeriesStatus,
    /// Findings explaining queue pressure, resets, or missing evidence.
    pub findings: Vec<PoolerFinding>,
    /// Confidence in this time-series interpretation.
    pub confidence: EvidenceConfidence,
}

/// Summary of observed pooler client/backend evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct PoolerEvidenceReport {
    /// Overall observed evidence status.
    pub status: PoolerEvidenceStatus,
    /// Pooler family that produced the evidence.
    pub pooler: ExternalPoolerKind,
    /// Multiplexing mode in effect for this evidence.
    pub mode: MultiplexingMode,
    /// Optional service, database, user, or pool label.
    pub label: Option<String>,
    /// Total observed client connections, active plus waiting, when known.
    pub observed_client_connections: Option<u32>,
    /// Total observed backend/server connections, active plus idle, when known.
    pub observed_backend_connections: Option<u32>,
    /// Observed client waiting count.
    pub client_waiting: Option<u32>,
    /// Backend utilization ratio against the supplied backend limit, when known.
    pub backend_utilization: Option<f64>,
    /// Client utilization ratio against the supplied client limit, when known.
    pub client_utilization: Option<f64>,
    /// Findings that explain risks, missing evidence, and remediation.
    pub findings: Vec<PoolerFinding>,
    /// Confidence in this evidence summary.
    pub confidence: EvidenceConfidence,
}

/// Observed application-pool counters used to diagnose downstream capacity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ApplicationPoolEvidence {
    /// Application connections currently in use.
    #[serde(default)]
    pub active: Option<u32>,
    /// Application requests waiting for a connection, when the client exposes it.
    #[serde(default)]
    pub waiting: Option<u32>,
    /// Configured maximum application connections per runtime unit or service.
    #[serde(default)]
    pub limit: Option<u32>,
}

impl ApplicationPoolEvidence {
    /// Creates application-pool evidence with active connections and a limit.
    pub fn new(active: u32, limit: u32) -> Self {
        Self {
            active: Some(active),
            waiting: None,
            limit: Some(limit),
        }
    }

    /// Sets the number of application requests waiting for a connection.
    #[must_use]
    pub fn with_waiting(mut self, value: u32) -> Self {
        self.waiting = Some(value);
        self
    }

    /// Sets or clears the observed active application connection count.
    #[must_use]
    pub fn with_active(mut self, value: Option<u32>) -> Self {
        self.active = value;
        self
    }

    /// Sets or clears the configured application connection limit.
    #[must_use]
    pub fn with_limit(mut self, value: Option<u32>) -> Self {
        self.limit = value;
        self
    }
}

/// Inputs for comparing application-pool pressure with downstream pooler evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DownstreamPoolerDiagnosisInput {
    /// Application-side pool counters.
    pub application: ApplicationPoolEvidence,
    /// Normalized downstream pooler evidence.
    pub pooler: PoolerEvidenceReport,
}

impl DownstreamPoolerDiagnosisInput {
    /// Creates a diagnosis input from application and normalized pooler evidence.
    pub fn new(application: ApplicationPoolEvidence, pooler: PoolerEvidenceReport) -> Self {
        Self {
            application,
            pooler,
        }
    }
}

/// Comparison of application-pool pressure and downstream pooler capacity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DownstreamPoolerDiagnosisReport {
    /// Highest-priority diagnosis for the observed layers.
    pub status: DownstreamPoolerDiagnosisStatus,
    /// Application active connections divided by its configured limit, when known.
    pub application_utilization: Option<f64>,
    /// Observed application requests waiting for a connection.
    pub application_waiting: Option<u32>,
    /// Existing normalized pooler evidence report.
    pub pooler: PoolerEvidenceReport,
    /// Findings explaining which layer is limiting capacity.
    pub findings: Vec<PoolerFinding>,
    /// Confidence in the cross-layer diagnosis.
    pub confidence: EvidenceConfidence,
}

/// Input for client-aware session-state compatibility analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionStateCompatibilityInput {
    /// Client library or framework to analyze.
    pub client: ClientLibraryKind,
    /// Pooler family to check.
    pub pooler: ExternalPoolerKind,
    /// Multiplexing mode used by the pooler.
    pub mode: MultiplexingMode,
    /// Application/database features explicitly used by this workload.
    #[serde(default)]
    pub features_used: Vec<SessionSemanticFeature>,
    /// Optional workflow that will run through the pooler.
    #[serde(default)]
    pub workflow: Option<DatabaseWorkflowKind>,
    /// Optional pooler configuration evidence.
    #[serde(default)]
    pub pooler_config: Option<PoolerConfigSnapshot>,
}

impl SessionStateCompatibilityInput {
    /// Creates client-aware session-state compatibility input.
    pub fn new(
        client: ClientLibraryKind,
        pooler: ExternalPoolerKind,
        mode: MultiplexingMode,
    ) -> Self {
        Self {
            client,
            pooler,
            mode,
            features_used: Vec::new(),
            workflow: None,
            pooler_config: None,
        }
    }

    /// Sets explicitly used session features.
    #[must_use]
    pub fn with_features(mut self, features: Vec<SessionSemanticFeature>) -> Self {
        self.features_used = features;
        self
    }

    /// Sets the workload workflow.
    #[must_use]
    pub fn with_workflow(mut self, workflow: DatabaseWorkflowKind) -> Self {
        self.workflow = Some(workflow);
        self
    }

    /// Sets pooler configuration evidence.
    #[must_use]
    pub fn with_pooler_config(mut self, config: PoolerConfigSnapshot) -> Self {
        self.pooler_config = Some(config);
        self
    }
}

/// Client-specific guidance for session-state and prepared-statement safety.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClientCompatibilityGuidance {
    /// Stable machine-readable guidance code.
    pub code: String,
    /// Severity/risk associated with this guidance.
    pub risk: RiskLevel,
    /// Human-readable explanation.
    pub message: String,
    /// Concrete client or pooler configuration action.
    pub remediation: String,
    /// Source URL used for this guidance.
    pub source_url: String,
    /// Whether the guidance requires a config or topology change before production use.
    pub requires_change: bool,
}

/// Result of client-aware session-state compatibility analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionStateCompatibilityReport {
    /// Overall compatibility decision after base pooler checks and client guidance.
    pub compatible: CompatibilityDecision,
    /// Client library or framework that was analyzed.
    pub client: ClientLibraryKind,
    /// Pooler family that was analyzed.
    pub pooler: ExternalPoolerKind,
    /// Multiplexing mode that was analyzed.
    pub mode: MultiplexingMode,
    /// Explicit plus client-inferred session features used for the base check.
    pub effective_features: Vec<SessionSemanticFeature>,
    /// Existing pooler compatibility report reused by this higher-level analysis.
    pub pooler_report: PoolerCompatibilityReport,
    /// Client-specific guidance and source-backed remediation.
    pub client_guidance: Vec<ClientCompatibilityGuidance>,
    /// Evidence confidence after applying client-specific assumptions.
    pub confidence: EvidenceConfidence,
}

/// Classifies a database endpoint and returns redacted, report-safe evidence.
pub fn classify_endpoint(input: &EndpointClassificationInput) -> EndpointClassificationReport {
    let redacted_endpoint = redact_endpoint(&input.endpoint);
    let lower = input.endpoint.to_ascii_lowercase();
    let provider = input
        .provider
        .unwrap_or_else(|| infer_provider_from_endpoint(&lower));
    let endpoint_kind = infer_endpoint_kind(&lower, provider);

    let mut findings = Vec::new();
    let mut confidence = if endpoint_kind == EndpointConnectionKind::Unknown {
        EvidenceConfidence::Low
    } else if input.provider.is_some() {
        EvidenceConfidence::High
    } else {
        EvidenceConfidence::Medium
    };

    if input.endpoint != redacted_endpoint {
        findings.push(PoolerFinding::new(
            "ENDPOINT_REDACTED",
            RiskLevel::Low,
            "endpoint credentials or secret-like query parameters were redacted",
            "use the redacted endpoint in logs, reports, and support tickets",
        ));
    }

    if endpoint_kind == EndpointConnectionKind::Unknown {
        findings.push(PoolerFinding::new(
            "ENDPOINT_KIND_UNKNOWN",
            RiskLevel::Medium,
            "poolsim could not classify this endpoint from known provider patterns",
            "pass an explicit provider hint or verify whether this is direct, pooled, proxied, or edge-managed",
        ));
    }

    let workflow_compatible = input
        .workflow
        .map(|workflow| endpoint_workflow_compatible(endpoint_kind, workflow));

    if matches!(workflow_compatible, Some(false)) {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "ENDPOINT_WORKFLOW_MISMATCH",
            RiskLevel::High,
            "the endpoint type is risky for the selected workflow",
            "use a direct database endpoint for migrations, backup/restore, replication, database GUIs, admin work, and long-running analytics",
        ));
    }

    EndpointClassificationReport {
        endpoint_kind,
        provider,
        workflow_compatible,
        redacted_endpoint,
        findings,
        confidence,
    }
}

/// Checks whether session features and workflow are compatible with a pooler mode.
pub fn check_pooler_compatibility(input: &PoolerCompatibilityInput) -> PoolerCompatibilityReport {
    let mut findings = Vec::new();
    let mut incompatible_features = Vec::new();
    let mut confidence =
        if input.pooler == ExternalPoolerKind::Unknown || input.mode == MultiplexingMode::Unknown {
            EvidenceConfidence::Low
        } else {
            EvidenceConfidence::High
        };

    if input.pooler == ExternalPoolerKind::Unknown {
        findings.push(PoolerFinding::new(
            "POOLER_UNKNOWN",
            RiskLevel::Medium,
            "pooler family is unknown",
            "provide a known pooler kind so poolsim can apply source-backed compatibility rules",
        ));
    }

    if input.mode == MultiplexingMode::Unknown {
        findings.push(PoolerFinding::new(
            "POOLER_MODE_UNKNOWN",
            RiskLevel::Medium,
            "pooler multiplexing mode is unknown",
            "identify whether the endpoint is direct, session, transaction, statement, or provider-managed before relying on compatibility results",
        ));
    }

    let migration_direct_connection_required = input.workflow.is_some_and(requires_direct_endpoint);
    let long_running_direct_connection_required = input
        .workflow
        .is_some_and(|workflow| matches!(workflow, DatabaseWorkflowKind::LongRunningAnalytics));

    if migration_direct_connection_required {
        findings.push(PoolerFinding::new(
            "DIRECT_ENDPOINT_RECOMMENDED",
            RiskLevel::High,
            "this workflow should normally use a direct database endpoint",
            "run migrations, backup/restore, replication, admin tools, and long-running analytics outside transaction/statement poolers",
        ));
    }

    for feature in &input.features_used {
        if feature_incompatible(
            input.pooler,
            input.mode,
            *feature,
            input.pooler_config.as_ref(),
        ) {
            incompatible_features.push(*feature);
            findings.push(feature_finding(*feature, input.mode));
        } else if feature_needs_review(
            input.pooler,
            input.mode,
            *feature,
            input.pooler_config.as_ref(),
        ) {
            confidence = confidence_min(confidence, EvidenceConfidence::Medium);
            findings.push(review_finding(*feature, input.mode));
        }
    }

    let compatible = if migration_direct_connection_required
        || long_running_direct_connection_required
        || !incompatible_features.is_empty()
    {
        CompatibilityDecision::Incompatible
    } else if confidence != EvidenceConfidence::High
        || findings.iter().any(|f| f.risk >= RiskLevel::Medium)
    {
        CompatibilityDecision::NeedsReview
    } else {
        CompatibilityDecision::Compatible
    };

    PoolerCompatibilityReport {
        compatible,
        incompatible_features,
        migration_direct_connection_required,
        long_running_direct_connection_required,
        findings,
        confidence,
    }
}

/// Checks pooler compatibility and adds client-specific session-state guidance.
///
/// This function is intentionally additive. It reuses
/// [`check_pooler_compatibility`] for the base pooler decision, then layers
/// framework/client guidance on top so callers can see both the generic pooler
/// result and the concrete configuration change for their library.
pub fn analyze_session_state_compatibility(
    input: &SessionStateCompatibilityInput,
) -> SessionStateCompatibilityReport {
    let effective_features = effective_session_features(input.client, &input.features_used);
    let mut base_input = PoolerCompatibilityInput::new(input.pooler, input.mode)
        .with_features(effective_features.clone());
    if let Some(workflow) = input.workflow {
        base_input = base_input.with_workflow(workflow);
    }
    if let Some(config) = input.pooler_config.clone() {
        base_input = base_input.with_pooler_config(config);
    }

    let pooler_report = check_pooler_compatibility(&base_input);
    let client_guidance = client_guidance(input, &effective_features);
    let confidence = client_guidance
        .iter()
        .fold(pooler_report.confidence, |acc, item| {
            if input.client == ClientLibraryKind::Unknown || item.risk >= RiskLevel::High {
                confidence_min(acc, EvidenceConfidence::Medium)
            } else {
                acc
            }
        });
    let compatible = merge_client_decision(&pooler_report, &client_guidance, confidence);

    SessionStateCompatibilityReport {
        compatible,
        client: input.client,
        pooler: input.pooler,
        mode: input.mode,
        effective_features,
        pooler_report,
        client_guidance,
        confidence,
    }
}

/// Summarizes observed external pooler evidence into capacity risk signals.
pub fn summarize_pooler_evidence(input: &PoolerEvidenceSnapshot) -> PoolerEvidenceReport {
    let observed_client_connections = input
        .client_active
        .zip(input.client_waiting)
        .map(|(active, waiting)| active.saturating_add(waiting));
    let observed_backend_connections = input
        .server_active
        .zip(input.server_idle)
        .map(|(active, idle)| active.saturating_add(idle));
    let backend_utilization = utilization(observed_backend_connections, input.pooler_backend_limit);
    let client_utilization = utilization(observed_client_connections, input.pooler_client_limit);

    let mut status = PoolerEvidenceStatus::Healthy;
    let mut confidence = EvidenceConfidence::High;
    let mut findings = Vec::new();

    if input.pooler == ExternalPoolerKind::Unknown || input.mode == MultiplexingMode::Unknown {
        status = PoolerEvidenceStatus::NeedsReview;
        confidence = EvidenceConfidence::Low;
        findings.push(PoolerFinding::new(
            "POOLER_EVIDENCE_KIND_UNKNOWN",
            RiskLevel::Medium,
            "pooler family or multiplexing mode is unknown",
            "classify the endpoint or provide explicit pooler and mode evidence",
        ));
    }

    if input.client_active.is_none()
        || input.client_waiting.is_none()
        || input.server_active.is_none()
        || input.server_idle.is_none()
    {
        status = max_evidence_status(status, PoolerEvidenceStatus::NeedsReview);
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "POOLER_EVIDENCE_COUNTS_INCOMPLETE",
            RiskLevel::Medium,
            "client or backend pooler counters are incomplete",
            "provide active/waiting client counts and active/idle backend counts from pooler telemetry",
        ));
    }

    if input.pooler_backend_limit.is_none() {
        status = max_evidence_status(status, PoolerEvidenceStatus::NeedsReview);
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "POOLER_BACKEND_LIMIT_UNKNOWN",
            RiskLevel::Medium,
            "pooler backend/server connection limit is unknown",
            "provide PgBouncer pool_size, Supavisor pool size, RDS Proxy max database connections, or equivalent provider evidence",
        ));
    }

    if input.client_waiting.unwrap_or(0) > 0 {
        status = max_evidence_status(status, PoolerEvidenceStatus::ClientWaiting);
        findings.push(PoolerFinding::new(
            "POOLER_CLIENTS_WAITING",
            RiskLevel::High,
            "clients are waiting for backend/server pooler capacity",
            "inspect long transactions, query latency, backend pool limits, and session pinning before increasing application pool size",
        ));
    }

    if backend_utilization.is_some_and(|rho| rho >= 1.0) {
        status = PoolerEvidenceStatus::BackendSaturated;
        findings.push(PoolerFinding::new(
            "POOLER_BACKEND_SATURATED",
            RiskLevel::Critical,
            "observed backend/server connections are at or above the supplied backend limit",
            "reduce client pressure, increase backend pool capacity only within the database budget, or split traffic by workload",
        ));
    } else if backend_utilization.is_some_and(|rho| rho >= 0.8) {
        status = max_evidence_status(status, PoolerEvidenceStatus::NeedsReview);
        findings.push(PoolerFinding::new(
            "POOLER_BACKEND_NEAR_LIMIT",
            RiskLevel::High,
            "observed backend/server connections are using at least 80% of the supplied backend limit",
            "leave headroom for bursts, failover, admin sessions, migrations, and other services",
        ));
    }

    PoolerEvidenceReport {
        status,
        pooler: input.pooler,
        mode: input.mode,
        label: input.label.clone(),
        observed_client_connections,
        observed_backend_connections,
        client_waiting: input.client_waiting,
        backend_utilization,
        client_utilization,
        findings,
        confidence,
    }
}

/// Compares application-pool counters with normalized downstream pooler evidence.
///
/// The downstream status takes precedence when pooler clients are waiting or
/// backend connections are saturated. This preserves the operational signal
/// that an application pool can have spare capacity while its pooler cannot
/// obtain another database connection. Application findings remain in the
/// report when both layers are constrained.
pub fn diagnose_downstream_pooler(
    input: &DownstreamPoolerDiagnosisInput,
) -> DownstreamPoolerDiagnosisReport {
    let application_utilization = utilization(input.application.active, input.application.limit);
    let application_missing =
        input.application.active.is_none() || input.application.limit.is_none();
    let application_saturated = application_utilization.is_some_and(|rho| rho >= 1.0);
    let application_near_limit = application_utilization.is_some_and(|rho| rho >= 0.8);
    let downstream_saturated = input.pooler.status == PoolerEvidenceStatus::BackendSaturated;
    let downstream_waiting = input.pooler.status == PoolerEvidenceStatus::ClientWaiting
        || input
            .pooler
            .client_waiting
            .is_some_and(|waiting| waiting > 0);

    let status = if downstream_saturated {
        DownstreamPoolerDiagnosisStatus::DownstreamPoolerSaturated
    } else if downstream_waiting {
        DownstreamPoolerDiagnosisStatus::DownstreamPoolerWaiting
    } else if application_saturated {
        DownstreamPoolerDiagnosisStatus::ApplicationPoolSaturated
    } else if application_missing
        || application_near_limit
        || input.pooler.status == PoolerEvidenceStatus::NeedsReview
    {
        DownstreamPoolerDiagnosisStatus::NeedsReview
    } else {
        DownstreamPoolerDiagnosisStatus::Healthy
    };

    let mut confidence = input.pooler.confidence;
    let mut findings = Vec::new();
    if application_missing {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "APPLICATION_POOL_EVIDENCE_INCOMPLETE",
            RiskLevel::Medium,
            "application active connections or its configured limit is unknown",
            "provide both application pool active connections and the configured maximum before comparing layers",
        ));
    }
    if application_utilization.is_some_and(|rho| rho >= 1.0) {
        findings.push(PoolerFinding::new(
            "APPLICATION_POOL_SATURATED",
            RiskLevel::Critical,
            "the application pool is at or above its configured connection limit",
            "reduce application concurrency or increase the application pool only after checking downstream and database budgets",
        ));
    } else if application_near_limit {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "APPLICATION_POOL_NEAR_LIMIT",
            RiskLevel::High,
            "the application pool is using at least 80% of its configured connection limit",
            "leave burst headroom and compare application demand with downstream pooler and database limits",
        ));
    }
    if input.application.waiting.is_some_and(|waiting| waiting > 0) {
        findings.push(PoolerFinding::new(
            "APPLICATION_REQUESTS_WAITING",
            RiskLevel::High,
            "application requests are waiting for an application-pool connection",
            "inspect application pool timeout, request concurrency, and downstream wait evidence before changing pool size",
        ));
    }
    if downstream_saturated {
        findings.push(PoolerFinding::new(
            "DOWNSTREAM_POOLER_BACKEND_SATURATED",
            RiskLevel::Critical,
            "the downstream pooler's backend/server connections reached the supplied limit",
            "increase downstream capacity only within the database connection budget, or reduce and split workload pressure",
        ));
    }
    if downstream_waiting {
        findings.push(PoolerFinding::new(
            "DOWNSTREAM_POOLER_CLIENTS_WAITING",
            RiskLevel::High,
            "clients are waiting in the downstream pooler for a backend/server connection",
            "inspect backend latency, long transactions, pooler limits, and database headroom before increasing the application pool",
        ));
    }

    DownstreamPoolerDiagnosisReport {
        status,
        application_utilization,
        application_waiting: input.application.waiting,
        pooler: input.pooler.clone(),
        findings,
        confidence,
    }
}

/// Parses PgBouncer `SHOW POOLS` output into normalized pool rows.
///
/// The parser accepts the two capture formats operators commonly use:
///
/// - default `psql` aligned output copied from a terminal
/// - `psql --csv` output with a header row
///
/// # Errors
///
/// Returns [`PoolsimError`] when the capture has no usable header, omits the
/// required PgBouncer counters, or contains non-integer counter values.
pub fn parse_pgbouncer_show_pools(text: &str) -> Result<Vec<PgbouncerPoolRow>, PoolsimError> {
    let (header_line, delimiter) =
        find_pgbouncer_header(text).ok_or_else(|| invalid_pgbouncer_show_pools(
            "PgBouncer SHOW POOLS output must include a header row containing cl_active, cl_waiting, sv_active, and sv_idle",
        ))?;
    let header = split_pgbouncer_record(header_line, delimiter)?;
    let required = PgbouncerShowPoolsColumns::from_header(&header)?;
    let mut rows = Vec::new();
    let mut after_header = false;

    for line in text.lines() {
        if !after_header {
            if line == header_line {
                after_header = true;
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty()
            || is_psql_separator(trimmed)
            || (trimmed.starts_with('(') && trimmed.ends_with("row)"))
            || (trimmed.starts_with('(') && trimmed.ends_with("rows)"))
        {
            continue;
        }

        let cells = split_pgbouncer_record(trimmed, delimiter)?;
        if cells.len() < header.len() {
            return Err(invalid_pgbouncer_show_pools(format!(
                "PgBouncer SHOW POOLS row has {} columns but the header has {}",
                cells.len(),
                header.len()
            )));
        }
        rows.push(required.row_from_cells(&cells)?);
    }

    if rows.is_empty() {
        return Err(invalid_pgbouncer_show_pools(
            "PgBouncer SHOW POOLS output did not include any data rows",
        ));
    }
    Ok(rows)
}

/// Summarizes parsed PgBouncer `SHOW POOLS` rows into pooler evidence.
///
/// Rows are aggregated across PgBouncer's `(database, user)` pools because the
/// evidence report describes total client pressure and total backend/server
/// usage for the captured pooler.
///
/// # Errors
///
/// Returns [`PoolsimError`] if the snapshot contains no rows.
pub fn summarize_pgbouncer_show_pools(
    input: &PgbouncerShowPoolsSnapshot,
) -> Result<PoolerEvidenceReport, PoolsimError> {
    let Some(first) = input.rows.first() else {
        return Err(invalid_pgbouncer_show_pools(
            "PgBouncer SHOW POOLS snapshot must include at least one row",
        ));
    };

    let client_active = input
        .rows
        .iter()
        .fold(0_u32, |sum, row| sum.saturating_add(row.cl_active));
    let client_waiting = input
        .rows
        .iter()
        .fold(0_u32, |sum, row| sum.saturating_add(row.cl_waiting));
    let server_active = input
        .rows
        .iter()
        .fold(0_u32, |sum, row| sum.saturating_add(row.sv_active));
    let server_idle = input
        .rows
        .iter()
        .fold(0_u32, |sum, row| sum.saturating_add(row.sv_idle));
    let mode = input
        .mode
        .or(first.pool_mode)
        .unwrap_or(MultiplexingMode::Unknown);

    let mut evidence = PoolerEvidenceSnapshot::new(ExternalPoolerKind::PgBouncer, mode)
        .with_client_active(client_active)
        .with_client_waiting(client_waiting)
        .with_server_active(server_active)
        .with_server_idle(server_idle);
    if let Some(label) = &input.label {
        evidence = evidence.with_label(label);
    }
    if let Some(limit) = input.pooler_client_limit {
        evidence = evidence.with_pooler_client_limit(limit);
    }
    if let Some(limit) = input.pooler_backend_limit {
        evidence = evidence.with_pooler_backend_limit(limit);
    }
    Ok(summarize_pooler_evidence(&evidence))
}

/// Parses PgBouncer `SHOW STATS` output into normalized statistics rows.
///
/// The parser accepts default `psql` aligned output and `psql --csv` output.
/// It requires the cumulative `total_query_count` and `total_wait_time`
/// columns, while accepting `database` and `avg_wait_time` when present.
///
/// # Errors
///
/// Returns [`PoolsimError`] when the capture has no usable header, omits a
/// required counter, contains an invalid counter, or has malformed CSV.
pub fn parse_pgbouncer_show_stats(text: &str) -> Result<Vec<PgbouncerStatsRow>, PoolsimError> {
    let (header_line, delimiter) = find_pgbouncer_stats_header(text).ok_or_else(|| {
        invalid_pgbouncer_show_stats(
            "PgBouncer SHOW STATS output must include total_query_count and total_wait_time",
        )
    })?;
    let header = split_pgbouncer_record(header_line, delimiter)
        .map_err(|error| invalid_pgbouncer_show_stats(error.to_string()))?;
    let required = PgbouncerShowStatsColumns::from_header(&header)?;
    let mut rows = Vec::new();
    let mut after_header = false;

    for line in text.lines() {
        if !after_header {
            if line == header_line {
                after_header = true;
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty()
            || is_psql_separator(trimmed)
            || (trimmed.starts_with('(') && trimmed.ends_with("row)"))
            || (trimmed.starts_with('(') && trimmed.ends_with("rows)"))
        {
            continue;
        }

        let cells = split_pgbouncer_record(trimmed, delimiter)
            .map_err(|error| invalid_pgbouncer_show_stats(error.to_string()))?;
        if cells.len() < header.len() {
            return Err(invalid_pgbouncer_show_stats(format!(
                "PgBouncer SHOW STATS row has {} columns but the header has {}",
                cells.len(),
                header.len()
            )));
        }
        rows.push(required.row_from_cells(&cells)?);
    }

    if rows.is_empty() {
        return Err(invalid_pgbouncer_show_stats(
            "PgBouncer SHOW STATS output did not include any data rows",
        ));
    }
    Ok(rows)
}

/// Aggregates parsed PgBouncer `SHOW STATS` rows into one time-series sample.
///
/// The cumulative counters are summed across database rows with saturating
/// arithmetic. `avg_wait_time` is intentionally not used for the aggregate:
/// the delta report derives a window average from counter differences, which
/// remains correct when databases have different traffic volumes.
///
/// # Errors
///
/// Returns [`PoolsimError`] if the timestamp is not finite or the snapshot has
/// no rows.
pub fn summarize_pgbouncer_show_stats(
    input: &PgbouncerShowStatsSnapshot,
) -> Result<PgbouncerTimeSeriesSample, PoolsimError> {
    if !input.timestamp_seconds.is_finite() {
        return Err(invalid_pgbouncer_time_series(
            "PgBouncer SHOW STATS timestamp_seconds must be finite",
        ));
    }
    if input.rows.is_empty() {
        return Err(invalid_pgbouncer_show_stats(
            "PgBouncer SHOW STATS snapshot must include at least one row",
        ));
    }
    if input
        .maxwait_seconds
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(invalid_pgbouncer_time_series(
            "PgBouncer maxwait_seconds must be finite and non-negative",
        ));
    }

    Ok(PgbouncerTimeSeriesSample {
        timestamp_seconds: input.timestamp_seconds,
        label: input.label.clone(),
        total_query_count: input
            .rows
            .iter()
            .fold(0_u64, |sum, row| sum.saturating_add(row.total_query_count)),
        total_wait_time_us: input
            .rows
            .iter()
            .fold(0_u64, |sum, row| sum.saturating_add(row.total_wait_time_us)),
        maxwait_seconds: input.maxwait_seconds,
        client_waiting: input.client_waiting,
    })
}

/// Compares two PgBouncer time-series samples without producing negative rates.
///
/// A decreasing cumulative counter is treated as a reset: the current value
/// is used as post-reset accumulation, and the reset is retained in the report
/// so callers can avoid treating the interval as a normal steady-state sample.
/// The timestamps must be finite and strictly increasing.
///
/// # Errors
///
/// Returns [`PoolsimError`] when either timestamp is non-finite, the current
/// timestamp is not after the previous timestamp, or a gauge is invalid.
pub fn diff_pgbouncer_time_series(
    previous: &PgbouncerTimeSeriesSample,
    current: &PgbouncerTimeSeriesSample,
) -> Result<PgbouncerTimeSeriesDeltaReport, PoolsimError> {
    if !previous.timestamp_seconds.is_finite() || !current.timestamp_seconds.is_finite() {
        return Err(invalid_pgbouncer_time_series(
            "PgBouncer time-series timestamps must be finite",
        ));
    }
    let interval_seconds = current.timestamp_seconds - previous.timestamp_seconds;
    if interval_seconds <= 0.0 {
        return Err(invalid_pgbouncer_time_series(
            "current PgBouncer timestamp must be greater than previous timestamp",
        ));
    }
    if previous
        .maxwait_seconds
        .is_some_and(|value| !value.is_finite() || value < 0.0)
        || current
            .maxwait_seconds
            .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(invalid_pgbouncer_time_series(
            "PgBouncer maxwait_seconds must be finite and non-negative",
        ));
    }

    let (query_delta, query_reset) =
        counter_delta(previous.total_query_count, current.total_query_count);
    let (wait_delta, wait_reset) =
        counter_delta(previous.total_wait_time_us, current.total_wait_time_us);
    let mut counter_resets = Vec::new();
    if query_reset {
        counter_resets.push(PgbouncerCounterKind::TotalQueryCount);
    }
    if wait_reset {
        counter_resets.push(PgbouncerCounterKind::TotalWaitTime);
    }

    let current_maxwait_seconds = current.maxwait_seconds;
    let maxwait_delta_seconds = previous
        .maxwait_seconds
        .zip(current.maxwait_seconds)
        .map(|(old, new)| new - old);
    let waiting_grew = previous
        .client_waiting
        .zip(current.client_waiting)
        .is_some_and(|(old, new)| new > old);
    let maxwait_grew = maxwait_delta_seconds.is_some_and(|delta| delta > 0.0);
    let queue_present = current_maxwait_seconds.is_some_and(|value| value > 0.0)
        || current.client_waiting.is_some_and(|value| value > 0);
    let queue_growing = maxwait_grew || waiting_grew;

    let mut findings = Vec::new();
    let mut confidence = EvidenceConfidence::High;
    if queue_growing {
        findings.push(PoolerFinding::new(
            "PGBOUNCER_QUEUE_GROWING",
            RiskLevel::High,
            "PgBouncer queue pressure increased during the observation interval",
            "inspect backend service time, pooler capacity, database headroom, and long-running transactions",
        ));
    } else if queue_present {
        findings.push(PoolerFinding::new(
            "PGBOUNCER_QUEUE_PRESENT",
            RiskLevel::Medium,
            "PgBouncer has clients waiting or an oldest-client wait age above zero",
            "confirm whether the queue is transient and compare it with backend latency and pool limits",
        ));
    }
    if !counter_resets.is_empty() {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "PGBOUNCER_COUNTER_RESET",
            RiskLevel::Medium,
            "one or more cumulative PgBouncer counters decreased between captures",
            "verify exporter continuity and PgBouncer restarts before using this interval for trend decisions",
        ));
    }
    if previous.maxwait_seconds.is_none() || current.maxwait_seconds.is_none() {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "PGBOUNCER_MAXWAIT_MISSING",
            RiskLevel::Medium,
            "maxwait was not supplied for both captures, so queue age growth cannot be confirmed",
            "capture SHOW POOLS.maxwait or the pgbouncer_pools_client_maxwait_seconds gauge with each sample",
        ));
    }
    if previous.client_waiting.is_none() || current.client_waiting.is_none() {
        confidence = confidence_min(confidence, EvidenceConfidence::Medium);
        findings.push(PoolerFinding::new(
            "PGBOUNCER_WAITING_GAUGE_MISSING",
            RiskLevel::Medium,
            "cl_waiting was not supplied for both captures, so waiting-client growth cannot be confirmed",
            "capture SHOW POOLS.cl_waiting or the matching exporter gauge with each sample",
        ));
    }

    let status = if queue_growing {
        PgbouncerTimeSeriesStatus::QueueGrowing
    } else if queue_present {
        PgbouncerTimeSeriesStatus::QueuePresent
    } else if !counter_resets.is_empty() {
        PgbouncerTimeSeriesStatus::CounterReset
    } else if previous.maxwait_seconds.is_none()
        || current.maxwait_seconds.is_none()
        || previous.client_waiting.is_none()
        || current.client_waiting.is_none()
    {
        PgbouncerTimeSeriesStatus::NeedsReview
    } else {
        PgbouncerTimeSeriesStatus::Healthy
    };

    let average_wait_ms_per_query =
        (query_delta > 0).then(|| (wait_delta as f64 / query_delta as f64) / 1_000.0);

    Ok(PgbouncerTimeSeriesDeltaReport {
        previous_timestamp_seconds: previous.timestamp_seconds,
        current_timestamp_seconds: current.timestamp_seconds,
        interval_seconds,
        query_rate_per_second: query_delta as f64 / interval_seconds,
        wait_time_rate_us_per_second: wait_delta as f64 / interval_seconds,
        average_wait_ms_per_query,
        current_maxwait_seconds,
        maxwait_delta_seconds,
        current_client_waiting: current.client_waiting,
        counter_resets,
        status,
        findings,
        confidence,
    })
}

#[derive(Debug, Clone, Copy)]
struct PgbouncerShowStatsColumns {
    database: Option<usize>,
    total_query_count: usize,
    total_wait_time: usize,
    avg_wait_time: Option<usize>,
}

impl PgbouncerShowStatsColumns {
    fn from_header(header: &[String]) -> Result<Self, PoolsimError> {
        let normalized: Vec<String> = header.iter().map(|cell| normalize_column(cell)).collect();
        let find = |name: &str| normalized.iter().position(|cell| cell == name);
        Ok(Self {
            database: find("database"),
            total_query_count: required_pgbouncer_stats_column(&normalized, "total_query_count")?,
            total_wait_time: required_pgbouncer_stats_column(&normalized, "total_wait_time")?,
            avg_wait_time: find("avg_wait_time"),
        })
    }

    fn row_from_cells(&self, cells: &[String]) -> Result<PgbouncerStatsRow, PoolsimError> {
        let mut row = PgbouncerStatsRow::new(
            parse_pgbouncer_stats_count(cells, self.total_query_count, "total_query_count")?,
            parse_pgbouncer_stats_count(cells, self.total_wait_time, "total_wait_time")?,
        );
        row.database = optional_pgbouncer_cell(cells, self.database);
        row.avg_wait_time_us =
            optional_pgbouncer_float(cells, self.avg_wait_time, "avg_wait_time")?;
        Ok(row)
    }
}

#[derive(Debug, Clone, Copy)]
struct PgbouncerShowPoolsColumns {
    database: Option<usize>,
    user: Option<usize>,
    cl_active: usize,
    cl_waiting: usize,
    sv_active: usize,
    sv_idle: usize,
    pool_mode: Option<usize>,
}

impl PgbouncerShowPoolsColumns {
    fn from_header(header: &[String]) -> Result<Self, PoolsimError> {
        let normalized: Vec<String> = header.iter().map(|cell| normalize_column(cell)).collect();
        let find = |name: &str| normalized.iter().position(|cell| cell == name);
        Ok(Self {
            database: find("database"),
            user: find("user"),
            cl_active: required_pgbouncer_column(&normalized, "cl_active")?,
            cl_waiting: required_pgbouncer_column(&normalized, "cl_waiting")?,
            sv_active: required_pgbouncer_column(&normalized, "sv_active")?,
            sv_idle: required_pgbouncer_column(&normalized, "sv_idle")?,
            pool_mode: find("pool_mode"),
        })
    }

    fn row_from_cells(&self, cells: &[String]) -> Result<PgbouncerPoolRow, PoolsimError> {
        let mut row = PgbouncerPoolRow::new(
            parse_pgbouncer_count(cells, self.cl_active, "cl_active")?,
            parse_pgbouncer_count(cells, self.cl_waiting, "cl_waiting")?,
            parse_pgbouncer_count(cells, self.sv_active, "sv_active")?,
            parse_pgbouncer_count(cells, self.sv_idle, "sv_idle")?,
        );
        row.database = optional_pgbouncer_cell(cells, self.database);
        row.user = optional_pgbouncer_cell(cells, self.user);
        row.pool_mode = optional_pgbouncer_mode(cells, self.pool_mode)?;
        Ok(row)
    }
}

fn find_pgbouncer_header(text: &str) -> Option<(&str, char)> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_psql_separator(trimmed) {
            return None;
        }
        if trimmed.contains('|') && trimmed.contains("cl_active") {
            Some((line, '|'))
        } else if trimmed.contains(',') && trimmed.contains("cl_active") {
            Some((line, ','))
        } else {
            None
        }
    })
}

fn find_pgbouncer_stats_header(text: &str) -> Option<(&str, char)> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_psql_separator(trimmed) {
            return None;
        }
        if trimmed.contains('|') && trimmed.contains("total_query_count") {
            Some((line, '|'))
        } else if trimmed.contains(',') && trimmed.contains("total_query_count") {
            Some((line, ','))
        } else {
            None
        }
    })
}

fn split_pgbouncer_record(line: &str, delimiter: char) -> Result<Vec<String>, PoolsimError> {
    if delimiter == ',' {
        split_csv_record(line)
    } else {
        Ok(line
            .split(delimiter)
            .map(|cell| cell.trim().to_string())
            .collect())
    }
}

fn split_csv_record(line: &str) -> Result<Vec<String>, PoolsimError> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                cells.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }

    if in_quotes {
        return Err(invalid_pgbouncer_show_pools(
            "PgBouncer SHOW POOLS CSV row contains an unterminated quoted field",
        ));
    }

    cells.push(cell.trim().to_string());
    Ok(cells)
}

fn required_pgbouncer_column(header: &[String], name: &str) -> Result<usize, PoolsimError> {
    header.iter().position(|cell| cell == name).ok_or_else(|| {
        invalid_pgbouncer_show_pools(format!(
            "PgBouncer SHOW POOLS output is missing required column {name}"
        ))
    })
}

fn required_pgbouncer_stats_column(header: &[String], name: &str) -> Result<usize, PoolsimError> {
    header.iter().position(|cell| cell == name).ok_or_else(|| {
        invalid_pgbouncer_show_stats(format!(
            "PgBouncer SHOW STATS output is missing required column {name}"
        ))
    })
}

fn parse_pgbouncer_count(
    cells: &[String],
    index: usize,
    column: &'static str,
) -> Result<u32, PoolsimError> {
    cells
        .get(index)
        .ok_or_else(|| {
            invalid_pgbouncer_show_pools(format!(
                "PgBouncer SHOW POOLS row is missing required column {column}"
            ))
        })?
        .parse::<u32>()
        .map_err(|_| {
            invalid_pgbouncer_show_pools(format!(
                "PgBouncer SHOW POOLS column {column} must be an unsigned integer"
            ))
        })
}

fn parse_pgbouncer_stats_count(
    cells: &[String],
    index: usize,
    column: &'static str,
) -> Result<u64, PoolsimError> {
    cells
        .get(index)
        .ok_or_else(|| {
            invalid_pgbouncer_show_stats(format!(
                "PgBouncer SHOW STATS row is missing required column {column}"
            ))
        })?
        .parse::<u64>()
        .map_err(|_| {
            invalid_pgbouncer_show_stats(format!(
                "PgBouncer SHOW STATS column {column} must be an unsigned integer"
            ))
        })
}

fn optional_pgbouncer_cell(cells: &[String], index: Option<usize>) -> Option<String> {
    index
        .and_then(|idx| cells.get(idx))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_pgbouncer_float(
    cells: &[String],
    index: Option<usize>,
    column: &'static str,
) -> Result<Option<f64>, PoolsimError> {
    let Some(value) = optional_pgbouncer_cell(cells, index) else {
        return Ok(None);
    };
    let parsed = value.parse::<f64>().map_err(|_| {
        invalid_pgbouncer_show_stats(format!(
            "PgBouncer SHOW STATS column {column} must be a number"
        ))
    })?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(invalid_pgbouncer_show_stats(format!(
            "PgBouncer SHOW STATS column {column} must be finite and non-negative"
        )));
    }
    Ok(Some(parsed))
}

fn optional_pgbouncer_mode(
    cells: &[String],
    index: Option<usize>,
) -> Result<Option<MultiplexingMode>, PoolsimError> {
    let Some(value) = optional_pgbouncer_cell(cells, index) else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "session" => Ok(Some(MultiplexingMode::Session)),
        "transaction" => Ok(Some(MultiplexingMode::Transaction)),
        "statement" => Ok(Some(MultiplexingMode::Statement)),
        "none" => Ok(Some(MultiplexingMode::None)),
        "provider-managed" | "provider_managed" => Ok(Some(MultiplexingMode::ProviderManaged)),
        "unknown" => Ok(Some(MultiplexingMode::Unknown)),
        _ => Err(invalid_pgbouncer_show_pools(format!(
            "PgBouncer SHOW POOLS pool_mode value {value:?} is not recognized"
        ))),
    }
}

fn normalize_column(cell: &str) -> String {
    cell.trim()
        .trim_matches('"')
        .to_ascii_lowercase()
        .replace('-', "_")
}

fn is_psql_separator(line: &str) -> bool {
    line.chars()
        .all(|ch| ch == '-' || ch == '+' || ch.is_whitespace())
}

fn invalid_pgbouncer_show_pools(message: impl Into<String>) -> PoolsimError {
    PoolsimError::invalid_input("INVALID_PGBOUNCER_SHOW_POOLS", message, None)
}

fn invalid_pgbouncer_show_stats(message: impl Into<String>) -> PoolsimError {
    PoolsimError::invalid_input("INVALID_PGBOUNCER_SHOW_STATS", message, None)
}

fn invalid_pgbouncer_time_series(message: impl Into<String>) -> PoolsimError {
    PoolsimError::invalid_input("INVALID_PGBOUNCER_TIME_SERIES", message, None)
}

fn counter_delta(previous: u64, current: u64) -> (u64, bool) {
    if current >= previous {
        (current - previous, false)
    } else {
        (current, true)
    }
}

/// Redacts credentials and secret-like query parameters from an endpoint string.
pub fn redact_endpoint(endpoint: &str) -> String {
    let (base, query) = endpoint.split_once('?').unwrap_or((endpoint, ""));
    let mut redacted = redact_userinfo(base);
    if !query.is_empty() {
        redacted.push('?');
        redacted.push_str(&redact_query(query));
    }
    redacted
}

fn infer_provider_from_endpoint(lower: &str) -> EndpointProviderKind {
    if lower.contains("pooler.supabase.com") || lower.contains(".supabase.co") {
        EndpointProviderKind::Supabase
    } else if lower.contains("-pooler.") && lower.contains(".neon.tech")
        || lower.contains(".neon.tech")
    {
        EndpointProviderKind::Neon
    } else if lower.contains("prisma") || lower.contains("accelerate") {
        EndpointProviderKind::PrismaPostgres
    } else if lower.contains("hyperdrive") || lower.contains("cloudflare") {
        EndpointProviderKind::CloudflareHyperdrive
    } else if lower.contains("proxy-") && lower.contains("rds.amazonaws.com") {
        EndpointProviderKind::AwsRdsProxy
    } else if lower.contains("rds.amazonaws.com") {
        EndpointProviderKind::AwsRds
    } else if lower.contains(":6432") || lower.contains("pgbouncer") {
        EndpointProviderKind::PgBouncer
    } else {
        EndpointProviderKind::Unknown
    }
}

fn infer_endpoint_kind(lower: &str, provider: EndpointProviderKind) -> EndpointConnectionKind {
    match provider {
        EndpointProviderKind::Supabase => {
            if lower.contains("pooler.supabase.com") && lower.contains(":6543") {
                EndpointConnectionKind::TransactionPooler
            } else if lower.contains("pooler.supabase.com") {
                EndpointConnectionKind::SessionPooler
            } else if lower.contains(".supabase.co") {
                EndpointConnectionKind::DirectDatabase
            } else {
                EndpointConnectionKind::Unknown
            }
        }
        EndpointProviderKind::Neon => {
            if lower.contains("-pooler.") || lower.contains("pooler") {
                EndpointConnectionKind::TransactionPooler
            } else if lower.contains(".neon.tech") {
                EndpointConnectionKind::DirectDatabase
            } else {
                EndpointConnectionKind::Unknown
            }
        }
        EndpointProviderKind::PrismaPostgres => {
            if lower.contains("accelerate") || lower.starts_with("prisma+postgres://") {
                EndpointConnectionKind::HttpDataApi
            } else if lower.contains("pool") {
                EndpointConnectionKind::TransactionPooler
            } else {
                EndpointConnectionKind::Unknown
            }
        }
        EndpointProviderKind::AwsRdsProxy => EndpointConnectionKind::DatabaseProxy,
        EndpointProviderKind::AwsRds => EndpointConnectionKind::DirectDatabase,
        EndpointProviderKind::CloudflareHyperdrive => EndpointConnectionKind::EdgePooler,
        EndpointProviderKind::PgBouncer => {
            if lower.contains("statement") {
                EndpointConnectionKind::StatementPooler
            } else if lower.contains("transaction") {
                EndpointConnectionKind::TransactionPooler
            } else if lower.contains("session") {
                EndpointConnectionKind::SessionPooler
            } else {
                EndpointConnectionKind::Unknown
            }
        }
        EndpointProviderKind::Unknown => EndpointConnectionKind::Unknown,
    }
}

fn endpoint_workflow_compatible(
    endpoint_kind: EndpointConnectionKind,
    workflow: DatabaseWorkflowKind,
) -> bool {
    if requires_direct_endpoint(workflow) {
        matches!(endpoint_kind, EndpointConnectionKind::DirectDatabase)
    } else {
        true
    }
}

fn requires_direct_endpoint(workflow: DatabaseWorkflowKind) -> bool {
    matches!(
        workflow,
        DatabaseWorkflowKind::Migration
            | DatabaseWorkflowKind::BackupRestore
            | DatabaseWorkflowKind::DatabaseGui
            | DatabaseWorkflowKind::Replication
            | DatabaseWorkflowKind::LongRunningAnalytics
            | DatabaseWorkflowKind::AdminTask
    )
}

fn feature_incompatible(
    pooler: ExternalPoolerKind,
    mode: MultiplexingMode,
    feature: SessionSemanticFeature,
    config: Option<&PoolerConfigSnapshot>,
) -> bool {
    if matches!(mode, MultiplexingMode::None | MultiplexingMode::Session) {
        return false;
    }

    match feature {
        SessionSemanticFeature::SetStatement
        | SessionSemanticFeature::ListenNotifyListener
        | SessionSemanticFeature::NamedPreparedStatements
        | SessionSemanticFeature::TemporaryTables
        | SessionSemanticFeature::HoldCursors
        | SessionSemanticFeature::AdvisoryLocks
        | SessionSemanticFeature::SessionVariables
        | SessionSemanticFeature::Migrations
        | SessionSemanticFeature::LongRunningQuery
        | SessionSemanticFeature::InteractiveTransaction
        | SessionSemanticFeature::CopyProtocol => true,
        SessionSemanticFeature::PreparedStatements
        | SessionSemanticFeature::ProtocolPreparedStatements => {
            pooler == ExternalPoolerKind::PgBouncer
                && config
                    .and_then(|cfg| cfg.max_prepared_statements)
                    .unwrap_or(0)
                    == 0
        }
        SessionSemanticFeature::NotifyOnly | SessionSemanticFeature::Unknown => false,
    }
}

fn feature_needs_review(
    pooler: ExternalPoolerKind,
    mode: MultiplexingMode,
    feature: SessionSemanticFeature,
    config: Option<&PoolerConfigSnapshot>,
) -> bool {
    if matches!(
        mode,
        MultiplexingMode::Unknown | MultiplexingMode::ProviderManaged
    ) {
        return true;
    }

    matches!(
        (pooler, mode, feature),
        (
            ExternalPoolerKind::RdsProxy
                | ExternalPoolerKind::CloudflareHyperdrive
                | ExternalPoolerKind::PrismaPostgresPooler
                | ExternalPoolerKind::NeonPooler
                | ExternalPoolerKind::Supavisor,
            MultiplexingMode::Transaction | MultiplexingMode::Statement,
            SessionSemanticFeature::PreparedStatements
                | SessionSemanticFeature::ProtocolPreparedStatements
                | SessionSemanticFeature::Unknown
        )
    ) || matches!(
        feature,
        SessionSemanticFeature::PreparedStatements
            | SessionSemanticFeature::ProtocolPreparedStatements
    ) && config.is_none()
        && !matches!(pooler, ExternalPoolerKind::PgBouncer)
}

fn effective_session_features(
    client: ClientLibraryKind,
    explicit_features: &[SessionSemanticFeature],
) -> Vec<SessionSemanticFeature> {
    let mut features = explicit_features.to_vec();
    for feature in default_client_features(client) {
        if !features.contains(&feature) {
            features.push(feature);
        }
    }
    features
}

fn default_client_features(client: ClientLibraryKind) -> Vec<SessionSemanticFeature> {
    match client {
        ClientLibraryKind::Prisma
        | ClientLibraryKind::Sqlx
        | ClientLibraryKind::SqlalchemyAsyncpg
        | ClientLibraryKind::Postgrest => vec![SessionSemanticFeature::PreparedStatements],
        ClientLibraryKind::GenericPostgres
        | ClientLibraryKind::NodePostgres
        | ClientLibraryKind::PgJdbc
        | ClientLibraryKind::Unknown => Vec::new(),
    }
}

fn client_guidance(
    input: &SessionStateCompatibilityInput,
    effective_features: &[SessionSemanticFeature],
) -> Vec<ClientCompatibilityGuidance> {
    let mut guidance = Vec::new();
    let uses_prepared = effective_features.iter().any(|feature| {
        matches!(
            feature,
            SessionSemanticFeature::PreparedStatements
                | SessionSemanticFeature::ProtocolPreparedStatements
                | SessionSemanticFeature::NamedPreparedStatements
        )
    });
    let transactional = matches!(
        input.mode,
        MultiplexingMode::Transaction | MultiplexingMode::Statement
    );

    match input.client {
        ClientLibraryKind::Prisma if transactional => guidance.push(guidance_item(
            "CLIENT_PRISMA_POOLER_PREPARED_STATEMENTS",
            RiskLevel::High,
            "Prisma uses prepared statements and needs PgBouncer-compatible configuration for pooled runtime traffic",
            "use a pooled runtime URL only when the pooler supports the Prisma mode; keep Prisma migration and schema commands on a direct URL",
            "https://docs.prisma.io/docs/orm/v6/prisma-client/setup-and-configuration/databases-connections/pgbouncer",
            input.workflow.is_some_and(requires_direct_endpoint) || uses_prepared_without_evidence(input),
        )),
        ClientLibraryKind::NodePostgres if transactional => guidance.push(guidance_item(
            "CLIENT_NODE_PG_NAMED_PREPARED_STATEMENTS",
            RiskLevel::Medium,
            "node-postgres only creates prepared statements when a query config includes a name",
            "avoid the query config name field with transaction poolers, or prove PgBouncer max_prepared_statements is non-zero before using named statements",
            "https://node-postgres.com/features/queries",
            effective_features.contains(&SessionSemanticFeature::NamedPreparedStatements),
        )),
        ClientLibraryKind::Sqlx if transactional => guidance.push(guidance_item(
            "CLIENT_SQLX_STATEMENT_CACHE",
            RiskLevel::High,
            "sqlx prepares and caches PostgreSQL statements by default",
            "set statement_cache_capacity to 0 for transaction poolers that cannot preserve prepared statements, or prove PgBouncer prepared-statement tracking is enabled",
            "https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html",
            uses_prepared_without_evidence(input),
        )),
        ClientLibraryKind::SqlalchemyAsyncpg if transactional => guidance.push(guidance_item(
            "CLIENT_SQLALCHEMY_ASYNCPG_PREPARED_CACHE",
            RiskLevel::High,
            "SQLAlchemy's asyncpg dialect prepares and caches statements per DBAPI connection",
            "set prepared_statement_cache_size=0, or use PgBouncer-safe dynamic prepared statement names plus NullPool and DISCARD cleanup",
            "https://docs.sqlalchemy.org/en/21/dialects/postgresql.html",
            uses_prepared_without_evidence(input),
        )),
        ClientLibraryKind::Postgrest if transactional => {
            guidance.push(guidance_item(
                "CLIENT_POSTGREST_EXTERNAL_POOLER",
                RiskLevel::High,
                "PostgREST requires prepared statements to be disabled for PgBouncer transaction pooling",
                "set db-prepared-statements=false and db-channel-enabled=false when using transaction pooling; avoid statement pooling",
                "https://docs.postgrest.org/en/v12/references/connection_pool.html",
                true,
            ));
            if input.mode == MultiplexingMode::Statement {
                guidance.push(guidance_item(
                    "CLIENT_POSTGREST_STATEMENT_POOLING_UNSUPPORTED",
                    RiskLevel::Critical,
                    "PostgREST does not support statement pooling",
                    "use PostgREST's internal pool, a session pooler, or transaction pooling with prepared statements and LISTEN disabled",
                    "https://docs.postgrest.org/en/v12/references/connection_pool.html",
                    true,
                ));
            }
        }
        ClientLibraryKind::PgJdbc if transactional && uses_prepared => guidance.push(guidance_item(
            "CLIENT_PGJDBC_PREPARE_THRESHOLD",
            RiskLevel::High,
            "JDBC prepared statements can conflict with transaction pooling unless disabled or backed by compatible pooler support",
            "add prepareThreshold=0 to disable prepared statements when PgBouncer prepared-statement tracking is unavailable",
            "https://www.pgbouncer.org/faq.html",
            uses_prepared_without_evidence(input),
        )),
        ClientLibraryKind::Unknown => guidance.push(guidance_item(
            "CLIENT_LIBRARY_UNKNOWN",
            RiskLevel::Medium,
            "client library behavior is unknown",
            "provide the client library or explicitly list session features so poolsim can apply source-backed rules",
            "https://www.pgbouncer.org/features.html",
            false,
        )),
        ClientLibraryKind::GenericPostgres | ClientLibraryKind::Prisma | ClientLibraryKind::NodePostgres | ClientLibraryKind::Sqlx | ClientLibraryKind::SqlalchemyAsyncpg | ClientLibraryKind::Postgrest | ClientLibraryKind::PgJdbc => {}
    }

    if input.pooler == ExternalPoolerKind::Supavisor
        && input.mode == MultiplexingMode::Transaction
        && uses_prepared
    {
        guidance.push(guidance_item(
            "PROVIDER_SUPAVISOR_TRANSACTION_PREPARED_STATEMENTS",
            RiskLevel::High,
            "Supabase documents that Supavisor transaction mode does not support prepared statements",
            "disable prepared statements for this client or use a session/direct endpoint for workloads that require them",
            "https://supabase.com/docs/guides/database/connecting-to-postgres",
            true,
        ));
    }

    if input.pooler == ExternalPoolerKind::RdsProxy && uses_prepared {
        guidance.push(guidance_item(
            "PROVIDER_RDS_PROXY_PINNING_REVIEW",
            RiskLevel::Medium,
            "RDS Proxy can pin sessions for stateful behavior, reducing multiplexing benefits",
            "measure pinning and remove prepared statements, SET state, temporary tables, and other session state before relying on backend reuse",
            "https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/rds-proxy-pinning.html",
            false,
        ));
    }

    guidance
}

fn uses_prepared_without_evidence(input: &SessionStateCompatibilityInput) -> bool {
    matches!(
        input.mode,
        MultiplexingMode::Transaction | MultiplexingMode::Statement
    ) && !matches!(input.pooler, ExternalPoolerKind::PgBouncer)
        || (input.pooler == ExternalPoolerKind::PgBouncer
            && input
                .pooler_config
                .as_ref()
                .and_then(|config| config.max_prepared_statements)
                .unwrap_or(0)
                == 0)
}

fn utilization(observed: Option<u32>, limit: Option<u32>) -> Option<f64> {
    observed.zip(limit).and_then(|(observed, limit)| {
        if limit == 0 {
            None
        } else {
            Some(f64::from(observed) / f64::from(limit))
        }
    })
}

fn max_evidence_status(
    current: PoolerEvidenceStatus,
    candidate: PoolerEvidenceStatus,
) -> PoolerEvidenceStatus {
    if evidence_status_rank(candidate) > evidence_status_rank(current) {
        candidate
    } else {
        current
    }
}

fn evidence_status_rank(status: PoolerEvidenceStatus) -> u8 {
    match status {
        PoolerEvidenceStatus::Healthy => 0,
        PoolerEvidenceStatus::NeedsReview => 1,
        PoolerEvidenceStatus::ClientWaiting => 2,
        PoolerEvidenceStatus::BackendSaturated => 3,
    }
}

fn merge_client_decision(
    pooler_report: &PoolerCompatibilityReport,
    guidance: &[ClientCompatibilityGuidance],
    confidence: EvidenceConfidence,
) -> CompatibilityDecision {
    if pooler_report.compatible == CompatibilityDecision::Incompatible
        || guidance.iter().any(|item| item.requires_change)
    {
        CompatibilityDecision::Incompatible
    } else if pooler_report.compatible == CompatibilityDecision::NeedsReview
        || confidence != EvidenceConfidence::High
        || guidance.iter().any(|item| item.risk >= RiskLevel::Medium)
    {
        CompatibilityDecision::NeedsReview
    } else {
        CompatibilityDecision::Compatible
    }
}

fn guidance_item(
    code: impl Into<String>,
    risk: RiskLevel,
    message: impl Into<String>,
    remediation: impl Into<String>,
    source_url: impl Into<String>,
    requires_change: bool,
) -> ClientCompatibilityGuidance {
    ClientCompatibilityGuidance {
        code: code.into(),
        risk,
        message: message.into(),
        remediation: remediation.into(),
        source_url: source_url.into(),
        requires_change,
    }
}

fn feature_finding(feature: SessionSemanticFeature, mode: MultiplexingMode) -> PoolerFinding {
    PoolerFinding::new(
        "POOLER_FEATURE_INCOMPATIBLE",
        RiskLevel::High,
        format!("{feature:?} is unsafe or incompatible with {mode:?} pooling"),
        "use a direct/session endpoint, remove the session-dependent feature, or provide source-backed pooler configuration that proves compatibility",
    )
}

fn review_finding(feature: SessionSemanticFeature, mode: MultiplexingMode) -> PoolerFinding {
    PoolerFinding::new(
        "POOLER_FEATURE_NEEDS_REVIEW",
        RiskLevel::Medium,
        format!("{feature:?} needs provider-specific review with {mode:?} pooling"),
        "verify provider documentation and live pooler configuration before treating this feature as safe",
    )
}

fn confidence_min(a: EvidenceConfidence, b: EvidenceConfidence) -> EvidenceConfidence {
    match (a, b) {
        (EvidenceConfidence::Low, _) | (_, EvidenceConfidence::Low) => EvidenceConfidence::Low,
        (EvidenceConfidence::Medium, _) | (_, EvidenceConfidence::Medium) => {
            EvidenceConfidence::Medium
        }
        (EvidenceConfidence::High, EvidenceConfidence::High) => EvidenceConfidence::High,
    }
}

fn redact_userinfo(base: &str) -> String {
    let Some(scheme_end) = base.find("://") else {
        return base.to_string();
    };
    let authority_start = scheme_end + 3;
    let rest = &base[authority_start..];
    let host_start = rest.find(['/', '@']).and_then(|idx| {
        if rest.as_bytes()[idx] == b'@' {
            Some(authority_start + idx + 1)
        } else {
            None
        }
    });

    if let Some(host_start) = host_start {
        format!(
            "{}://<redacted>@{}",
            &base[..scheme_end],
            &base[host_start..]
        )
    } else {
        base.to_string()
    }
}

fn redact_query(query: &str) -> String {
    query
        .split('&')
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            if is_secret_query_key(key) {
                format!("{key}=<redacted>")
            } else if value.is_empty() {
                key.to_string()
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn is_secret_query_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("token")
        || lower.contains("secret")
        || lower == "apikey"
        || lower == "api_key"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_userinfo_and_secret_query_values() {
        let redacted = redact_endpoint(
            "postgres://alice:secret@db.example.com/app?sslmode=require&password=secret&token=abc",
        );
        assert_eq!(
            redacted,
            "postgres://<redacted>@db.example.com/app?sslmode=require&password=<redacted>&token=<redacted>"
        );
    }

    #[test]
    fn classifies_known_provider_endpoint_patterns() {
        let supabase = classify_endpoint(&EndpointClassificationInput::new(
            "postgres://u:p@aws-0-us.pooler.supabase.com:6543/postgres",
        ));
        assert_eq!(supabase.provider, EndpointProviderKind::Supabase);
        assert_eq!(
            supabase.endpoint_kind,
            EndpointConnectionKind::TransactionPooler
        );
        assert_eq!(supabase.confidence, EvidenceConfidence::Medium);

        let neon = classify_endpoint(&EndpointClassificationInput::new(
            "postgres://u:p@ep-name-pooler.us-east-2.aws.neon.tech/neondb",
        ));
        assert_eq!(neon.provider, EndpointProviderKind::Neon);
        assert_eq!(
            neon.endpoint_kind,
            EndpointConnectionKind::TransactionPooler
        );

        let rds_proxy = classify_endpoint(&EndpointClassificationInput::new(
            "postgres://u:p@my-proxy.proxy-abc.us-east-1.rds.amazonaws.com/app",
        ));
        assert_eq!(rds_proxy.provider, EndpointProviderKind::AwsRdsProxy);
        assert_eq!(
            rds_proxy.endpoint_kind,
            EndpointConnectionKind::DatabaseProxy
        );
    }

    #[test]
    fn workflow_mismatch_requires_direct_endpoint_for_migrations() {
        let report = classify_endpoint(
            &EndpointClassificationInput::new(
                "postgres://u:p@aws-0-us.pooler.supabase.com:6543/postgres",
            )
            .with_workflow(DatabaseWorkflowKind::Migration),
        );

        assert_eq!(report.workflow_compatible, Some(false));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "ENDPOINT_WORKFLOW_MISMATCH"));
    }

    #[test]
    fn transaction_pooling_rejects_session_features() {
        let report = check_pooler_compatibility(
            &PoolerCompatibilityInput::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_features(vec![
                SessionSemanticFeature::TemporaryTables,
                SessionSemanticFeature::AdvisoryLocks,
            ]),
        );

        assert_eq!(report.compatible, CompatibilityDecision::Incompatible);
        assert_eq!(report.incompatible_features.len(), 2);
    }

    #[test]
    fn pgbouncer_prepared_statements_need_config() {
        let unsafe_report = check_pooler_compatibility(
            &PoolerCompatibilityInput::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_features(vec![SessionSemanticFeature::PreparedStatements]),
        );
        assert_eq!(
            unsafe_report.compatible,
            CompatibilityDecision::Incompatible
        );

        let safe_report = check_pooler_compatibility(
            &PoolerCompatibilityInput::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_features(vec![SessionSemanticFeature::PreparedStatements])
            .with_pooler_config(PoolerConfigSnapshot::new().with_max_prepared_statements(100)),
        );
        assert_eq!(safe_report.compatible, CompatibilityDecision::Compatible);
    }

    #[test]
    fn session_state_analysis_reuses_pooler_check_and_adds_client_guidance() {
        let report = analyze_session_state_compatibility(
            &SessionStateCompatibilityInput::new(
                ClientLibraryKind::Sqlx,
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_pooler_config(PoolerConfigSnapshot::new().with_max_prepared_statements(100)),
        );

        assert_eq!(report.compatible, CompatibilityDecision::NeedsReview);
        assert!(report
            .effective_features
            .contains(&SessionSemanticFeature::PreparedStatements));
        assert_eq!(
            report.pooler_report.compatible,
            CompatibilityDecision::Compatible
        );
        assert!(report
            .client_guidance
            .iter()
            .any(|item| item.code == "CLIENT_SQLX_STATEMENT_CACHE"));
    }

    #[test]
    fn session_state_analysis_marks_supavisor_prepared_statements_incompatible() {
        let report = analyze_session_state_compatibility(&SessionStateCompatibilityInput::new(
            ClientLibraryKind::Prisma,
            ExternalPoolerKind::Supavisor,
            MultiplexingMode::Transaction,
        ));

        assert_eq!(report.compatible, CompatibilityDecision::Incompatible);
        assert!(report.client_guidance.iter().any(|item| item.code
            == "PROVIDER_SUPAVISOR_TRANSACTION_PREPARED_STATEMENTS"
            && item.requires_change));
    }

    #[test]
    fn session_state_analysis_covers_node_postgres_and_postgrest_branches() {
        let node_report = analyze_session_state_compatibility(
            &SessionStateCompatibilityInput::new(
                ClientLibraryKind::NodePostgres,
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_features(vec![SessionSemanticFeature::NamedPreparedStatements]),
        );
        assert_eq!(node_report.compatible, CompatibilityDecision::Incompatible);
        assert!(node_report
            .client_guidance
            .iter()
            .any(|item| item.code == "CLIENT_NODE_PG_NAMED_PREPARED_STATEMENTS"));

        let postgrest_report = analyze_session_state_compatibility(
            &SessionStateCompatibilityInput::new(
                ClientLibraryKind::Postgrest,
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Statement,
            )
            .with_pooler_config(PoolerConfigSnapshot::new().with_max_prepared_statements(100)),
        );
        assert_eq!(
            postgrest_report.compatible,
            CompatibilityDecision::Incompatible
        );
        assert!(postgrest_report
            .client_guidance
            .iter()
            .any(|item| item.code == "CLIENT_POSTGREST_STATEMENT_POOLING_UNSUPPORTED"));
    }

    #[test]
    fn pooler_evidence_summarizes_healthy_client_and_backend_counts() {
        let report = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_label("checkout-api")
            .with_client_active(42)
            .with_client_waiting(0)
            .with_server_active(8)
            .with_server_idle(7)
            .with_pooler_client_limit(500)
            .with_pooler_backend_limit(30),
        );

        assert_eq!(report.status, PoolerEvidenceStatus::Healthy);
        assert_eq!(report.observed_client_connections, Some(42));
        assert_eq!(report.observed_backend_connections, Some(15));
        assert_eq!(report.backend_utilization, Some(0.5));
        assert_eq!(report.client_utilization, Some(0.084));
    }

    #[test]
    fn pooler_evidence_flags_waiting_clients_and_backend_saturation() {
        let report = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_client_active(100)
            .with_client_waiting(5)
            .with_server_active(25)
            .with_server_idle(5)
            .with_pooler_backend_limit(30),
        );

        assert_eq!(report.status, PoolerEvidenceStatus::BackendSaturated);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "POOLER_CLIENTS_WAITING"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "POOLER_BACKEND_SATURATED"));
    }

    #[test]
    fn pooler_evidence_incomplete_counts_need_review() {
        let report = summarize_pooler_evidence(&PoolerEvidenceSnapshot::new(
            ExternalPoolerKind::Unknown,
            MultiplexingMode::Unknown,
        ));

        assert_eq!(report.status, PoolerEvidenceStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Low);
        assert!(report.observed_client_connections.is_none());
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "POOLER_EVIDENCE_COUNTS_INCOMPLETE"));
    }

    #[test]
    fn downstream_diagnosis_identifies_pooler_bottleneck_with_app_headroom() {
        let pooler = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_client_active(180)
            .with_client_waiting(4)
            .with_server_active(30)
            .with_server_idle(0)
            .with_pooler_backend_limit(30),
        );
        let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
            ApplicationPoolEvidence::new(4, 16),
            pooler,
        ));

        assert_eq!(
            report.status,
            DownstreamPoolerDiagnosisStatus::DownstreamPoolerSaturated
        );
        assert_eq!(report.application_utilization, Some(0.25));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "DOWNSTREAM_POOLER_BACKEND_SATURATED"));
        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.code == "APPLICATION_POOL_SATURATED"));
    }

    #[test]
    fn downstream_diagnosis_preserves_application_and_pooler_findings() {
        let pooler = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_client_active(16)
            .with_client_waiting(2)
            .with_server_active(16)
            .with_server_idle(0)
            .with_pooler_backend_limit(16),
        );
        let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
            ApplicationPoolEvidence::new(16, 16).with_waiting(3),
            pooler,
        ));

        assert_eq!(
            report.status,
            DownstreamPoolerDiagnosisStatus::DownstreamPoolerSaturated
        );
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "APPLICATION_POOL_SATURATED"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "APPLICATION_REQUESTS_WAITING"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "DOWNSTREAM_POOLER_CLIENTS_WAITING"));
    }

    #[test]
    fn downstream_diagnosis_reports_waiting_pooler_before_app_near_limit() {
        let pooler = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_client_active(80)
            .with_client_waiting(1)
            .with_server_active(10)
            .with_server_idle(0)
            .with_pooler_backend_limit(20),
        );
        let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
            ApplicationPoolEvidence::new(8, 10),
            pooler,
        ));

        assert_eq!(
            report.status,
            DownstreamPoolerDiagnosisStatus::DownstreamPoolerWaiting
        );
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "APPLICATION_POOL_NEAR_LIMIT"));
    }

    #[test]
    fn downstream_diagnosis_requires_review_for_missing_app_capacity() {
        let pooler = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_client_active(8)
            .with_client_waiting(0)
            .with_server_active(4)
            .with_server_idle(4)
            .with_pooler_backend_limit(16),
        );
        let report = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
            ApplicationPoolEvidence::default(),
            pooler,
        ));

        assert_eq!(report.status, DownstreamPoolerDiagnosisStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Medium);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "APPLICATION_POOL_EVIDENCE_INCOMPLETE"));
    }

    #[test]
    fn pgbouncer_show_pools_parser_accepts_psql_aligned_output() {
        let rows = parse_pgbouncer_show_pools(
            r#"
 database | user | cl_active | cl_waiting | sv_active | sv_idle | pool_mode
----------+------+-----------+------------+-----------+---------+-----------
 app      | web  |        40 |          0 |         8 |       7 | transaction
 app      | jobs |         2 |          1 |         1 |       0 | transaction
(2 rows)
"#,
        )
        .expect("aligned PgBouncer output should parse");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].database.as_deref(), Some("app"));
        assert_eq!(rows[0].user.as_deref(), Some("web"));
        assert_eq!(rows[0].cl_active, 40);
        assert_eq!(rows[0].pool_mode, Some(MultiplexingMode::Transaction));
        assert_eq!(rows[1].cl_waiting, 1);
    }

    #[test]
    fn pgbouncer_show_pools_parser_accepts_csv_output() {
        let rows = parse_pgbouncer_show_pools(
            r#"database,user,cl_active,cl_waiting,sv_active,sv_idle,pool_mode
"checkout,primary","web",42,0,8,7,transaction
"#,
        )
        .expect("CSV PgBouncer output should parse");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].database.as_deref(), Some("checkout,primary"));
        assert_eq!(rows[0].cl_active, 42);
        assert_eq!(rows[0].sv_idle, 7);
    }

    #[test]
    fn pgbouncer_show_pools_summary_aggregates_rows() {
        let rows = vec![
            PgbouncerPoolRow::new(40, 0, 8, 7)
                .with_database("app")
                .with_user("web")
                .with_pool_mode(MultiplexingMode::Transaction),
            PgbouncerPoolRow::new(2, 1, 1, 0)
                .with_database("app")
                .with_user("jobs")
                .with_pool_mode(MultiplexingMode::Transaction),
        ];
        let report = summarize_pgbouncer_show_pools(
            &PgbouncerShowPoolsSnapshot::new(rows)
                .with_label("checkout-pgbouncer")
                .with_pooler_client_limit(500)
                .with_pooler_backend_limit(30),
        )
        .expect("PgBouncer summary should build");

        assert_eq!(report.status, PoolerEvidenceStatus::ClientWaiting);
        assert_eq!(report.pooler, ExternalPoolerKind::PgBouncer);
        assert_eq!(report.mode, MultiplexingMode::Transaction);
        assert_eq!(report.label.as_deref(), Some("checkout-pgbouncer"));
        assert_eq!(report.observed_client_connections, Some(43));
        assert_eq!(report.observed_backend_connections, Some(16));
        assert_eq!(report.client_waiting, Some(1));
    }

    #[test]
    fn pgbouncer_show_pools_parser_rejects_missing_required_columns() {
        let err = parse_pgbouncer_show_pools(
            r#"database,user,cl_active,cl_waiting,sv_active,pool_mode
app,web,42,0,8,transaction
"#,
        )
        .expect_err("missing sv_idle should fail");

        assert_eq!(err.code(), "INVALID_PGBOUNCER_SHOW_POOLS");
    }

    #[test]
    fn pgbouncer_show_stats_parser_accepts_csv_and_aligned_output() {
        let csv = parse_pgbouncer_show_stats(
            "database,total_query_count,total_wait_time,avg_wait_time\napp,100,50000,500\n",
        )
        .expect("CSV PgBouncer stats should parse");
        assert_eq!(csv.len(), 1);
        assert_eq!(csv[0].database.as_deref(), Some("app"));
        assert_eq!(csv[0].total_query_count, 100);
        assert_eq!(csv[0].total_wait_time_us, 50_000);
        assert_eq!(csv[0].avg_wait_time_us, Some(500.0));

        let aligned = parse_pgbouncer_show_stats(
            r#"
 database | total_query_count | total_wait_time | avg_wait_time
----------+--------------------+-----------------+---------------
 app      |                100 |           50000 |           500
(1 row)
"#,
        )
        .expect("aligned PgBouncer stats should parse");
        assert_eq!(aligned, csv);
    }

    #[test]
    fn pgbouncer_show_stats_summary_aggregates_counters() {
        let sample = summarize_pgbouncer_show_stats(
            &PgbouncerShowStatsSnapshot::new(
                100.0,
                vec![
                    PgbouncerStatsRow::new(100, 50_000),
                    PgbouncerStatsRow::new(25, 10_000),
                ],
            )
            .with_label("checkout-pgbouncer")
            .with_maxwait_seconds(0.25)
            .with_client_waiting(2),
        )
        .expect("stats summary should build");

        assert_eq!(sample.total_query_count, 125);
        assert_eq!(sample.total_wait_time_us, 60_000);
        assert_eq!(sample.label.as_deref(), Some("checkout-pgbouncer"));
        assert_eq!(sample.maxwait_seconds, Some(0.25));
        assert_eq!(sample.client_waiting, Some(2));
    }

    #[test]
    fn pgbouncer_time_series_diff_calculates_rates_and_healthy_status() {
        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 100.0,
            label: Some("checkout".to_string()),
            total_query_count: 1_000,
            total_wait_time_us: 20_000,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 110.0,
            label: Some("checkout".to_string()),
            total_query_count: 1_200,
            total_wait_time_us: 70_000,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };

        let report = diff_pgbouncer_time_series(&previous, &current)
            .expect("healthy time-series diff should build");

        assert_eq!(report.status, PgbouncerTimeSeriesStatus::Healthy);
        assert_eq!(report.interval_seconds, 10.0);
        assert_eq!(report.query_rate_per_second, 20.0);
        assert_eq!(report.wait_time_rate_us_per_second, 5_000.0);
        assert_eq!(report.average_wait_ms_per_query, Some(0.25));
        assert!(report.counter_resets.is_empty());
        assert_eq!(report.confidence, EvidenceConfidence::High);
    }

    #[test]
    fn pgbouncer_time_series_diff_flags_growing_queue() {
        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 100.0,
            label: None,
            total_query_count: 100,
            total_wait_time_us: 10_000,
            maxwait_seconds: Some(0.1),
            client_waiting: Some(1),
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 105.0,
            label: None,
            total_query_count: 200,
            total_wait_time_us: 30_000,
            maxwait_seconds: Some(0.8),
            client_waiting: Some(4),
        };

        let report = diff_pgbouncer_time_series(&previous, &current)
            .expect("queue-growth diff should build");

        assert_eq!(report.status, PgbouncerTimeSeriesStatus::QueueGrowing);
        assert!((report.maxwait_delta_seconds.expect("maxwait delta") - 0.7).abs() < f64::EPSILON);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "PGBOUNCER_QUEUE_GROWING"));
    }

    #[test]
    fn pgbouncer_time_series_diff_reports_counter_resets_without_negative_rates() {
        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 100.0,
            label: None,
            total_query_count: 1_000,
            total_wait_time_us: 50_000,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 110.0,
            label: None,
            total_query_count: 25,
            total_wait_time_us: 2_000,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };

        let report = diff_pgbouncer_time_series(&previous, &current)
            .expect("counter-reset diff should build");

        assert_eq!(report.status, PgbouncerTimeSeriesStatus::CounterReset);
        assert_eq!(report.query_rate_per_second, 2.5);
        assert_eq!(report.wait_time_rate_us_per_second, 200.0);
        assert_eq!(
            report.counter_resets,
            vec![
                PgbouncerCounterKind::TotalQueryCount,
                PgbouncerCounterKind::TotalWaitTime
            ]
        );
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "PGBOUNCER_COUNTER_RESET"));
    }

    #[test]
    fn pgbouncer_time_series_diff_requires_complete_gauges_for_healthy_status() {
        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 100.0,
            label: None,
            total_query_count: 100,
            total_wait_time_us: 10_000,
            maxwait_seconds: None,
            client_waiting: None,
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 110.0,
            label: None,
            total_query_count: 200,
            total_wait_time_us: 20_000,
            maxwait_seconds: None,
            client_waiting: None,
        };

        let report = diff_pgbouncer_time_series(&previous, &current)
            .expect("incomplete-gauge diff should build");

        assert_eq!(report.status, PgbouncerTimeSeriesStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Medium);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "PGBOUNCER_MAXWAIT_MISSING"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "PGBOUNCER_WAITING_GAUGE_MISSING"));
    }

    #[test]
    fn pgbouncer_time_series_rejects_invalid_inputs() {
        let stats_error = parse_pgbouncer_show_stats("database,total_query_count\napp,100\n")
            .expect_err("missing total_wait_time should fail");
        assert_eq!(stats_error.code(), "INVALID_PGBOUNCER_SHOW_STATS");

        let summary_error = summarize_pgbouncer_show_stats(&PgbouncerShowStatsSnapshot::new(
            f64::NAN,
            vec![PgbouncerStatsRow::new(1, 1)],
        ))
        .expect_err("non-finite timestamp should fail");
        assert_eq!(summary_error.code(), "INVALID_PGBOUNCER_TIME_SERIES");

        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 10.0,
            label: None,
            total_query_count: 1,
            total_wait_time_us: 1,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 10.0,
            ..previous.clone()
        };
        let diff_error = diff_pgbouncer_time_series(&previous, &current)
            .expect_err("non-increasing timestamps should fail");
        assert_eq!(diff_error.code(), "INVALID_PGBOUNCER_TIME_SERIES");
    }

    #[test]
    fn pgbouncer_show_pools_summary_rejects_empty_snapshots() {
        let err = summarize_pgbouncer_show_pools(&PgbouncerShowPoolsSnapshot::new(Vec::new()))
            .expect_err("empty PgBouncer snapshots should fail");

        assert_eq!(err.code(), "INVALID_PGBOUNCER_SHOW_POOLS");
    }

    #[test]
    fn pooler_builders_and_provider_matrix_cover_supported_evidence() {
        let endpoint = EndpointClassificationInput::new("postgres://db.example.com")
            .with_provider(EndpointProviderKind::AwsRds)
            .with_workflow(DatabaseWorkflowKind::ApiTraffic);
        assert_eq!(endpoint.provider, Some(EndpointProviderKind::AwsRds));

        let config = PoolerConfigSnapshot::new()
            .with_max_prepared_statements(10)
            .with_resets_session_state(true);
        assert_eq!(config.resets_session_state, Some(true));

        let compatibility = PoolerCompatibilityInput::new(
            ExternalPoolerKind::PgBouncer,
            MultiplexingMode::Transaction,
        )
        .with_workflow(DatabaseWorkflowKind::Migration)
        .with_pooler_config(config.clone());
        assert_eq!(
            compatibility.workflow,
            Some(DatabaseWorkflowKind::Migration)
        );

        let row = PgbouncerPoolRow::new(1, 0, 1, 1)
            .with_database("app")
            .with_user("web")
            .with_pool_mode(MultiplexingMode::Session);
        let snapshot = PgbouncerShowPoolsSnapshot::new(vec![row])
            .with_mode(MultiplexingMode::Session)
            .with_pooler_client_limit(20)
            .with_pooler_backend_limit(10);
        assert_eq!(snapshot.mode, Some(MultiplexingMode::Session));

        let stats = PgbouncerStatsRow::new(2, 10).with_avg_wait_time_us(5.0);
        assert_eq!(stats.avg_wait_time_us, Some(5.0));

        let application = ApplicationPoolEvidence::new(2, 4)
            .with_waiting(1)
            .with_active(Some(3))
            .with_limit(Some(5));
        assert_eq!(application.active, Some(3));
        assert_eq!(application.limit, Some(5));

        let session = SessionStateCompatibilityInput::new(
            ClientLibraryKind::GenericPostgres,
            ExternalPoolerKind::PgBouncer,
            MultiplexingMode::Session,
        )
        .with_workflow(DatabaseWorkflowKind::ApiTraffic)
        .with_pooler_config(config);
        assert_eq!(session.workflow, Some(DatabaseWorkflowKind::ApiTraffic));

        for endpoint in [
            "postgres://u:p@accelerate.prisma-data.net/db",
            "postgres://u:p@db.cloudflare-hyperdrive.com/db",
            "postgres://u:p@db.rds.amazonaws.com/db",
            "postgres://u:p@db.example.com:6432/db",
            "postgres://u:p@db.example.com/db",
        ] {
            let _ = infer_provider_from_endpoint(endpoint);
        }

        assert_eq!(
            infer_provider_from_endpoint("postgres://u:p@accelerate.prisma-data.net/db"),
            EndpointProviderKind::PrismaPostgres
        );
        assert_eq!(
            infer_provider_from_endpoint("postgres://u:p@db.cloudflare-hyperdrive.com/db"),
            EndpointProviderKind::CloudflareHyperdrive
        );
        assert_eq!(
            infer_provider_from_endpoint("postgres://u:p@db.rds.amazonaws.com/db"),
            EndpointProviderKind::AwsRds
        );
        assert_eq!(
            infer_provider_from_endpoint("postgres://u:p@db.example.com:6432/db"),
            EndpointProviderKind::PgBouncer
        );
        assert_eq!(
            infer_provider_from_endpoint("postgres://u:p@db.example.com/db"),
            EndpointProviderKind::Unknown
        );
        let unknown_report = classify_endpoint(&EndpointClassificationInput::new(
            "postgres://db.example.com/db",
        ));
        assert_eq!(unknown_report.confidence, EvidenceConfidence::Low);
        let hinted_report = classify_endpoint(
            &EndpointClassificationInput::new("postgres://db.example.com/db")
                .with_provider(EndpointProviderKind::AwsRds),
        );
        assert_eq!(hinted_report.confidence, EvidenceConfidence::High);

        assert_eq!(
            infer_endpoint_kind(
                "postgres://u:p@pooler.supabase.com/db",
                EndpointProviderKind::Supabase
            ),
            EndpointConnectionKind::SessionPooler
        );
        assert_eq!(
            infer_endpoint_kind(
                "postgres://u:p@db.supabase.co/db",
                EndpointProviderKind::Supabase
            ),
            EndpointConnectionKind::DirectDatabase
        );
        assert_eq!(
            infer_endpoint_kind(
                "postgres://u:p@supabase.example/db",
                EndpointProviderKind::Supabase
            ),
            EndpointConnectionKind::Unknown
        );
        assert_eq!(
            infer_endpoint_kind(
                "postgres://u:p@pooler.neon.tech/db",
                EndpointProviderKind::Neon
            ),
            EndpointConnectionKind::TransactionPooler
        );
        assert_eq!(
            infer_endpoint_kind("postgres://u:p@db.neon.tech/db", EndpointProviderKind::Neon),
            EndpointConnectionKind::DirectDatabase
        );
        assert_eq!(
            infer_endpoint_kind("postgres://u:p@neon.example/db", EndpointProviderKind::Neon),
            EndpointConnectionKind::Unknown
        );
        assert_eq!(
            infer_endpoint_kind(
                "prisma+postgres://accelerate/db",
                EndpointProviderKind::PrismaPostgres
            ),
            EndpointConnectionKind::HttpDataApi
        );
        assert_eq!(
            infer_endpoint_kind(
                "postgres://pool.example/db",
                EndpointProviderKind::PrismaPostgres
            ),
            EndpointConnectionKind::TransactionPooler
        );
        assert_eq!(
            infer_endpoint_kind(
                "postgres://db.example/db",
                EndpointProviderKind::PrismaPostgres
            ),
            EndpointConnectionKind::Unknown
        );
        assert_eq!(
            infer_endpoint_kind("postgres://db.example/db", EndpointProviderKind::AwsRds),
            EndpointConnectionKind::DirectDatabase
        );
        assert_eq!(
            infer_endpoint_kind(
                "https://db.cloudflare.com",
                EndpointProviderKind::CloudflareHyperdrive
            ),
            EndpointConnectionKind::EdgePooler
        );
        for (endpoint, expected) in [
            ("statement", EndpointConnectionKind::StatementPooler),
            ("transaction", EndpointConnectionKind::TransactionPooler),
            ("session", EndpointConnectionKind::SessionPooler),
            ("other", EndpointConnectionKind::Unknown),
        ] {
            assert_eq!(
                infer_endpoint_kind(endpoint, EndpointProviderKind::PgBouncer),
                expected
            );
        }
        assert_eq!(
            infer_endpoint_kind("unknown", EndpointProviderKind::Unknown),
            EndpointConnectionKind::Unknown
        );
    }

    #[test]
    fn pooler_compatibility_and_diagnosis_cover_review_and_healthy_paths() {
        let review = check_pooler_compatibility(&PoolerCompatibilityInput::new(
            ExternalPoolerKind::Unknown,
            MultiplexingMode::Unknown,
        ));
        assert_eq!(review.compatible, CompatibilityDecision::NeedsReview);
        assert_eq!(review.confidence, EvidenceConfidence::Low);

        let near_limit = summarize_pooler_evidence(
            &PoolerEvidenceSnapshot::new(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_server_active(24)
            .with_server_idle(0)
            .with_pooler_backend_limit(30),
        );
        assert_eq!(near_limit.status, PoolerEvidenceStatus::NeedsReview);
        assert!(near_limit
            .findings
            .iter()
            .any(|finding| finding.code == "POOLER_BACKEND_NEAR_LIMIT"));

        let healthy = diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
            ApplicationPoolEvidence::new(2, 10),
            summarize_pooler_evidence(
                &PoolerEvidenceSnapshot::new(
                    ExternalPoolerKind::PgBouncer,
                    MultiplexingMode::Transaction,
                )
                .with_client_active(2)
                .with_client_waiting(0)
                .with_server_active(2)
                .with_server_idle(8)
                .with_pooler_client_limit(20)
                .with_pooler_backend_limit(20),
            ),
        ));
        assert_eq!(healthy.status, DownstreamPoolerDiagnosisStatus::Healthy);

        let application_saturated =
            diagnose_downstream_pooler(&DownstreamPoolerDiagnosisInput::new(
                ApplicationPoolEvidence::new(10, 10),
                summarize_pooler_evidence(
                    &PoolerEvidenceSnapshot::new(
                        ExternalPoolerKind::PgBouncer,
                        MultiplexingMode::Transaction,
                    )
                    .with_client_active(2)
                    .with_client_waiting(0)
                    .with_server_active(2)
                    .with_server_idle(8)
                    .with_pooler_client_limit(20)
                    .with_pooler_backend_limit(20),
                ),
            ));
        assert_eq!(
            application_saturated.status,
            DownstreamPoolerDiagnosisStatus::ApplicationPoolSaturated
        );

        let _ = analyze_session_state_compatibility(
            &SessionStateCompatibilityInput::new(
                ClientLibraryKind::GenericPostgres,
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
            )
            .with_workflow(DatabaseWorkflowKind::Migration),
        );
        assert_eq!(
            evidence_status_rank(PoolerEvidenceStatus::BackendSaturated),
            3
        );
        assert_eq!(utilization(Some(1), Some(0)), None);
        assert_eq!(
            merge_client_decision(
                &PoolerCompatibilityReport {
                    compatible: CompatibilityDecision::Compatible,
                    incompatible_features: Vec::new(),
                    migration_direct_connection_required: false,
                    long_running_direct_connection_required: false,
                    findings: Vec::new(),
                    confidence: EvidenceConfidence::High,
                },
                &[],
                EvidenceConfidence::High,
            ),
            CompatibilityDecision::Compatible
        );
        assert_eq!(
            confidence_min(EvidenceConfidence::High, EvidenceConfidence::High),
            EvidenceConfidence::High
        );
    }

    #[test]
    fn pooler_client_guidance_and_parser_errors_cover_edge_inputs() {
        for client in [
            ClientLibraryKind::SqlalchemyAsyncpg,
            ClientLibraryKind::PgJdbc,
            ClientLibraryKind::Unknown,
        ] {
            let report = analyze_session_state_compatibility(
                &SessionStateCompatibilityInput::new(
                    client,
                    ExternalPoolerKind::RdsProxy,
                    MultiplexingMode::Transaction,
                )
                .with_features(vec![SessionSemanticFeature::PreparedStatements]),
            );
            assert!(!report.client_guidance.is_empty());
        }

        for feature in [
            SessionSemanticFeature::NotifyOnly,
            SessionSemanticFeature::Unknown,
        ] {
            assert!(!feature_incompatible(
                ExternalPoolerKind::PgBouncer,
                MultiplexingMode::Transaction,
                feature,
                None,
            ));
        }
        assert!(!feature_incompatible(
            ExternalPoolerKind::RdsProxy,
            MultiplexingMode::Session,
            SessionSemanticFeature::TemporaryTables,
            None,
        ));
        assert!(feature_needs_review(
            ExternalPoolerKind::RdsProxy,
            MultiplexingMode::Unknown,
            SessionSemanticFeature::Unknown,
            None,
        ));
        assert!(feature_needs_review(
            ExternalPoolerKind::RdsProxy,
            MultiplexingMode::Transaction,
            SessionSemanticFeature::PreparedStatements,
            None,
        ));
        assert!(feature_needs_review(
            ExternalPoolerKind::Unknown,
            MultiplexingMode::Transaction,
            SessionSemanticFeature::PreparedStatements,
            None,
        ));
        assert!(endpoint_workflow_compatible(
            EndpointConnectionKind::TransactionPooler,
            DatabaseWorkflowKind::ApiTraffic,
        ));
        assert_eq!(
            default_client_features(ClientLibraryKind::Unknown),
            Vec::<SessionSemanticFeature>::new()
        );
        assert_eq!(
            confidence_min(EvidenceConfidence::High, EvidenceConfidence::High),
            EvidenceConfidence::High
        );

        for mode in [
            "session",
            "transaction",
            "statement",
            "none",
            "provider-managed",
            "provider_managed",
            "unknown",
        ] {
            assert!(optional_pgbouncer_mode(&[mode.to_string()], Some(0)).is_ok());
        }
        assert!(optional_pgbouncer_mode(&["invalid".to_string()], Some(0)).is_err());
        assert_eq!(optional_pgbouncer_mode(&[], None).unwrap(), None);
        assert_eq!(optional_pgbouncer_cell(&[" ".to_string()], Some(0)), None);
        assert_eq!(optional_pgbouncer_cell(&[], Some(0)), None);
        assert!(optional_pgbouncer_float(&["bad".to_string()], Some(0), "avg").is_err());
        assert!(optional_pgbouncer_float(&["-1".to_string()], Some(0), "avg").is_err());
        assert_eq!(optional_pgbouncer_float(&[], None, "avg").unwrap(), None);
        assert!(parse_pgbouncer_count(&[], 0, "cl_active").is_err());
        assert!(parse_pgbouncer_stats_count(&[], 0, "total_query_count").is_err());

        let quoted = split_csv_record("\"app\"\"east\",web").expect("escaped quote should parse");
        assert_eq!(quoted, vec!["app\"east".to_string(), "web".to_string()]);

        assert!(parse_pgbouncer_show_pools("not a header\n").is_err());
        assert!(parse_pgbouncer_show_pools(
            "database,user,cl_active,cl_waiting,sv_active,sv_idle\n"
        )
        .is_err());
        assert!(parse_pgbouncer_show_pools(
            "database,user,cl_active,cl_waiting,sv_active,sv_idle\na,b,1\n"
        )
        .is_err());
        assert!(parse_pgbouncer_show_pools(
            "database,user,cl_active,cl_waiting,sv_active,sv_idle\na,b,bad,0,1,1\n"
        )
        .is_err());
        assert!(parse_pgbouncer_show_pools(
            "database,user,cl_active,cl_waiting,sv_active,sv_idle,pool_mode\n\"unterminated,a,1,0,1,1,transaction\n"
        )
        .is_err());

        assert!(parse_pgbouncer_show_stats("not a header\n").is_err());
        assert!(
            parse_pgbouncer_show_stats("database,total_query_count,total_wait_time\na,1\n")
                .is_err()
        );
        assert!(parse_pgbouncer_show_stats(
            "database,total_query_count,total_wait_time\na,bad,1\n"
        )
        .is_err());
        assert!(
            parse_pgbouncer_show_stats("database,total_query_count,total_wait_time\n").is_err()
        );
        assert!(
            summarize_pgbouncer_show_stats(&PgbouncerShowStatsSnapshot::new(1.0, Vec::new(),))
                .is_err()
        );
        assert!(summarize_pgbouncer_show_stats(
            &PgbouncerShowStatsSnapshot::new(1.0, vec![PgbouncerStatsRow::new(1, 1)])
                .with_maxwait_seconds(-1.0),
        )
        .is_err());

        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: f64::NAN,
            label: None,
            total_query_count: 1,
            total_wait_time_us: 1,
            maxwait_seconds: Some(0.0),
            client_waiting: Some(0),
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 2.0,
            ..previous.clone()
        };
        assert!(diff_pgbouncer_time_series(&previous, &current).is_err());
        let previous = PgbouncerTimeSeriesSample {
            timestamp_seconds: 1.0,
            ..current.clone()
        };
        let current = PgbouncerTimeSeriesSample {
            timestamp_seconds: 2.0,
            maxwait_seconds: Some(-1.0),
            ..current
        };
        assert!(diff_pgbouncer_time_series(&previous, &current).is_err());

        let queue_present = diff_pgbouncer_time_series(
            &PgbouncerTimeSeriesSample {
                timestamp_seconds: 1.0,
                label: None,
                total_query_count: 1,
                total_wait_time_us: 1,
                maxwait_seconds: Some(0.5),
                client_waiting: Some(0),
            },
            &PgbouncerTimeSeriesSample {
                timestamp_seconds: 2.0,
                label: None,
                total_query_count: 2,
                total_wait_time_us: 2,
                maxwait_seconds: Some(0.5),
                client_waiting: Some(0),
            },
        )
        .expect("queue-present diff should build");
        assert_eq!(
            queue_present.status,
            PgbouncerTimeSeriesStatus::QueuePresent
        );

        assert_eq!(
            redact_userinfo("postgres://db.example.com/db"),
            "postgres://db.example.com/db"
        );
        assert_eq!(redact_userinfo("db.example.com"), "db.example.com");
        assert_eq!(
            redact_query("flag&password=secret&token=&mode=fast"),
            "flag&password=<redacted>&token=<redacted>&mode=fast"
        );
        assert_eq!(normalize_column(" total-query-count "), "total_query_count");
        assert!(is_psql_separator("---+---"));
    }
}
