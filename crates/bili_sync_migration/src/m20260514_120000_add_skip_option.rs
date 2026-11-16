use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .add_column(string(Favorite::SkipOption).null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .add_column(string(Collection::SkipOption).null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .add_column(string(Submission::SkipOption).null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .add_column(string(WatchLater::SkipOption).null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .drop_column(Favorite::SkipOption)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .drop_column(Collection::SkipOption)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .drop_column(Submission::SkipOption)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .drop_column(WatchLater::SkipOption)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Favorite {
    Table,
    SkipOption,
}

#[derive(DeriveIden)]
enum Collection {
    Table,
    SkipOption,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    SkipOption,
}

#[derive(DeriveIden)]
enum WatchLater {
    Table,
    SkipOption,
}