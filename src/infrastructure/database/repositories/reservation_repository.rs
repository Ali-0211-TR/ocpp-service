//! SeaORM implementation of ReservationRepository

use async_trait::async_trait;
use log::debug;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::domain::reservation::{Reservation, ReservationRepository, ReservationStatus};
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::reservation;

pub struct SeaOrmReservationRepository {
    db: DatabaseConnection,
}

impl SeaOrmReservationRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Conversion helpers ──────────────────────────────────────────

fn model_to_domain(m: reservation::Model) -> Reservation {
    Reservation {
        id: m.id,
        charge_point_id: m.charge_point_id,
        connector_id: m.connector_id,
        id_tag: m.id_tag,
        parent_id_tag: m.parent_id_tag,
        expiry_date: m.expiry_date,
        status: ReservationStatus::from_str(&m.status),
        created_at: m.created_at,
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

// ── ReservationRepository impl ──────────────────────────────────

#[async_trait]
impl ReservationRepository for SeaOrmReservationRepository {
    async fn save(&self, r: Reservation) -> DomainResult<()> {
        debug!("Saving reservation: {}", r.id);

        let model = reservation::ActiveModel {
            id: Set(r.id),
            charge_point_id: Set(r.charge_point_id),
            connector_id: Set(r.connector_id),
            id_tag: Set(r.id_tag),
            parent_id_tag: Set(r.parent_id_tag),
            expiry_date: Set(r.expiry_date),
            status: Set(r.status.as_str().to_string()),
            created_at: Set(r.created_at),
        };
        model.insert(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_by_id(&self, id: i32) -> DomainResult<Option<Reservation>> {
        let model = reservation::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(model_to_domain))
    }

    async fn update(&self, r: Reservation) -> DomainResult<()> {
        debug!("Updating reservation: {}", r.id);

        let existing = reservation::Entity::find_by_id(r.id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        if existing.is_none() {
            return Err(DomainError::NotFound {
                entity: "Reservation",
                field: "id",
                value: r.id.to_string(),
            });
        }

        let model = reservation::ActiveModel {
            id: Set(r.id),
            charge_point_id: Set(r.charge_point_id),
            connector_id: Set(r.connector_id),
            id_tag: Set(r.id_tag),
            parent_id_tag: Set(r.parent_id_tag),
            expiry_date: Set(r.expiry_date),
            status: Set(r.status.as_str().to_string()),
            created_at: Set(r.created_at),
        };
        model.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn find_active_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<Reservation>> {
        let models = reservation::Entity::find()
            .filter(reservation::Column::ChargePointId.eq(charge_point_id))
            .filter(reservation::Column::Status.eq("Accepted"))
            .order_by_desc(reservation::Column::Id)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn find_active_for_connector(
        &self,
        charge_point_id: &str,
        connector_id: i32,
    ) -> DomainResult<Option<Reservation>> {
        let model = reservation::Entity::find()
            .filter(reservation::Column::ChargePointId.eq(charge_point_id))
            .filter(reservation::Column::ConnectorId.eq(connector_id))
            .filter(reservation::Column::Status.eq("Accepted"))
            .one(&self.db)
            .await
            .map_err(db_err)?;
        Ok(model.map(model_to_domain))
    }

    async fn find_all(&self) -> DomainResult<Vec<Reservation>> {
        let models = reservation::Entity::find()
            .order_by_desc(reservation::Column::Id)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn find_expired(&self) -> DomainResult<Vec<Reservation>> {
        use chrono::Utc;
        let models = reservation::Entity::find()
            .filter(reservation::Column::Status.eq("Accepted"))
            .filter(reservation::Column::ExpiryDate.lt(Utc::now()))
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn cancel(&self, id: i32) -> DomainResult<()> {
        let existing = reservation::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(db_err)?;

        let Some(existing) = existing else {
            return Err(DomainError::NotFound {
                entity: "Reservation",
                field: "id",
                value: id.to_string(),
            });
        };

        let mut active: reservation::ActiveModel = existing.into();
        active.status = Set("Cancelled".to_string());
        active.update(&self.db).await.map_err(db_err)?;
        Ok(())
    }

    async fn next_id(&self) -> i32 {
        reservation::Entity::find()
            .all(&self.db)
            .await
            .map(|rs| rs.into_iter().map(|r| r.id).max().unwrap_or(0) + 1)
            .unwrap_or(1)
    }
}
