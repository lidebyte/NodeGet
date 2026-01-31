# Agent Terminal 实现

实际上，官方 Agent 几乎全部源自 `GenshinMinecraft/komari-monitor-rs:src/callbacks/pty.rs`，Komari 提供的 WebShell 是较为成熟的方案，可以参考

## 处理内容

Agent 需要处理来自 用户(而不是 Server) 发送的心跳包、Resize 请求，以及最重要的数据

心跳包与 Resize 均通过 文本 类型发送

### 心跳包

```rust
struct HeartBeat {
    #[serde(rename = "type")]
    type_str: String,
    timestamp: String,
}
```

解析如下:

```json
{
    "type": "xx",
    "timestamp": 123
}
```

返回控制 (无数据) 即可

### Resize

Resize 用于调整终端大小

```rust
struct NeedResize {
    #[serde(rename = "type")]
    type_str: String,
    cols: u16,
    rows: u16,
}
```

解析如下:

```json
{
    "type": "xx",
    "cols": 114,
    "rows": 514
}
```

根据 `cols` 与 `rows` 通知 Pty 即可，返回控制 (无数据) 即可


### Binary

Binary 类型数据直接发送到终端，无需二次处理