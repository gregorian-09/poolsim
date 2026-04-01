//! Erlang-C queueing helpers used by sizing and sensitivity calculations.
//!
//! This module is the analytical queueing side of `poolsim-core`.
//! It is primarily used when the queue model is M/M/c and provides:
//!
//! - utilisation (`rho`)
//! - wait probability
//! - mean queue wait
//! - queue-wait percentiles
//!
//! When the queue model is M/D/c, the crate falls back to Monte Carlo probing
//! for the quantities that do not have a direct closed-form implementation here.

use crate::error::PoolsimError;

/// Computes utilization (`rho = lambda / (c * mu)`).
pub fn utilisation(lambda: f64, mu: f64, c: u32) -> f64 {
    if c == 0 || mu <= 0.0 {
        return f64::INFINITY;
    }
    lambda / (c as f64 * mu)
}

/// Computes Erlang-C waiting probability for an M/M/c queue.
///
/// # Errors
///
/// Returns [`PoolsimError::InvalidInput`] when `c == 0` or `mu <= 0`,
/// and [`PoolsimError::Saturated`] when `rho >= 1.0`.
pub fn erlang_c(lambda: f64, mu: f64, c: u32) -> Result<f64, PoolsimError> {
    if c == 0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_SERVER_COUNT",
            "server count must be > 0",
            None,
        ));
    }
    if mu <= 0.0 {
        return Err(PoolsimError::invalid_input(
            "INVALID_SERVICE_RATE",
            "service rate must be > 0",
            None,
        ));
    }
    if lambda <= 0.0 {
        return Ok(0.0);
    }

    let rho = utilisation(lambda, mu, c);
    if rho >= 1.0 {
        return Err(PoolsimError::Saturated { rho });
    }

    let offered_load = lambda / mu;
    let mut sum = 1.0;
    let mut term = 1.0;

    for k in 1..c {
        term *= offered_load / k as f64;
        sum += term;
    }

    let term_c = term * offered_load / c as f64;
    let top = term_c / (1.0 - rho);
    Ok(top / (sum + top))
}

/// Computes mean queue wait (milliseconds) for an M/M/c queue.
///
/// # Errors
///
/// Returns the same errors as [`erlang_c`] and saturated errors when the
/// denominator term becomes non-positive.
pub fn mean_queue_wait_ms(lambda: f64, mu: f64, c: u32) -> Result<f64, PoolsimError> {
    if lambda <= 0.0 {
        return Ok(0.0);
    }

    let p_wait = erlang_c(lambda, mu, c)?;
    let denom = c as f64 * mu - lambda;
    if !denom.is_finite() || denom <= 0.0 {
        return Err(PoolsimError::Saturated {
            rho: utilisation(lambda, mu, c),
        });
    }

    Ok((p_wait / denom) * 1_000.0)
}

/// Computes queue-wait percentile (milliseconds) for an M/M/c queue.
///
/// `quantile` is clamped into `[0, 1]`.
///
/// # Errors
///
/// Returns the same errors as [`erlang_c`] and saturated errors when the
/// tail rate becomes non-positive.
pub fn queue_wait_percentile_ms(lambda: f64, mu: f64, c: u32, quantile: f64) -> Result<f64, PoolsimError> {
    if lambda <= 0.0 {
        return Ok(0.0);
    }

    let q = quantile.clamp(0.0, 1.0);
    if q == 0.0 {
        return Ok(0.0);
    }

    let p_wait = erlang_c(lambda, mu, c)?;
    if q <= 1.0 - p_wait {
        return Ok(0.0);
    }

    let rate = c as f64 * mu - lambda;
    if !rate.is_finite() || rate <= 0.0 {
        return Err(PoolsimError::Saturated {
            rho: utilisation(lambda, mu, c),
        });
    }

    let tail = ((1.0 - q) / p_wait).max(f64::MIN_POSITIVE);
    Ok((-tail.ln() / rate) * 1_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erlang_c_known_case() {
        let c = 10;
        let mu = 1.0;
        let lambda = 8.0;
        let p_wait = erlang_c(lambda, mu, c).expect("valid erlang c");
        assert!((p_wait - 0.40918).abs() < 0.005);
    }

    #[test]
    fn erlang_c_reference_matrix() {
        let mu = 1.0;
        let cases = [
            (2, 0.5, 0.33333333),
            (2, 0.8, 0.71111111),
            (2, 0.9, 0.85263158),
            (3, 0.7, 0.49234450),
            (3, 0.9, 0.81706102),
            (4, 0.5, 0.17391304),
            (4, 0.8, 0.59643247),
            (4, 0.95, 0.89141900),
            (5, 0.7, 0.37783823),
            (5, 0.9, 0.76249322),
            (6, 0.8, 0.51777200),
            (6, 0.95, 0.86558880),
            (8, 0.7, 0.27060293),
            (8, 0.9, 0.70153299),
            (10, 0.8, 0.40918015),
            (10, 0.95, 0.82558558),
            (12, 0.7, 0.18388863),
            (12, 0.9, 0.64004291),
            (16, 0.8, 0.30488391),
            (20, 0.9, 0.55076900),
        ];

        for (c, rho, expected) in cases {
            let lambda = rho * c as f64 * mu;
            let actual = erlang_c(lambda, mu, c).expect("reference case should be valid");
            assert!(
                (actual - expected).abs() < 1e-6,
                "c={c}, rho={rho}, expected={expected}, actual={actual}"
            );
        }
    }

    #[test]
    fn mean_queue_wait_increases_as_utilisation_rises() {
        let c = 8;
        let mu = 1.0;
        let low = mean_queue_wait_ms(0.5 * c as f64 * mu, mu, c).expect("low utilisation should work");
        let high = mean_queue_wait_ms(0.9 * c as f64 * mu, mu, c).expect("high utilisation should work");
        assert!(high > low);
    }

    #[test]
    fn queue_percentile_is_zero_when_quantile_in_non_waiting_mass() {
        let c = 4;
        let mu = 1.0;
        let lambda = 0.5 * c as f64 * mu;
        let p_wait = erlang_c(lambda, mu, c).expect("valid erlang c");
        let threshold = 1.0 - p_wait;
        let q = threshold * 0.99;
        let value = queue_wait_percentile_ms(lambda, mu, c, q).expect("valid percentile");
        assert_eq!(value, 0.0);
    }

    #[test]
    fn nan_service_rate_maps_to_saturated_in_wait_metrics() {
        let err = mean_queue_wait_ms(1.0, f64::NAN, 2).expect_err("nan service rate should fail");
        assert_eq!(err.code(), "SATURATED");

        let err = queue_wait_percentile_ms(1.0, f64::NAN, 2, 0.99).expect_err("nan service rate should fail");
        assert_eq!(err.code(), "SATURATED");
    }
}
