//! Connection ownership graph for database capacity planning.
//!
//! A pool size is only meaningful when users know which layer owns that pool
//! and whether it consumes real backend database connections. This module maps
//! application pools, external poolers, and database backends into an explicit
//! graph with conservative capacity findings.

use serde::{Deserialize, Serialize};

use crate::{
    error::PoolsimError,
    pooler::{EndpointConnectionKind, EvidenceConfidence, ExternalPoolerKind, PoolerFinding},
    types::RiskLevel,
};

/// Connection layer represented in a connection ownership graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ConnectionLayerKind {
    /// Application process, replica, worker, function, or execution environment.
    ApplicationRuntime,
    /// Application-side driver/framework pool.
    ApplicationPool,
    /// Client-facing side of an external pooler or proxy.
    ExternalPoolerClient,
    /// Backend-facing side of an external pooler or proxy.
    ExternalPoolerBackend,
    /// Real database backend connection/session capacity.
    DatabaseBackend,
    /// HTTP or Data API layer that hides direct socket ownership.
    HttpDataApi,
    /// Layer is not known to poolsim.
    Unknown,
}

/// Relationship between two connection layers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ConnectionRelationshipKind {
    /// Parent runtime owns or creates the next layer.
    Owns,
    /// Client-side connection attempt or pool checkout.
    Borrows,
    /// Pooler/proxy can reuse backend connections across clients.
    Multiplexes,
    /// Layer consumes real backend database capacity.
    ConsumesBackendCapacity,
    /// Relationship is hidden by a provider or HTTP API.
    ProviderManaged,
    /// Relationship is unknown.
    Unknown,
}

/// Status of a connection ownership graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ConnectionOwnershipStatus {
    /// Supplied evidence is sufficient and no unsafe backend limit was found.
    Complete,
    /// More topology, limit, or telemetry evidence is required.
    NeedsReview,
    /// Supplied evidence shows an unsafe connection-capacity path.
    Unsafe,
}

/// Input for building a connection ownership graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct ConnectionOwnershipInput {
    /// Optional service or workload label used in reports.
    #[serde(default)]
    pub service_name: Option<String>,
    /// Number of runtime units that can own app-side pools.
    #[serde(default)]
    pub runtime_units: Option<u32>,
    /// Maximum application pool size per runtime unit.
    #[serde(default)]
    pub app_pool_size_per_runtime_unit: Option<u32>,
    /// Endpoint class used by this workload.
    #[serde(default)]
    pub endpoint_kind: Option<EndpointConnectionKind>,
    /// External pooler/proxy family, when present.
    #[serde(default)]
    pub external_pooler: Option<ExternalPoolerKind>,
    /// Client connection cap accepted by the external pooler/proxy.
    #[serde(default)]
    pub pooler_client_limit: Option<u32>,
    /// Backend database connection cap owned by the external pooler/proxy.
    #[serde(default)]
    pub pooler_backend_limit: Option<u32>,
    /// Real database backend connection budget for this workload.
    #[serde(default)]
    pub database_backend_limit: Option<u32>,
    /// Risk that sessions are pinned and multiplexing becomes ineffective.
    #[serde(default)]
    pub session_pinning_risk: Option<RiskLevel>,
}

impl ConnectionOwnershipInput {
    /// Creates an empty connection ownership input.
    pub fn new() -> Self {
        Self {
            service_name: None,
            runtime_units: None,
            app_pool_size_per_runtime_unit: None,
            endpoint_kind: None,
            external_pooler: None,
            pooler_client_limit: None,
            pooler_backend_limit: None,
            database_backend_limit: None,
            session_pinning_risk: None,
        }
    }

    /// Sets the service name.
    #[must_use]
    pub fn with_service_name(mut self, value: impl Into<String>) -> Self {
        self.service_name = Some(value.into());
        self
    }

    /// Sets the number of runtime units.
    #[must_use]
    pub fn with_runtime_units(mut self, value: u32) -> Self {
        self.runtime_units = Some(value);
        self
    }

    /// Sets the application pool size per runtime unit.
    #[must_use]
    pub fn with_app_pool_size_per_runtime_unit(mut self, value: u32) -> Self {
        self.app_pool_size_per_runtime_unit = Some(value);
        self
    }

    /// Sets the endpoint kind.
    #[must_use]
    pub fn with_endpoint_kind(mut self, value: EndpointConnectionKind) -> Self {
        self.endpoint_kind = Some(value);
        self
    }

    /// Sets the external pooler family.
    #[must_use]
    pub fn with_external_pooler(mut self, value: ExternalPoolerKind) -> Self {
        self.external_pooler = Some(value);
        self
    }

    /// Sets the external pooler client limit.
    #[must_use]
    pub fn with_pooler_client_limit(mut self, value: u32) -> Self {
        self.pooler_client_limit = Some(value);
        self
    }

    /// Sets the external pooler backend limit.
    #[must_use]
    pub fn with_pooler_backend_limit(mut self, value: u32) -> Self {
        self.pooler_backend_limit = Some(value);
        self
    }

    /// Sets the database backend limit.
    #[must_use]
    pub fn with_database_backend_limit(mut self, value: u32) -> Self {
        self.database_backend_limit = Some(value);
        self
    }

    /// Sets the session pinning risk.
    #[must_use]
    pub fn with_session_pinning_risk(mut self, value: RiskLevel) -> Self {
        self.session_pinning_risk = Some(value);
        self
    }
}

impl Default for ConnectionOwnershipInput {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the connection ownership graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectionOwnershipNode {
    /// Stable node identifier.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Layer kind represented by this node.
    pub layer_kind: ConnectionLayerKind,
    /// Component that owns this layer.
    pub owner: String,
    /// Maximum connections represented by this layer, when known.
    pub max_connections: Option<u64>,
    /// Whether this layer directly consumes database backend connections.
    pub consumes_database_connections: bool,
    /// Additional interpretation notes.
    pub notes: Vec<String>,
}

/// An edge in the connection ownership graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectionOwnershipEdge {
    /// Source node identifier.
    pub from: String,
    /// Destination node identifier.
    pub to: String,
    /// Relationship between the source and destination layers.
    pub relationship: ConnectionRelationshipKind,
    /// Worst-case connections that can cross this edge, when known.
    pub worst_case_connections: Option<u64>,
    /// Whether this edge can consume real database backend capacity.
    pub consumes_backend_capacity: bool,
    /// Additional interpretation notes.
    pub notes: Vec<String>,
}

/// Report produced by [`build_connection_ownership_graph`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectionOwnershipReport {
    /// Overall graph status.
    pub status: ConnectionOwnershipStatus,
    /// Optional service name supplied by the caller.
    pub service_name: Option<String>,
    /// Graph nodes ordered from application layer to database backend.
    pub nodes: Vec<ConnectionOwnershipNode>,
    /// Graph edges ordered from application layer to database backend.
    pub edges: Vec<ConnectionOwnershipEdge>,
    /// Worst-case app-side pool connections across runtime units.
    pub app_connection_upper_bound: Option<u64>,
    /// Upper bound for direct database backend consumption, when known.
    pub database_backend_upper_bound: Option<u64>,
    /// Real database backend limit supplied by the caller.
    pub database_backend_limit: Option<u32>,
    /// Node or layer that currently constrains the plan, when known.
    pub bottleneck_layer: Option<String>,
    /// Findings that explain risk, missing evidence, and remediation.
    pub findings: Vec<PoolerFinding>,
    /// Confidence in the graph from the supplied evidence.
    pub confidence: EvidenceConfidence,
}

/// Builds a connection ownership graph from deployment and pooler evidence.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when supplied numeric capacity values
/// are impossible, such as zero runtime units, zero pool size, or zero limits.
pub fn build_connection_ownership_graph(
    input: &ConnectionOwnershipInput,
) -> Result<ConnectionOwnershipReport, PoolsimError> {
    validate_input(input)?;

    let endpoint_kind = input
        .endpoint_kind
        .unwrap_or(EndpointConnectionKind::DirectDatabase);
    let has_external_pooler =
        input.external_pooler.is_some() || endpoint_uses_pooler(endpoint_kind);
    let app_upper = input
        .runtime_units
        .zip(input.app_pool_size_per_runtime_unit)
        .map(|(runtime_units, pool_size)| u64::from(runtime_units) * u64::from(pool_size));

    let mut status = ConnectionOwnershipStatus::Complete;
    let mut confidence = EvidenceConfidence::High;
    let mut findings = Vec::new();

    if input.runtime_units.is_none() {
        status = ConnectionOwnershipStatus::NeedsReview;
        confidence = EvidenceConfidence::Low;
        findings.push(finding(
            "OWNERSHIP_RUNTIME_UNITS_UNKNOWN",
            RiskLevel::High,
            "runtime unit count is unknown",
            "provide replicas, processes, workers, or execution environments that can each own an application pool",
        ));
    }
    if input.app_pool_size_per_runtime_unit.is_none() {
        status = ConnectionOwnershipStatus::NeedsReview;
        confidence = min_confidence(confidence, EvidenceConfidence::Low);
        findings.push(finding(
            "OWNERSHIP_APP_POOL_SIZE_UNKNOWN",
            RiskLevel::High,
            "application pool size per runtime unit is unknown",
            "provide the driver or framework maximum pool size for each runtime unit",
        ));
    }

    if has_external_pooler {
        confidence = min_confidence(confidence, EvidenceConfidence::Medium);
        findings.push(finding(
            "OWNERSHIP_EXTERNAL_POOLER_PRESENT",
            RiskLevel::Medium,
            "traffic crosses an external pooler or proxy boundary",
            "track client-side and backend-side pooler limits separately; do not treat client connections as database backend connections",
        ));
        if input.pooler_backend_limit.is_none() {
            status = max_status(status, ConnectionOwnershipStatus::NeedsReview);
            confidence = min_confidence(confidence, EvidenceConfidence::Low);
            findings.push(finding(
                "OWNERSHIP_POOLER_BACKEND_LIMIT_UNKNOWN",
                RiskLevel::High,
                "pooler backend database connection cap is unknown",
                "provide pooler backend capacity or telemetry before relying on the pooler as a capacity boundary",
            ));
        }
    }

    if input.database_backend_limit.is_none() {
        status = max_status(status, ConnectionOwnershipStatus::NeedsReview);
        confidence = min_confidence(confidence, EvidenceConfidence::Medium);
        findings.push(finding(
            "OWNERSHIP_DATABASE_LIMIT_UNKNOWN",
            RiskLevel::Medium,
            "database backend connection limit is unknown",
            "provide the effective database connection budget after reserved and shared-service slots",
        ));
    }

    if let Some(risk) = input.session_pinning_risk {
        if risk >= RiskLevel::High && has_external_pooler {
            status = max_status(status, ConnectionOwnershipStatus::NeedsReview);
            confidence = min_confidence(confidence, EvidenceConfidence::Medium);
            findings.push(finding(
                "OWNERSHIP_PINNING_REDUCES_MULTIPLEXING",
                risk,
                "session pinning can make backend usage approach client connection usage",
                "measure pinned sessions and remove session-state features before increasing application pool size",
            ));
        }
    }

    let database_backend_upper_bound = if has_external_pooler {
        input.pooler_backend_limit.map(u64::from)
    } else {
        app_upper
    };

    if let (Some(upper), Some(limit)) = (
        database_backend_upper_bound,
        input.database_backend_limit.map(u64::from),
    ) {
        if upper > limit {
            status = ConnectionOwnershipStatus::Unsafe;
            findings.push(finding(
                "OWNERSHIP_DATABASE_LIMIT_EXCEEDED",
                RiskLevel::Critical,
                format!(
                    "database backend upper bound is {upper} connections, which exceeds the effective limit of {limit}"
                ),
                "reduce app pool footprint, lower pooler backend capacity, cap runtime units, or reserve more database capacity",
            ));
        } else if upper.saturating_mul(100) >= limit.saturating_mul(80) {
            status = max_status(status, ConnectionOwnershipStatus::NeedsReview);
            findings.push(finding(
                "OWNERSHIP_DATABASE_LIMIT_NEAR",
                RiskLevel::High,
                format!(
                    "database backend upper bound uses at least 80% of the effective limit ({upper}/{limit})"
                ),
                "leave headroom for migrations, failover, admin sessions, monitoring, and other services",
            ));
        }
    }

    let nodes = build_nodes(input, endpoint_kind, has_external_pooler, app_upper);
    let edges = build_edges(
        input,
        has_external_pooler,
        app_upper,
        database_backend_upper_bound,
    );
    let bottleneck_layer =
        bottleneck_layer(input, has_external_pooler, database_backend_upper_bound);

    Ok(ConnectionOwnershipReport {
        status,
        service_name: input.service_name.clone(),
        nodes,
        edges,
        app_connection_upper_bound: app_upper,
        database_backend_upper_bound,
        database_backend_limit: input.database_backend_limit,
        bottleneck_layer,
        findings,
        confidence,
    })
}

fn validate_input(input: &ConnectionOwnershipInput) -> Result<(), PoolsimError> {
    if input.runtime_units == Some(0) {
        return Err(PoolsimError::invalid_input(
            "INVALID_RUNTIME_UNITS",
            "runtime_units must be greater than 0 when provided",
            None,
        ));
    }
    if input.app_pool_size_per_runtime_unit == Some(0) {
        return Err(PoolsimError::invalid_input(
            "INVALID_APP_POOL_SIZE_PER_RUNTIME_UNIT",
            "app_pool_size_per_runtime_unit must be greater than 0 when provided",
            None,
        ));
    }
    if input.pooler_client_limit == Some(0)
        || input.pooler_backend_limit == Some(0)
        || input.database_backend_limit == Some(0)
    {
        return Err(PoolsimError::invalid_input(
            "INVALID_CONNECTION_LIMIT",
            "connection limits must be greater than 0 when provided",
            None,
        ));
    }
    Ok(())
}

fn endpoint_uses_pooler(endpoint_kind: EndpointConnectionKind) -> bool {
    matches!(
        endpoint_kind,
        EndpointConnectionKind::SessionPooler
            | EndpointConnectionKind::TransactionPooler
            | EndpointConnectionKind::StatementPooler
            | EndpointConnectionKind::DatabaseProxy
            | EndpointConnectionKind::EdgePooler
    )
}

fn build_nodes(
    input: &ConnectionOwnershipInput,
    endpoint_kind: EndpointConnectionKind,
    has_external_pooler: bool,
    app_upper: Option<u64>,
) -> Vec<ConnectionOwnershipNode> {
    let mut nodes = vec![
        node(
            "application-runtime",
            "Application runtime units",
            ConnectionLayerKind::ApplicationRuntime,
            "application/platform",
            input.runtime_units.map(u64::from),
            false,
            vec!["replicas, workers, processes, or execution environments that can own pools"],
        ),
        node(
            "application-pool",
            "Application-side pool",
            ConnectionLayerKind::ApplicationPool,
            "driver/framework",
            app_upper,
            !has_external_pooler,
            vec!["configured driver or framework pool multiplied by runtime units"],
        ),
    ];

    if endpoint_kind == EndpointConnectionKind::HttpDataApi {
        nodes.push(node(
            "http-data-api",
            "HTTP/Data API",
            ConnectionLayerKind::HttpDataApi,
            "provider",
            None,
            false,
            vec!["provider API hides direct database socket ownership"],
        ));
    } else if has_external_pooler {
        nodes.push(node(
            "pooler-client",
            "External pooler client side",
            ConnectionLayerKind::ExternalPoolerClient,
            pooler_owner(input.external_pooler),
            input.pooler_client_limit.map(u64::from),
            false,
            vec!["client connections accepted by the pooler or proxy"],
        ));
        nodes.push(node(
            "pooler-backend",
            "External pooler backend side",
            ConnectionLayerKind::ExternalPoolerBackend,
            pooler_owner(input.external_pooler),
            input.pooler_backend_limit.map(u64::from),
            true,
            vec!["server/database connections owned by the pooler or proxy"],
        ));
    }

    nodes.push(node(
        "database-backend",
        "Database backend",
        ConnectionLayerKind::DatabaseBackend,
        "database",
        input.database_backend_limit.map(u64::from),
        true,
        vec!["real database sessions, processes, threads, or backend connection slots"],
    ));

    nodes
}

fn build_edges(
    input: &ConnectionOwnershipInput,
    has_external_pooler: bool,
    app_upper: Option<u64>,
    database_backend_upper_bound: Option<u64>,
) -> Vec<ConnectionOwnershipEdge> {
    let mut edges = vec![edge(
        "application-runtime",
        "application-pool",
        ConnectionRelationshipKind::Owns,
        app_upper,
        false,
        vec!["each runtime unit can create or retain its own application pool"],
    )];

    if has_external_pooler {
        edges.push(edge(
            "application-pool",
            "pooler-client",
            ConnectionRelationshipKind::Borrows,
            app_upper,
            false,
            vec!["application pool connections terminate at the pooler client side"],
        ));
        edges.push(edge(
            "pooler-client",
            "pooler-backend",
            ConnectionRelationshipKind::Multiplexes,
            input.pooler_backend_limit.map(u64::from),
            true,
            vec!["pooler mode and pinning determine how many client sessions share backend connections"],
        ));
        edges.push(edge(
            "pooler-backend",
            "database-backend",
            ConnectionRelationshipKind::ConsumesBackendCapacity,
            database_backend_upper_bound,
            true,
            vec!["pooler backend connections consume real database capacity"],
        ));
    } else {
        edges.push(edge(
            "application-pool",
            "database-backend",
            ConnectionRelationshipKind::ConsumesBackendCapacity,
            app_upper,
            true,
            vec!["direct application pool connections consume database capacity"],
        ));
    }

    edges
}

fn bottleneck_layer(
    input: &ConnectionOwnershipInput,
    has_external_pooler: bool,
    database_backend_upper_bound: Option<u64>,
) -> Option<String> {
    let database_limit = input.database_backend_limit.map(u64::from);
    if let (Some(upper), Some(limit)) = (database_backend_upper_bound, database_limit) {
        if upper >= limit.saturating_mul(80) / 100 {
            return Some("database-backend".to_string());
        }
    }
    if has_external_pooler && input.pooler_backend_limit.is_none() {
        return Some("pooler-backend".to_string());
    }
    if input.app_pool_size_per_runtime_unit.is_none() || input.runtime_units.is_none() {
        return Some("application-pool".to_string());
    }
    None
}

fn node(
    id: impl Into<String>,
    label: impl Into<String>,
    layer_kind: ConnectionLayerKind,
    owner: impl Into<String>,
    max_connections: Option<u64>,
    consumes_database_connections: bool,
    notes: Vec<&str>,
) -> ConnectionOwnershipNode {
    ConnectionOwnershipNode {
        id: id.into(),
        label: label.into(),
        layer_kind,
        owner: owner.into(),
        max_connections,
        consumes_database_connections,
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

fn edge(
    from: impl Into<String>,
    to: impl Into<String>,
    relationship: ConnectionRelationshipKind,
    worst_case_connections: Option<u64>,
    consumes_backend_capacity: bool,
    notes: Vec<&str>,
) -> ConnectionOwnershipEdge {
    ConnectionOwnershipEdge {
        from: from.into(),
        to: to.into(),
        relationship,
        worst_case_connections,
        consumes_backend_capacity,
        notes: notes.into_iter().map(str::to_string).collect(),
    }
}

fn pooler_owner(pooler: Option<ExternalPoolerKind>) -> &'static str {
    match pooler {
        Some(ExternalPoolerKind::PgBouncer) => "pgbouncer",
        Some(ExternalPoolerKind::RdsProxy) => "aws-rds-proxy",
        Some(ExternalPoolerKind::Supavisor) => "supavisor",
        Some(ExternalPoolerKind::PrismaPostgresPooler) => "prisma-postgres",
        Some(ExternalPoolerKind::NeonPooler) => "neon-pooler",
        Some(ExternalPoolerKind::CloudflareHyperdrive) => "cloudflare-hyperdrive",
        Some(ExternalPoolerKind::Unknown) | None => "external-pooler",
    }
}

fn max_status(
    current: ConnectionOwnershipStatus,
    next: ConnectionOwnershipStatus,
) -> ConnectionOwnershipStatus {
    if status_severity(next) > status_severity(current) {
        next
    } else {
        current
    }
}

fn status_severity(status: ConnectionOwnershipStatus) -> u8 {
    match status {
        ConnectionOwnershipStatus::Complete => 0,
        ConnectionOwnershipStatus::NeedsReview => 1,
        ConnectionOwnershipStatus::Unsafe => 2,
    }
}

fn min_confidence(current: EvidenceConfidence, next: EvidenceConfidence) -> EvidenceConfidence {
    match (current, next) {
        (EvidenceConfidence::Low, _) | (_, EvidenceConfidence::Low) => EvidenceConfidence::Low,
        (EvidenceConfidence::Medium, _) | (_, EvidenceConfidence::Medium) => {
            EvidenceConfidence::Medium
        }
        (EvidenceConfidence::High, EvidenceConfidence::High) => EvidenceConfidence::High,
    }
}

fn finding(
    code: impl Into<String>,
    risk: RiskLevel,
    message: impl Into<String>,
    remediation: impl Into<String>,
) -> PoolerFinding {
    PoolerFinding {
        code: code.into(),
        risk,
        message: message.into(),
        remediation: remediation.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_database_graph_marks_app_pool_as_backend_consumer() {
        let report = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new()
                .with_service_name("checkout-api")
                .with_runtime_units(4)
                .with_app_pool_size_per_runtime_unit(10)
                .with_database_backend_limit(80),
        )
        .expect("graph should build");

        assert_eq!(report.status, ConnectionOwnershipStatus::Complete);
        assert_eq!(report.app_connection_upper_bound, Some(40));
        assert_eq!(report.database_backend_upper_bound, Some(40));
        assert!(report
            .nodes
            .iter()
            .any(|node| { node.id == "application-pool" && node.consumes_database_connections }));
    }

    #[test]
    fn direct_database_graph_reports_unsafe_backend_limit() {
        let report = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new()
                .with_runtime_units(12)
                .with_app_pool_size_per_runtime_unit(10)
                .with_database_backend_limit(100),
        )
        .expect("graph should build");

        assert_eq!(report.status, ConnectionOwnershipStatus::Unsafe);
        assert_eq!(report.database_backend_upper_bound, Some(120));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "OWNERSHIP_DATABASE_LIMIT_EXCEEDED"));
    }

    #[test]
    fn external_pooler_graph_separates_client_and_backend_layers() {
        let report = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new()
                .with_runtime_units(100)
                .with_app_pool_size_per_runtime_unit(2)
                .with_endpoint_kind(EndpointConnectionKind::DatabaseProxy)
                .with_external_pooler(ExternalPoolerKind::RdsProxy)
                .with_pooler_client_limit(1_000)
                .with_pooler_backend_limit(90)
                .with_database_backend_limit(120),
        )
        .expect("graph should build");

        assert_eq!(report.status, ConnectionOwnershipStatus::Complete);
        assert_eq!(report.app_connection_upper_bound, Some(200));
        assert_eq!(report.database_backend_upper_bound, Some(90));
        assert!(report.nodes.iter().any(|node| node.id == "pooler-client"));
        assert!(report.nodes.iter().any(|node| node.id == "pooler-backend"));
        assert!(report
            .edges
            .iter()
            .any(|edge| edge.relationship == ConnectionRelationshipKind::Multiplexes));
    }

    #[test]
    fn external_pooler_without_backend_limit_needs_review() {
        let report = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new()
                .with_runtime_units(100)
                .with_app_pool_size_per_runtime_unit(2)
                .with_endpoint_kind(EndpointConnectionKind::TransactionPooler)
                .with_external_pooler(ExternalPoolerKind::PgBouncer)
                .with_database_backend_limit(120)
                .with_session_pinning_risk(RiskLevel::High),
        )
        .expect("graph should build");

        assert_eq!(report.status, ConnectionOwnershipStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Low);
        assert_eq!(report.bottleneck_layer.as_deref(), Some("pooler-backend"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "OWNERSHIP_POOLER_BACKEND_LIMIT_UNKNOWN"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "OWNERSHIP_PINNING_REDUCES_MULTIPLEXING"));
    }

    #[test]
    fn invalid_zero_inputs_are_rejected() {
        let runtime_err = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new().with_runtime_units(0),
        )
        .expect_err("zero runtime units should fail");
        assert_eq!(runtime_err.code(), "INVALID_RUNTIME_UNITS");

        let limit_err = build_connection_ownership_graph(
            &ConnectionOwnershipInput::new().with_database_backend_limit(0),
        )
        .expect_err("zero limit should fail");
        assert_eq!(limit_err.code(), "INVALID_CONNECTION_LIMIT");
    }
}
