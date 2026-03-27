use rquickjs::prelude::{Async, Func};
use rquickjs::{AsyncContext, AsyncRuntime, Error, Promise, Value as JsValue};
use serde_json::Value;

pub mod nodeget;

pub fn js_error(stage: &'static str, message: impl ToString) -> Error {
    Error::new_from_js_message(stage, "String", message.to_string())
}

pub async fn js_runner(js_code: impl ToString) -> Result<Value, Error> {
    let rt = AsyncRuntime::new()?;
    let ctx = AsyncContext::full(&rt).await?;
    let js_code = js_code.to_string();

    let js_result: Result<Value, Error> = rquickjs::async_with!(ctx => |ctx| {
        let global = ctx.globals();
        global.set("nodeget", Func::from(Async(nodeget::js_nodeget)))?;

        let promise: Promise<'_> = ctx.eval_promise(js_code)?;
        let js_value: JsValue<'_> = promise.into_future::<JsValue<'_>>().await?;

        let raw_json = if let Some(js_string) = js_value.as_string() {
            js_string.to_string()?
        } else {
            let js_json_string = ctx
                .json_stringify(js_value)?
                .ok_or_else(|| {
                    js_error(
                        "json_parse",
                        "Script return is not JSON-serializable (got undefined/function/symbol)",
                    )
                })?;
            js_json_string.to_string()?
        };

        serde_json::from_str(&raw_json).map_err(|e| {
            js_error(
                "json_parse",
                format!("Script return is not valid JSON: {e}"),
            )
        })
    })
    .await;

    rt.idle().await;
    js_result
}
