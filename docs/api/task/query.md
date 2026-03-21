# 调用者查询

调用者可以通过 `task_query` 查询

需要传入 `token` / `task_data_query`:

```json
{
    "token": "demo_token",
    "task_data_query": {
        "condition": [
            // QueryCondition 结构体，该结构体参考 Task 总览
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

## 删除任务记录

调用者可以通过 `task_delete` 删除任务记录。

需要传入 `token` / `conditions`：

```json
{
    "token": "demo_token",
    "conditions": [
        // TaskQueryCondition 结构体，参考 Task 总览
        // 查询能命中的记录，就是删除会影响的记录
    ]
}
```

语义说明：

1. `condition` 使用与 `task_query` 完全一致的 `TaskQueryCondition`。
2. 若包含 `last` / `limit`，会按 `timestamp desc, id desc` 先选中再删除。
3. 若不含 `last` / `limit`，则按过滤条件批量删除。

返回结构：

```json
{
    "success": true,
    "deleted": 12,
    "condition_count": 2
}
```

权限要求：

- 需要 `Task::Delete(String)` 权限。
- 当 `condition` 包含 `type` 时，需要对应类型的删除权限。
- 当 `condition` 不包含 `type` 时，要求覆盖所有任务类型的删除权限。
