use crate::entity::task;
use crate::rpc::RpcHelper;
use jsonrpsee::PendingSubscriptionSink;
use jsonrpsee::SubscriptionMessage;
use jsonrpsee::core::{JsonRawValue, SubscriptionResult};
use jsonrpsee::proc_macros::rpc;
use log::{debug, error, info};
use migration::async_trait::async_trait;
use nodeget_lib::task::TaskEventType;
use nodeget_lib::task::{TaskEvent, TaskEventResponse};
use nodeget_lib::utils::error_message::generate_error_message;
use nodeget_lib::utils::generate_random_string;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, Set};
use sea_orm::{ColumnTrait, QueryFilter};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

#[rpc(server, namespace = "task")]
pub trait Rpc {
    #[subscription(name = "register_task", item = TaskEvent, unsubscribe = "unregister_task")]
    async fn register_task(&self, uuid: Uuid) -> SubscriptionResult;

    #[method(name = "create_task")]
    async fn create_task(
        &self,
        token: String,
        target_uuid: Uuid,
        task_type: TaskEventType,
    ) -> Value;

    #[method(name = "upload_task_result")]
    async fn upload_task_result(&self, token: String, task_response: TaskEventResponse) -> Value;
}

pub struct TaskRpcImpl {
    pub manager: TaskManager,
}

impl RpcHelper for TaskRpcImpl {}

#[async_trait]
impl RpcServer for TaskRpcImpl {
    async fn create_task(
        &self,
        _token: String,
        target_uuid: Uuid,
        task_type: TaskEventType,
    ) -> Value {
        let process_logic = async {
            let db = Self::get_db().map_err(|e| (e.0 as u32, e.1))?;

            let token = generate_random_string(10);

            let in_data = task::ActiveModel {
                id: ActiveValue::default(),
                uuid: Set(target_uuid),
                token: Set(token.clone()),

                timestamp: Set(None),
                success: Set(None),
                error_message: Set(None),
                task_event_type: Self::try_set_json(task_type.clone()).map_err(|e| (101, e))?,
                task_event_result: Set(None),
            };

            debug!("Received task for [{}]", target_uuid.clone());

            let result = task::Entity::insert(in_data).exec(db).await.map_err(|e| {
                error!("Database insert error: {e}");
                (103, format!("Database insert error: {e}"))
            })?;

            debug!("Inserted task with id [{}]", result.last_insert_id);

            let task = TaskEvent {
                task_id: result.last_insert_id as u64,
                task_token: token,
                task_event_type: task_type,
            };

            match self.manager.send_event(target_uuid, task).await {
                Ok(()) => Ok(result.last_insert_id),
                Err(e) => {
                    let _ = task::Entity::delete(task::ActiveModel {
                        id: Set(result.last_insert_id),
                        ..Default::default()
                    })
                    .exec(db)
                    .await
                    .map_err(|e| {
                        error!("Database delete error: {e}");
                        (103, format!("Database delete error: {e}"))
                    });

                    error!("Error sending task event: {}", e.1);
                    Err((e.0, format!("Error sending task event: {}", e.1)))
                }
            }
        };

        match process_logic.await {
            Ok(new_id) => json!({ "id": new_id }),
            Err((code, msg)) => generate_error_message(code, &msg),
        }
    }

    async fn upload_task_result(&self, _token: String, task_response: TaskEventResponse) -> Value {
        let process_logic = async {
            let db = Self::get_db().map_err(|e| (e.0 as u32, e.1))?;

            // ✅ 修复 2: 引入 Trait 后，这里的 filter 和 eq 就能正常工作了
            let task_model = task::Entity::find_by_id(task_response.task_id as i64)
                .filter(task::Column::Uuid.eq(task_response.agent_uuid))
                .filter(task::Column::Token.eq(task_response.task_token))
                .one(db)
                .await
                .map_err(|e| {
                    error!("Database query error: {e}");
                    (103, format!("Database query error: {e}"))
                })?
                .ok_or_else(|| {
                    (
                        105,
                        "Task validation failed: Invalid ID, UUID, or Token".to_string(),
                    )
                })?;

            let mut active_model: task::ActiveModel = task_model.into();

            active_model.timestamp = Set(Some(task_response.timestamp as i64));
            active_model.success = Set(Some(task_response.success));

            active_model.error_message = Set(task_response.error_message);

            active_model.task_event_result = match task_response.task_event_result {
                None => Set(None),
                Some(res) => ActiveValue::from(Self::try_set_json(res).map_err(|e| (101, e))?),
            };

            active_model.update(db).await.map_err(|e| {
                error!("Database update error: {e}");
                (103, format!("Database update error: {e}"))
            })?;

            debug!(
                "Task [{}] result uploaded successfully",
                task_response.task_id
            );

            Ok(true)
        };

        match process_logic.await {
            Ok(_) => json!({ "id": task_response.task_id }),
            Err((code, msg)) => generate_error_message(code, &msg),
        }
    }

    async fn register_task(
        &self,
        subscription_sink: PendingSubscriptionSink,
        uuid: Uuid,
    ) -> SubscriptionResult {
        let sink = subscription_sink.accept().await?;

        let (tx, mut rx) = mpsc::channel(32);

        let reg_id = Uuid::new_v4();

        self.manager.add_session(uuid, reg_id, tx).await;

        let manager_clone = self.manager.clone();
        let uuid_clone = uuid;
        let reg_id_clone = reg_id;

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(msg) => {
                        let sub_msg = SubscriptionMessage::from(
                            JsonRawValue::from_string(serde_json::to_string(&msg).unwrap())
                                .unwrap(),
                        );

                        if sink.send(sub_msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }

            manager_clone
                .remove_session(&uuid_clone, &reg_id_clone)
                .await;
            info!("Client {uuid_clone} (RegID: {reg_id_clone}) disconnected, logic handled.");
        });

        Ok(())
    }
}

// Task 连接池
#[derive(Clone)]
pub struct TaskManager {
    peers: Arc<RwLock<HashMap<Uuid, (Uuid, mpsc::Sender<TaskEvent>)>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_session(&self, uuid: Uuid, reg_id: Uuid, tx: mpsc::Sender<TaskEvent>) {
        self.peers.write().await.insert(uuid, (reg_id, tx));
    }

    pub async fn remove_session(&self, uuid: &Uuid, reg_id: &Uuid) {
        let mut peers = self.peers.write().await;

        if let Some((current_reg_id, _)) = peers.get(uuid)
            && current_reg_id == reg_id
        {
            peers.remove(uuid);
        }
    }

    pub async fn send_event(&self, uuid: Uuid, event: TaskEvent) -> Result<(), (u32, String)> {
        let peers = self.peers.read().await;

        if let Some((_, tx)) = peers.get(&uuid) {
            tx.send(event)
                .await
                .map_err(|_| (104, "Error sending event".to_string()))
        } else {
            Err((106, "Uuid not found".to_string()))
        }
    }
}
