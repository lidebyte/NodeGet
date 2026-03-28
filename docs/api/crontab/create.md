# 创建 Crontab

创建新的定时任务。

## 方法

调用方法名为 `crontab_create`，需要提供以下参数:

```json
{
    "token": "demo_token",
    "name": "task_name",                    // 任务名称
    "cron_expression": "0 * * * *",         // Cron 表达式
    "cron_type": {
        // 任务类型，详情见下文
    }
}
```

该方法仅用于 **创建**。如果 `name` 已存在，会直接返回错误，不会覆盖原有 Crontab。

### Cron 表达式

Cron 表达式遵循标准格式，包含秒、分、时、日、月、周字段。

例如：

- `0 * * * * *` 表示每小时执行一次
- `0 0 * * * *` 表示每天零点执行一次
- `0 0 1 * * *` 表示每月1号零点执行一次

### Cron 类型

Cron 任务支持两种类型：

#### Agent 任务类型

在特定 Agent 上执行任务:

```json
{
    "agent": [
        [
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000002"
        ],
        {
            "task": {
                "ping": "www.example.com"
            }
        }
    ]
}
```

此示例表示在 UUID 为 `00000000-0000-0000-0000-000000000001` 和 `00000000-0000-0000-0000-000000000002` 的 Agent 上执行
ping 任务。

#### Server 任务类型

在服务器上执行任务:

```json
{
    "server": "clean_up_database"
}
```

此示例表示执行数据库清理任务。

触发已注册的 JsWorker 脚本:

```json
{
    "server": {
        "js_worker": [
            "demo_nodeget_fetch",
            {
                "hello": "from_cron"
            }
        ]
    }
}
```

说明：

- 第一个参数是脚本名（`js_worker.name`）
- 第二个参数是传给脚本的 `params`（任意 JSON）
- Cron 触发时不传 `env`，会使用脚本自身在数据库保存的 `env`
- 触发成功后会生成 `js_result` 记录，`crontab_result.special_id` 即该 `js_result.id`

## 权限要求

创建 Crontab 需要：

- `Crontab::Write`
- 若是 Agent 类型，还需要对应任务类型的 `Task::Create`
- 若是 `server.js_worker` 类型，还需要 `JsWorker::RunDefinedJsWorker`（作用域需覆盖该脚本名）

并且必须覆盖 `cron_type` 中声明的 **所有 Scope**（例如 Agent 列表中的每个 UUID）。

示例权限配置:

```json
{
    "scopes": [
        {"agent_uuid": "00000000-0000-0000-0000-000000000001"},
        {"agent_uuid": "00000000-0000-0000-0000-000000000002"}
    ],
    "permissions": [
        {"crontab": "write"},
        {"task": {"create": "ping"}},
        {"task": {"create": "tcp_ping"}}
    ]
}
```

`server.js_worker` 权限示例：

```json
{
    "scopes": [
        {"global": null},
        {"js_worker": "demo_*"}
    ],
    "permissions": [
        {"crontab": "write"},
        {"js_worker": "run_defined_js_worker"}
    ]
}
```

## 返回值

创建成功后返回任务 ID:

```json
{
    "id": 123
}
```
