# 设置 Crontab 启用状态

强制设置指定定时任务的启用/禁用状态。

## 方法

调用方法名为 `crontab_set_enable`，需要提供以下参数:

```json
{
    "token": "demo_token",
    "name": "task_name",
    "enable": true
}
```

此操作会将任务的状态强制设置为指定的启用/禁用状态：

- `enable: true` 将任务设置为启用
- `enable: false` 将任务设置为禁用
