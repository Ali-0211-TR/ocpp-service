//! Create reservations table
//!
//! Stores connector reservations with expiry tracking.
//! Supports ReserveNow / CancelReservation for OCPP v1.6 and v2.0.1.

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
                    .table(Reservations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Reservations::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Reservations::ChargePointId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Reservations::ConnectorId)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Reservations::IdTag)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Reservations::ParentIdTag).string())
                    .col(
                        ColumnDef::new(Reservations::ExpiryDate)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Reservations::Status)
                            .string()
                            .not_null()
                            .default("Accepted"),
                    )
                    .col(
                        ColumnDef::new(Reservations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reservations_charge_point")
                            .from(Reservations::Table, Reservations::ChargePointId)
                            .to(ChargePoints::Table, ChargePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reservations_charge_point")
                    .table(Reservations::Table)
                    .col(Reservations::ChargePointId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reservations_status")
                    .table(Reservations::Table)
                    .col(Reservations::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reservations_expiry")
                    .table(Reservations::Table)
                    .col(Reservations::ExpiryDate)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Reservations::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Reservations {
    Table,
    Id,
    ChargePointId,
    ConnectorId,
    IdTag,
    ParentIdTag,
    ExpiryDate,
    Status,
    CreatedAt,
}
