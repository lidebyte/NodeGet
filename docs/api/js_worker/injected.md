# JS Runtime 外部注入能力

注入入口：`server/src/js_runtime/mod.rs` 的 `init_js_runtime_globals`。

## 自定义注入

### 全局函数

- `globalThis.nodeget(json)` — 调用 NodeGet JSON-RPC API，支持以下调用方式：
    - `nodeget(json)` — 传入完整的 JSON-RPC 请求（string 或 object），返回解析后的 JS 对象
    - `nodeget(method, params)` — 快捷方式，自动构造 `{ jsonrpc: "2.0", method, params, id: randomUUID() }`
    - `nodeget(method, params, id)` — 同上，但指定请求 id
- `globalThis.inlineCall(js_worker_name, params, timeout_sec?)` — 调用其他 JS Worker。`timeout_sec` 为可选的软超时（秒，正有限数），最终生效超时取 `timeout_sec` 与目标 Worker `max_run_time` 中较小者；不传时仅受目标 Worker `max_run_time` 约束。
- `globalThis.randomUUID()` — 生成随机 UUID v4 字符串

### runtimeCtx（handler 第三参数）

脚本 handler 签名为 `handler(input, env, runtimeCtx)`，其中 `runtimeCtx` 包含以下属性：

- `runtimeCtx.runType` — 当前运行类型字符串：`"onCall"` / `"onCron"` / `"onRoute"` / `"onInlineCall"`
- `runtimeCtx.workerName` — 当前 Worker 的名字
- `runtimeCtx.inlineCall(js_worker_name, params, timeout_sec?)` — 等价于 `globalThis.inlineCall`（软超时与目标 Worker `max_run_time` 取较小者）
- `runtimeCtx.inlineCaller` — 调用当前脚本的调用者脚本名；顶层调用时为 `null`

## llrt_* 模块支持

- `llrt_fetch::init`
    - `fetch`、`Request`、`Response`、`Headers`、`FormData`
- `llrt_buffer::init`
    - `Buffer`、`Blob`、`File`、`atob`、`btoa`
- `llrt_stream_web::init`
    - `ReadableStream`、`WritableStream`、`TransformStream`
- `llrt_url::init`
    - `URL`、`URLSearchParams`
- `llrt_util::init`
    - `TextEncoder`、`TextDecoder`
- `llrt_timers::init`
    - `setTimeout`、`clearTimeout`、`setInterval`、`clearInterval`、`setImmediate`、`queueMicrotask`
