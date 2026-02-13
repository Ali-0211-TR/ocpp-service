//! SeaORM implementation of ChargingProfileRepository

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, UpdateResult,
};
use tracing::debug;

use crate::domain::charging_profile::{ChargingProfile, ChargingProfileRepository};
use crate::domain::{DomainError, DomainResult};
use crate::infrastructure::database::entities::charging_profile;

pub struct SeaOrmChargingProfileRepository {
    db: DatabaseConnection,
}

impl SeaOrmChargingProfileRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Conversion helpers ──────────────────────────────────────────

fn model_to_domain(m: charging_profile::Model) -> ChargingProfile {
    ChargingProfile {
        id: m.id,
        charge_point_id: m.charge_point_id,
        evse_id: m.evse_id,
        profile_id: m.profile_id,
        stack_level: m.stack_level,
        purpose: m.purpose,
        kind: m.kind,
        recurrency_kind: m.recurrency_kind,
        valid_from: m.valid_from,
        valid_to: m.valid_to,
        schedule_json: m.schedule_json,
        is_active: m.is_active,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn db_err(e: sea_orm::DbErr) -> DomainError {
    DomainError::Validation(format!("Database error: {}", e))
}

// ── ChargingProfileRepository impl ─────────────────────────────

#[async_trait]
impl ChargingProfileRepository for SeaOrmChargingProfileRepository {
    async fn save(&self, profile: ChargingProfile) -> DomainResult<ChargingProfile> {
        debug!(
            "Saving charging profile: cp={}, profile_id={}, evse={}, purpose={}",
            profile.charge_point_id, profile.profile_id, profile.evse_id, profile.purpose
        );

        let now = Utc::now();
        let model = charging_profile::ActiveModel {
            id: Default::default(), // auto-increment
            charge_point_id: Set(profile.charge_point_id),
            evse_id: Set(profile.evse_id),
            profile_id: Set(profile.profile_id),
            stack_level: Set(profile.stack_level),
            purpose: Set(profile.purpose),
            kind: Set(profile.kind),
            recurrency_kind: Set(profile.recurrency_kind),
            valid_from: Set(profile.valid_from),
            valid_to: Set(profile.valid_to),
            schedule_json: Set(profile.schedule_json),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = model.insert(&self.db).await.map_err(db_err)?;
        Ok(model_to_domain(result))
    }

    async fn find_active_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<ChargingProfile>> {
        let models = charging_profile::Entity::find()
            .filter(charging_profile::Column::ChargePointId.eq(charge_point_id))
            .filter(charging_profile::Column::IsActive.eq(true))
            .order_by_desc(charging_profile::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn find_all_for_charge_point(
        &self,
        charge_point_id: &str,
    ) -> DomainResult<Vec<ChargingProfile>> {
        let models = charging_profile::Entity::find()
            .filter(charging_profile::Column::ChargePointId.eq(charge_point_id))
            .order_by_desc(charging_profile::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(db_err)?;
        Ok(models.into_iter().map(model_to_domain).collect())
    }

    async fn deactivate_by_profile_id(
        &self,
        charge_point_id: &str,
        profile_id: i32,
    ) -> DomainResult<u64> {
        debug!(
            "Deactivating profile: cp={}, profile_id={}",
            charge_point_id, profile_id
        );

        let result: UpdateResult = charging_profile::Entity::update_many()
            .col_expr(
                charging_profile::Column::IsActive,
                sea_orm::sea_query::Expr::value(false),
            )
            .col_expr(
                charging_profile::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now()),
            )
            .filter(charging_profile::Column::ChargePointId.eq(charge_point_id))
            .filter(charging_profile::Column::ProfileId.eq(profile_id))
            .filter(charging_profile::Column::IsActive.eq(true))
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        Ok(result.rows_affected)
    }

    async fn deactivate_by_criteria(
        &self,
        charge_point_id: &str,
        evse_id: Option<i32>,
        purpose: Option<&str>,
        stack_level: Option<i32>,
    ) -> DomainResult<u64> {
        debug!(
            "Deactivating profiles by criteria: cp={}, evse={:?}, purpose={:?}, stack={:?}",
            charge_point_id, evse_id, purpose, stack_level
        );

        let mut condition = Condition::all()
            .add(charging_profile::Column::ChargePointId.eq(charge_point_id))
            .add(charging_profile::Column::IsActive.eq(true));

        if let Some(eid) = evse_id {
            condition = condition.add(charging_profile::Column::EvseId.eq(eid));
        }
        if let Some(p) = purpose {
            condition = condition.add(charging_profile::Column::Purpose.eq(p));
        }
        if let Some(sl) = stack_level {
            condition = condition.add(charging_profile::Column::StackLevel.eq(sl));
        }

        let result: UpdateResult = charging_profile::Entity::update_many()
            .col_expr(
                charging_profile::Column::IsActive,
                sea_orm::sea_query::Expr::value(false),
            )
            .col_expr(
                charging_profile::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now()),
            )
            .filter(condition)
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        Ok(result.rows_affected)
    }

    async fn deactivate_all(&self, charge_point_id: &str) -> DomainResult<u64> {
        debug!("Deactivating ALL profiles for cp={}", charge_point_id);

        let result: UpdateResult = charging_profile::Entity::update_many()
            .col_expr(
                charging_profile::Column::IsActive,
                sea_orm::sea_query::Expr::value(false),
            )
            .col_expr(
                charging_profile::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(Utc::now()),
            )
            .filter(charging_profile::Column::ChargePointId.eq(charge_point_id))
            .filter(charging_profile::Column::IsActive.eq(true))
            .exec(&self.db)
            .await
            .map_err(db_err)?;

        Ok(result.rows_affected)
    }
}
