//! Create api_keys table migration

use sea_orm_migration::prelude::*;

use super::m20240101_000004_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApiKeys::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApiKeys::Name).string_len(255).not_null())
                    .col(
                        ColumnDef::new(ApiKeys::KeyHash)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(ApiKeys::Prefix).string_len(50).not_null())
                    .col(ColumnDef::new(ApiKeys::UserId).string().null())
                    .col(
                        ColumnDef::new(ApiKeys::Scopes)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_keys_user_id")
                            .from(ApiKeys::Table, ApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on key_hash for fast lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_api_keys_key_hash")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::KeyHash)
                    .to_owned(),
            )
            .await?;

        // Create index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_api_keys_user_id")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ApiKeys {
    Table,
    Id,
    Name,
    KeyHash,
    Prefix,
    UserId,
    Scopes,
    IsActive,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}
