//! Retry with exponential backoff
//!
//! Generic retry helper for transient failures (DB timeouts, network blips).
//! Used for critical operations where a single failure should not be fatal
//! (e.g. billing, stop_transaction).

use std::future::Future;
use std::time::Duration;
use tracing::{warn, info};

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the first one).
    pub max_attempts: u32,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Multiplier applied to the delay after each retry.
    pub backoff_multiplier: f64,
    /// Maximum delay between retries (cap).
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(200),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(5),
        }
    }
}

/// Execute an async operation with exponential backoff retry.
///
/// The `should_retry` closure determines whether a given error is transient
/// (and therefore retryable) or permanent (bail immediately).
///
/// # Example
/// ```ignore
/// let result = retry_with_backoff(
///     RetryConfig::default(),
///     || billing_service.calculate_transaction_billing(tx_id, tariff_id),
///     |err| matches!(err, DomainError::Infra(_)),
///     "calculate_billing",
/// ).await;
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut operation: F,
    should_retry: impl Fn(&E) -> bool,
    operation_name: &str,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = config.initial_delay;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(value) => {
                if attempt > 1 {
                    info!(
                        operation = operation_name,
                        attempt,
                        "Succeeded after retry"
                    );
                }
                return Ok(value);
            }
            Err(err) => {
                if attempt == config.max_attempts || !should_retry(&err) {
                    warn!(
                        operation = operation_name,
                        attempt,
                        max_attempts = config.max_attempts,
                        error = %err,
                        "Operation failed permanently"
                    );
                    return Err(err);
                }

                warn!(
                    operation = operation_name,
                    attempt,
                    max_attempts = config.max_attempts,
                    error = %err,
                    retry_in_ms = delay.as_millis() as u64,
                    "Transient failure, retrying…"
                );

                tokio::time::sleep(delay).await;

                // Exponential backoff with cap
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.backoff_multiplier)
                        .min(config.max_delay.as_secs_f64()),
                );
            }
        }
    }

    unreachable!("Loop exits via return")
}
