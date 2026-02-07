# 删除 Crontab

删除指定的定时任务。

## 方法

调用方法名为 `crontab_delete`，需要提供以下参数:

```json
{
    "token": "demo_token",
    "name": "task_name_to_delete"
}
```

## 权限要求

删除 Crontab 需要 `Crontab::Delete` 权限。