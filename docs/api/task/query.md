# 调用者查询

调用者可以通过 `task_query` 查询

需要传入 `token` / `task_data_query`:

```json
{
    "token": "demo_token",
    "task_data_query": {
        "condition": [
            // QueryCondition 结构体，该结构体参考 Monitoring 总览
            // 该字段为 Vec<_>，可指定多个
        ]
    }
}
```

返回结构:

```json
[
    {
        "error_message": null,
        "success": true,
        "task_event_result": {
            // 任务回报结构体，该结构体参考 Task 总览
        },
        "task_event_type": {
            // 任务主体，该结构体参考 Task 总览
        },
        "task_id": 6,
        "timestamp": 1769341269012,
        "uuid": "42e89a61-39de-4569-b6ef-e86bc3ed8f82"
    }
    // 该字段为 Vec<_>，可指定多个
]
```