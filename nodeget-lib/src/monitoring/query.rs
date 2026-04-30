use serde::{Deserialize, Serialize};
use serde_json::Value;

// 静态监控数据查询字段枚举，定义可查询的静态数据类型
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StaticDataQueryField {
    // CPU 相关信息
    Cpu,
    // 系统相关信息
    System,
    // GPU 相关信息
    Gpu,
}

impl StaticDataQueryField {
    /// 获取字段对应的数据库列名
    #[must_use]
    pub const fn column_name(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu_data",
            Self::System => "system_data",
            Self::Gpu => "gpu_data",
        }
    }

    /// 获取字段的 JSON 键名
    #[must_use]
    pub const fn json_key(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::System => "system",
            Self::Gpu => "gpu",
        }
    }
}

// 动态监控数据查询字段枚举，定义可查询的动态数据类型
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DynamicDataQueryField {
    // CPU 相关信息
    Cpu,
    // 内存相关信息
    Ram,
    // 系统负载相关信息
    Load,
    // 系统相关信息
    System,
    // 磁盘相关信息
    Disk,
    // 网络相关信息
    Network,
    // GPU 相关信息
    Gpu,
}

impl DynamicDataQueryField {
    /// 获取字段对应的数据库列名
    #[must_use]
    pub const fn column_name(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu_data",
            Self::Ram => "ram_data",
            Self::Load => "load_data",
            Self::System => "system_data",
            Self::Disk => "disk_data",
            Self::Network => "network_data",
            Self::Gpu => "gpu_data",
        }
    }

    /// 获取字段的 JSON 键名
    #[must_use]
    pub const fn json_key(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Ram => "ram",
            Self::Load => "load",
            Self::System => "system",
            Self::Disk => "disk",
            Self::Network => "network",
            Self::Gpu => "gpu",
        }
    }
}

// 查询条件枚举，定义各种查询过滤条件
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryCondition {
    // 按 UUID 过滤
    Uuid(uuid::Uuid),
    // 按时间戳范围过滤（开始时间，结束时间）
    TimestampFromTo(i64, i64), // start, end
    // 按时间戳起始点过滤
    TimestampFrom(i64), // start,
    // 按时间戳结束点过滤
    TimestampTo(i64), // end

    // 限制返回结果数量
    Limit(u64), // limit

    // 获取最后一条记录
    Last,
}

// 静态监控数据查询结构体
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticDataQuery {
    // 要查询的字段列表
    pub fields: Vec<StaticDataQueryField>,
    // 查询条件列表
    pub condition: Vec<QueryCondition>,
}

// 动态监控数据查询结构体
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicDataQuery {
    // 要查询的字段列表
    pub fields: Vec<DynamicDataQueryField>,
    // 查询条件列表
    pub condition: Vec<QueryCondition>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticDataAvgQuery {
    // 要查询并聚合平均值的字段列表
    pub fields: Vec<StaticDataQueryField>,
    // 指定要查询的 Agent UUID
    pub uuid: uuid::Uuid,
    // 可选：起始时间戳（毫秒）
    pub timestamp_from: Option<i64>,
    // 可选：结束时间戳（毫秒）
    pub timestamp_to: Option<i64>,
    // 分段点数
    pub points: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicDataAvgQuery {
    // 要查询并聚合平均值的字段列表
    pub fields: Vec<DynamicDataQueryField>,
    // 指定要查询的 Agent UUID
    pub uuid: uuid::Uuid,
    // 可选：起始时间戳（毫秒）
    pub timestamp_from: Option<i64>,
    // 可选：结束时间戳（毫秒）
    pub timestamp_to: Option<i64>,
    // 分段点数
    pub points: u64,
}

// 静态监控数据响应项结构体
#[derive(Serialize)]
pub struct StaticResponseItem {
    // 设备 UUID
    pub uuid: String,
    // 时间戳
    pub timestamp: i64,
    // CPU 数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Value>,
    // 系统数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Value>,
    // GPU 数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<Value>,
}

// 动态监控数据响应项结构体
#[derive(Serialize)]
pub struct DynamicResponseItem {
    // 设备 UUID
    pub uuid: String,
    // 时间戳
    pub timestamp: i64,
    // CPU 数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Value>,
    // 内存数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram: Option<Value>,
    // 负载数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<Value>,
    // 系统数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Value>,
    // 磁盘数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<Value>,
    // 网络数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Value>,
    // GPU 数据，可选
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<Value>,
}

// 动态监控摘要数据查询字段枚举
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DynamicSummaryQueryField {
    CpuUsage,
    GpuUsage,
    UsedSwap,
    TotalSwap,
    UsedMemory,
    TotalMemory,
    AvailableMemory,
    LoadOne,
    LoadFive,
    LoadFifteen,
    Uptime,
    BootTime,
    ProcessCount,
    TotalSpace,
    AvailableSpace,
    ReadSpeed,
    WriteSpeed,
    TcpConnections,
    UdpConnections,
    TotalReceived,
    TotalTransmitted,
    TransmitSpeed,
    ReceiveSpeed,
}

impl DynamicSummaryQueryField {
    /// 获取字段对应的数据库列名
    #[must_use]
    pub const fn column_name(&self) -> &'static str {
        match self {
            Self::CpuUsage => "cpu_usage",
            Self::GpuUsage => "gpu_usage",
            Self::UsedSwap => "used_swap",
            Self::TotalSwap => "total_swap",
            Self::UsedMemory => "used_memory",
            Self::TotalMemory => "total_memory",
            Self::AvailableMemory => "available_memory",
            Self::LoadOne => "load_one",
            Self::LoadFive => "load_five",
            Self::LoadFifteen => "load_fifteen",
            Self::Uptime => "uptime",
            Self::BootTime => "boot_time",
            Self::ProcessCount => "process_count",
            Self::TotalSpace => "total_space",
            Self::AvailableSpace => "available_space",
            Self::ReadSpeed => "read_speed",
            Self::WriteSpeed => "write_speed",
            Self::TcpConnections => "tcp_connections",
            Self::UdpConnections => "udp_connections",
            Self::TotalReceived => "total_received",
            Self::TotalTransmitted => "total_transmitted",
            Self::TransmitSpeed => "transmit_speed",
            Self::ReceiveSpeed => "receive_speed",
        }
    }

    /// 获取字段的 JSON 键名（与列名相同，因为是扁平列）
    #[must_use]
    pub const fn json_key(&self) -> &'static str {
        self.column_name()
    }

    /// 该字段是否在数据库中以 *10 缩放存储（读取时需要 /10.0 还原）
    #[must_use]
    pub const fn is_scaled(&self) -> bool {
        matches!(
            self,
            Self::CpuUsage | Self::LoadOne | Self::LoadFive | Self::LoadFifteen
        )
    }
}

// 动态监控摘要数据查询结构体
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicSummaryQuery {
    pub fields: Vec<DynamicSummaryQueryField>,
    pub condition: Vec<QueryCondition>,
}

// 动态监控摘要平均值查询结构体
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicSummaryAvgQuery {
    pub fields: Vec<DynamicSummaryQueryField>,
    pub uuid: uuid::Uuid,
    pub timestamp_from: Option<i64>,
    pub timestamp_to: Option<i64>,
    pub points: u64,
}

// 动态监控摘要数据响应项结构体
#[derive(Serialize)]
pub struct DynamicSummaryResponseItem {
    pub uuid: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_usage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_swap: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_swap: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_memory: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_memory: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_memory: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_one: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_five: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_fifteen: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_time: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_count: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_space: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_space: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_speed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_speed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_connections: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_connections: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_received: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_transmitted: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transmit_speed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_speed: Option<Value>,
}

/// Apply /10.0 descaling to known scaled columns in the JSON object.
///
/// This is done in application code rather than SQL to work around
/// `SQLite` limitations with expression aliases in raw query-to-JSON mapping.
pub fn apply_descaling_to_json_object(obj: &mut serde_json::Map<String, serde_json::Value>) {
    const SCALED_FIELDS: &[&str] = &["cpu_usage", "load_one", "load_five", "load_fifteen"];
    for key in SCALED_FIELDS {
        if let Some(val) = obj.get_mut(*key)
            && let serde_json::Value::Number(n) = val
        {
            if let Some(i) = n.as_i64() {
                if let Some(scaled) = serde_json::Number::from_f64(i as f64 / 10.0) {
                    *val = serde_json::Value::Number(scaled);
                }
            } else if let Some(f) = n.as_f64()
                && let Some(scaled) = serde_json::Number::from_f64(f / 10.0)
            {
                *val = serde_json::Value::Number(scaled);
            }
        }
    }
}
