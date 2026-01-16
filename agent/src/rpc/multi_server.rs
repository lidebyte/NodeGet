use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::config::Server;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tokio::net::TcpStream;
use tokio::sync::{OnceCell, RwLock, broadcast, mpsc};
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

// 一个服务器的句柄
pub struct ServerHandle {
    // 写入通道：发给这个 sender，消息就会被写入 WebSocket
    tx: mpsc::Sender<Message>,
    // 订阅通道：通过这个 sender 创建 receiver，就能收到 WebSocket 的消息
    broadcast_tx: broadcast::Sender<Message>,
}

// 全局连接池
// UUID -> 句柄
static CONNECTION_POOL: OnceCell<RwLock<HashMap<String, Arc<ServerHandle>>>> =
    OnceCell::const_new();

pub async fn init_connections(servers: Vec<Server>) {
    let mut map = HashMap::new();
    let mut join_set = JoinSet::new();

    for server in servers {
        join_set.spawn(async move {
            info!("Connecting Server: [{}]", server.name);
            if let Ok(Ok((ws_stream, _))) = timeout(Duration::from_secs(5), connect_async(&server.ws_url)).await {
                info!("[{}] connected", server.name);
                Some((server.name, ws_stream))
            } else {
                error!("[{}] connect failed", server.name);
                None
            }
        });
    }

    // 启动后台任务
    while let Some(res) = join_set.join_next().await {
        if let Ok(Some((name, stream))) = res {
            let handle = start_background_tasks(name.clone(), stream);
            map.insert(name, Arc::new(handle));
        }
    }

    if CONNECTION_POOL.set(RwLock::new(map)).is_err() {
        warn!("Connection pool has been created");
    }
}

fn start_background_tasks(
    name: String,
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> ServerHandle {
    let (mut writer, mut reader) = stream.split();

    // 写通道
    let (tx, mut rx) = mpsc::channel::<Message>(32);

    // 广播通道
    let (broadcast_tx, _) = broadcast::channel::<Message>(32);
    let broadcast_tx_clone = broadcast_tx.clone();

    let name_for_read = name.clone();
    let name_for_write = name.clone();

    // 接收 mpsc 消息 -> 写入 WS
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = writer.send(msg).await {
                warn!("{name_for_write} Write error: {e}");
                break;
            }
        }
        debug!("[{name_for_write}] Write task done");
    });

    // 读取 WS -> 广播
    tokio::spawn(async move {
        while let Some(msg_result) = reader.next().await {
            match msg_result {
                Ok(msg) => {
                    let _ = broadcast_tx_clone.send(msg);
                }
                Err(e) => {
                    warn!("{name_for_read} Read error: {e}");
                    break;
                }
            }
        }
        debug!("[{name_for_read}] Read task done (connection closed)");
    });

    ServerHandle { tx, broadcast_tx }
}

pub async fn send_to(server_name: &str, msg: Message) -> Result<(), String> {
    let pool = CONNECTION_POOL
        .get()
        .ok_or("Connection pool not initialized")?
        .read()
        .await;

    if let Some(handle) = pool.get(server_name) {
        handle
            .tx
            .send(msg)
            .await
            .map_err(|_| "Sending channel is closed".to_string())
    } else {
        Err(format!("Server not found: {server_name}"))
    }
}

pub async fn subscribe_to(server_name: &str) -> Result<broadcast::Receiver<Message>, String> {
    let pool = CONNECTION_POOL
        .get()
        .ok_or("Connection pool not initialized")?
        .read()
        .await;

    if let Some(handle) = pool.get(server_name) {
        Ok(handle.broadcast_tx.subscribe())
    } else {
        Err(format!("Server not found: {server_name}"))
    }
}
