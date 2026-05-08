use crate::AGENT_CONFIG;
use log::error;
use nodeget_lib::error::NodegetError;
use nodeget_lib::task::ExecuteTask;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

// 命令执行超时时间，设定为 60 秒
const EXECUTE_TIMEOUT: Duration = Duration::from_mins(1);

/// 命令执行结果类型
pub type Result<T> = std::result::Result<T, NodegetError>;

// 安全地获取 Agent 配置
fn get_agent_config() -> Result<crate::AgentConfig> {
    AGENT_CONFIG
        .get()
        .ok_or_else(|| NodegetError::Other("Agent config not initialized".to_owned()))?
        .read()
        .map(|guard| guard.clone())
        .map_err(|_| NodegetError::Other("AGENT_CONFIG lock poisoned".to_owned()))
}

// 执行指定的命令
//
// 该函数直接执行 cmd + args，不提供字符串拼接 shell 的接口。
//
// # 参数
// * `task` - 结构化命令参数
//
// # 返回值
// 成功时返回命令输出字符串，失败时返回错误信息
pub async fn execute_command(task: ExecuteTask) -> Result<String> {
    let config = get_agent_config()?;
    let max_chars = config.exec_max_character_or_default();

    if task.cmd.trim().is_empty() {
        return Err(NodegetError::InvalidInput(
            "Execute command cannot be empty".to_owned(),
        ));
    }

    let mut cmd = Command::new(&task.cmd);
    cmd.args(&task.args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    let child = cmd.spawn().map_err(|e| {
        // 内部记录详细错误，但向外部返回通用错误
        error!("Failed to spawn command '{}': {e}", task.cmd);
        NodegetError::Other("Command execution failed".to_owned())
    })?;

    match timeout(EXECUTE_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stderr.is_empty() && !result.is_empty() {
                result.push_str("\n--- STDERR ---\n");
            }
            result.push_str(&stderr);

            if result.is_empty() {
                result.push_str("(No Output)");
            }

            if !output.status.success() {
                use std::fmt::Write;
                let _ = write!(
                    result,
                    "\n\n[Process exited with code {}]",
                    output.status.code().unwrap_or(-1)
                );
            }

            if result.len() > max_chars {
                let original_len = result.len();
                let split_at = result.ceil_char_boundary(original_len - max_chars);
                let truncated_part = result.split_off(split_at);
                result = format!(
                    "[... Output truncated from {original_len} to {} bytes ...]\n{truncated_part}",
                    truncated_part.len()
                );
            }

            Ok(result)
        }
        Ok(Err(e)) => Err(NodegetError::Other(format!(
            "Failed to wait for process: {e}"
        ))),
        Err(_) => Err(NodegetError::Other(format!(
            "Execution timed out (Limit: {}s)",
            EXECUTE_TIMEOUT.as_secs()
        ))),
    }
}
