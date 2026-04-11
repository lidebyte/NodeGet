pub mod clean_up;

use crate::{DB, SERVER_CONFIG};
use log::LevelFilter;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database};
use std::process;
use std::time::Duration;
use tracing::{error, info};

// 初始化数据库连接并应用迁移
//
// 该函数连接到数据库，应用必要的迁移，并根据数据库类型进行特定配置。
// 如果配置无效或连接失败，则会记录错误并退出进程。
pub async fn init_db_connection() {
    let config = SERVER_CONFIG
        .get()
        .expect("Server config not initialized")
        .read()
        .expect("SERVER_CONFIG lock poisoned")
        .clone();

    DB.get_or_init(|| async {
        let mut opt = ConnectOptions::new(&config.database.database_url);
        opt.sqlx_logging_level(LevelFilter::Trace)
            .connect_timeout(Duration::from_millis(
                config.database.connect_timeout_ms.unwrap_or(3000),
            ))
            .acquire_timeout(Duration::from_millis(
                config.database.acquire_timeout_ms.unwrap_or(3000),
            ))
            .idle_timeout(Duration::from_millis(
                config.database.idle_timeout_ms.unwrap_or(3000),
            ))
            .max_lifetime(Duration::from_millis(
                config.database.max_lifetime_ms.unwrap_or(30000),
            ))
            .max_connections(config.database.max_connections.unwrap_or(10));

        let db = Database::connect(opt).await.unwrap_or_else(|e| {
            error!(target: "db", error = %e, "Unable to connect to the database");
            process::exit(1);
        });

        info!(target: "db", "Database connected successfully");

        Migrator::up(&db, None).await.unwrap_or_else(|e| {
            error!(target: "db", error = %e, "Unable to apply migrations");
            process::exit(1);
        });

        info!(target: "db", "Migrations applied successfully");

        if db.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            let _ = db
                .execute_unprepared("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .await;
            info!(target: "db", "SQLite WAL mode enabled");
        }

        db
    })
    .await;
}
