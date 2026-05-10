# Static 静态文件服务总览

NodeGet Server 内置了一个轻量级的静态文件服务，可用于托管前端网站、静态资源或任意文件。

## 方法列表

| 方法名                                         | 描述                    |
|---------------------------------------------|-----------------------|
| [static_create](./crud.md#create)           | 创建静态服务配置              |
| [static_read](./crud.md#read)               | 读取静态服务配置              |
| [static_update](./crud.md#update)           | 更新静态服务配置              |
| [static_delete](./crud.md#delete)           | 删除静态服务配置              |
| [static_upload_file](./crud.md#upload-file) | 上传文件到静态目录             |
| [static_read_file](./crud.md#read-file)     | 读取静态目录中的文件（返回 base64） |
| [static_delete_file](./crud.md#delete-file) | 删除静态目录中的文件            |
| [static_rename_file](./crud.md#rename-file) | 在静态目录内移动/重命名文件        |
| [static_list](./crud.md#list)               | 列出所有静态服务名称（仅 `SuperToken`） |
| [static_list_file](./crud.md#list-file)     | 列出指定静态目录下的所有文件（含体积与修改时间） |

## HTTP 路由

静态文件可通过以下 HTTP 路由直接访问：

```
/nodeget/static/{name}/{*file_path}
```

- `name`：创建静态服务时指定的名称（RPC / URL 标识符，不参与磁盘路径）
- `file_path`：相对该静态服务磁盘根目录（`{static_path}/{path}/`）的文件路径；空路径默认返回 `index.html`

::: info name 与 path 的区别

- **`static_path`**（全局配置，`config.toml`）：所有静态站点的公共根目录，默认 `./static/`
- **`path`**（每条 static 记录的字段）：相对 `static_path` 的子目录，例如 `"hugo"` 或 `"sites/blog-2026"`
- **`name`**（每条 static 记录的字段）：RPC 参数和 URL 里使用的标识符，与磁盘无关

最终磁盘位置 = `{static_path}/{path}/...`，URL 是 `/nodeget/static/{name}/...`，二者解耦。这意味着你可以给同一个磁盘目录起多个不同的
`name`（虽然唯一约束在 `name` 上，但你可以用新 `name` 指向旧 `path` 来做蓝绿切换）。
:::

示例：若配置 `static_path = "./static/"`，某条记录 `name = "my-site"`、`path = "sites/blog"`，则：

- 磁盘文件：`./static/sites/blog/index.html`
- URL：`/nodeget/static/my-site/index.html`

如果某条静态配置的 `is_http_root` 为 `true`，则服务器的根路由 `/`（以及所有未匹配路由）会直接代理到该静态目录，**但不会覆盖
WebSocket、`/nodeget/*`、`/terminal` 等精确路由**。此时原有的默认占位 HTML 不再展示。

`is_http_root` 在同一时刻只能有一条配置为 `true`，由数据库层 partial unique index（SQLite / Postgres）强制保证。

## 配置项

在 `config.toml` 中可配置：

```toml
ws_listener = "0.0.0.0:3000"

# 所有 static 站点的公共根目录，默认 `./static/`
# static_path = "./static/"
```

`static_path` 指定了所有静态文件的磁盘总根目录。每条静态服务记录通过它自己的 `path` 字段决定相对于 `static_path` 的子目录。

## 基本结构体

数据库中的结构体定义如下：

```rust
pub struct Static {
    id: i64,              // 自增主键
    name: String,         // 静态服务名称，RPC/URL 标识符，全局唯一
    path: String,         // 相对 `static_path` 的子目录（真正的磁盘路径）
    is_http_root: bool,   // 是否接管根路由
    cors: bool,           // 是否开启跨域（Access-Control-Allow-Origin: *）
}
```

## 基本权限

```rust
pub enum Scope {
    Global,
    AgentUuid(uuid::Uuid),
    KvNamespace(String),
    JsWorker(String),
    StaticFile(String), // 静态文件服务作用域，通过名称指定
}
```

```rust
pub enum Permission {
    // ...
    StaticFile(StaticFile),
}
```

```rust
pub enum StaticFile {
    Read,   // 读取配置与文件
    Write,  // 创建/更新配置、上传文件
    Delete, // 删除配置与文件
}
```

### 注意事项

- `static_read` 与 `static_read_file` 需要 `StaticFile::Read` 权限
- `static_list_file` 需要 `StaticFile::Read` 权限
- `static_list` 仅 **SuperToken** 可调用
- `static_create`、`static_update`、`static_upload_file` 需要 `StaticFile::Write` 权限
- `static_rename_file` 需要同时持有 `StaticFile::Write` 和 `StaticFile::Delete` 权限
- `static_delete`、`static_delete_file` 需要 `StaticFile::Delete` 权限
- Scope 支持 `StaticFile(name)` 或 `Global`

## 权限 Demo Json

```json
{
  "scopes": [
    {
      "static_file": "my-site"
    }
  ],
  "permissions": [
    {
      "static_file": "read"
    },
    {
      "static_file": "write"
    },
    {
      "static_file": "delete"
    }
  ]
}
```

该权限表示在 `my-site` 静态服务下，拥有读写删的完整权限。

## 安全说明

- `name` 只允许 `[A-Za-z0-9_.-]`，最长 128 字符，且不能是 `.` 或 `..` 等纯点组合
- `path` 字段按组件校验：每段允许 `[A-Za-z0-9_.-]`，允许 `/` 分隔多级子目录（如 `"sites/blog"`），长度 ≤ 512；不允许绝对路径、
  `..` 穿透、反斜杠、Windows 盘符前缀
- 所有文件操作均经过严格的目录遍历防护，无法访问 `{static_path}/{path}/` 以外的目录
- 文件 `path` 参数（上传/读取/删除的那个）不允许使用绝对路径或 `..` 向上穿透
- `is_http_root` 由数据库层 partial unique index 强制唯一（SQLite / Postgres），MySQL 退化到应用层检查
- 静态 HTTP 服务仅响应 `GET` / `HEAD`（以及启用 CORS 时的 `OPTIONS` 预检）；其它方法返回 `405 Method Not Allowed`
- 静态文件路由不影响 WebSocket（`terminal`）和 JSON-RPC 的正常工作
