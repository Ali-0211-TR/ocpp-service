//! Create charge_points table

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChargePoints::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChargePoints::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ChargePoints::Vendor).string().not_null())
                    .col(ColumnDef::new(ChargePoints::Model).string().not_null())
                    .col(ColumnDef::new(ChargePoints::SerialNumber).string())
                    .col(ColumnDef::new(ChargePoints::FirmwareVersion).string())
                    .col(ColumnDef::new(ChargePoints::Iccid).string())
                    .col(ColumnDef::new(ChargePoints::Imsi).string())
                    .col(
                        ColumnDef::new(ChargePoints::Status)
                            .string()
                            .not_null()
                            .default("Unknown"),
                    )
                    .col(ColumnDef::new(ChargePoints::LastHeartbeat).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ChargePoints::RegisteredAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChargePoints::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChargePoints::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ChargePoints {
    Table,
    Id,
    Vendor,
    Model,
    SerialNumber,
    FirmwareVersion,
    Iccid,
    Imsi,
    Status,
    LastHeartbeat,
    RegisteredAt,
    UpdatedAt,
}
