//! Create transactions table

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
                    .table(Transactions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Transactions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Transactions::ChargePointId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Transactions::ConnectorId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Transactions::IdTag).string().not_null())
                    .col(
                        ColumnDef::new(Transactions::MeterStart)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Transactions::MeterStop).integer())
                    .col(
                        ColumnDef::new(Transactions::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Transactions::StoppedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Transactions::StopReason).string())
                    .col(ColumnDef::new(Transactions::EnergyConsumed).integer())
                    .col(
                        ColumnDef::new(Transactions::Status)
                            .string()
                            .not_null()
                            .default("Active"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_transactions_charge_point")
                            .from(Transactions::Table, Transactions::ChargePointId)
                            .to(ChargePoints::Table, ChargePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for querying active transactions
        manager
            .create_index(
                Index::create()
                    .name("idx_transactions_status")
                    .table(Transactions::Table)
                    .col(Transactions::Status)
                    .to_owned(),
            )
            .await?;

        // Create index for charge point transactions
        manager
            .create_index(
                Index::create()
                    .name("idx_transactions_charge_point")
                    .table(Transactions::Table)
                    .col(Transactions::ChargePointId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Transactions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Transactions {
    Table,
    Id,
    ChargePointId,
    ConnectorId,
    IdTag,
    MeterStart,
    MeterStop,
    StartedAt,
    StoppedAt,
    StopReason,
    EnergyConsumed,
    Status,
}
