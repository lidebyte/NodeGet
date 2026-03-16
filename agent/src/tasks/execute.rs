use crate::AGENT_CONFIG;
use nodeget_lib::error::NodegetError;
use nodeget_lib::task::ExecuteTask;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

// 命令执行超时时间，设定为 60 秒
const EXECUTE_TIMEOUT: Duration = Duration::from_mins(1);

/// 命令执行结果类型
pub type Result<T> = std::result::Result<T, NodegetError>;

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
    let config = AGENT_CONFIG
        .get()
        .expect("Agent config not initialized")
        .read()
        .expect("AGENT_CONFIG lock poisoned")
        .clone();
    let max_chars = config.exec_max_character.unwrap_or(10000);

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

    let child = cmd
        .spawn()
        .map_err(|e| NodegetError::Other(format!("Failed to spawn command '{}': {e}", task.cmd)))?;

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
                let truncated_part = result.split_off(original_len - max_chars);
                result = format!(
                    "[... Output truncated from {original_len} to {max_chars} chars ...]\n{truncated_part}"
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
