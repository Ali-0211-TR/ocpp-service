//! Background task that periodically expires overdue reservations.
//!
//! Runs in a tokio::spawn loop, checking every 60 seconds for active
//! reservations past their `expiry_date` and marking them as `Expired`.

use std::sync::Arc;

use tokio::time::Duration;
use tracing::{info, warn};

use crate::domain::RepositoryProvider;
use crate::shared::shutdown::ShutdownSignal;

/// Start the reservation expiry background task.
///
/// The task checks every `check_interval_secs` (default 60) for
/// reservations with status "Accepted" and `expiry_date < now()`,
/// then updates them to "Expired".
pub fn start_reservation_expiry_task(
    repos: Arc<dyn RepositoryProvider>,
    shutdown: ShutdownSignal,
    check_interval_secs: u64,
) {
    tokio::spawn(async move {
        info!(
            check_interval = check_interval_secs,
            "📅 Reservation expiry task started"
        );

        let mut interval = tokio::time::interval(Duration::from_secs(check_interval_secs));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = expire_reservations(&repos).await {
                        warn!(error = %e, "Reservation expiry check error");
                    }
                }
                _ = shutdown.notified().wait() => {
                    info!("📅 Reservation expiry task shutting down");
                    break;
                }
            }
        }

        info!("📅 Reservation expiry task stopped");
    });
}

async fn expire_reservations(
    repos: &Arc<dyn RepositoryProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
    let expired = repos.reservations().find_expired().await?;

    if expired.is_empty() {
        return Ok(());
    }

    info!(count = expired.len(), "Expiring overdue reservations");

    for mut reservation in expired {
        reservation.expire();
        if let Err(e) = repos.reservations().update(reservation).await {
            warn!(error = %e, "Failed to expire reservation");
        }
    }

    Ok(())
}
