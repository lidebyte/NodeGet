use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use nodeget_lib::error::NodegetError;
use nodeget_lib::task::{HttpRequestTask, HttpRequestTaskResult};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;

pub type Result<T> = anyhow::Result<T>;

static RUSTLS_PROVIDER_INIT: OnceLock<()> = OnceLock::new();
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn ensure_rustls_ring_provider() {
    let () = RUSTLS_PROVIDER_INIT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn parse_http_method(method: &str) -> Result<Method> {
    let method = method.trim();
    if method.is_empty() {
        return Err(
            NodegetError::InvalidInput("http_request.method cannot be empty".to_owned()).into(),
        );
    }

    Method::from_bytes(method.to_ascii_uppercase().as_bytes()).map_err(|e| {
        NodegetError::InvalidInput(format!("Invalid http_request.method '{method}': {e}")).into()
    })
}

fn parse_bind_ip(ip: Option<&str>) -> Result<Option<IpAddr>> {
    let Some(ip_raw) = ip.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };

    let lower = ip_raw.to_ascii_lowercase();
    if lower == "ipv4 auto" {
        return Ok(Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    }
    if lower == "ipv6 auto" {
        return Ok(Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    }

    IpAddr::from_str(ip_raw).map(Some).map_err(|e| {
        NodegetError::InvalidInput(format!(
            "Invalid http_request.ip '{ip_raw}', expected IP literal or 'ipv4 auto'/'ipv6 auto': {e}"
        ))
            .into()
    })
}

fn decode_request_body(task: &HttpRequestTask) -> Result<Option<Vec<u8>>> {
    match (&task.body, &task.body_base64) {
        (Some(_), Some(_)) => Err(NodegetError::InvalidInput(
            "http_request.body and http_request.body_base64 are mutually exclusive".to_owned(),
        )
        .into()),
        (Some(body), None) => Ok(Some(body.as_bytes().to_vec())),
        (None, Some(body_base64)) => BASE64_STANDARD.decode(body_base64).map(Some).map_err(|e| {
            NodegetError::InvalidInput(format!("Invalid http_request.body_base64: {e}")).into()
        }),
        (None, None) => Ok(None),
    }
}

fn build_request_headers(headers: &BTreeMap<String, String>) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::new();

    for (key, value) in headers {
        let normalized_key = key.trim().to_ascii_lowercase();
        let header_name = HeaderName::from_str(&normalized_key).map_err(|e| {
            NodegetError::InvalidInput(format!("Invalid http_request header name '{key}': {e}"))
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|e| {
            NodegetError::InvalidInput(format!(
                "Invalid http_request header value for '{key}': {e}"
            ))
        })?;
        header_map.append(header_name, header_value);
    }

    Ok(header_map)
}

fn decode_response_body(bytes: &[u8]) -> (Option<String>, Option<String>) {
    std::str::from_utf8(bytes).map_or_else(
        |_| (None, Some(BASE64_STANDARD.encode(bytes))),
        |text| (Some(text.to_owned()), None),
    )
}

pub async fn execute_http_request(task: HttpRequestTask) -> Result<HttpRequestTaskResult> {
    ensure_rustls_ring_provider();

    let bind_ip = parse_bind_ip(task.ip.as_deref())?;
    let method = parse_http_method(&task.method)?;
    let request_body = decode_request_body(&task)?;
    let header_map = build_request_headers(&task.headers)?;

    let mut client_builder = Client::builder().timeout(HTTP_REQUEST_TIMEOUT);
    if let Some(local_ip) = bind_ip {
        client_builder = client_builder.local_address(local_ip);
    }
    let client = client_builder
        .build()
        .map_err(|e| NodegetError::Other(format!("Failed to build HTTP client: {e}")))?;

    let mut request_builder = client.request(method, task.url);
    if !header_map.is_empty() {
        request_builder = request_builder.headers(header_map);
    }
    if let Some(body) = request_body {
        request_builder = request_builder.body(body);
    }

    let response = request_builder
        .send()
        .await
        .map_err(|e| NodegetError::Other(format!("HTTP request failed: {e}")))?;

    let status = response.status().as_u16();

    let mut headers = Vec::new();
    for (name, value) in response.headers() {
        let value_string = value.to_str().map_or_else(
            |_| BASE64_STANDARD.encode(value.as_bytes()),
            ToOwned::to_owned,
        );
        let mut one = BTreeMap::new();
        one.insert(name.as_str().to_owned(), value_string);
        headers.push(one);
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| NodegetError::Other(format!("Failed to read HTTP response body: {e}")))?;
    let (body, body_base64) = decode_response_body(&body_bytes);

    Ok(HttpRequestTaskResult {
        status,
        headers,
        body,
        body_base64,
    })
}
