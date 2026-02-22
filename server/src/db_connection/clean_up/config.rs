/// 数据清理配置
/// 所有 limit 字段单位为毫秒
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    pub agent_uuid: String,
    pub static_monitoring_limit: Option<i64>,
    pub dynamic_monitoring_limit: Option<i64>,
    pub task_limit: Option<i64>,
    pub crontab_result_limit: Option<i64>,
}
