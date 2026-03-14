# 批量读取键值

按请求数组批量读取多个 Namespace 下的多个 Key。

## 方法

调用方法名为 `kv_get_multi_value`，需要提供以下参数：

```json
{
  "token": "demo_token",
  "namespace_key": [
    {
      "namespace": "ns1",
      "key": "key1"
    },
    {
      "namespace": "ns1",
      "key": "metadata_*"
    },
    {
      "namespace": "ns2",
      "key": "key2"
    }
  ]
}
```

## 参数规则

- `namespace_key` 为数组，可传任意数量项
- 每一项格式固定为 `{namespace, key}`
- `key` 支持后缀通配符 `*`，例如 `metadata_*`
- 仅支持后缀通配符（`*` 只能出现一次且必须在末尾）

## 权限要求

- 每一项都必须在其 `KvNamespace(namespace)` Scope 下具备对应 `Kv::Read(key)` 权限覆盖
- 只要有一项无权限，整体直接返回权限错误，不返回部分结果

## 返回结果

返回数组结构：

```json
[
  {
    "namespace": "ns1",
    "key": "key1",
    "value": "value1"
  },
  {
    "namespace": "ns1",
    "key": "metadata_cpu",
    "value": 10
  },
  {
    "namespace": "ns1",
    "key": "metadata_ram",
    "value": 20
  }
]
```

说明：

- 返回顺序按请求顺序拼接
- 对于通配符项，命中的 key 按字典序返回
- 对于精确 key，若不存在会返回 `value: null`
