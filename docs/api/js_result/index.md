# JsResult 总览

`JsResult` 是 `js-worker_run` 的异步执行结果表。

一次 `run` 会先创建一条结果记录并返回 `id`，脚本执行完成后再回填该记录。

## 方法列表

| 方法名                   | 描述     |
|-----------------------|--------|
| [query](./query.md)   | 查询执行结果 |
| [delete](./delete.md) | 删除执行结果 |

## 数据结构

`JsResult` 每条记录包含：

```json
{
  "id": 1,
  "js_worker_id": 10,
  "js_worker_name": "demo_worker",
  "start_time": 1775000000000,
  "finish_time": 1775000000123,
  "param": {
    "hello": "world"
  },
  "result": {
    "ok": true
  },
  "error_message": null
}
```

说明：

- `result` 与 `error_message` 至少有一个会被回填。
- 运行中状态定义为：`result == null && error_message == null`。

## 查询条件

统一使用 `JsResultQueryCondition`：

```rust
#[serde(rename_all = "snake_case")]
pub enum JsResultQueryCondition {
    Id(i64),
    JsWorkerId(i64),
    JsWorkerName(String),
    StartTimeFromTo(i64, i64),
    StartTimeFrom(i64),
    StartTimeTo(i64),
    FinishTimeFromTo(i64, i64),
    FinishTimeFrom(i64),
    FinishTimeTo(i64),
    IsSuccess,
    IsFailure,
    IsRunning,
    Limit(u64),
    Last,
}
```

多个条件并存时为 `AND`。

## 权限说明

`JsResult` 权限基于 `Scope::JsWorker(String)` 生效，支持后缀 `*` 通配符。

```json
{
  "scopes": [
    {
      "js_worker": "demo_*"
    }
  ],
  "permissions": [
    {
      "js_result": {
        "read": "demo_*"
      }
    },
    {
      "js_result": {
        "delete": "demo_*"
      }
    }
  ]
}
```

- `read`：可查询匹配脚本名的结果。
- `delete`：可删除匹配脚本名的结果。
