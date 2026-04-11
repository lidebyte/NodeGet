# JsWorker CRUD

## Create JsWorker

调用者可以通过 `js-worker_create` 创建脚本。

### 方法

调用方法名为 `js-worker_create`，需要提供以下参数：

```json
{
  "token": "demo_token",            // 鉴权 Token
  "name": "demo_worker",            // 脚本唯一名称
  "description": "demo worker for monitoring", // 可选，脚本描述
  "js_script_base64": "ZXhwb3J0IGRlZmF1bHQg...", // Base64 编码后的 UTF-8 JS 源码
  "route_name": "demo_route",       // 可选，HTTP 路由入口，路径前缀为 /worker-route/{route_name}
  "runtime_clean_time": 60000,       // 脚本 Runtime 空闲清理时间（毫秒），null 表示不自动清理
  "env": {                           // 可选，任意 JSON 结构，存入数据库并可在运行时传给脚本
    "region": "ap-east-1"
  }
}
```

参数说明：

- `name`：脚本唯一名称。
- `description`：可选，脚本描述。
- `js_script_base64`：Base64 编码后的 UTF-8 JS 源码。
- `route_name`：可选。若设置则开启 HTTP 路由入口，对应路径前缀为 `/worker-route/{route_name}`。
- `runtime_clean_time`：脚本 Runtime 空闲清理时间（毫秒），`null` 表示不自动清理。
- `env`：可选，任意 JSON 结构，存入数据库并可在运行时传给脚本。

### 权限要求

- Permission: `JsWorker::Create`
- Scope: `JsWorker(name)` 必须覆盖目标脚本名，支持后缀 `*` 通配（如 `test_*`）

### 返回值

```json
{
  "id": 1,                                     // 数据库中的记录 ID
  "name": "demo_worker",                       // 脚本名称
  "description": "demo worker for monitoring", // 脚本描述
  "route_name": "demo_route",                  // HTTP 路由名称
  "create_at": 1774652000123,                  // 创建时间戳（毫秒）
  "update_at": 1774652000123                   // 更新时间戳（毫秒）
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_create",
  "params": {
    "token": "demo_token",
    "name": "demo_worker",
    "description": "demo worker for monitoring",
    "js_script_base64": "ZXhwb3J0IGRlZmF1bHQgeyBhc3luYyBvbkNhbGwocGFyYW1zLCBlbnYsIGN0eCkgeyByZXR1cm4geyBvazogdHJ1ZSwgcGFyYW1zLCBlbnYgfTsgfSwgYXN5bmMgb25Dcm9uKHBhcmFtcywgZW52LCBjdHgpIHsgcmV0dXJuIHsgb2s6IHRydWUsIGNyb246IHRydWUgfTsgfSB9Ow==",
    "route_name": "demo_route",
    "runtime_clean_time": 60000,
    "env": {
      "project": "NodeGet"
    }
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": 1,
    "name": "demo_worker",
    "description": "demo worker for monitoring",
    "route_name": "demo_route",
    "create_at": 1774652000123,
    "update_at": 1774652000123
  }
}
```

## Read JsWorker

调用者可以通过 `js-worker_read` 读取脚本信息。

### 方法

调用方法名为 `js-worker_read`，需要提供以下参数：

```json
{
  "token": "demo_token",  // 鉴权 Token
  "name": "demo_worker"   // 脚本唯一名称
}
```

### 权限要求

- Permission: `JsWorker::Read`
- Scope: `JsWorker(name)`

### 返回值

```json
{
  "name": "demo_worker",                       // 脚本名称
  "description": "demo worker for monitoring", // 脚本描述
  "route_name": "demo_route",                  // HTTP 路由名称
  "js_script_base64": "ZXhwb3J0IGRlZmF1bHQg...", // Base64 编码的 JS 源码
  "runtime_clean_time": 60000,                 // 空闲清理时间（毫秒）
  "env": {                                     // 脚本环境变量
    "region": "ap-east-1"
  },
  "create_at": 1774652000123,                  // 创建时间戳（毫秒）
  "update_at": 1774652000123                   // 更新时间戳（毫秒）
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_read",
  "params": {
    "token": "demo_token",
    "name": "demo_worker"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "name": "demo_worker",
    "description": "demo worker for monitoring",
    "route_name": "demo_route",
    "js_script_base64": "ZXhwb3J0IGRlZmF1bHQgeyBhc3luYyBvbkNhbGwocGFyYW1zLCBlbnYsIGN0eCkgeyByZXR1cm4geyBvazogdHJ1ZSB9OyB9IH07",
    "runtime_clean_time": 60000,
    "env": {
      "region": "ap-east-1"
    },
    "create_at": 1774652000123,
    "update_at": 1774652000123
  }
}
```

## Update JsWorker

调用者可以通过 `js-worker_update` 更新脚本。

### 方法

调用方法名为 `js-worker_update`，需要提供以下参数：

```json
{
  "token": "demo_token",            // 鉴权 Token
  "name": "demo_worker",            // 脚本唯一名称
  "description": "demo worker v2",  // 可选，脚本描述；传 null 可清空描述
  "js_script_base64": "ZXhwb3J0IGRlZmF1bHQg...", // Base64 编码后的 UTF-8 JS 源码
  "route_name": "demo_route_v2",    // 可选，HTTP 路由名称；null 可关闭路由绑定
  "runtime_clean_time": 120000,      // 脚本 Runtime 空闲清理时间（毫秒），null 表示不自动清理
  "env": {                           // 可选，任意 JSON 结构
    "region": "ap-southeast-1"
  }
}
```

参数说明：

- `name`：要更新的脚本唯一名称。
- `description`：可选；传 `null` 可清空描述。
- `js_script_base64`：Base64 编码后的 UTF-8 JS 源码。
- `route_name`：可选；`null` 可关闭该脚本的 HTTP 路由绑定。
- `runtime_clean_time`：脚本 Runtime 空闲清理时间（毫秒），`null` 表示不自动清理。
- `env`：可选，任意 JSON 结构。
- 更新后会重新预编译字节码。
- 已存在的 Runtime 实例会被立即驱逐，后续运行会使用新版本脚本。

### 权限要求

- Permission: `JsWorker::Write`
- Scope: `JsWorker(name)`

### 返回值

```json
{
  "success": true,                  // 是否成功
  "name": "demo_worker",           // 脚本名称
  "description": "demo worker v2", // 更新后的描述
  "route_name": "demo_route_v2",   // 更新后的路由名称
  "update_at": 1774652666000       // 更新时间戳（毫秒）
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_update",
  "params": {
    "token": "demo_token",
    "name": "demo_worker",
    "description": "demo worker v2",
    "js_script_base64": "ZXhwb3J0IGRlZmF1bHQgeyBhc3luYyBvbkNhbGwocGFyYW1zLCBlbnYsIGN0eCkgeyByZXR1cm4geyBvazogdHJ1ZSwgdmVyc2lvbjogMiB9OyB9IH07",
    "route_name": "demo_route_v2",
    "runtime_clean_time": 120000,
    "env": {
      "project": "NodeGet"
    }
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "name": "demo_worker",
    "description": "demo worker v2",
    "route_name": "demo_route_v2",
    "update_at": 1774652666000
  }
}
```

## Delete JsWorker

调用者可以通过 `js-worker_delete` 删除脚本。

### 方法

调用方法名为 `js-worker_delete`，需要提供以下参数：

```json
{
  "token": "demo_token",  // 鉴权 Token
  "name": "demo_worker"   // 脚本唯一名称
}
```

删除成功后，脚本对应的 Runtime 实例会被立即驱逐。

### 权限要求

- Permission: `JsWorker::Delete`
- Scope: `JsWorker(name)`

### 返回值

```json
{
  "success": true,      // 是否成功
  "rows_affected": 1    // 影响的数据库行数
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_delete",
  "params": {
    "token": "demo_token",
    "name": "demo_worker"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "rows_affected": 1
  }
}
```

## Run JsWorker

调用者可以通过 `js-worker_run` 异步运行已注册脚本。

### 方法

调用方法名为 `js-worker_run`，需要提供以下参数：

```json
{
  "token": "demo_token",          // 鉴权 Token
  "js_script_name": "demo_worker", // 要运行的脚本名称
  "run_type": "call",             // 可选，call / inline_call / cron / route，默认 call
  "params": {                     // 必填，任意 JSON，传给脚本入口函数第一个参数
    "hello": "world"
  },
  "env": {                        // 可选，传入时使用请求里的 env；不传时使用数据库中该脚本保存的 env
    "override": true
  },
  "compile_mode": "bytecode"      // 可选，bytecode / source，默认 bytecode
}
```

参数说明：

- `js_script_name`：要运行的已注册脚本名称。
- `run_type`：可选，`call` / `inline_call` / `cron` / `route`，默认 `call`。
- `params`：必填，任意 JSON，传给脚本入口函数第一个参数。
- `env`：可选：
    - 传入时：使用请求里的 `env`
    - 不传时：使用数据库中该脚本保存的 `env`，若为空则使用 `{}`
- `compile_mode`：可选，执行模式：
    - `bytecode`：使用预编译的字节码执行（默认，性能更好）
    - `source`：使用原始源码实时编译执行（调试时使用，错误堆栈包含准确行号）

`run` 不会等待脚本执行结束，返回的 `id` 可用于后续查询执行结果。

**关于 `compile_mode`**：

- `bytecode` 模式：使用脚本创建时预编译的字节码，执行效率高，但错误堆栈可能不显示准确的源码行号
- `source` 模式：使用原始源码实时编译，执行效率略低，但错误堆栈会显示准确的源码行号（如 `photobed.js:23:5`），便于调试
- 其他调用方式（WebRoute、inline_call）始终使用 `bytecode` 模式

**关于 `run_type: "route"` 的注意事项**：

- 当使用 `onRoute` 处理函数时，`params` 需要传入序列化的 HTTP Request 对象，格式如下：
  ```json
  {
    "url": "https://example.com/worker-route/photobed/test.png",
    "method": "GET",
    "headers": [
      {"name": "User-Agent", "value": "Mozilla/5.0"}
    ],
    "body_bytes": []
  }
  ```
- 执行结果保存到数据库的是序列化的 HTTP Response 对象：
  ```json
  {
    "status": 200,
    "headers": [
      {"name": "content-type", "value": "image/png"}
    ],
    "body_bytes": [137, 80, 78, 71, ...]
  }
  ```

### 权限要求

- Permission: `JsWorker::RunDefinedJsWorker`
- Scope: `JsWorker(js_script_name)`

### 返回值

```json
{
  "id": 123 // js_result 表中的记录 ID
}
```

脚本执行结果请通过 `js-result_query` 查询。

### 完整示例

请求:

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

使用 source 模式（便于调试）：

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_run",
  "params": {
    "token": "demo_token",
    "js_script_name": "demo_worker",
    "run_type": "call",
    "compile_mode": "source",
    "params": {
      "hello": "world"
    }
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": 123
  }
}
```

## List All JsWorker

调用者可以通过 `js-worker_list_all_js_worker` 列出当前 Token 可见且真实存在于数据库中的脚本名。

### 方法

调用方法名为 `js-worker_list_all_js_worker`，需要提供以下参数：

```json
{
  "token": "demo_token" // 鉴权 Token
}
```

- SuperToken：可返回数据库中全部脚本。
- 普通 Token：仅返回同时满足以下条件的脚本：
    1. 数据库中存在
    2. Token 在该脚本名作用域下拥有 `JsWorker::ListAllJsWorker` 权限

### 权限要求

- Permission: `JsWorker::ListAllJsWorker`
- Scope: `JsWorker(name)`，支持后缀 `*` 通配

### 返回值

```json
[
  "demo_worker",
  "test_ping_worker"
]
```

### 完整示例

请求:

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

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": [
    "demo_worker",
    "test_ping_worker"
  ]
}
```

## Get Runtime Pool

调用者可以通过 `js-worker_get_rt_pool` 查看当前 JS Runtime 池状态。

### 方法

调用方法名为 `js-worker_get_rt_pool`，需要提供以下参数：

```json
{
  "token": "demo_token" // 鉴权 Token
}
```

仅需传入 `token`，无其他参数。

### 权限要求

- Permission: `NodeGet::GetRtPool`
- Scope: 建议在 `Global` 下授予

### 返回值

```json
{
  "total_workers": 2,          // 当前池中 Worker 总数
  "workers": [
    {
      "script_name": "demo_worker",    // 脚本名称
      "active_requests": 0,            // 当前活跃请求数
      "last_used_ms": 1774652000123,   // 最后使用时间戳（毫秒）
      "idle_ms": 4200,                 // 空闲时长（毫秒）
      "runtime_clean_time_ms": 60000   // 空闲清理阈值（毫秒）
    }
  ]
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "js-worker_get_rt_pool",
  "params": {
    "token": "demo_token"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "total_workers": 2,
    "workers": [
      {
        "script_name": "demo_worker",
        "active_requests": 0,
        "last_used_ms": 1774652000123,
        "idle_ms": 4200,
        "runtime_clean_time_ms": 60000
      }
    ]
  }
}
```
