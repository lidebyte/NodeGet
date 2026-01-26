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

## Error

该方法可能返回错误

UUID 指定的 Agent 未注册:

```json
{
    "error_id": 106,
    "error_message": "Error sending task event: Uuid not found"
}
```