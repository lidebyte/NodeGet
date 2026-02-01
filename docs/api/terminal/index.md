# Terminal WebShell 总览

WebShell 是 Task 任务系统下的一个特殊功能，也叫 `网页 SSH` / `Terminal`

## 通信实现

Server 任务下发 与 Agent 接收到任务，还是由 Task 的监听器通信

在接收到来自 Server 的 WebShell Task 后，Agent 会主动通过 WebSocket 连接到 Server 提供的 Url，等待 用户(网页) 连接后，开始双向
Binary Message 通信

## Agent 获取的 URL

由 NodeGet Server 提供的，Agent 连接的 Terminal Url 格式如下:

```
ws(s)://HOST(:PORT)/terminal?agent_uuid={agent_uuid}&task_id={task_id}&task_token={task_token}
```

参数用于校验对应的 Task

该 Url 有以下两种生成方式:

- 以 `ws(s)://HOST(:PORT)/auto_gen` 为格式的 Url，将自动格式化成上述格式
- 用户指定 Url，可以是任意外部链接，包括但不限于其他监控 Server 提供的

## 用户连接 Url

由 NodeGet Server 提供的，用户 连接的 Terminal Url 格式如下:

```
ws(s)://HOST(:PORT)/terminal?agent_uuid={agent_uuid}&token=demo_token
```

用户在 Agent 连接后，可以与 Agent 进行双向 WebSocket 通信

## 注意事项

实际上，该通道可以传输任意类型的 WebSocket 数据，包括但不限于心跳包、文本类型与 Binary 类型

后续会把该通道拓展使用方向，敬请期待

