# nodeget-server_edit_config

写入新的 Server 配置文本，并触发服务端配置热重载。

## 方法

调用方法名为 `nodeget-server_edit_config`，需要提供参数：

```json
{
  "token": "SUPER_TOKEN_KEY:SUPER_TOKEN_SECRET",
  "config_string": "log_level = \"info\"\\nws_listener = \"0.0.0.0:6000\"\\n..."
}
```

## 权限要求

该方法仅允许 **SuperToken** 调用。

`token` 支持以下格式之一：

- `token_key:token_secret`
- `username|password`

## 请求示例

```json
{
  "jsonrpc": "2.0",
  "method": "nodeget-server_edit_config",
  "params": {
    "token": "SUPER_TOKEN_KEY:SUPER_TOKEN_SECRET",
    "config_string": "log_level = \"info\"\\nserver_uuid = \"auto_gen\"\\nws_listener = \"0.0.0.0:6000\"\\njsonrpc_max_connections = 100\\n\\n[database]\\ndatabase_url = \"sqlite://data/server.db?mode=rwc\""
  },
  "id": 1
}
```

## 响应示例

```json
{
  "jsonrpc": "2.0",
  "result": true,
  "id": 1
}
```

## 行为说明

- 服务端会先校验 `config_string` 是否是可解析的 Server TOML 配置。
- 校验通过后写入配置文件。
- 写入成功后触发配置重载流程。
