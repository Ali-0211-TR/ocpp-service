//! Add password_hash column to charge_points table
//!
//! Supports OCPP Security Profile 1 (Basic Auth) for WebSocket connections.
//! Charge points authenticate with `Authorization: Basic base64(id:password)`.

use sea_orm_migration::prelude::*;

use super::m20240101_000001_create_charge_points::ChargePoints;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .add_column(ColumnDef::new(Alias::new("password_hash")).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChargePoints::Table)
                    .drop_column(Alias::new("password_hash"))
                    .to_owned(),
            )
            .await
    }
}
