use crate::rpc::RpcHelper;
use crate::rpc::{rpc_exec, token_identity};
use jsonrpsee::core::RpcResult;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use serde_json::value::RawValue;
use tracing::Instrument;

mod auth;
mod create;
mod delete;
mod delete_file;
mod list;
mod read;
mod read_file;
mod update;
mod upload_file;

#[rpc(server, namespace = "static")]
pub trait Rpc {
    #[method(name = "create")]
    async fn create(
        &self,
        token: String,
        name: String,
        path: String,
        is_http_root: bool,
        cors: bool,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "read")]
    async fn read(&self, token: String, name: String) -> RpcResult<Box<RawValue>>;

    #[method(name = "update")]
    async fn update(
        &self,
        token: String,
        name: String,
        path: String,
        is_http_root: bool,
        cors: bool,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "delete")]
    async fn delete(&self, token: String, name: String) -> RpcResult<Box<RawValue>>;

    #[method(name = "upload_file")]
    async fn upload_file(
        &self,
        token: String,
        name: String,
        path: String,
        body: Option<Vec<u8>>,
        base64: Option<String>,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "read_file")]
    async fn read_file(
        &self,
        token: String,
        name: String,
        path: String,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "delete_file")]
    async fn delete_file(
        &self,
        token: String,
        name: String,
        path: String,
    ) -> RpcResult<Box<RawValue>>;

    #[method(name = "list")]
    async fn list(&self, token: String, name: String) -> RpcResult<Box<RawValue>>;
}

pub struct StaticFileRpcImpl;

impl RpcHelper for StaticFileRpcImpl {}

#[async_trait]
impl RpcServer for StaticFileRpcImpl {
    async fn create(
        &self,
        token: String,
        name: String,
        path: String,
        is_http_root: bool,
        cors: bool,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::create", token_key = tk, username = un, name = %name, path = %path, is_http_root = is_http_root, cors = cors);
        async { rpc_exec!(create::create(token, name, path, is_http_root, cors).await) }
            .instrument(span)
            .await
    }

    async fn read(&self, token: String, name: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::read", token_key = tk, username = un, name = %name);
        async { rpc_exec!(read::read(token, name).await) }
            .instrument(span)
            .await
    }

    async fn update(
        &self,
        token: String,
        name: String,
        path: String,
        is_http_root: bool,
        cors: bool,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::update", token_key = tk, username = un, name = %name, path = %path, is_http_root = is_http_root, cors = cors);
        async { rpc_exec!(update::update(token, name, path, is_http_root, cors).await) }
            .instrument(span)
            .await
    }

    async fn delete(&self, token: String, name: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::delete", token_key = tk, username = un, name = %name);
        async { rpc_exec!(delete::delete(token, name).await) }
            .instrument(span)
            .await
    }

    async fn upload_file(
        &self,
        token: String,
        name: String,
        path: String,
        body: Option<Vec<u8>>,
        base64: Option<String>,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::upload_file", token_key = tk, username = un, name = %name, path = %path, has_body = body.is_some(), has_base64 = base64.is_some());
        async { rpc_exec!(upload_file::upload_file_rpc(token, name, path, body, base64).await) }
            .instrument(span)
            .await
    }

    async fn read_file(
        &self,
        token: String,
        name: String,
        path: String,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::read_file", token_key = tk, username = un, name = %name, path = %path);
        async { rpc_exec!(read_file::read_file_rpc(token, name, path).await) }
            .instrument(span)
            .await
    }

    async fn delete_file(
        &self,
        token: String,
        name: String,
        path: String,
    ) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::delete_file", token_key = tk, username = un, name = %name, path = %path);
        async { rpc_exec!(delete_file::delete_file_rpc(token, name, path).await) }
            .instrument(span)
            .await
    }

    async fn list(&self, token: String, name: String) -> RpcResult<Box<RawValue>> {
        let (tk, un) = token_identity(&token);
        let span = tracing::info_span!(target: "static", "static::list", token_key = tk, username = un, name = %name);
        async { rpc_exec!(list::list_rpc(token, name).await) }
            .instrument(span)
            .await
    }
}
