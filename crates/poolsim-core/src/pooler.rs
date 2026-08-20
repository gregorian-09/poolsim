//! Endpoint classification and external-pooler compatibility checks.
//!
//! This module is intentionally conservative. It helps callers distinguish
//! direct database connections from pooled/proxied endpoints, then checks
//! whether application session features are compatible with the selected
//! pooling mode. Unknown provider behavior lowers confidence instead of being
//! treated as safe.

use serde::{Deserialize, Serialize};

use crate::types::RiskLevel;

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
}
