# 下发 Task

调用者可以通过 Server 给指定 Agent 下发 Task

## 方法

调用方法名为 `task_create_task`，需要提供以下参数:

```json
{
    "token": "demo_token",
    "target_uuid": "AGENT_UUID_HERE", // 指定的 Agent UUID
    "task_type": {
        // 任务主体，该结构体参考 Task 总览
    }
}
```

调用完成后，返回类似:

```json
{
    "id": 4 // 数据库中的 ID 字段，可通过该字段作为条件查询
}
```

当 `task_type` 为 `web_shell` 时，必须携带 `terminal_id`（随机 UUID）。例如：

```json
{
    "token": "demo_token",
    "target_uuid": "AGENT_UUID_HERE",
    "task_type": {
        "web_shell": {
            "url": "wss://YOUR_SERVER/auto_gen",
            "terminal_id": "4c8d1cba-244e-4baf-9b65-c881f86ca60a"
        }
    }
}
```

当 `task_type` 为 `execute` 时，必须使用结构化参数（`cmd + args`）：

```json
{
    "token": "demo_token",
    "target_uuid": "AGENT_UUID_HERE",
    "task_type": {
        "execute": {
            "cmd": "ls",
            "args": [
                "-1",
                "tmp"
            ]
        }
    }
}
```

如需 shell 语法，请显式调用 shell 程序并传参（示例：`bash -c` 或 `cmd /C`），而不是直接传一整段 shell 字符串。
同时 `execute.cmd` 不能为空字符串。

## Error

该方法可能返回错误

UUID 指定的 Agent 未注册:

```json
{
    "error_id": 104,
    "error_message": "Error sending task event: Agent AGENT_UUID_HERE is not connected"
}
```
