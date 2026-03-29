# 列出可见脚本

调用者可以通过 `js-worker_list_all_js_worker` 列出当前 Token 可见且真实存在于数据库中的脚本名。

需要传入 `token`：

```json
{
  "token": "demo_token"
}
```

返回结构：

```json
[
  "demo_worker",
  "test_ping_worker"
]
```

说明：

- SuperToken：可返回数据库中全部脚本。
- 普通 Token：仅返回同时满足以下条件的脚本：
    1. 数据库中存在
    2. Token 在该脚本名作用域下拥有 `JsWorker::ListALlJsWorker` 权限

## 完整示例

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_list_all_js_worker",
  "params": {
    "token": "demo_token"
  },
  "id": 1
}
```

## 权限要求

- 需要 `Permission::JsWorker(JsWorker::ListALlJsWorker)`。
- 作用域要求：`Scope::JsWorker(name)`，支持后缀 `*` 通配。
