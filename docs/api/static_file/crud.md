# Static CRUD 与文件操作

## Create

创建一条静态文件服务配置，同时在磁盘上初始化对应目录。

### 方法

调用方法名为 `static_create`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site",
  "path": "/",
  "is_http_root": false,
  "cors": true
}
```

### 权限要求

- Permission: `StaticFile::Write`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

创建成功时返回完整的静态配置对象：

```json
{
  "id": 1,
  "name": "my-site",
  "path": "/",
  "is_http_root": false,
  "cors": true
}
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_create",
  "params": {
    "token": "demo_token",
    "name": "my-site",
    "path": "/",
    "is_http_root": false,
    "cors": true
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
    "name": "my-site",
    "path": "/",
    "is_http_root": false,
    "cors": true
  }
}
```

## Read

读取指定名称的静态服务配置。直接从内存缓存返回，性能极高。

### 方法

调用方法名为 `static_read`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site"
}
```

### 权限要求

- Permission: `StaticFile::Read`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

返回配置对象；若不存在返回 `null`。

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_read",
  "params": {
    "token": "demo_token",
    "name": "my-site"
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
    "name": "my-site",
    "path": "/",
    "is_http_root": false,
    "cors": true
  }
}
```

## Update

更新现有静态服务配置。

### 方法

调用方法名为 `static_update`，参数与 `static_create` 相同。

### 权限要求

- Permission: `StaticFile::Write`
- Scope: `StaticFile(name)` 或 `Global`

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_update",
  "params": {
    "token": "demo_token",
    "name": "my-site",
    "path": "/",
    "is_http_root": true,
    "cors": true
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
    "name": "my-site",
    "path": "/",
    "is_http_root": true,
    "cors": true
  }
}
```

## Delete

删除指定名称的静态服务配置。**不会删除磁盘上的文件**。

### 方法

调用方法名为 `static_delete`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site"
}
```

### 权限要求

- Permission: `StaticFile::Delete`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

删除成功返回 `{"success": true}`。

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_delete",
  "params": {
    "token": "demo_token",
    "name": "my-site"
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
    "success": true
  }
}
```

## Upload File

上传文件到指定静态服务的目录下。

### 方法

调用方法名为 `static_upload_file`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site",
  "path": "/css/style.css",
  "body": [
    /* 文件二进制内容 */
  ]
}
```

或

```json
{
  "token": "demo_token",
  "name": "my-site",
  "path": "/css/style.css",
  "base64": "Ym9keSB7IGNvbG9yOiByZWQ7IH0="
}
```

### 参数说明

- `name`：静态服务名称
- `path`：文件在 `{static_path}/{name}/` 下的相对路径（如 `/index.html`）
- `body` 与 `base64` **只能二选一**，同时提供会报错
- 文件会自动覆盖原有内容
- 父目录不存在时会自动创建

### 权限要求

- Permission: `StaticFile::Write`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

上传成功返回 `{"success": true}`。

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_upload_file",
  "params": {
    "token": "demo_token",
    "name": "my-site",
    "path": "/index.html",
    "base64": "PCFET0NUWVBFIGh0bWw+PGh0bWw+PGJvZHk+SGVsbG88L2JvZHk+PC9odG1sPg=="
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
    "success": true
  }
}
```

## Read File

读取指定静态目录下的文件内容，以 **base64** 编码返回。

### 方法

调用方法名为 `static_read_file`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site",
  "path": "/index.html"
}
```

### 权限要求

- Permission: `StaticFile::Read`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

返回 base64 编码的字符串。

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_read_file",
  "params": {
    "token": "demo_token",
    "name": "my-site",
    "path": "/index.html"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "PCFET0NUWVBFIGh0bWw+..."
}
```

## Delete File

删除指定静态目录下的文件。

### 方法

调用方法名为 `static_delete_file`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "name": "my-site",
  "path": "/index.html"
}
```

### 权限要求

- Permission: `StaticFile::Delete`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

删除成功返回 `{"success": true}`。文件不存在时同样返回成功（幂等）。

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_delete_file",
  "params": {
    "token": "demo_token",
    "name": "my-site",
    "path": "/index.html"
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
    "success": true
  }
}
```

## List

列出所有已创建的静态服务名称。

### 方法

调用方法名为 `static_list`，需要提供以下参数：

```json
{
  "token": "demo_super_token" // SuperToken
}
```

### 权限要求

只有 **SuperToken** 可以调用该方法。

普通 Token 会返回权限错误。

数据来源为内存缓存，不会访问数据库或磁盘。

### 返回值

返回所有静态服务 `name` 字段组成的数组，**按字典序排序**：

```json
["api-docs", "blog", "my-site"]
```

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_list",
  "params": {
    "token": "demo_super_token"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": ["api-docs", "blog", "my-site"]
}
```

## List File

列出某个静态目录下的所有相对文件路径。

### 方法

调用方法名为 `static_list_file`，需要提供以下参数:

```json
{
  "token": "demo_token",
  "name": "my-site"
}
```

### 权限要求

- Permission: `StaticFile::Read`
- Scope: `StaticFile(name)` 或 `Global`

### 返回值

返回该静态目录下所有文件的相对路径数组，**按字典序排序**，统一使用 `/` 作为分隔符（跨平台一致）：

```json
["404.html", "docs/1.md", "docs/guide/intro.md", "index.html"]
```

注意事项：

- 只列出**文件**，不包括目录本身
- 软链接不会被跟随，避免越权访问 `{static_path}/{path}/` 外部内容
- 磁盘目录不存在（例如刚 `static_create` 但还没上传）时返回空数组 `[]`
- 路径中包含非 UTF-8 分段的文件会被跳过（仅日志告警）

### 完整示例

请求:

```json
{
  "jsonrpc": "2.0",
  "method": "static_list_file",
  "params": {
    "token": "demo_token",
    "name": "my-site"
  },
  "id": 1
}
```

响应:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": ["404.html", "docs/1.md", "index.html"]
}
```

## HTTP 访问静态文件

### 默认路由

```
GET /nodeget/static/{name}/{*path}
```

访问示例：

```bash
curl http://localhost:3000/nodeget/static/my-site/index.html
```

当 `cors: true` 时，响应头会携带：

```
Access-Control-Allow-Origin: *
```

同时支持 `OPTIONS` 预检请求，返回 `204 No Content` 和对应的 CORS 头。

### 根路由代理（is_http_root）

当某条静态配置的 `is_http_root` 为 `true` 时：

- 服务器的根路由 `/` 和所有未匹配路由会直接返回该静态目录下的文件
- `WebSocket`（`/terminal`）和 `JSON-RPC` 不受影响
- 未绑定 `is_http_root` 时，根路由继续返回默认的占位 HTML

示例：

```bash
curl http://localhost:3000/index.html
# 实际返回磁盘上 {static_path}/{path}/index.html 的内容（path 为该 static 记录里的 path 字段）
```

`is_http_root` 在同一时刻只能存在一个。尝试创建或更新第二条 `is_http_root` 为 `true` 的配置会返回错误（数据库层 partial
unique index 强制保证）。

## 缓存说明

静态服务配置表会在程序启动时**全量加载到内存**。所有 `static_read` 和 HTTP 路由直接读取内存缓存，无需访问数据库。

`static_create`、`static_update`、`static_delete` 会在写库成功后自动重新加载内存缓存，确保一致性。

## 注意事项

- `name` 只作为 RPC 参数 / URL 标识符，不会拼接到磁盘路径
- `path` 字段才是决定磁盘位置的关键，实际磁盘根 = `{static_path(config)}/{path}`
- `path` 允许 `/` 分隔多级子目录（如 `"sites/blog"`），每段必须符合 `[A-Za-z0-9_.-]`
- `body` 与 `base64` 不能同时出现，否则返回 `InvalidInput`
- 文件 `path` 参数严格禁止目录遍历（`..` 和绝对路径会被拒绝）
- 未配置 `static_path` 时，默认磁盘根目录为 `./static/`
