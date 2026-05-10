use crate::error::{NodegetError, Result};
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use serde::Deserialize;
use serde::Serialize;
use std::sync::atomic::{AtomicI64, Ordering};

// 服务器错误消息处理模块
#[cfg(feature = "for-server")]
pub mod error_message;

// 版本信息模块
pub mod version;

// Uuid 相关
pub mod uuid;

// 服务器 Json Parser
#[cfg(feature = "for-server")]
pub mod server_json;

// JSON-RPC 公共错误结构体
//
// 错误代码说明：
// 101: Parse Error
// 102: Permission Denied
// 103: Database Error
// 104: Unable to connect agent
// 105: Not Found in Database
// 106: Uuid Not Found
// 107: Config Not Found
//
// 999: 详情请看 error_message
#[derive(Serialize, Deserialize)]
pub struct JsonError {
    // 错误 ID
    pub error_id: i128,
    // 错误消息
    pub error_message: String,
}

/// 全局 NTP 时间偏移量（毫秒），正值表示本地时间慢，负值表示本地时间快
static NTP_OFFSET_MS: AtomicI64 = AtomicI64::new(0);

/// 设置 NTP 时间偏移量（毫秒）
pub fn set_ntp_offset_ms(offset_ms: i64) {
    NTP_OFFSET_MS.store(offset_ms, Ordering::Relaxed);
}

/// 获取本地毫秒级时间戳（已应用 NTP 偏移校正）
///
/// # Errors
///
/// 当系统时间获取失败或偏移后下溢时返回错误
pub fn get_local_timestamp_ms() -> Result<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| NodegetError::Other(format!("System time error: {e}")))?;
    let offset = NTP_OFFSET_MS.load(Ordering::Relaxed);
    let millis = i64::try_from(duration.as_millis())
        .map_err(|e| NodegetError::Other(format!("Timestamp conversion error: {e}")))?
        .saturating_add(offset);
    if millis < 0 {
        return Err(NodegetError::Other("Timestamp underflow after NTP offset".to_owned()).into());
    }
    u64::try_from(millis)
        .map_err(|e| NodegetError::Other(format!("Timestamp conversion error: {e}")).into())
}

/// 获取本地毫秒级时间戳（带符号 i64 版本）
///
/// # Errors
///
/// 当系统时间获取失败或转换失败时返回错误
pub fn get_local_timestamp_ms_i64() -> Result<i64> {
    get_local_timestamp_ms().and_then(|ts| {
        i64::try_from(ts).map_err(|e| {
            NodegetError::Other(format!("Timestamp conversion error: {e}")).into()
        })
    })
}

/// 生成指定长度的随机字符串
///
/// # 参数
/// * `len` - 需要生成的随机字符串长度
///
/// # 返回值
/// 返回生成的随机字符串
#[must_use]
pub fn generate_random_string(len: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
