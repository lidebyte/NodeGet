use crate::AGENT_CONFIG;
use log::error;
use nodeget_lib::error::NodegetError;
use nodeget_lib::task::ExecuteTask;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

// 命令执行超时时间，设定为 60 秒
const EXECUTE_TIMEOUT: Duration = Duration::from_mins(1);
// 超时后等待进程响应 SIGTERM 的时间；超过则 SIGKILL
#[cfg(unix)]
const GRACE_AFTER_SIGTERM: Duration = Duration::from_secs(2);

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
    // 在 Unix 上把子进程放进独立进程组，超时时可以整组信号回收，
    // 避免 shell 脚本 fork 出的孙子进程变孤儿继续消耗资源。
    // pgid 与 child pid 相同（因为 setpgid(0, 0) 等价于 process_group(0)）。
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = cmd.spawn().map_err(|e| {
        // 内部记录详细错误，但向外部返回通用错误
        error!("Failed to spawn command '{}': {e}", task.cmd);
        NodegetError::Other("Command execution failed".to_owned())
    })?;

    // 取出 stdout/stderr 的 pipe，自己读取。不用 wait_with_output
    // 因为它会 move 掉 child，超时时就没法主动 kill。
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let read_stdout = async {
        let mut buf = Vec::new();
        if let Some(p) = stdout_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf).await;
        }
        buf
    };
    let read_stderr = async {
        let mut buf = Vec::new();
        if let Some(p) = stderr_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf).await;
        }
        buf
    };

    let wait_and_collect = async {
        let (status, out, err) = tokio::join!(child.wait(), read_stdout, read_stderr);
        (status, out, err)
    };

    match timeout(EXECUTE_TIMEOUT, wait_and_collect).await {
        Ok((Ok(status), stdout_buf, stderr_buf)) => {
            let mut result = String::from_utf8_lossy(&stdout_buf).into_owned();
            let stderr = String::from_utf8_lossy(&stderr_buf);

            if !stderr.is_empty() && !result.is_empty() {
                result.push_str("\n--- STDERR ---\n");
            }
            result.push_str(&stderr);

            if result.is_empty() {
                result.push_str("(No Output)");
            }

            if !status.success() {
                use std::fmt::Write;
                let _ = write!(
                    result,
                    "\n\n[Process exited with code {}]",
                    status.code().unwrap_or(-1)
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
        Ok((Err(e), _, _)) => Err(NodegetError::Other(format!(
            "Failed to wait for process: {e}"
        ))),
        Err(_) => {
            // 超时：Unix 下先 SIGTERM 整个进程组（回收孙子进程），
            // 给 GRACE_AFTER_SIGTERM 秒自愿退出；过期仍存活则 SIGKILL。
            // 非 Unix 平台退回到 kill_on_drop 语义（child drop 时 SIGKILL）。
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    // pid 永远 > 0，转换成 i32 由 libc::killpg 使用
                    // 对信号发送失败容忍：进程可能已经自己退出
                    #[allow(clippy::cast_possible_wrap)]
                    let pgid = pid as i32;
                    unsafe {
                        libc::killpg(pgid, libc::SIGTERM);
                    }
                }
                if timeout(GRACE_AFTER_SIGTERM, child.wait()).await.is_err() {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }
            }
            #[cfg(not(unix))]
            {
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
            Err(NodegetError::Other(format!(
                "Execution timed out (Limit: {}s)",
                EXECUTE_TIMEOUT.as_secs()
            )))
        }
    }
}
