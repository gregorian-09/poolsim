//! Database-contention classification for pool-sizing decisions.
//!
//! The module consumes normalized evidence rather than querying a database.
//! PostgreSQL `pg_stat_activity`/`pg_locks`, MySQL Performance Schema, managed
//! database metrics, and pooler exporters can all map into the same input.
//! Missing evidence remains visible and lowers confidence instead of being
//! treated as proof that the database is healthy.

use serde::{Deserialize, Serialize};

use crate::{error::PoolsimError, pooler::EvidenceConfidence, types::RiskLevel};

/// Primary operational cause inferred from database and pool evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DatabaseContentionCause {
    /// No material contention signal was found.
    None,
    /// Application threads are waiting for the pool while database evidence is healthy.
    PoolStarvation,
    /// Sessions are waiting on database locks.
    LockContention,
    /// Sessions are holding open transactions while not executing a query.
    IdleTransaction,
    /// Deadlock activity was observed.
    Deadlock,
    /// Database connection, CPU, or I/O capacity is near its supplied limit.
    DatabaseSaturation,
    /// Database service latency is elevated without enough evidence to identify a narrower cause.
    SlowDatabase,
    /// More than one material cause is present.
    Mixed,
    /// Evidence is too incomplete to identify a cause.
    Unknown,
}

/// Overall result of database-contention classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DatabaseContentionStatus {
    /// Evidence indicates no material pool or database contention.
    Healthy,
    /// Pool wait is elevated while supplied database evidence is healthy.
    PoolStarvation,
    /// One or more database-side contention signals are material.
    DatabaseContention,
    /// Evidence is incomplete or contradictory and requires human review.
    NeedsReview,
}

/// Thresholds used to classify normalized contention evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DatabaseContentionPolicy {
    /// p99 application-pool wait at or above which pool pressure is elevated.
    pub pool_wait_p99_threshold_ms: f64,
    /// p99 database latency at or above which backend latency is elevated.
    pub database_latency_p99_threshold_ms: f64,
    /// Longest idle-in-transaction duration at or above which the session is material.
    pub idle_transaction_threshold_seconds: f64,
    /// Deadlock rate strictly above this value is material.
    pub deadlocks_per_second_threshold: f64,
    /// Database CPU utilization at or above which capacity is near its limit.
    pub database_cpu_utilization_threshold: f64,
    /// Database I/O utilization at or above which capacity is near its limit.
    pub database_io_utilization_threshold: f64,
    /// Active/max connection utilization at or above which capacity is near its limit.
    pub connection_utilization_threshold: f64,
}

impl Default for DatabaseContentionPolicy {
    fn default() -> Self {
        Self {
            pool_wait_p99_threshold_ms: 25.0,
            database_latency_p99_threshold_ms: 100.0,
            idle_transaction_threshold_seconds: 60.0,
            deadlocks_per_second_threshold: 0.0,
            database_cpu_utilization_threshold: 0.90,
            database_io_utilization_threshold: 0.90,
            connection_utilization_threshold: 0.90,
        }
    }
}

impl DatabaseContentionPolicy {
    /// Creates the conservative default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the p99 application-pool wait threshold.
    #[must_use]
    pub fn with_pool_wait_p99_threshold_ms(mut self, value: f64) -> Self {
        self.pool_wait_p99_threshold_ms = value;
        self
    }

    /// Sets the p99 database-latency threshold.
    #[must_use]
    pub fn with_database_latency_p99_threshold_ms(mut self, value: f64) -> Self {
        self.database_latency_p99_threshold_ms = value;
        self
    }

    /// Sets the idle-in-transaction duration threshold.
    #[must_use]
    pub fn with_idle_transaction_threshold_seconds(mut self, value: f64) -> Self {
        self.idle_transaction_threshold_seconds = value;
        self
    }

    /// Sets the deadlocks-per-second threshold.
    #[must_use]
    pub fn with_deadlocks_per_second_threshold(mut self, value: f64) -> Self {
        self.deadlocks_per_second_threshold = value;
        self
    }

    /// Sets the database CPU utilization threshold.
    #[must_use]
    pub fn with_database_cpu_utilization_threshold(mut self, value: f64) -> Self {
        self.database_cpu_utilization_threshold = value;
        self
    }

    /// Sets the database I/O utilization threshold.
    #[must_use]
    pub fn with_database_io_utilization_threshold(mut self, value: f64) -> Self {
        self.database_io_utilization_threshold = value;
        self
    }

    /// Sets the active/max connection utilization threshold.
    #[must_use]
    pub fn with_connection_utilization_threshold(mut self, value: f64) -> Self {
        self.connection_utilization_threshold = value;
        self
    }
}

/// Normalized database and application-pool evidence for classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DatabaseContentionInput {
    /// p99 time waiting to acquire an application-pool connection, in milliseconds.
    #[serde(default)]
    pub pool_wait_p99_ms: Option<f64>,
    /// p99 database service or query latency, in milliseconds.
    #[serde(default)]
    pub database_latency_p99_ms: Option<f64>,
    /// Sessions currently waiting for a database lock.
    #[serde(default)]
    pub lock_waiting_sessions: Option<u32>,
    /// Sessions currently idle while holding an open transaction.
    #[serde(default)]
    pub idle_in_transaction_sessions: Option<u32>,
    /// Longest observed idle-in-transaction duration, in seconds.
    #[serde(default)]
    pub longest_idle_in_transaction_seconds: Option<f64>,
    /// Deadlock rate over the observation window, in events per second.
    #[serde(default)]
    pub deadlocks_per_second: Option<f64>,
    /// Active database sessions in the observation window.
    #[serde(default)]
    pub active_sessions: Option<u32>,
    /// Maximum database connections available to the observed allocation.
    #[serde(default)]
    pub max_connections: Option<u32>,
    /// Database CPU utilization as a fraction from `0.0` to `1.0`.
    #[serde(default)]
    pub database_cpu_utilization: Option<f64>,
    /// Database I/O utilization as a fraction from `0.0` to `1.0`.
    #[serde(default)]
    pub database_io_utilization: Option<f64>,
    /// Threshold policy used for classification.
    #[serde(default)]
    pub policy: DatabaseContentionPolicy,
}

impl Default for DatabaseContentionInput {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseContentionInput {
    /// Creates empty evidence with the conservative default policy.
    pub fn new() -> Self {
        Self {
            pool_wait_p99_ms: None,
            database_latency_p99_ms: None,
            lock_waiting_sessions: None,
            idle_in_transaction_sessions: None,
            longest_idle_in_transaction_seconds: None,
            deadlocks_per_second: None,
            active_sessions: None,
            max_connections: None,
            database_cpu_utilization: None,
            database_io_utilization: None,
            policy: DatabaseContentionPolicy::default(),
        }
    }

    /// Sets p99 application-pool acquisition wait.
    #[must_use]
    pub fn with_pool_wait_p99_ms(mut self, value: f64) -> Self {
        self.pool_wait_p99_ms = Some(value);
        self
    }

    /// Sets p99 database service latency.
    #[must_use]
    pub fn with_database_latency_p99_ms(mut self, value: f64) -> Self {
        self.database_latency_p99_ms = Some(value);
        self
    }

    /// Sets the current database lock-waiting session count.
    #[must_use]
    pub fn with_lock_waiting_sessions(mut self, value: u32) -> Self {
        self.lock_waiting_sessions = Some(value);
        self
    }

    /// Sets the current idle-in-transaction session count.
    #[must_use]
    pub fn with_idle_in_transaction_sessions(mut self, value: u32) -> Self {
        self.idle_in_transaction_sessions = Some(value);
        self
    }

    /// Sets the longest observed idle-in-transaction duration.
    #[must_use]
    pub fn with_longest_idle_in_transaction_seconds(mut self, value: f64) -> Self {
        self.longest_idle_in_transaction_seconds = Some(value);
        self
    }

    /// Sets the observed deadlock rate.
    #[must_use]
    pub fn with_deadlocks_per_second(mut self, value: f64) -> Self {
        self.deadlocks_per_second = Some(value);
        self
    }

    /// Sets active database sessions.
    #[must_use]
    pub fn with_active_sessions(mut self, value: u32) -> Self {
        self.active_sessions = Some(value);
        self
    }

    /// Sets the maximum database connections for the observed allocation.
    #[must_use]
    pub fn with_max_connections(mut self, value: u32) -> Self {
        self.max_connections = Some(value);
        self
    }

    /// Sets database CPU utilization.
    #[must_use]
    pub fn with_database_cpu_utilization(mut self, value: f64) -> Self {
        self.database_cpu_utilization = Some(value);
        self
    }

    /// Sets database I/O utilization.
    #[must_use]
    pub fn with_database_io_utilization(mut self, value: f64) -> Self {
        self.database_io_utilization = Some(value);
        self
    }

    /// Replaces the default thresholds with a caller-provided policy.
    #[must_use]
    pub fn with_policy(mut self, policy: DatabaseContentionPolicy) -> Self {
        self.policy = policy;
        self
    }
}

/// A remediation-oriented explanation from database-contention classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseContentionFinding {
    /// Stable machine-readable finding code.
    pub code: String,
    /// Severity associated with this finding.
    pub risk: RiskLevel,
    /// Human-readable explanation.
    pub message: String,
    /// Recommended remediation or next verification step.
    pub remediation: String,
}

impl DatabaseContentionFinding {
    fn new(
        code: &'static str,
        risk: RiskLevel,
        message: &'static str,
        remediation: &'static str,
    ) -> Self {
        Self {
            code: code.to_string(),
            risk,
            message: message.to_string(),
            remediation: remediation.to_string(),
        }
    }
}

/// Classification report for pool and database contention evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DatabaseContentionReport {
    /// Overall classification status.
    pub status: DatabaseContentionStatus,
    /// Dominant cause inferred from the evidence.
    pub dominant_cause: DatabaseContentionCause,
    /// Whether a simple pool increase should be suppressed by this report.
    pub suppress_pool_increase: bool,
    /// Number of evidence categories available to the classifier.
    pub evidence_categories: u32,
    /// Confidence in the classification.
    pub confidence: EvidenceConfidence,
    /// Findings and remediation guidance.
    pub findings: Vec<DatabaseContentionFinding>,
}

/// Classifies whether observed waits are caused by the application pool or the database.
///
/// This function is deliberately evidence-first. Lock waits, deadlocks, long
/// idle transactions, and near-limit database capacity produce a database
/// contention result. High pool wait is classified as pool starvation only
/// when the relevant database evidence is present and healthy; otherwise the
/// result is `NeedsReview`.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when numeric evidence is negative or
/// non-finite, utilization is outside `0.0..=1.0`, a connection limit is zero,
/// active sessions exceed that limit, or policy thresholds are invalid.
pub fn classify_database_contention(
    input: &DatabaseContentionInput,
) -> Result<DatabaseContentionReport, PoolsimError> {
    validate_input(input)?;

    let mut findings = Vec::new();
    let mut confidence = EvidenceConfidence::High;
    let mut causes = Vec::new();
    let mut evidence_categories = 0;

    let pool_wait_high = input
        .pool_wait_p99_ms
        .is_some_and(|value| value >= input.policy.pool_wait_p99_threshold_ms);
    if input.pool_wait_p99_ms.is_some() {
        evidence_categories += 1;
        if pool_wait_high {
            findings.push(DatabaseContentionFinding::new(
                "POOL_WAIT_ELEVATED",
                RiskLevel::High,
                "application requests are waiting materially for a pool connection",
                "compare database-side evidence before increasing the pool; pool wait can be downstream of database contention",
            ));
        }
    }

    let lock_waiting = input.lock_waiting_sessions.unwrap_or(0);
    if input.lock_waiting_sessions.is_some() {
        evidence_categories += 1;
        if lock_waiting > 0 {
            causes.push(DatabaseContentionCause::LockContention);
            findings.push(DatabaseContentionFinding::new(
                "LOCK_WAITING_SESSIONS",
                RiskLevel::High,
                "one or more database sessions are waiting for locks",
                "inspect blocking sessions, lock order, transaction duration, and query plans before increasing the application pool",
            ));
        }
    }

    let idle_sessions = input.idle_in_transaction_sessions.unwrap_or(0);
    if input.idle_in_transaction_sessions.is_some() {
        evidence_categories += 1;
        if idle_sessions > 0 {
            match input.longest_idle_in_transaction_seconds {
                Some(duration) if duration >= input.policy.idle_transaction_threshold_seconds => {
                    causes.push(DatabaseContentionCause::IdleTransaction);
                    findings.push(DatabaseContentionFinding::new(
                        "IDLE_IN_TRANSACTION",
                        RiskLevel::High,
                        "sessions are idle inside open transactions long enough to retain database state",
                        "commit or roll back promptly, fix transaction-scope handling, and review idle-in-transaction timeouts",
                    ));
                }
                Some(_) => {}
                None => {
                    confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
                    findings.push(DatabaseContentionFinding::new(
                        "IDLE_TRANSACTION_DURATION_MISSING",
                        RiskLevel::Medium,
                        "idle-in-transaction sessions exist but their duration is unknown",
                        "export transaction age or xact_start/state_change timestamps before treating the sessions as harmless",
                    ));
                }
            }
        }
    }

    if let Some(rate) = input.deadlocks_per_second {
        evidence_categories += 1;
        if rate > input.policy.deadlocks_per_second_threshold {
            causes.push(DatabaseContentionCause::Deadlock);
            findings.push(DatabaseContentionFinding::new(
                "DEADLOCKS_DETECTED",
                RiskLevel::Critical,
                "deadlock activity is above the configured threshold",
                "fix lock ordering or transaction boundaries and verify retry multiplication before increasing concurrency",
            ));
        }
    }

    let connection_saturated = match (input.active_sessions, input.max_connections) {
        (Some(active), Some(max)) => {
            evidence_categories += 1;
            let saturated =
                f64::from(active) / f64::from(max) >= input.policy.connection_utilization_threshold;
            if saturated {
                findings.push(DatabaseContentionFinding::new(
                    "DATABASE_CONNECTIONS_NEAR_LIMIT",
                    RiskLevel::High,
                    "active database sessions are near the supplied connection limit",
                    "allocate a shared connection budget before increasing any application pool",
                ));
            }
            saturated
        }
        (Some(_), None) | (None, Some(_)) => {
            confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
            findings.push(DatabaseContentionFinding::new(
                "CONNECTION_CAPACITY_EVIDENCE_INCOMPLETE",
                RiskLevel::Medium,
                "only one side of active-session connection utilization was supplied",
                "supply both active sessions and the applicable max connection limit",
            ));
            false
        }
        (None, None) => false,
    };
    if connection_saturated {
        causes.push(DatabaseContentionCause::DatabaseSaturation);
    }

    let resource_saturated = [
        (
            input.database_cpu_utilization,
            input.policy.database_cpu_utilization_threshold,
            "DATABASE_CPU_NEAR_LIMIT",
            "database CPU utilization is near its configured threshold",
            "reduce query cost or concurrency and validate database CPU headroom before increasing the pool",
        ),
        (
            input.database_io_utilization,
            input.policy.database_io_utilization_threshold,
            "DATABASE_IO_NEAR_LIMIT",
            "database I/O utilization is near its configured threshold",
            "inspect slow I/O, indexes, cache efficiency, and query concurrency before increasing the pool",
        ),
    ];
    let mut resource_limit_hit = false;
    for (value, threshold, code, message, remediation) in resource_saturated {
        if let Some(value) = value {
            evidence_categories += 1;
            if value >= threshold {
                resource_limit_hit = true;
                findings.push(DatabaseContentionFinding::new(
                    code,
                    RiskLevel::High,
                    message,
                    remediation,
                ));
            }
        }
    }
    if resource_limit_hit {
        causes.push(DatabaseContentionCause::DatabaseSaturation);
    }

    let database_latency_high = input
        .database_latency_p99_ms
        .is_some_and(|value| value >= input.policy.database_latency_p99_threshold_ms);
    if input.database_latency_p99_ms.is_some() {
        evidence_categories += 1;
        if database_latency_high {
            findings.push(DatabaseContentionFinding::new(
                "DATABASE_LATENCY_ELEVATED",
                RiskLevel::Medium,
                "database service latency is above the configured threshold",
                "inspect query latency and database waits; do not assume a larger application pool will reduce service time",
            ));
            if causes.is_empty() {
                causes.push(DatabaseContentionCause::SlowDatabase);
            }
        }
    }

    let database_signals_complete = input.lock_waiting_sessions.is_some()
        && input.idle_in_transaction_sessions.is_some()
        && input.deadlocks_per_second.is_some()
        && input.database_latency_p99_ms.is_some()
        && (input.active_sessions.is_some() == input.max_connections.is_some());

    if pool_wait_high && causes.is_empty() && database_signals_complete && !database_latency_high {
        causes.push(DatabaseContentionCause::PoolStarvation);
        findings.push(DatabaseContentionFinding::new(
            "POOL_STARVATION_LIKELY",
            RiskLevel::Medium,
            "pool acquisition wait is elevated while supplied database contention signals are healthy",
            "validate pool size, request concurrency, and acquisition timeout before making a change",
        ));
    } else if pool_wait_high && causes.is_empty() && !database_signals_complete {
        confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
        findings.push(DatabaseContentionFinding::new(
            "DATABASE_EVIDENCE_INCOMPLETE",
            RiskLevel::Medium,
            "pool wait is elevated but database evidence is incomplete",
            "collect lock, transaction, deadlock, latency, and connection-capacity signals before attributing the wait to the application pool",
        ));
    }

    if evidence_categories == 0 {
        confidence = EvidenceConfidence::Low;
        findings.push(DatabaseContentionFinding::new(
            "INSUFFICIENT_CONTENTION_EVIDENCE",
            RiskLevel::High,
            "no pool or database contention evidence was supplied",
            "collect application pool wait, database latency, lock, transaction, deadlock, and capacity signals",
        ));
    }

    let distinct_causes = causes
        .iter()
        .copied()
        .fold(Vec::new(), |mut values, cause| {
            if !values.contains(&cause) {
                values.push(cause);
            }
            values
        });
    let dominant_cause = if distinct_causes.len() > 1 {
        DatabaseContentionCause::Mixed
    } else if distinct_causes.is_empty() && evidence_categories > 0 && findings.is_empty() {
        DatabaseContentionCause::None
    } else {
        distinct_causes
            .first()
            .copied()
            .unwrap_or(DatabaseContentionCause::Unknown)
    };
    let status = match dominant_cause {
        DatabaseContentionCause::PoolStarvation => DatabaseContentionStatus::PoolStarvation,
        DatabaseContentionCause::LockContention
        | DatabaseContentionCause::IdleTransaction
        | DatabaseContentionCause::Deadlock
        | DatabaseContentionCause::DatabaseSaturation
        | DatabaseContentionCause::SlowDatabase
        | DatabaseContentionCause::Mixed => DatabaseContentionStatus::DatabaseContention,
        DatabaseContentionCause::None => DatabaseContentionStatus::Healthy,
        DatabaseContentionCause::Unknown => {
            if evidence_categories > 0 && findings.is_empty() {
                DatabaseContentionStatus::Healthy
            } else {
                DatabaseContentionStatus::NeedsReview
            }
        }
    };
    if status == DatabaseContentionStatus::Healthy && findings.is_empty() {
        confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
    }

    Ok(DatabaseContentionReport {
        status,
        dominant_cause,
        suppress_pool_increase: matches!(status, DatabaseContentionStatus::DatabaseContention),
        evidence_categories,
        confidence,
        findings,
    })
}

fn validate_input(input: &DatabaseContentionInput) -> Result<(), PoolsimError> {
    for (name, value) in [
        ("pool_wait_p99_ms", input.pool_wait_p99_ms),
        ("database_latency_p99_ms", input.database_latency_p99_ms),
        (
            "longest_idle_in_transaction_seconds",
            input.longest_idle_in_transaction_seconds,
        ),
        ("deadlocks_per_second", input.deadlocks_per_second),
        ("database_cpu_utilization", input.database_cpu_utilization),
        ("database_io_utilization", input.database_io_utilization),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || value < 0.0 {
                return Err(invalid_contention(format!(
                    "{name} must be finite and non-negative"
                )));
            }
        }
    }
    for (name, value) in [
        ("database_cpu_utilization", input.database_cpu_utilization),
        ("database_io_utilization", input.database_io_utilization),
    ] {
        if value.is_some_and(|value| value > 1.0) {
            return Err(invalid_contention(format!(
                "{name} must be between 0.0 and 1.0"
            )));
        }
    }
    if input.max_connections == Some(0) {
        return Err(invalid_contention(
            "max_connections must be greater than zero",
        ));
    }
    if let (Some(active), Some(max)) = (input.active_sessions, input.max_connections) {
        if active > max {
            return Err(invalid_contention(
                "active_sessions cannot exceed max_connections",
            ));
        }
    }

    for (name, value) in [
        (
            "pool_wait_p99_threshold_ms",
            input.policy.pool_wait_p99_threshold_ms,
        ),
        (
            "database_latency_p99_threshold_ms",
            input.policy.database_latency_p99_threshold_ms,
        ),
        (
            "idle_transaction_threshold_seconds",
            input.policy.idle_transaction_threshold_seconds,
        ),
        (
            "deadlocks_per_second_threshold",
            input.policy.deadlocks_per_second_threshold,
        ),
        (
            "database_cpu_utilization_threshold",
            input.policy.database_cpu_utilization_threshold,
        ),
        (
            "database_io_utilization_threshold",
            input.policy.database_io_utilization_threshold,
        ),
        (
            "connection_utilization_threshold",
            input.policy.connection_utilization_threshold,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(invalid_contention(format!(
                "{name} must be finite and non-negative"
            )));
        }
    }
    for (name, value) in [
        (
            "database_cpu_utilization_threshold",
            input.policy.database_cpu_utilization_threshold,
        ),
        (
            "database_io_utilization_threshold",
            input.policy.database_io_utilization_threshold,
        ),
        (
            "connection_utilization_threshold",
            input.policy.connection_utilization_threshold,
        ),
    ] {
        if value > 1.0 {
            return Err(invalid_contention(format!(
                "{name} must be between 0.0 and 1.0"
            )));
        }
    }
    Ok(())
}

fn invalid_contention(message: impl Into<String>) -> PoolsimError {
    PoolsimError::invalid_input("INVALID_DATABASE_CONTENTION", message.into(), None)
}

fn lower_confidence(
    current: EvidenceConfidence,
    candidate: EvidenceConfidence,
) -> EvidenceConfidence {
    match (current, candidate) {
        (EvidenceConfidence::Low, _) | (_, EvidenceConfidence::Low) => EvidenceConfidence::Low,
        (EvidenceConfidence::Medium, _) | (_, EvidenceConfidence::Medium) => {
            EvidenceConfidence::Medium
        }
        _ => EvidenceConfidence::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_waits_suppress_pool_increase() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new()
                .with_pool_wait_p99_ms(80.0)
                .with_database_latency_p99_ms(120.0)
                .with_lock_waiting_sessions(3)
                .with_idle_in_transaction_sessions(0)
                .with_deadlocks_per_second(0.0)
                .with_active_sessions(70)
                .with_max_connections(100),
        )
        .expect("lock evidence should classify");
        assert_eq!(report.status, DatabaseContentionStatus::DatabaseContention);
        assert_eq!(
            report.dominant_cause,
            DatabaseContentionCause::LockContention
        );
        assert!(report.suppress_pool_increase);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "LOCK_WAITING_SESSIONS"));
    }

    #[test]
    fn long_idle_transactions_are_database_contention() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new()
                .with_idle_in_transaction_sessions(2)
                .with_longest_idle_in_transaction_seconds(90.0)
                .with_lock_waiting_sessions(0)
                .with_deadlocks_per_second(0.0)
                .with_database_latency_p99_ms(20.0),
        )
        .expect("idle transaction evidence should classify");
        assert_eq!(
            report.dominant_cause,
            DatabaseContentionCause::IdleTransaction
        );
        assert!(report.suppress_pool_increase);
    }

    #[test]
    fn deadlocks_are_critical_contention() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new()
                .with_deadlocks_per_second(0.1)
                .with_lock_waiting_sessions(0)
                .with_idle_in_transaction_sessions(0)
                .with_database_latency_p99_ms(20.0),
        )
        .expect("deadlock evidence should classify");
        assert_eq!(report.dominant_cause, DatabaseContentionCause::Deadlock);
        assert!(report
            .findings
            .iter()
            .any(|f| f.risk == RiskLevel::Critical));
    }

    #[test]
    fn healthy_database_evidence_can_identify_pool_starvation() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new()
                .with_pool_wait_p99_ms(40.0)
                .with_database_latency_p99_ms(20.0)
                .with_lock_waiting_sessions(0)
                .with_idle_in_transaction_sessions(0)
                .with_deadlocks_per_second(0.0)
                .with_active_sessions(40)
                .with_max_connections(100)
                .with_database_cpu_utilization(0.4)
                .with_database_io_utilization(0.4),
        )
        .expect("complete healthy evidence should classify");
        assert_eq!(report.status, DatabaseContentionStatus::PoolStarvation);
        assert_eq!(
            report.dominant_cause,
            DatabaseContentionCause::PoolStarvation
        );
        assert!(!report.suppress_pool_increase);
    }

    #[test]
    fn complete_quiet_evidence_is_healthy_without_a_cause() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new()
                .with_database_latency_p99_ms(20.0)
                .with_lock_waiting_sessions(0)
                .with_idle_in_transaction_sessions(0)
                .with_deadlocks_per_second(0.0)
                .with_active_sessions(40)
                .with_max_connections(100)
                .with_database_cpu_utilization(0.4)
                .with_database_io_utilization(0.4),
        )
        .expect("complete quiet evidence should classify");
        assert_eq!(report.status, DatabaseContentionStatus::Healthy);
        assert_eq!(report.dominant_cause, DatabaseContentionCause::None);
        assert!(!report.suppress_pool_increase);
    }

    #[test]
    fn incomplete_pool_evidence_needs_review() {
        let report = classify_database_contention(
            &DatabaseContentionInput::new().with_pool_wait_p99_ms(40.0),
        )
        .expect("partial evidence should classify");
        assert_eq!(report.status, DatabaseContentionStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Medium);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "DATABASE_EVIDENCE_INCOMPLETE"));
    }

    #[test]
    fn no_evidence_is_low_confidence_review() {
        let report = classify_database_contention(&DatabaseContentionInput::new())
            .expect("empty input should produce review report");
        assert_eq!(report.status, DatabaseContentionStatus::NeedsReview);
        assert_eq!(report.dominant_cause, DatabaseContentionCause::Unknown);
        assert_eq!(report.confidence, EvidenceConfidence::Low);
    }

    #[test]
    fn invalid_evidence_and_policy_return_stable_errors() {
        let invalid = DatabaseContentionInput::new().with_database_cpu_utilization(1.1);
        assert_eq!(
            classify_database_contention(&invalid)
                .expect_err("utilization above one should fail")
                .code(),
            "INVALID_DATABASE_CONTENTION"
        );

        let invalid_policy = DatabaseContentionInput::new()
            .with_policy(DatabaseContentionPolicy::new().with_pool_wait_p99_threshold_ms(f64::NAN));
        assert_eq!(
            classify_database_contention(&invalid_policy)
                .expect_err("non-finite threshold should fail")
                .code(),
            "INVALID_DATABASE_CONTENTION"
        );

        let invalid_capacity = DatabaseContentionInput::new()
            .with_active_sessions(2)
            .with_max_connections(1);
        assert_eq!(
            classify_database_contention(&invalid_capacity)
                .expect_err("active sessions above capacity should fail")
                .code(),
            "INVALID_DATABASE_CONTENTION"
        );
    }
}
