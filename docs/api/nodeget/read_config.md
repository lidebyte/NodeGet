# nodeget-server_read_config

读取当前 Server 使用的配置文件原文（`config.toml` 文本）。

## 方法

调用方法名为 `nodeget-server_read_config`，需要提供参数：

```json
{
  "token": "SUPER_TOKEN_KEY:SUPER_TOKEN_SECRET"
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
  "method": "nodeget-server_read_config",
  "params": {
    "token": "SUPER_TOKEN_KEY:SUPER_TOKEN_SECRET"
  },
  "id": 1
}
```

## 响应示例

```json
{
  "jsonrpc": "2.0",
  "result": "log_level = \"info\"\\nws_listener = \"0.0.0.0:6000\"\\n...",
  "id": 1
}
```

## 说明

返回值是配置文件在磁盘上的原始文本内容。
