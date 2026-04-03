# JS Runtime 外部注入能力

注入入口：`server/src/js_runtime/mod.rs` 的 `init_js_runtime_globals`。

## 我们自己实现的注入

- `globalThis.nodeget(rawJsonString)`
- `globalThis.inline_call(js_worker_name, params, timeout_sec?)`
- `globalThis.uuid()`（生成随机 UUID v4 字符串）
- `ctx.nodeget(rawJsonString)`（脚本入口第三参）
- `ctx.inline_call(js_worker_name, params, timeout_sec?)`
- `ctx.uuid()`（等价于全局 `uuid`）
- `ctx.runType`（脚本入口第三参）

## llrt_* 模块支持

- `llrt_fetch::init`
    - `fetch`、`Request`、`Response`、`Headers`、`FormData`
- `llrt_stream_web::init`
    - `ReadableStream`、`WritableStream`、`TransformStream`
- `llrt_url::init`
    - `URL`、`URLSearchParams`
- `llrt_timers::init`
    - `setTimeout`、`clearTimeout`、`setInterval`、`clearInterval`、`setImmediate`、`queueMicrotask`
