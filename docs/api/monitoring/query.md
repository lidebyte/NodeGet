# 调用者查询

调用者可以通过 `agent_query_static(dynamic)` 查询

需要传入 `token` / `static(dynamic)_data_query`:

```json
{
    "token": "demo_token",
    "static(dynamic)_data_query": {
        "fields": [
            // DataQueryField 结构体，该结构体参考 Monitoring 总览
            // 该字段为 Vec<_>，可指定多个
        ],
        "condition": [
            // QueryCondition 结构体，该结构体参考 Monitoring 总览
            // 该字段为 Vec<_>，可指定多个
        ]
    }
}
```

返回结构:

```json
[
    // Monitoring 回报结构体，该结构体参考 Monitoring 总览
    // 该字段为 Vec<_>，可指定多个
    // 只会存在 DataQueryField 结构中指定的数据字段
]
```