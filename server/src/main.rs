#![feature(duration_millis_float)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::too_many_lines,
    dead_code
)]

use crate::rpc::agent::RpcServer as AgentRpcServer;
use crate::rpc::nodeget::RpcServer as NodeGetRpcServer;
use crate::rpc::task::RpcServer as TaskRpcServer;
use axum::routing::any;
use log::info;
use std::str::FromStr;
use tower::Service;

#[cfg(all(not(target_os = "windows"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_os = "windows"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod db_connection;
mod entity;
mod rpc;
mod terminal;

static DB: tokio::sync::OnceCell<sea_orm::DatabaseConnection> = tokio::sync::OnceCell::const_new();
static SERVER_CONFIG: std::sync::OnceLock<nodeget_lib::config::server::ServerConfig> =
    std::sync::OnceLock::new();

#[tokio::main]
async fn main() {
    println!("Starting nodeget-server");

    // Config Parse
    let config = nodeget_lib::config::server::ServerConfig::get_and_parse_config("./config.toml")
        .await
        .unwrap();

    // Log init
    simple_logger::init_with_level(log::Level::from_str(&config.log_level).unwrap()).unwrap();

    // Jemalloc Mem Debug
    #[cfg(all(not(target_os = "windows"), feature = "jemalloc"))]
    tokio::spawn(async {
        loop {
            use tikv_jemalloc_ctl::{epoch, stats};
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if epoch::advance().is_err() {
                return;
            }

            let allocated = stats::allocated::read().unwrap();
            let active = stats::active::read().unwrap();
            let resident = stats::resident::read().unwrap();
            let mapped = stats::mapped::read().unwrap();

            info!(
                "MEM STATS (Jemalloc Only): App Logic: {:.2} MB | Allocator Active: {:.2} MB | RSS (Resident): {:.2} MB | Mapped: {:.2} MB",
                allocated as f64 / 1024.0 / 1024.0,
                active as f64 / 1024.0 / 1024.0,
                resident as f64 / 1024.0 / 1024.0,
                mapped as f64 / 1024.0 / 1024.0
            );
        }
    });

    // 对比 Uuid，发送警告
    let _ = nodeget_lib::utils::compare_uuid(config.server_uuid);

    info!("Starting nodeget-server with config: {config:?}");

    // 初始化全局 Config
    SERVER_CONFIG.set(config.clone()).unwrap();

    // 连接数据库
    db_connection::init_db_connection().await;

    let task_manager = rpc::task::TaskManager::new();
    let terminal_state = terminal::TerminalState {
        sessions: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
    };

    let mut rpc_module = rpc::nodeget::NodegetServerRpcImpl.into_rpc();
    rpc_module
        .merge(rpc::agent::AgentRpcImpl.into_rpc())
        .unwrap();
    rpc_module
        .merge(
            rpc::task::TaskRpcImpl {
                manager: task_manager.clone(),
            }
            .into_rpc(),
        )
        .unwrap();

    let (stop_handle, _server_handle) = jsonrpsee::server::stop_channel();

    let jsonrpc_service = jsonrpsee::server::Server::builder()
        .set_config(
            jsonrpsee::server::ServerConfig::builder()
                .max_response_body_size(u32::MAX)
                .max_request_body_size(u32::MAX)
                .build(),
        )
        .to_service_builder()
        .build(rpc_module, stop_handle);

    let app = axum::Router::new()
        .route("/terminal", any(terminal::terminal_ws_handler))
        .with_state(terminal_state)
        .fallback(any(move |req: axum::extract::Request| {
            let mut rpc_service = jsonrpc_service.clone();
            async move { rpc_service.call(req).await.unwrap() }
        }));

    let listener =
        tokio::net::TcpListener::bind(config.ws_listener.parse::<std::net::SocketAddr>().unwrap())
            .await
            .unwrap();

    axum::serve(listener, app).await.unwrap();
}
