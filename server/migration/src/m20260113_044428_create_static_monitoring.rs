use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StaticMonitoringInDatabase::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::Uuid)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::Timestamp)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::CpuData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::SystemData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StaticMonitoringInDatabase::GpuData)
                            .json_binary()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(StaticMonitoringInDatabase::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum StaticMonitoringInDatabase {
    #[sea_orm(iden = "static_monitoring")]
    Table,
    Id,
    Uuid,
    Timestamp,

    CpuData,
    SystemData,
    GpuData,
}
