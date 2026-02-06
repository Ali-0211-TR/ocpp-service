//! Create tariffs table

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tariffs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tariffs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Tariffs::Name)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tariffs::Description).string())
                    .col(
                        ColumnDef::new(Tariffs::TariffType)
                            .string()
                            .not_null()
                            .default("PerKwh"),
                    )
                    .col(
                        ColumnDef::new(Tariffs::PricePerKwh)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tariffs::PricePerMinute)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tariffs::SessionFee)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tariffs::Currency)
                            .string()
                            .not_null()
                            .default("UZS"),
                    )
                    .col(
                        ColumnDef::new(Tariffs::MinFee)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tariffs::MaxFee)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Tariffs::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Tariffs::IsDefault)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Tariffs::ValidFrom).timestamp_with_time_zone())
                    .col(ColumnDef::new(Tariffs::ValidUntil).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Tariffs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Tariffs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on name
        manager
            .create_index(
                Index::create()
                    .name("idx_tariffs_name")
                    .table(Tariffs::Table)
                    .col(Tariffs::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Insert default tariff
        let insert = Query::insert()
            .into_table(Tariffs::Table)
            .columns([
                Tariffs::Name,
                Tariffs::Description,
                Tariffs::TariffType,
                Tariffs::PricePerKwh,
                Tariffs::PricePerMinute,
                Tariffs::SessionFee,
                Tariffs::Currency,
                Tariffs::MinFee,
                Tariffs::MaxFee,
                Tariffs::IsActive,
                Tariffs::IsDefault,
                Tariffs::CreatedAt,
                Tariffs::UpdatedAt,
            ])
            .values_panic([
                "Standard".into(),
                "Default tariff for all charge points".into(),
                "PerKwh".into(),
                250.into(),  // 2.50 per kWh (in cents)
                0.into(),
                0.into(),
                "UZS".into(),
                100.into(),  // Min fee 1.00
                0.into(),    // No max
                true.into(),
                true.into(),
                chrono::Utc::now().to_rfc3339().into(),
                chrono::Utc::now().to_rfc3339().into(),
            ])
            .to_owned();

        manager.exec_stmt(insert).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tariffs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Tariffs {
    Table,
    Id,
    Name,
    Description,
    TariffType,
    PricePerKwh,
    PricePerMinute,
    SessionFee,
    Currency,
    MinFee,
    MaxFee,
    IsActive,
    IsDefault,
    ValidFrom,
    ValidUntil,
    CreatedAt,
    UpdatedAt,
}
