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

## 权限要求

只有 **SuperToken** 可以删除 Token，且必须提供 `target_token_key` 参数指定要删除的目标 Token。

普通 Token 无法删除自己或其他 Token。

实际上，Target Token Key 并不需要 Secret 部分，不需要校验 Target Token 的权限