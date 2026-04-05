#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    dead_code
)]

use crate::rpc_timing::parse_rpc_timing_log_level;
use log::info;
use nodeget_lib::args_parse::server::{ServerArgs, ServerCommand};
use std::str::FromStr;
#[cfg(all(not(target_os = "windows"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "windows"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// 数据库连接模块
mod db_connection;
// 实体模块，定义数据库实体
mod entity;
// RPC 接口模块
mod rpc;
// 终端模块，处理终端连接
mod terminal;
// 令牌模块，处理令牌相关功能
mod crontab;
pub mod js_runtime;
mod kv;
mod rpc_timing;
mod subcommands;
mod token;

// 全局数据库连接单例
pub static DB: tokio::sync::OnceCell<sea_orm::DatabaseConnection> =
    tokio::sync::OnceCell::const_new();

// 全局服务器配置单例
static SERVER_CONFIG: std::sync::OnceLock<
    std::sync::RwLock<nodeget_lib::config::server::ServerConfig>,
> = std::sync::OnceLock::new();
pub(crate) static SERVER_CONFIG_PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
pub(crate) static RELOAD_NOTIFY: std::sync::OnceLock<tokio::sync::Notify> =
    std::sync::OnceLock::new();

// 服务器主函数
//
// 该函数启动 NodeGet 服务器，初始化配置、日志、数据库连接、超级令牌，
// 然后设置 RPC 服务和 WebSocket 终端处理器，并最终启动 HTTP 服务器。
#[tokio::main]
async fn main() {
    println!("Starting nodeget-server");

    let args = ServerArgs::par();
    let config_path = args.config_path().to_owned();
    let _ = SERVER_CONFIG_PATH.set(config_path.clone());
    RELOAD_NOTIFY.get_or_init(tokio::sync::Notify::new);

    // Config Parse
    let mut config =
        match nodeget_lib::config::server::ServerConfig::get_and_parse_config(&config_path).await {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Failed to parse config: {e}");
                std::process::exit(1);
            }
        };

    // Log init
    let base_log_level = match log::LevelFilter::from_str(&config.log_level) {
        Ok(level) => level,
        Err(_) => {
            eprintln!(
                "Warning: Invalid log_level '{}', using INFO",
                config.log_level
            );
            log::LevelFilter::Info
        }
    };
    let (rpc_timing_log_level, invalid_rpc_timing_log_level) =
        parse_rpc_timing_log_level(config.jsonrpc_timing_log_level.as_deref());

    if let Err(e) = simple_logger::SimpleLogger::new()
        .with_level(base_log_level)
        .with_module_level(
            "nodeget_server::rpc_timing",
            rpc_timing_log_level.to_level_filter(),
        )
        .init()
    {
        eprintln!("Failed to initialize logger: {e}");
        std::process::exit(1);
    }

    if let Some(invalid_level) = invalid_rpc_timing_log_level {
        log::warn!("Invalid jsonrpc_timing_log_level '{invalid_level}', fallback to 'trace'");
    }

    info!("Starting nodeget-server with config: {config:?}");

    // 初始化全局 Config
    if let Err(e) = update_global_config(config.clone()) {
        log::error!("Failed to update global config: {e}");
        std::process::exit(1);
    }

    match args.command {
        ServerCommand::Serve { .. } => {
            db_connection::init_db_connection().await;
            loop {
                subcommands::serve::run(&config, rpc_timing_log_level).await;

                let reloaded_config =
                    match nodeget_lib::config::server::ServerConfig::get_and_parse_config(
                        &config_path,
                    )
                    .await
                    {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            log::error!(
                                "Failed to reload config after edit: {e}, keeping current config"
                            );
                            continue; // 保留当前配置，继续循环
                        }
                    };
                if let Err(e) = update_global_config(reloaded_config.clone()) {
                    log::error!(
                        "Failed to update global config after reload: {e}, keeping current config"
                    );
                    continue;
                }
                config = reloaded_config;
                info!("Config hot reload applied.");
            }
        }
        ServerCommand::Init { .. } => {
            db_connection::init_db_connection().await;
            subcommands::init::run().await;
        }
        ServerCommand::RollSuperToken { .. } => {
            db_connection::init_db_connection().await;
            subcommands::roll_super_token::run().await;
        }
        ServerCommand::GetUuid { .. } => {
            subcommands::get_uuid::run(&config);
        }
    }
}

fn update_global_config(config: nodeget_lib::config::server::ServerConfig) -> anyhow::Result<()> {
    if let Some(lock) = SERVER_CONFIG.get() {
        {
            let mut guard = lock.write().map_err(|e| anyhow::anyhow!("{e}"))?;
            *guard = config;
        }
        return Ok(());
    }

    SERVER_CONFIG
        .set(std::sync::RwLock::new(config))
        .map_err(|_| anyhow::anyhow!("Failed to set SERVER_CONFIG"))?;
    Ok(())
}
