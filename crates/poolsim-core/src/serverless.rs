//! Serverless concurrency planning for database connection pools.
//!
//! Serverless and edge runtimes change pool ownership: each concurrent
//! execution environment can own an application-side pool. This module turns
//! that topology into an explicit capacity report instead of treating a single
//! pool size as the whole deployment's database footprint.

use serde::{Deserialize, Serialize};

use crate::{
    error::PoolsimError,
    pooler::{EvidenceConfidence, ExternalPoolerKind, PoolerFinding},
    types::RiskLevel,
};

/// Serverless or edge platform family used by a workload.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ServerlessPlatformKind {
    /// AWS Lambda.
    AwsLambda,
    /// Vercel Functions or Vercel Fluid compute.
    VercelFunctions,
    /// Cloudflare Workers.
    CloudflareWorkers,
    /// Netlify Functions.
    NetlifyFunctions,
    /// Azure Functions.
    AzureFunctions,
    /// Google Cloud Functions or Cloud Run functions.
    GoogleCloudFunctions,
    /// Platform is not known to poolsim.
    Unknown,
}

/// Overall status of a serverless concurrency plan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ServerlessConcurrencyStatus {
    /// Supplied evidence is within the configured direct database limit.
    Pass,
    /// Supplied evidence is close to a limit or relies on missing assumptions.
    Warning,
    /// Supplied evidence exceeds the configured direct database limit.
    Critical,
    /// Required capacity evidence is missing, so the plan cannot be judged safely.
    NeedsReview,
}

/// Inputs for serverless concurrency planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct ServerlessConcurrencyInput {
    /// Serverless or edge platform family.
    pub platform: ServerlessPlatformKind,
    /// Maximum concurrent invocations or execution environments expected.
    #[serde(default)]
    pub max_concurrent_invocations: Option<u32>,
    /// Explicit concurrency cap, such as AWS Lambda reserved concurrency.
    #[serde(default)]
    pub reserved_concurrency: Option<u32>,
    /// Maximum application-side pool size in each execution environment.
    #[serde(default)]
    pub app_pool_size_per_environment: Option<u32>,
    /// Whether traffic goes through an external pooler or database proxy.
    #[serde(default)]
    pub uses_external_pooler: bool,
    /// External pooler or proxy family, when known.
    #[serde(default)]
    pub external_pooler: Option<ExternalPoolerKind>,
    /// Real database backend connection limit available to this workload.
    #[serde(default)]
    pub database_backend_limit: Option<u32>,
    /// Observed or estimated warm execution-environment reuse ratio, from 0.0 to 1.0.
    #[serde(default)]
    pub warm_reuse_ratio: Option<f64>,
}

impl ServerlessConcurrencyInput {
    /// Creates serverless concurrency input for a platform.
    pub fn new(platform: ServerlessPlatformKind) -> Self {
        Self {
            platform,
            max_concurrent_invocations: None,
            reserved_concurrency: None,
            app_pool_size_per_environment: None,
            uses_external_pooler: false,
            external_pooler: None,
            database_backend_limit: None,
            warm_reuse_ratio: None,
        }
    }

    /// Sets the expected maximum concurrent invocations.
    #[must_use]
    pub fn with_max_concurrent_invocations(mut self, value: u32) -> Self {
        self.max_concurrent_invocations = Some(value);
        self
    }

    /// Sets the explicit concurrency cap.
    #[must_use]
    pub fn with_reserved_concurrency(mut self, value: u32) -> Self {
        self.reserved_concurrency = Some(value);
        self
    }

    /// Sets the app-side pool size per execution environment.
    #[must_use]
    pub fn with_app_pool_size_per_environment(mut self, value: u32) -> Self {
        self.app_pool_size_per_environment = Some(value);
        self
    }

    /// Marks the workload as using an external pooler or proxy.
    #[must_use]
    pub fn with_external_pooler(mut self, pooler: ExternalPoolerKind) -> Self {
        self.uses_external_pooler = true;
        self.external_pooler = Some(pooler);
        self
    }

    /// Sets the real database backend connection limit.
    #[must_use]
    pub fn with_database_backend_limit(mut self, value: u32) -> Self {
        self.database_backend_limit = Some(value);
        self
    }

    /// Sets the warm execution-environment reuse ratio.
    #[must_use]
    pub fn with_warm_reuse_ratio(mut self, value: f64) -> Self {
        self.warm_reuse_ratio = Some(value);
        self
    }
}

/// Report produced by [`plan_serverless_concurrency`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct ServerlessConcurrencyReport {
    /// Overall status for the supplied evidence.
    pub status: ServerlessConcurrencyStatus,
    /// Platform family that was evaluated.
    pub platform: ServerlessPlatformKind,
    /// Effective concurrency after applying known caps.
    pub effective_concurrency: Option<u32>,
    /// Worst-case app-side pool connections across all execution environments.
    pub worst_case_app_pool_connections: Option<u64>,
    /// Direct database backend upper bound when no external pooler is used.
    pub direct_database_backend_upper_bound: Option<u64>,
    /// Real database backend connection limit supplied by the caller.
    pub database_backend_limit: Option<u32>,
    /// Whether traffic uses an external pooler or proxy.
    pub uses_external_pooler: bool,
    /// External pooler family supplied by the caller.
    pub external_pooler: Option<ExternalPoolerKind>,
    /// Risk that cold starts, scale-out, or suspended cleanup create connection churn.
    pub connection_churn_risk: RiskLevel,
    /// Findings that explain risk, uncertainty, and remediation.
    pub findings: Vec<PoolerFinding>,
    /// Confidence in the plan from the supplied evidence.
    pub confidence: EvidenceConfidence,
}

/// Builds a conservative serverless connection-capacity report.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when numeric inputs are impossible,
/// such as zero pool size, zero concurrency, zero backend limit, or a warm reuse
/// ratio outside `0.0..=1.0`.
pub fn plan_serverless_concurrency(
    input: &ServerlessConcurrencyInput,
) -> Result<ServerlessConcurrencyReport, PoolsimError> {
    validate_serverless_input(input)?;

    let mut findings = Vec::new();
    let mut confidence = EvidenceConfidence::High;
    let mut status = ServerlessConcurrencyStatus::Pass;

    let effective_concurrency = effective_concurrency(input);
    if effective_concurrency.is_none() {
        status = ServerlessConcurrencyStatus::NeedsReview;
        confidence = EvidenceConfidence::Low;
        findings.push(finding(
            "SERVERLESS_CONCURRENCY_UNKNOWN",
            RiskLevel::High,
            "maximum concurrent execution environments are not known",
            "set a platform concurrency cap or provide max_concurrent_invocations before treating the pool plan as safe",
        ));
    }

    let app_pool_size = input.app_pool_size_per_environment;
    if app_pool_size.is_none() {
        status = ServerlessConcurrencyStatus::NeedsReview;
        confidence = min_confidence(confidence, EvidenceConfidence::Low);
        findings.push(finding(
            "SERVERLESS_POOL_SIZE_UNKNOWN",
            RiskLevel::High,
            "application pool size per execution environment is not known",
            "provide the configured driver/framework pool size for each serverless execution environment",
        ));
    }

    let worst_case_app_pool_connections = effective_concurrency
        .zip(app_pool_size)
        .map(|(concurrency, pool_size)| u64::from(concurrency) * u64::from(pool_size));

    let connection_churn_risk = connection_churn_risk(input.warm_reuse_ratio);
    if input.warm_reuse_ratio.is_none() {
        confidence = min_confidence(confidence, EvidenceConfidence::Medium);
        findings.push(finding(
            "SERVERLESS_WARM_REUSE_UNKNOWN",
            RiskLevel::Medium,
            "warm execution-environment reuse is unknown",
            "measure cold starts and environment reuse; do not reduce worst-case capacity using warm reuse assumptions",
        ));
    }

    if input.uses_external_pooler {
        confidence = min_confidence(
            confidence,
            external_pooler_confidence(input.external_pooler),
        );
        status = max_status(status, ServerlessConcurrencyStatus::Warning);
        findings.push(finding(
            "SERVERLESS_EXTERNAL_POOLER_REVIEW",
            RiskLevel::Medium,
            "traffic uses an external pooler or proxy, so app-side connections are not the same as backend database connections",
            "verify pooler mode, backend connection cap, pinning behavior, and live backend connection telemetry before increasing app-side pool size",
        ));
        if input.external_pooler.is_none()
            || input.external_pooler == Some(ExternalPoolerKind::Unknown)
        {
            confidence = min_confidence(confidence, EvidenceConfidence::Low);
            findings.push(finding(
                "SERVERLESS_POOLER_KIND_UNKNOWN",
                RiskLevel::Medium,
                "external pooler kind is unknown",
                "classify the endpoint or provide the pooler kind so poolsim can reason about provider-specific behavior",
            ));
        }
    }

    let direct_database_backend_upper_bound = if input.uses_external_pooler {
        None
    } else {
        worst_case_app_pool_connections
    };

    if let (Some(backend_upper), Some(limit)) = (
        direct_database_backend_upper_bound,
        input.database_backend_limit.map(u64::from),
    ) {
        let ratio = backend_upper as f64 / limit as f64;
        if backend_upper > limit {
            status = ServerlessConcurrencyStatus::Critical;
            findings.push(finding(
                "SERVERLESS_CONNECTION_LIMIT_EXCEEDED",
                RiskLevel::Critical,
                format!(
                    "worst-case app-side pool footprint is {backend_upper} connections, which exceeds the database backend limit of {limit}"
                ),
                "reduce per-environment pool size, cap serverless concurrency, reserve database headroom, or add a verified external pooler/backend cap",
            ));
        } else if ratio >= 0.80 {
            status = max_status(status, ServerlessConcurrencyStatus::Warning);
            findings.push(finding(
                "SERVERLESS_CONNECTION_LIMIT_NEAR",
                RiskLevel::High,
                format!(
                    "worst-case app-side pool footprint uses {:.1}% of the database backend limit",
                    ratio * 100.0
                ),
                "leave backend headroom for migrations, admin sessions, replicas, failover, deployments, and other services",
            ));
        }
    } else if input.database_backend_limit.is_none() {
        confidence = min_confidence(confidence, EvidenceConfidence::Medium);
        status = max_status(status, ServerlessConcurrencyStatus::Warning);
        findings.push(finding(
            "SERVERLESS_BACKEND_LIMIT_UNKNOWN",
            RiskLevel::Medium,
            "database backend connection limit was not provided",
            "provide the effective backend limit after reserved slots and shared-service budget are subtracted",
        ));
    }

    if connection_churn_risk >= RiskLevel::High {
        status = max_status(status, ServerlessConcurrencyStatus::Warning);
        findings.push(finding(
            "SERVERLESS_CONNECTION_CHURN_HIGH",
            RiskLevel::High,
            "low warm reuse implies scale-out can create frequent connection setup and teardown",
            "use keep-alive where supported, low idle timeouts, deployment rollouts, and provider poolers that are verified for the workload",
        ));
    }

    Ok(ServerlessConcurrencyReport {
        status,
        platform: input.platform,
        effective_concurrency,
        worst_case_app_pool_connections,
        direct_database_backend_upper_bound,
        database_backend_limit: input.database_backend_limit,
        uses_external_pooler: input.uses_external_pooler,
        external_pooler: input.external_pooler,
        connection_churn_risk,
        findings,
        confidence,
    })
}

fn validate_serverless_input(input: &ServerlessConcurrencyInput) -> Result<(), PoolsimError> {
    if input.max_concurrent_invocations == Some(0) || input.reserved_concurrency == Some(0) {
        return Err(PoolsimError::invalid_input(
            "INVALID_SERVERLESS_CONCURRENCY",
            "serverless concurrency values must be greater than 0 when provided",
            None,
        ));
    }
    if input.app_pool_size_per_environment == Some(0) {
        return Err(PoolsimError::invalid_input(
            "INVALID_SERVERLESS_POOL_SIZE",
            "app_pool_size_per_environment must be greater than 0 when provided",
            None,
        ));
    }
    if input.database_backend_limit == Some(0) {
        return Err(PoolsimError::invalid_input(
            "INVALID_DATABASE_BACKEND_LIMIT",
            "database_backend_limit must be greater than 0 when provided",
            None,
        ));
    }
    if let Some(value) = input.warm_reuse_ratio {
        if !(0.0..=1.0).contains(&value) || !value.is_finite() {
            return Err(PoolsimError::invalid_input(
                "INVALID_WARM_REUSE_RATIO",
                "warm_reuse_ratio must be a finite value from 0.0 through 1.0",
                None,
            ));
        }
    }
    Ok(())
}

fn effective_concurrency(input: &ServerlessConcurrencyInput) -> Option<u32> {
    match (input.max_concurrent_invocations, input.reserved_concurrency) {
        (Some(max), Some(reserved)) => Some(max.min(reserved)),
        (Some(max), None) => Some(max),
        (None, Some(reserved)) => Some(reserved),
        (None, None) => None,
    }
}

fn connection_churn_risk(warm_reuse_ratio: Option<f64>) -> RiskLevel {
    match warm_reuse_ratio {
        Some(value) if value >= 0.70 => RiskLevel::Low,
        Some(value) if value >= 0.35 => RiskLevel::Medium,
        Some(_) => RiskLevel::High,
        None => RiskLevel::Medium,
    }
}

fn external_pooler_confidence(pooler: Option<ExternalPoolerKind>) -> EvidenceConfidence {
    match pooler {
        Some(ExternalPoolerKind::Unknown) | None => EvidenceConfidence::Low,
        Some(ExternalPoolerKind::PgBouncer | ExternalPoolerKind::RdsProxy) => {
            EvidenceConfidence::Medium
        }
        Some(
            ExternalPoolerKind::Supavisor
            | ExternalPoolerKind::PrismaPostgresPooler
            | ExternalPoolerKind::NeonPooler
            | ExternalPoolerKind::CloudflareHyperdrive,
        ) => EvidenceConfidence::Medium,
    }
}

fn max_status(
    current: ServerlessConcurrencyStatus,
    next: ServerlessConcurrencyStatus,
) -> ServerlessConcurrencyStatus {
    if status_severity(next) > status_severity(current) {
        next
    } else {
        current
    }
}

fn status_severity(status: ServerlessConcurrencyStatus) -> u8 {
    match status {
        ServerlessConcurrencyStatus::Pass => 0,
        ServerlessConcurrencyStatus::Warning => 1,
        ServerlessConcurrencyStatus::NeedsReview => 2,
        ServerlessConcurrencyStatus::Critical => 3,
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
    fn direct_serverless_pool_exceeding_limit_is_critical() {
        let report = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::AwsLambda)
                .with_max_concurrent_invocations(100)
                .with_app_pool_size_per_environment(3)
                .with_database_backend_limit(200)
                .with_warm_reuse_ratio(0.80),
        )
        .expect("serverless report should build");

        assert_eq!(report.status, ServerlessConcurrencyStatus::Critical);
        assert_eq!(report.effective_concurrency, Some(100));
        assert_eq!(report.worst_case_app_pool_connections, Some(300));
        assert_eq!(report.direct_database_backend_upper_bound, Some(300));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "SERVERLESS_CONNECTION_LIMIT_EXCEEDED"));
    }

    #[test]
    fn reserved_concurrency_caps_effective_footprint() {
        let report = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::AwsLambda)
                .with_max_concurrent_invocations(100)
                .with_reserved_concurrency(40)
                .with_app_pool_size_per_environment(2)
                .with_database_backend_limit(150)
                .with_warm_reuse_ratio(0.90),
        )
        .expect("serverless report should build");

        assert_eq!(report.status, ServerlessConcurrencyStatus::Pass);
        assert_eq!(report.effective_concurrency, Some(40));
        assert_eq!(report.worst_case_app_pool_connections, Some(80));
        assert_eq!(report.connection_churn_risk, RiskLevel::Low);
    }

    #[test]
    fn external_pooler_preserves_app_footprint_but_requires_review() {
        let report = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::CloudflareWorkers)
                .with_max_concurrent_invocations(500)
                .with_app_pool_size_per_environment(1)
                .with_external_pooler(ExternalPoolerKind::CloudflareHyperdrive)
                .with_database_backend_limit(100)
                .with_warm_reuse_ratio(0.20),
        )
        .expect("serverless report should build");

        assert_eq!(report.status, ServerlessConcurrencyStatus::Warning);
        assert_eq!(report.worst_case_app_pool_connections, Some(500));
        assert_eq!(report.direct_database_backend_upper_bound, None);
        assert_eq!(report.connection_churn_risk, RiskLevel::High);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "SERVERLESS_EXTERNAL_POOLER_REVIEW"));
    }

    #[test]
    fn missing_capacity_evidence_needs_review() {
        let report = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::VercelFunctions)
                .with_database_backend_limit(100),
        )
        .expect("incomplete evidence should produce review report");

        assert_eq!(report.status, ServerlessConcurrencyStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Low);
        assert_eq!(report.worst_case_app_pool_connections, None);
    }

    #[test]
    fn invalid_values_return_typed_errors() {
        let pool_err = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::AwsLambda)
                .with_app_pool_size_per_environment(0),
        )
        .expect_err("zero pool size should fail");
        assert_eq!(pool_err.code(), "INVALID_SERVERLESS_POOL_SIZE");

        let reuse_err = plan_serverless_concurrency(
            &ServerlessConcurrencyInput::new(ServerlessPlatformKind::AwsLambda)
                .with_warm_reuse_ratio(1.1),
        )
        .expect_err("invalid warm reuse should fail");
        assert_eq!(reuse_err.code(), "INVALID_WARM_REUSE_RATIO");
    }
}
