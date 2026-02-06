//! Add billing fields to transactions table

use sea_orm_migration::prelude::*;

use super::m20240101_000003_create_transactions::Transactions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add tariff_id column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(ColumnDef::new(TransactionsBilling::TariffId).integer())
                    .to_owned(),
            )
            .await?;

        // Add total_cost column (in smallest currency unit)
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(ColumnDef::new(TransactionsBilling::TotalCost).integer())
                    .to_owned(),
            )
            .await?;

        // Add currency column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(
                        ColumnDef::new(TransactionsBilling::Currency)
                            .string()
                            .default("UZS"),
                    )
                    .to_owned(),
            )
            .await?;

        // Add energy_cost column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(ColumnDef::new(TransactionsBilling::EnergyCost).integer())
                    .to_owned(),
            )
            .await?;

        // Add time_cost column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(ColumnDef::new(TransactionsBilling::TimeCost).integer())
                    .to_owned(),
            )
            .await?;

        // Add session_fee column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(ColumnDef::new(TransactionsBilling::SessionFee).integer())
                    .to_owned(),
            )
            .await?;

        // Add billing_status column
        manager
            .alter_table(
                Table::alter()
                    .table(Transactions::Table)
                    .add_column(
                        ColumnDef::new(TransactionsBilling::BillingStatus)
                            .string()
                            .default("Pending"),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for tariff lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_transactions_tariff")
                    .table(Transactions::Table)
                    .col(TransactionsBilling::TariffId)
                    .to_owned(),
            )
            .await?;

        // Create index for billing status
        manager
            .create_index(
                Index::create()
                    .name("idx_transactions_billing_status")
                    .table(Transactions::Table)
                    .col(TransactionsBilling::BillingStatus)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_transactions_billing_status")
                    .table(Transactions::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_transactions_tariff")
                    .table(Transactions::Table)
                    .to_owned(),
            )
            .await?;

        // SQLite doesn't support DROP COLUMN, so we need to recreate the table
        // For now, we'll just leave the columns (this is a development migration)
        Ok(())
    }
}

#[derive(Iden)]
enum TransactionsBilling {
    TariffId,
    TotalCost,
    Currency,
    EnergyCost,
    TimeCost,
    SessionFee,
    BillingStatus,
}
