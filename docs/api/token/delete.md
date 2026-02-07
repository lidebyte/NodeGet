# 删除 Token

删除指定的令牌。

## 方法

调用方法名为 `token_delete`，需要提供以下参数:

```json
{
  "token": "demo_token",
  "target_token_key": "target_token_key_to_delete"
}
```

或删除自己的 Token：

```json
{
  "token": "demo_token"
}
```

## 权限要求

删除 Token 需要满足以下条件之一：

1. **SuperToken**: SuperToken 可以删除任何 Token，但必须提供 `target_token_key` 参数
2. **删除自己的 Token**: 任何 Token 都可以删除自己，无需特殊权限，只需不提供 `target_token_key` 参数或调用时不包含该参数