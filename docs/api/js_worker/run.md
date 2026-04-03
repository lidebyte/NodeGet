# 运行脚本

调用者可以通过 `js-worker_run` 运行已注册脚本。

需要传入 `token` / `js_script_name` / `params`，可选传入 `run_type` / `env`：

```json
{
  "token": "demo_token",
  "js_script_name": "demo_worker",
  "run_type": "call",
  "params": {
    "hello": "world"
  },
  "env": {
    "override": true
  }
}
```

参数说明：

- `run_type` 可选：`call` / `inline_call` / `cron` / `route`，默认 `call`。
- `params` 必填：任意 JSON，传给脚本入口函数第一个参数。
- `env` 可选：
    - 传入时：使用请求里的 `env`
    - 不传时：使用数据库中该脚本保存的 `env`，若为空则使用 `{}`

返回结构：

```json
{
  "id": 123
}
```

`id` 是 `js_result` 表中的记录 ID。`run` 不会等待脚本执行结束。

## 结果查询

脚本执行结果请通过 `js-result_query` 查询：

```json
{
  "jsonrpc": "2.0",
  "method": "js-result_query",
  "params": {
    "token": "demo_token",
    "query": {
      "condition": [
        { "id": 123 }
      ]
    }
  },
  "id": 2
}
```

## 完整示例

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_run",
  "params": {
    "token": "demo_token",
    "js_script_name": "demo_worker",
    "run_type": "call",
    "params": {
      "hello": "world"
    }
  },
  "id": 1
}
```

## 权限要求

- 需要 `Permission::JsWorker(JsWorker::RunDefinedJsWorker)`。
- 作用域要求：`Scope::JsWorker(js_script_name)`。
