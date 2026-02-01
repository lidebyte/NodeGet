# Agent 任务处理

## 注册任务

若 Agent 需要获取 Server 下发的任务，需要在一个 WebSocket 长连接内订阅任务获取的方法。中途退出、WebSocket 断线、使用
Http、或主动取消订阅 均不会再接收到来自 Server 的任务

涉及到的方法名称为 `task_register_task`

使用需要提供:

```json
{
  "uuid": "AGENT_UUID_HERE"
}
```

返回值的 `result` 字段为订阅 ID

## 接收任务

Agent 会在这一 Websocket 长连接中接收到 method 为 `task_register_task` 的 JsonRpc Request，其中 `params` 字段如下:

```json
{
  "subscription": 5293582878088374,
  // 为上面的订阅 ID，可用于校验 (若在同一长连接中注册多个任务接收器)
  "result": {
    "task_id": 3,
    // 数据库中的 ID 字段，上报任务结果需要使用
    "task_token": "k6bsrBv1hS",
    // 字段仅任务注册者可获取，用于校验上传者是否为接收者，任务下发方 / Server 均不主动知晓
    "task_event_type": {
      // 任务主体，该结构体参考 Task 总览
    }
  }
}
```

## 上报结果

在处理完下发的 Task 后，可以通过 `task_upload_task_result` 方法来上传结果，构建结构体如下:

```json
{
  "token": "demo_token",
  // 上报用 Token，非 Task Token
  "task_response": {
    "task_id": 3,
    // Task ID
    "agent_uuid": "AGENT_UUID_HERE",
    // 下发任务时指定的 Agent UUID
    "task_token": "k6bsrBv1hS",
    // 下发任务时生成的 Task Token

    "timestamp": 1769341269012,
    // 完成时的毫秒时间戳
    "success": true,
    // 是否成功
    "error_message": "XXXXXX",
    // 若 success 为 false，该字段必需；若 success 为 true，该字段可选
    "task_event_result": {
      //若 success 为 true，该字段必需；若 success 为 false，该字段为空
      // 任务回报结构体，该结构体参考 Task 总览
    }
  }
}
```

Server 会使用 `token` / `task_id` / `agent_uuid` / `task_token` 进行鉴权，需四项均统一

## Error

该方法可能返回错误

Task 验证未通过:

```json
{
  "error_id": 105,
  "error_message": "Task validation failed: Invalid ID, UUID, or Token"
}
```