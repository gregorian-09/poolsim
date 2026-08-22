//! Evidence-quality checks for telemetry used in capacity planning.
//!
//! A latency percentile is not enough to justify a pool-size change. This
//! module checks whether the capture describes its arrival model, sustained
//! offered rate, sample window, failure signals, and the pool/database wait
//! signals needed to distinguish a pool bottleneck from backend contention.

use serde::{Deserialize, Serialize};

use crate::{error::PoolsimError, pooler::EvidenceConfidence, types::RiskLevel};

/// Arrival model used to produce a telemetry or load-test capture.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TelemetryArrivalModel {
    /// Requests are scheduled independently of completion latency.
    OpenLoop,
    /// A new request is scheduled after a prior request completes.
    ClosedLoop,
    /// The capture does not identify its arrival model.
    Unknown,
}

/// Input evidence used to assess whether telemetry is suitable for capacity planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct TelemetryQualityInput {
    /// Arrival model used by the source or load generator.
    pub arrival_model: TelemetryArrivalModel,
    /// Intended offered request rate in requests per second, when known.
    #[serde(default)]
    pub expected_requests_per_second: Option<f64>,
    /// Actually observed completed request rate in requests per second, when known.
    #[serde(default)]
    pub observed_requests_per_second: Option<f64>,
    /// Duration of the analyzed capture window in seconds, when known.
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    /// Number of latency observations in the analyzed window, when known.
    #[serde(default)]
    pub sample_count: Option<u64>,
    /// Number of request timeouts in the analyzed window, when known.
    #[serde(default)]
    pub timeout_count: Option<u64>,
    /// Number of request errors in the analyzed window, when known.
    #[serde(default)]
    pub error_count: Option<u64>,
    /// p50 end-to-end latency in milliseconds, when known.
    #[serde(default)]
    pub latency_p50_ms: Option<f64>,
    /// p95 end-to-end latency in milliseconds, when known.
    #[serde(default)]
    pub latency_p95_ms: Option<f64>,
    /// p99 end-to-end latency in milliseconds, when known.
    #[serde(default)]
    pub latency_p99_ms: Option<f64>,
    /// p99 time waiting to acquire an application-pool connection, in milliseconds.
    #[serde(default)]
    pub pool_wait_p99_ms: Option<f64>,
    /// p99 database service or query latency, in milliseconds.
    #[serde(default)]
    pub database_latency_p99_ms: Option<f64>,
    /// Whether the latency distribution was corrected for coordinated omission.
    #[serde(default)]
    pub corrected_for_coordinated_omission: Option<bool>,
}

impl TelemetryQualityInput {
    /// Creates quality evidence with an explicit arrival model.
    pub fn new(arrival_model: TelemetryArrivalModel) -> Self {
        Self {
            arrival_model,
            expected_requests_per_second: None,
            observed_requests_per_second: None,
            duration_seconds: None,
            sample_count: None,
            timeout_count: None,
            error_count: None,
            latency_p50_ms: None,
            latency_p95_ms: None,
            latency_p99_ms: None,
            pool_wait_p99_ms: None,
            database_latency_p99_ms: None,
            corrected_for_coordinated_omission: None,
        }
    }

    /// Sets the intended offered request rate.
    #[must_use]
    pub fn with_expected_requests_per_second(mut self, value: f64) -> Self {
        self.expected_requests_per_second = Some(value);
        self
    }

    /// Sets the observed completed request rate.
    #[must_use]
    pub fn with_observed_requests_per_second(mut self, value: f64) -> Self {
        self.observed_requests_per_second = Some(value);
        self
    }

    /// Sets the analyzed capture duration in seconds.
    #[must_use]
    pub fn with_duration_seconds(mut self, value: f64) -> Self {
        self.duration_seconds = Some(value);
        self
    }

    /// Sets the number of latency observations.
    #[must_use]
    pub fn with_sample_count(mut self, value: u64) -> Self {
        self.sample_count = Some(value);
        self
    }

    /// Sets the timeout count.
    #[must_use]
    pub fn with_timeout_count(mut self, value: u64) -> Self {
        self.timeout_count = Some(value);
        self
    }

    /// Sets the error count.
    #[must_use]
    pub fn with_error_count(mut self, value: u64) -> Self {
        self.error_count = Some(value);
        self
    }

    /// Sets p50, p95, and p99 end-to-end latency values in milliseconds.
    #[must_use]
    pub fn with_latency_percentiles(mut self, p50_ms: f64, p95_ms: f64, p99_ms: f64) -> Self {
        self.latency_p50_ms = Some(p50_ms);
        self.latency_p95_ms = Some(p95_ms);
        self.latency_p99_ms = Some(p99_ms);
        self
    }

    /// Sets p99 application-pool acquisition wait in milliseconds.
    #[must_use]
    pub fn with_pool_wait_p99_ms(mut self, value: f64) -> Self {
        self.pool_wait_p99_ms = Some(value);
        self
    }

    /// Sets p99 database service latency in milliseconds.
    #[must_use]
    pub fn with_database_latency_p99_ms(mut self, value: f64) -> Self {
        self.database_latency_p99_ms = Some(value);
        self
    }

    /// Records whether the source corrected latency for coordinated omission.
    #[must_use]
    pub fn with_coordinated_omission_correction(mut self, value: bool) -> Self {
        self.corrected_for_coordinated_omission = Some(value);
        self
    }
}

/// Machine-readable finding from telemetry-quality assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct TelemetryQualityFinding {
    /// Stable machine-readable finding code.
    pub code: String,
    /// Severity/risk associated with this finding.
    pub risk: RiskLevel,
    /// Human-readable explanation.
    pub message: String,
    /// Recommended remediation or next verification step.
    pub remediation: String,
}

impl TelemetryQualityFinding {
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

/// Overall suitability of telemetry for automatic capacity decisions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TelemetryQualityStatus {
    /// Required evidence is present and no material quality risk was found.
    Valid,
    /// Evidence is useful, but a human should review limitations before acting.
    NeedsReview,
    /// The capture cannot support a capacity decision without a new run or better evidence.
    Rejected,
}

/// Result of assessing telemetry quality and coordinated-omission risk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct TelemetryQualityReport {
    /// Overall suitability for automatic capacity planning.
    pub status: TelemetryQualityStatus,
    /// Arrival model supplied by the caller.
    pub arrival_model: TelemetryArrivalModel,
    /// Estimated observed/expected offered-rate ratio, when both rates exist.
    #[serde(default)]
    pub observed_rate_ratio: Option<f64>,
    /// Risk that the latency distribution omitted time spent waiting to issue a request.
    pub coordinated_omission_risk: RiskLevel,
    /// Findings explaining missing evidence or quality risks.
    pub findings: Vec<TelemetryQualityFinding>,
    /// Confidence in the quality assessment.
    pub confidence: EvidenceConfidence,
}

/// Assesses whether telemetry is defensible for pool-size capacity planning.
///
/// This function does not alter the existing telemetry recommendation workflow.
/// It produces an independent report that callers can run before passing a
/// [`crate::telemetry::TelemetrySnapshot`] to the sizing engine.
///
/// # Errors
///
/// Returns [`PoolsimError`] when numeric values are non-finite, negative where
/// impossible, or the latency percentiles are not monotonically ordered.
pub fn assess_telemetry_quality(
    input: &TelemetryQualityInput,
) -> Result<TelemetryQualityReport, PoolsimError> {
    validate_numeric_input(input)?;

    let mut findings = Vec::new();
    let mut confidence = EvidenceConfidence::High;
    let mut coordinated_omission_risk = RiskLevel::Low;

    match input.arrival_model {
        TelemetryArrivalModel::OpenLoop => {}
        TelemetryArrivalModel::ClosedLoop => {
            let corrected = input.corrected_for_coordinated_omission == Some(true);
            coordinated_omission_risk = if corrected {
                RiskLevel::Medium
            } else {
                RiskLevel::High
            };
            confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
            findings.push(TelemetryQualityFinding::new(
                "COORDINATED_OMISSION_RISK",
                if corrected {
                    RiskLevel::Medium
                } else {
                    RiskLevel::High
                },
                if corrected {
                    "closed-loop arrival can coordinate with service stalls even though correction was recorded"
                } else {
                    "closed-loop arrival can stop issuing work during service stalls and hide tail latency"
                },
                if corrected {
                    "retain the correction metadata and prefer an open-loop or constant-rate run for the next capacity decision"
                } else {
                    "rerun with an open-loop constant-rate model or apply a documented expected-interval correction"
                },
            ));
        }
        TelemetryArrivalModel::Unknown => {
            coordinated_omission_risk = RiskLevel::Medium;
            confidence = lower_confidence(confidence, EvidenceConfidence::Low);
            findings.push(TelemetryQualityFinding::new(
                "ARRIVAL_MODEL_UNKNOWN",
                RiskLevel::High,
                "the capture does not identify whether arrivals were open-loop or closed-loop",
                "record the load generator arrival model and intended rate with every capacity run",
            ));
        }
    }

    let observed_rate_ratio = match (
        input.expected_requests_per_second,
        input.observed_requests_per_second,
    ) {
        (Some(expected), Some(observed)) => {
            let ratio = observed / expected;
            if ratio < 0.90 {
                findings.push(TelemetryQualityFinding::new(
                    "OFFERED_RATE_NOT_REACHED",
                    RiskLevel::Critical,
                    "observed throughput is less than 90% of the intended offered rate",
                    "fix load-generator capacity, errors, timeouts, or admission limits before using the run for sizing",
                ));
            } else if ratio < 0.95 {
                confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
                findings.push(TelemetryQualityFinding::new(
                    "OFFERED_RATE_DRIFT",
                    RiskLevel::High,
                    "observed throughput is below the intended offered rate",
                    "investigate generator saturation and report the achieved rate alongside the intended rate",
                ));
            }
            Some(ratio)
        }
        _ => {
            confidence = lower_confidence(confidence, EvidenceConfidence::Medium);
            findings.push(TelemetryQualityFinding::new(
                "ARRIVAL_RATE_MISSING",
                RiskLevel::High,
                "expected and observed request rates are required to verify offered load",
                "record both the intended rate and achieved completed rate for the analyzed window",
            ));
            None
        }
    };

    add_missing_count_finding(
        &mut findings,
        &mut confidence,
        input.sample_count,
        "SAMPLE_COUNT_MISSING",
        RiskLevel::Medium,
        "latency sample count is missing",
        "export observation count with p50, p95, and p99 so sparse windows can be identified",
    );
    add_missing_float_finding(
        &mut findings,
        &mut confidence,
        input.duration_seconds,
        "DURATION_MISSING",
        "capture duration is missing",
        "record the exact analysis-window duration with the telemetry snapshot",
    );
    add_missing_count_finding(
        &mut findings,
        &mut confidence,
        input.timeout_count,
        "TIMEOUT_COUNT_MISSING",
        RiskLevel::High,
        "request timeout count is missing",
        "export timeout counts separately from generic errors before making a pool decision",
    );
    add_missing_count_finding(
        &mut findings,
        &mut confidence,
        input.error_count,
        "ERROR_COUNT_MISSING",
        RiskLevel::Medium,
        "request error count is missing",
        "export error counts and classify them by timeout, cancellation, and backend failure",
    );
    add_missing_float_finding(
        &mut findings,
        &mut confidence,
        input.pool_wait_p99_ms,
        "POOL_WAIT_METRIC_MISSING",
        "application-pool acquisition wait is missing",
        "export pool acquire or checkout latency to distinguish pool contention from database latency",
    );
    add_missing_float_finding(
        &mut findings,
        &mut confidence,
        input.database_latency_p99_ms,
        "DATABASE_LATENCY_METRIC_MISSING",
        "database service latency is missing",
        "export database query or service latency with pool wait so a slower backend is not mistaken for a small pool",
    );

    let status = if findings.iter().any(|finding| {
        matches!(
            finding.code.as_str(),
            "OFFERED_RATE_NOT_REACHED" | "COORDINATED_OMISSION_RISK"
        ) && finding.risk >= RiskLevel::High
    }) {
        TelemetryQualityStatus::Rejected
    } else if findings.is_empty() {
        TelemetryQualityStatus::Valid
    } else {
        TelemetryQualityStatus::NeedsReview
    };

    Ok(TelemetryQualityReport {
        status,
        arrival_model: input.arrival_model,
        observed_rate_ratio,
        coordinated_omission_risk,
        findings,
        confidence,
    })
}

fn validate_numeric_input(input: &TelemetryQualityInput) -> Result<(), PoolsimError> {
    for (name, value) in [
        (
            "expected_requests_per_second",
            input.expected_requests_per_second,
        ),
        (
            "observed_requests_per_second",
            input.observed_requests_per_second,
        ),
        ("duration_seconds", input.duration_seconds),
        ("latency_p50_ms", input.latency_p50_ms),
        ("latency_p95_ms", input.latency_p95_ms),
        ("latency_p99_ms", input.latency_p99_ms),
        ("pool_wait_p99_ms", input.pool_wait_p99_ms),
        ("database_latency_p99_ms", input.database_latency_p99_ms),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || value < 0.0 {
                return Err(invalid_quality(format!(
                    "{name} must be finite and non-negative"
                )));
            }
        }
    }
    for (name, value) in [
        (
            "expected_requests_per_second",
            input.expected_requests_per_second,
        ),
        ("duration_seconds", input.duration_seconds),
    ] {
        if value == Some(0.0) {
            return Err(invalid_quality(format!("{name} must be greater than zero")));
        }
    }
    if input.sample_count == Some(0) {
        return Err(invalid_quality("sample_count must be greater than zero"));
    }
    if let (Some(p50), Some(p95)) = (input.latency_p50_ms, input.latency_p95_ms) {
        if p50 > p95 {
            return Err(invalid_quality(
                "latency_p50_ms cannot be greater than latency_p95_ms",
            ));
        }
    }
    if let (Some(p95), Some(p99)) = (input.latency_p95_ms, input.latency_p99_ms) {
        if p95 > p99 {
            return Err(invalid_quality(
                "latency_p95_ms cannot be greater than latency_p99_ms",
            ));
        }
    }
    Ok(())
}

fn add_missing_count_finding(
    findings: &mut Vec<TelemetryQualityFinding>,
    confidence: &mut EvidenceConfidence,
    value: Option<u64>,
    code: &'static str,
    risk: RiskLevel,
    message: &'static str,
    remediation: &'static str,
) {
    if value.is_none() {
        *confidence = lower_confidence(*confidence, EvidenceConfidence::Medium);
        findings.push(TelemetryQualityFinding::new(
            code,
            risk,
            message,
            remediation,
        ));
    }
}

fn add_missing_float_finding(
    findings: &mut Vec<TelemetryQualityFinding>,
    confidence: &mut EvidenceConfidence,
    value: Option<f64>,
    code: &'static str,
    message: &'static str,
    remediation: &'static str,
) {
    if value.is_none() {
        *confidence = lower_confidence(*confidence, EvidenceConfidence::Medium);
        findings.push(TelemetryQualityFinding::new(
            code,
            RiskLevel::Medium,
            message,
            remediation,
        ));
    }
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

fn invalid_quality(message: impl Into<String>) -> PoolsimError {
    PoolsimError::invalid_input("INVALID_TELEMETRY_QUALITY", message, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_input(model: TelemetryArrivalModel) -> TelemetryQualityInput {
        TelemetryQualityInput::new(model)
            .with_expected_requests_per_second(1_000.0)
            .with_observed_requests_per_second(1_000.0)
            .with_duration_seconds(60.0)
            .with_sample_count(60_000)
            .with_timeout_count(2)
            .with_error_count(5)
            .with_latency_percentiles(10.0, 30.0, 70.0)
            .with_pool_wait_p99_ms(4.0)
            .with_database_latency_p99_ms(18.0)
    }

    #[test]
    fn open_loop_complete_evidence_is_valid() {
        let report = assess_telemetry_quality(&complete_input(TelemetryArrivalModel::OpenLoop))
            .expect("complete open-loop evidence should assess");

        assert_eq!(report.status, TelemetryQualityStatus::Valid);
        assert_eq!(report.observed_rate_ratio, Some(1.0));
        assert_eq!(report.coordinated_omission_risk, RiskLevel::Low);
        assert!(report.findings.is_empty());
        assert_eq!(report.confidence, EvidenceConfidence::High);
    }

    #[test]
    fn closed_loop_uncorrected_evidence_is_rejected_for_capacity_planning() {
        let report = assess_telemetry_quality(&complete_input(TelemetryArrivalModel::ClosedLoop))
            .expect("closed-loop evidence should assess");

        assert_eq!(report.status, TelemetryQualityStatus::Rejected);
        assert_eq!(report.coordinated_omission_risk, RiskLevel::High);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "COORDINATED_OMISSION_RISK"));
    }

    #[test]
    fn corrected_closed_loop_evidence_needs_review_but_is_not_rejected() {
        let input = complete_input(TelemetryArrivalModel::ClosedLoop)
            .with_coordinated_omission_correction(true);
        let report =
            assess_telemetry_quality(&input).expect("corrected closed-loop evidence should assess");

        assert_eq!(report.status, TelemetryQualityStatus::NeedsReview);
        assert_eq!(report.coordinated_omission_risk, RiskLevel::Medium);
        assert_eq!(report.confidence, EvidenceConfidence::Medium);
    }

    #[test]
    fn rate_shortfall_is_rejected_and_ratio_is_preserved() {
        let input = complete_input(TelemetryArrivalModel::OpenLoop)
            .with_observed_requests_per_second(850.0);
        let report = assess_telemetry_quality(&input).expect("rate shortfall should assess");

        assert_eq!(report.status, TelemetryQualityStatus::Rejected);
        assert_eq!(report.observed_rate_ratio, Some(0.85));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "OFFERED_RATE_NOT_REACHED"));
    }

    #[test]
    fn missing_operational_signals_need_review() {
        let input = TelemetryQualityInput::new(TelemetryArrivalModel::OpenLoop)
            .with_expected_requests_per_second(100.0)
            .with_observed_requests_per_second(100.0)
            .with_duration_seconds(30.0)
            .with_sample_count(3_000)
            .with_latency_percentiles(5.0, 10.0, 20.0);
        let report = assess_telemetry_quality(&input).expect("partial evidence should assess");

        assert_eq!(report.status, TelemetryQualityStatus::NeedsReview);
        for code in [
            "TIMEOUT_COUNT_MISSING",
            "ERROR_COUNT_MISSING",
            "POOL_WAIT_METRIC_MISSING",
            "DATABASE_LATENCY_METRIC_MISSING",
        ] {
            assert!(report.findings.iter().any(|finding| finding.code == code));
        }
    }

    #[test]
    fn unknown_arrival_model_and_missing_rate_lower_confidence() {
        let report = assess_telemetry_quality(
            &TelemetryQualityInput::new(TelemetryArrivalModel::Unknown)
                .with_duration_seconds(30.0)
                .with_sample_count(3_000)
                .with_timeout_count(0)
                .with_error_count(0),
        )
        .expect("unknown arrival model should assess");

        assert_eq!(report.status, TelemetryQualityStatus::NeedsReview);
        assert_eq!(report.confidence, EvidenceConfidence::Low);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "ARRIVAL_MODEL_UNKNOWN"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "ARRIVAL_RATE_MISSING"));
    }

    #[test]
    fn invalid_quality_values_return_stable_errors() {
        let error = assess_telemetry_quality(
            &complete_input(TelemetryArrivalModel::OpenLoop)
                .with_latency_percentiles(30.0, 20.0, 70.0),
        )
        .expect_err("non-monotonic percentiles should fail");
        assert_eq!(error.code(), "INVALID_TELEMETRY_QUALITY");

        let error = assess_telemetry_quality(
            &complete_input(TelemetryArrivalModel::OpenLoop).with_duration_seconds(f64::NAN),
        )
        .expect_err("non-finite duration should fail");
        assert_eq!(error.code(), "INVALID_TELEMETRY_QUALITY");
    }

    #[test]
    fn quality_validation_covers_drift_zero_and_percentile_order_errors() {
        let report = assess_telemetry_quality(
            &complete_input(TelemetryArrivalModel::OpenLoop)
                .with_observed_requests_per_second(920.0),
        )
        .expect("rate drift should assess");
        assert_eq!(report.status, TelemetryQualityStatus::NeedsReview);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "OFFERED_RATE_DRIFT"));

        for input in [
            complete_input(TelemetryArrivalModel::OpenLoop).with_expected_requests_per_second(0.0),
            complete_input(TelemetryArrivalModel::OpenLoop).with_duration_seconds(0.0),
            complete_input(TelemetryArrivalModel::OpenLoop).with_sample_count(0),
            complete_input(TelemetryArrivalModel::OpenLoop)
                .with_latency_percentiles(10.0, 70.0, 30.0),
        ] {
            let error = assess_telemetry_quality(&input)
                .expect_err("invalid quality input should be rejected");
            assert_eq!(error.code(), "INVALID_TELEMETRY_QUALITY");
        }

        assert_eq!(
            lower_confidence(EvidenceConfidence::High, EvidenceConfidence::High),
            EvidenceConfidence::High
        );
    }
}
