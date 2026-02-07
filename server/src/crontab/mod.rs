mod task;
mod server_cron;

use crate::DB;
use crate::entity::crontab;
use chrono::{TimeZone, Utc};
use cron::Schedule;
use log::info;
use log::{error, warn};
use nodeget_lib::crontab::{AgentCronType, Cron, CronType, ServerCronType};
use sea_orm::{ActiveModelTrait, ColumnTrait, Set};
use sea_orm::{EntityTrait, QueryFilter};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

pub async fn delete_crontab_by_name(name: String) -> Result<bool, sea_orm::DbErr> {
    let db = match DB.get() {
        None => {
            return Err(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal("Database not initialized".to_string())));
        }
        Some(db) => db,
    };

    let result = crontab::Entity::delete_many()
        .filter(crontab::Column::Name.eq(name))
        .exec(db)
        .await?;

    Ok(result.rows_affected > 0)
}

pub async fn toggle_crontab_enable_by_name(name: String) -> Result<Option<bool>, sea_orm::DbErr> {
    let db = match DB.get() {
        None => {
            return Err(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal("Database not initialized".to_string())));
        }
        Some(db) => db,
    };

    // 首先查找 crontab
    let crontab_option = crontab::Entity::find()
        .filter(crontab::Column::Name.eq(&name))
        .one(db)
        .await?;

    match crontab_option {
        Some(mut model) => {
            // 获取当前的启用状态并切换
            let current_enable = model.enable;
            let new_enable = !current_enable;
            
            // 更新 enable 状态
            model.enable = new_enable;
            let active_model: crontab::ActiveModel = model.into();
            active_model.update(db).await?;
            
            Ok(Some(new_enable))
        }
        None => Ok(None) // 没找到对应的 crontab
    }
}

pub async fn set_crontab_enable_by_name(name: String, enable: bool) -> Result<Option<bool>, sea_orm::DbErr> {
    let db = match DB.get() {
        None => {
            return Err(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal("Database not initialized".to_string())));
        }
        Some(db) => db,
    };

    // 首先查找 crontab
    let crontab_option = crontab::Entity::find()
        .filter(crontab::Column::Name.eq(&name))
        .one(db)
        .await?;

    match crontab_option {
        Some(mut model) => {
            // 设置为指定的启用状态
            model.enable = enable;
            let active_model: crontab::ActiveModel = model.into();
            active_model.update(db).await?;
            
            Ok(Some(enable))
        }
        None => Ok(None) // 没找到对应的 crontab
    }
}

pub fn init_crontab_worker() {
    tokio::spawn(async move {
        info!("Crontab scheduler started.");
        loop {
            sleep(Duration::from_secs(1)).await;

            tokio::spawn(async move {
                process_crontab().await;
            });
        }
    });
}

async fn process_crontab() {
    let db = match DB.get() {
        None => {
            error!("DB not initialized");
            return;
        }
        Some(db) => db,
    };

    let jobs = match crontab::Entity::find()
        .filter(crontab::Column::Enable.eq(true))
        .all(db)
        .await
    {
        Ok(jobs) => jobs,
        Err(err) => {
            error!("{}", err);
            return;
        }
    };

    let now = Utc::now();

    for job in jobs {
        let schedule = match Schedule::from_str(&job.cron_expression) {
            Ok(s) => s,
            Err(e) => {
                warn!("Invalid cron expression for job {}: {}", job.id, e);
                continue;
            }
        };

        let last_run = job
            .last_run_time
            .map(|t| Utc.timestamp_millis_opt(t).unwrap())
            .unwrap_or_else(|| now - chrono::Duration::seconds(1));

        if let Some(next_run) = schedule.after(&last_run).next() {
            if next_run <= now {
                info!("Triggering cron job: {} ({})", job.name, job.id);

                let mut active_model: crontab::ActiveModel = job.clone().into();
                active_model.last_run_time = Set(Some(now.timestamp_millis()));
                if let Err(e) = active_model.update(db).await {
                    error!("Failed to update last_run_time for job {}: {}", job.id, e);
                    continue;
                }

                let job_parsed = Cron {
                    id: job.id,
                    name: job.name,
                    enable: job.enable,
                    cron_expression: job.cron_expression,
                    cron_type: {
                        match serde_json::from_str(&job.cron_type.to_string()) {
                            Ok(cron_type) => cron_type,
                            Err(e) => {
                                warn!("Invalid cron type for job {}: {}", job.id, e);
                                continue;
                            }
                        }
                    },
                    last_run_time: job.last_run_time,
                };

                tokio::spawn(async move {
                    run_job_logic(job_parsed).await;
                });
            }
        }
    }
}

async fn run_job_logic(job: Cron) {
    match job.cron_type {
        CronType::Agent(uuids, agent_cron) => match agent_cron {
            AgentCronType::Task(task_event_type) => {
                task::crontab_task(job.id, job.name, uuids, task_event_type).await;
            }
        },
        CronType::Server(server_cron) => {
             match server_cron {
                 ServerCronType::CleanUpDatabase => {
                     todo!()
                 }
             }
        }
    }
}
