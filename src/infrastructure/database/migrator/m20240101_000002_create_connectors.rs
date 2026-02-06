//! Create connectors table

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
                    .table(Connectors::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Connectors::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Connectors::ChargePointId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Connectors::ConnectorId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Connectors::Status)
                            .string()
                            .not_null()
                            .default("Available"),
                    )
                    .col(ColumnDef::new(Connectors::ErrorCode).string())
                    .col(ColumnDef::new(Connectors::ErrorInfo).string())
                    .col(ColumnDef::new(Connectors::VendorId).string())
                    .col(ColumnDef::new(Connectors::VendorErrorCode).string())
                    .col(
                        ColumnDef::new(Connectors::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_connectors_charge_point")
                            .from(Connectors::Table, Connectors::ChargePointId)
                            .to(ChargePoints::Table, ChargePoints::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on charge_point_id + connector_id
        manager
            .create_index(
                Index::create()
                    .name("idx_connectors_cp_connector")
                    .table(Connectors::Table)
                    .col(Connectors::ChargePointId)
                    .col(Connectors::ConnectorId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Connectors::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Connectors {
    Table,
    Id,
    ChargePointId,
    ConnectorId,
    Status,
    ErrorCode,
    ErrorInfo,
    VendorId,
    VendorErrorCode,
    UpdatedAt,
}
