//! Add ocpp_version and meter fields to charge_points table

use sea_orm_migration::prelude::*;

use super::m20240101_000001_create_charge_points::ChargePoints;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // OCPP protocol version: "V16", "V201", "V21"
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .add_column(ColumnDef::new(Alias::new("ocpp_version")).string())
                    .to_owned(),
            )
            .await?;

        // Meter type (v1.6 BootNotification optional field)
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .add_column(ColumnDef::new(Alias::new("meter_type")).string())
                    .to_owned(),
            )
            .await?;

        // Meter serial number (v1.6 BootNotification optional field)
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .add_column(ColumnDef::new(Alias::new("meter_serial_number")).string())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .drop_column(Alias::new("ocpp_version"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .drop_column(Alias::new("meter_type"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .drop_column(Alias::new("meter_serial_number"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
