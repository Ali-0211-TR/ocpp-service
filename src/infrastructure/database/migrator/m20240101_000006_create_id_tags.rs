//! Migration to create id_tags table

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create id_tags table
        manager
            .create_table(
                Table::create()
                    .table(IdTags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IdTags::IdTag)
                            .string_len(64)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(IdTags::ParentIdTag).string_len(64).null())
                    .col(
                        ColumnDef::new(IdTags::Status)
                            .string_len(20)
                            .not_null()
                            .default("Accepted"),
                    )
                    .col(ColumnDef::new(IdTags::UserId).string().null())
                    .col(ColumnDef::new(IdTags::Name).string_len(255).null())
                    .col(ColumnDef::new(IdTags::ExpiryDate).timestamp_with_time_zone().null())
                    .col(ColumnDef::new(IdTags::MaxActiveTransactions).integer().null())
                    .col(
                        ColumnDef::new(IdTags::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(IdTags::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IdTags::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(IdTags::LastUsedAt).timestamp_with_time_zone().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_id_tags_user")
                            .from(IdTags::Table, IdTags::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_id_tags_parent")
                            .from(IdTags::Table, IdTags::ParentIdTag)
                            .to(IdTags::Table, IdTags::IdTag)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_id_tags_status")
                    .table(IdTags::Table)
                    .col(IdTags::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_id_tags_user_id")
                    .table(IdTags::Table)
                    .col(IdTags::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_id_tags_parent")
                    .table(IdTags::Table)
                    .col(IdTags::ParentIdTag)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(IdTags::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum IdTags {
    Table,
    IdTag,
    ParentIdTag,
    Status,
    UserId,
    Name,
    ExpiryDate,
    MaxActiveTransactions,
    IsActive,
    CreatedAt,
    UpdatedAt,
    LastUsedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
