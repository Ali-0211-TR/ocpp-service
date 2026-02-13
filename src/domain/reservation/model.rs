//! Reservation domain entity

use chrono::{DateTime, Utc};

/// Reservation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationStatus {
    /// Reservation accepted by the charge point
    Accepted,
    /// Reservation cancelled by user or system
    Cancelled,
    /// Reservation expired (past expiry_date)
    Expired,
    /// Reservation was used (transaction started)
    Used,
}

impl ReservationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "Accepted",
            Self::Cancelled => "Cancelled",
            Self::Expired => "Expired",
            Self::Used => "Used",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Accepted" => Self::Accepted,
            "Cancelled" => Self::Cancelled,
            "Expired" => Self::Expired,
            "Used" => Self::Used,
            _ => Self::Cancelled,
        }
    }
}

impl std::fmt::Display for ReservationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Connector reservation
#[derive(Debug, Clone)]
pub struct Reservation {
    /// Unique reservation ID
    pub id: i32,
    /// Charge point ID
    pub charge_point_id: String,
    /// Connector ID (0 = any connector)
    pub connector_id: i32,
    /// ID tag (RFID) authorized for this reservation
    pub id_tag: String,
    /// Parent ID tag (group)
    pub parent_id_tag: Option<String>,
    /// Reservation expiry date
    pub expiry_date: DateTime<Utc>,
    /// Current status
    pub status: ReservationStatus,
    /// When the reservation was created
    pub created_at: DateTime<Utc>,
}

impl Reservation {
    pub fn new(
        id: i32,
        charge_point_id: impl Into<String>,
        connector_id: i32,
        id_tag: impl Into<String>,
        parent_id_tag: Option<String>,
        expiry_date: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            charge_point_id: charge_point_id.into(),
            connector_id,
            id_tag: id_tag.into(),
            parent_id_tag,
            expiry_date,
            status: ReservationStatus::Accepted,
            created_at: Utc::now(),
        }
    }

    /// Cancel this reservation
    pub fn cancel(&mut self) {
        self.status = ReservationStatus::Cancelled;
    }

    /// Mark as expired
    pub fn expire(&mut self) {
        self.status = ReservationStatus::Expired;
    }

    /// Mark as used (transaction started)
    pub fn mark_used(&mut self) {
        self.status = ReservationStatus::Used;
    }

    /// Check if this reservation is still active
    pub fn is_active(&self) -> bool {
        self.status == ReservationStatus::Accepted
    }

    /// Check if this reservation has expired
    pub fn is_expired(&self) -> bool {
        self.status == ReservationStatus::Expired || Utc::now() > self.expiry_date
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_reservation() -> Reservation {
        Reservation::new(
            1,
            "CP001",
            1,
            "TAG-001",
            None,
            Utc::now() + Duration::hours(1),
        )
    }

    #[test]
    fn new_reservation_is_active() {
        let r = sample_reservation();
        assert!(r.is_active());
        assert!(!r.is_expired());
        assert_eq!(r.status, ReservationStatus::Accepted);
        assert_eq!(r.connector_id, 1);
    }

    #[test]
    fn cancel_sets_cancelled() {
        let mut r = sample_reservation();
        r.cancel();
        assert_eq!(r.status, ReservationStatus::Cancelled);
        assert!(!r.is_active());
    }

    #[test]
    fn expire_sets_expired() {
        let mut r = sample_reservation();
        r.expire();
        assert_eq!(r.status, ReservationStatus::Expired);
        assert!(r.is_expired());
    }

    #[test]
    fn mark_used_sets_used() {
        let mut r = sample_reservation();
        r.mark_used();
        assert_eq!(r.status, ReservationStatus::Used);
        assert!(!r.is_active());
    }

    #[test]
    fn expired_when_past_expiry_date() {
        let r = Reservation::new(
            2,
            "CP001",
            1,
            "TAG-001",
            None,
            Utc::now() - Duration::hours(1), // already expired
        );
        assert!(r.is_expired());
    }

    #[test]
    fn status_display_roundtrip() {
        for status in &[
            ReservationStatus::Accepted,
            ReservationStatus::Cancelled,
            ReservationStatus::Expired,
            ReservationStatus::Used,
        ] {
            let s = status.as_str();
            let parsed = ReservationStatus::from_str(s);
            assert_eq!(&parsed, status);
        }
    }

    #[test]
    fn unknown_status_defaults_to_cancelled() {
        let s = ReservationStatus::from_str("Unknown");
        assert_eq!(s, ReservationStatus::Cancelled);
    }

    #[test]
    fn with_parent_id_tag() {
        let r = Reservation::new(
            3,
            "CP002",
            0,
            "TAG-002",
            Some("PARENT-001".into()),
            Utc::now() + Duration::hours(2),
        );
        assert_eq!(r.parent_id_tag.as_deref(), Some("PARENT-001"));
        assert_eq!(r.connector_id, 0); // any connector
    }
}
