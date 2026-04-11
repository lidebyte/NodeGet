mod server_cron;
mod task;

use crate::DB;
use crate::db_connection::clean_up::cleanup_expired_data;
use crate::entity::{crontab, crontab_result};
use crate::rpc::js_worker::service::enqueue_defined_js_worker_run;
use chrono::{TimeZone, Utc};
use cron::Schedule;
use nodeget_lib::crontab::{AgentCronType, Cron, CronType, ServerCronType};
use nodeget_lib::js_runtime::RunType;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, Set};
use sea_orm::{EntityTrait, QueryFilter};
use serde_json::Value;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{Instrument, debug, error, info, info_span, warn};

pub async fn delete_crontab_by_name(name: String) -> Result<bool, sea_orm::DbErr> {
    let db = DB.get().ok_or_else(|| {
        sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "Database not initialized".to_string(),
        ))
    })?;

    let result = crontab::Entity::delete_many()
        .filter(crontab::Column::Name.eq(&name))
        .exec(db)
        .await?;

    let deleted = result.rows_affected > 0;
    if deleted {
        info!(target: "crontab", name = %name, "crontab deleted");
    } else {
        warn!(target: "crontab", name = %name, "crontab not found for deletion");
    }
    Ok(deleted)
}

pub async fn set_crontab_enable_by_name(
    name: String,
    enable: bool,
) -> Result<Option<bool>, sea_orm::DbErr> {
    let db = DB.get().ok_or_else(|| {
        sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "Database not initialized".to_string(),
        ))
    })?;

    let crontab_option = crontab::Entity::find()
        .filter(crontab::Column::Name.eq(&name))
        .one(db)
        .await?;

    if let Some(model) = crontab_option {
        let mut active_model: crontab::ActiveModel = model.into();
        active_model.enable = Set(enable);
        let updated = active_model.update(db).await?;
        info!(target: "crontab", name = %name, enable = updated.enable, "crontab enable updated");
        Ok(Some(updated.enable))
    } else {
        warn!(target: "crontab", name = %name, enable, "crontab not found for set_enable");
        Ok(None)
    }
}

pub fn init_crontab_worker() {
    static CRONTAB_WORKER_STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if CRONTAB_WORKER_STARTED.set(()).is_err() {
        return;
    }

    tokio::spawn(async move {
        info!(target: "crontab", "scheduler started");
        loop {
            sleep(Duration::from_secs(1)).await;

            tokio::spawn(async move {
                process_crontab().await;
            });
        }
    });
}

async fn process_crontab() {
    let Some(db) = DB.get() else {
        error!(target: "crontab", "DB not initialized");
        return;
    };

    let jobs = match crontab::Entity::find()
        .filter(crontab::Column::Enable.eq(true))
        .all(db)
        .await
    {
        Ok(jobs) => jobs,
        Err(err) => {
            error!(target: "crontab", error = %err, "failed to query enabled crontab jobs");
            return;
        }
    };

    let now = Utc::now();

    for job in jobs {
        let schedule = match Schedule::from_str(&job.cron_expression) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    target: "crontab",
                    job_id = job.id,
                    job_name = %job.name,
                    cron_expression = %job.cron_expression,
                    error = %e,
                    "invalid cron expression, skipping"
                );
                continue;
            }
        };

        let last_run = job.last_run_time.map_or_else(
            || now - chrono::Duration::seconds(1),
            |t| Utc.timestamp_millis_opt(t).unwrap(),
        );

        let should_run = schedule
            .after(&last_run)
            .next()
            .is_some_and(|next_run| next_run <= now);

        if !should_run {
            continue;
        }

        info!(
            target: "crontab",
            job_id = job.id,
            job_name = %job.name,
            cron_expression = %job.cron_expression,
            "triggering cron job"
        );

        let cron_type = serde_json::from_str(&format!("{}", job.cron_type))
            .map_err(|e| {
                warn!(
                    target: "crontab",
                    job_id = job.id,
                    job_name = %job.name,
                    error = %e,
                    "invalid cron type, skipping"
                );
            })
            .ok();

        let Some(cron_type) = cron_type else { continue };

        // 克隆需要在闭包中使用的数据
        let job_id = job.id;
        let job_name = job.name.clone();

        let job_parsed = Cron {
            id: job.id,
            name: job.name,
            enable: job.enable,
            cron_expression: job.cron_expression,
            cron_type,
            last_run_time: job.last_run_time,
        };

        // 先更新 last_run_time，防止任务执行超时导致重复触发
        // 注意：这是可接受的，因为 crontab 任务是幂等的
        let active_model = crontab::ActiveModel {
            id: Set(job.id),
            last_run_time: Set(Some(now.timestamp_millis())),
            ..Default::default()
        };
        if let Err(e) = active_model.update(db).await {
            error!(
                target: "crontab",
                job_id = job.id,
                job_name = %job_name,
                error = %e,
                "failed to update last_run_time"
            );
            // 继续执行，不要因为记录失败而跳过任务
        }

        let span = info_span!(
            target: "crontab",
            "crontab::run_job",
            job_id,
            job_name = %job_name,
        );
        tokio::spawn(
            async move {
                run_job_logic(job_parsed).await;
                debug!(target: "crontab", "cron job completed");
            }
            .instrument(span),
        );
    }
}

async fn run_job_logic(job: Cron) {
    match job.cron_type {
        CronType::Agent(uuids, AgentCronType::Task(task_event_type)) => {
            let agent_count = uuids.len();
            info!(
                target: "crontab",
                agent_count,
                task_type = ?task_event_type,
                "dispatching agent task"
            );
            task::crontab_task(job.id, job.name, uuids, task_event_type).await;
        }

        CronType::Server(ServerCronType::CleanUpDatabase) => {
            info!(target: "crontab", "running cleanup_database job");
            run_cleanup_database_job(job.id, job.name).await;
        }
        CronType::Server(ServerCronType::JsWorker(js_script_name, params)) => {
            info!(
                target: "crontab",
                js_script_name = %js_script_name,
                "running js_worker job"
            );
            run_js_worker_job(job.id, job.name, js_script_name, params).await;
        }
    }
}

/// 运行数据库清理任务并记录结果
async fn run_cleanup_database_job(cron_id: i64, cron_name: String) {
    let Some(db) = DB.get() else {
        error!(target: "crontab", cron_id, cron_name = %cron_name, "DB not initialized for cleanup job");
        return;
    };

    // 执行清理
    let (success, message) = match cleanup_expired_data().await {
        Ok(result) => {
            info!(
                target: "crontab",
                cron_id,
                cron_name = %cron_name,
                static_monitoring = result.static_monitoring,
                dynamic_monitoring = result.dynamic_monitoring,
                task = result.task,
                crontab_result = result.crontab_result,
                "database cleanup completed"
            );
            let msg = format!(
                "数据库清理完成。已删除：static_monitoring={}，dynamic_monitoring={}，task={}，crontab_result={}",
                result.static_monitoring,
                result.dynamic_monitoring,
                result.task,
                result.crontab_result
            );
            (true, msg)
        }
        Err(e) => {
            error!(
                target: "crontab",
                cron_id,
                cron_name = %cron_name,
                error = %e,
                "database cleanup failed"
            );
            let msg = format!("数据库清理失败：{e}");
            (false, msg)
        }
    };

    // 记录执行结果到 crontab_result
    let crontab_log = crontab_result::ActiveModel {
        id: ActiveValue::NotSet,
        cron_id: Set(cron_id),
        cron_name: Set(cron_name.clone()),
        special_id: Set(None),
        run_time: Set(Some(Utc::now().timestamp_millis())),
        success: Set(Some(success)),
        message: Set(Some(message)),
    };

    if let Err(e) = crontab_result::Entity::insert(crontab_log).exec(db).await {
        error!(
            target: "crontab",
            cron_id,
            cron_name = %cron_name,
            error = %e,
            "failed to save crontab_result for cleanup job"
        );
    }
}

async fn run_js_worker_job(cron_id: i64, cron_name: String, js_script_name: String, params: Value) {
    let Some(db) = DB.get() else {
        error!(
            target: "crontab",
            cron_id,
            cron_name = %cron_name,
            js_script_name = %js_script_name,
            "DB not initialized for js_worker job"
        );
        return;
    };

    let run_result =
        enqueue_defined_js_worker_run(js_script_name.clone(), RunType::Cron, params, None).await;

    let (success, message, special_id) = match run_result {
        Ok(id) => {
            info!(
                target: "crontab",
                cron_id,
                cron_name = %cron_name,
                js_script_name = %js_script_name,
                special_id = id,
                "js_worker cron job triggered"
            );
            (
                true,
                format!("已触发 JsWorker 定时任务，脚本名：{js_script_name}，special_id：{id}"),
                Some(id),
            )
        }
        Err(e) => {
            error!(
                target: "crontab",
                cron_id,
                cron_name = %cron_name,
                js_script_name = %js_script_name,
                error = %e,
                "js_worker cron job trigger failed"
            );
            (
                false,
                format!("触发 JsWorker 定时任务失败，脚本名：{js_script_name}，错误：{e}"),
                None,
            )
        }
    };

    let crontab_log = crontab_result::ActiveModel {
        id: ActiveValue::NotSet,
        cron_id: Set(cron_id),
        cron_name: Set(cron_name.clone()),
        special_id: Set(special_id),
        run_time: Set(Some(Utc::now().timestamp_millis())),
        success: Set(Some(success)),
        message: Set(Some(message)),
    };

    if let Err(e) = crontab_result::Entity::insert(crontab_log).exec(db).await {
        error!(
            target: "crontab",
            cron_id,
            cron_name = %cron_name,
            error = %e,
            "failed to save crontab_result for js_worker job"
        );
    }
}
