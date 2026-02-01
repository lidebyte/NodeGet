---
outline: deep
---

# Agent Monitoring 实现

该文档会简述 JsonRpc 的基本使用，若完全不了解请看本文档的 `上报方法`，此后只会提供方法名与上报结构体

## 上报方法

在 `nodeget-server` 中，上报方法为 `report_static` 与 `report_dynamic`，这两个方法位于 `agent` 下，使用需要添加 `agent_` 前缀

这两个方法用法类似，需要传入 `token`, `static(dynamic)_monitoring_data` 两个参数，`token` 为 String 类型，
`static(dynamic)_monitoring_data` 即为上面的结构体

需要构建如下的结构体以上报:

```json
{
  "jsonrpc": "2.0",
  "method": "report_static(dynamic)",
  "params": {
    "token": "demo_token",
    "static(dynamic)_monitoring_data": {
        // Monitoring 回报结构体，该结构体参考 Monitoring 总览
    }
  },
  "id": 1
}
```

或在 `params` 字段使用元组，需要确保位置正确:

```json
{
  "jsonrpc": "2.0",
  "method": "report_static(dynamic)",
  "params": [
    "demo_token",
    {
        // Monitoring 回报结构体，该结构体参考 Monitoring 总览
    }
  ],
  "id": 1 // 该 ID 可自定义，返回值也带统一 ID 用于辨别哪一个请求
}
```

两种调用方式等价

## 返回值

上报成功后，会收到来自 服务器的返回信息:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": 11858 // 在数据库中表的 ID 字段
  }
}
```