//! Create charging_profiles table
//!
//! Stores charging profiles sent to charge points via SetChargingProfile.
//! Tracks active/inactive status so profiles can be listed and cleared.

use sea_orm_migration::prelude::*;

use super::m20240101_000001_create_charge_points::ChargePoints;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChargingProfiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChargingProfiles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::ChargePointId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::EvseId)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::ProfileId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::StackLevel)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::Purpose)
                            .string()
                            .not_null()
                            .default("TxDefaultProfile"),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::Kind)
                            .string()
                            .not_null()
                            .default("Absolute"),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::RecurrencyKind)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::ValidFrom)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::ValidTo)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::ScheduleJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChargingProfiles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_charging_profiles_charge_point")
                            .from(ChargingProfiles::Table, ChargingProfiles::ChargePointId)
                            .to(ChargePoints::Table, ChargePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_cp_profiles_charge_point")
                    .table(ChargingProfiles::Table)
                    .col(ChargingProfiles::ChargePointId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_cp_profiles_active")
                    .table(ChargingProfiles::Table)
                    .col(ChargingProfiles::IsActive)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_cp_profiles_cp_active")
                    .table(ChargingProfiles::Table)
                    .col(ChargingProfiles::ChargePointId)
                    .col(ChargingProfiles::IsActive)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChargingProfiles::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ChargingProfiles {
    Table,
    Id,
    ChargePointId,
    EvseId,
    ProfileId,
    StackLevel,
    Purpose,
    Kind,
    RecurrencyKind,
    ValidFrom,
    ValidTo,
    ScheduleJson,
    IsActive,
    CreatedAt,
    UpdatedAt,
}
