use crate::AGENT_CONFIG;
use log::trace;
use nodeget_lib::config::agent::IpProvider;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;

#[derive(Clone, Copy)]
enum IpFamily {
    Ipv4Only,
    Ipv6Only,
}

static CLIENT_V4: OnceCell<Client> = OnceCell::const_new();
static CLIENT_V6: OnceCell<Client> = OnceCell::const_new();
static RUSTLS_PROVIDER_INIT: OnceLock<()> = OnceLock::new();

#[derive(Debug)]
pub struct IPInfo {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
}

pub async fn ip() -> IPInfo {
    let provider = AGENT_CONFIG
        .get()
        .and_then(|lock| {
            lock.read()
                .ok()
                .and_then(|config| config.ip_provider.clone())
        })
        .unwrap_or(IpProvider::Cloudflare);

    match provider {
        IpProvider::Cloudflare => ip_cloudflare().await,
        IpProvider::IpInfo => ip_ipinfo().await,
    }
}

fn ensure_rustls_ring_provider() {
    let () = RUSTLS_PROVIDER_INIT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

async fn get_client(family: IpFamily) -> &'static Client {
    match family {
        IpFamily::Ipv4Only => {
            CLIENT_V4
                .get_or_init(|| async {
                    ensure_rustls_ring_provider();
                    Client::builder()
                        .timeout(Duration::from_secs(5))
                        .local_address(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED))
                        .build()
                        .expect("Failed to build IPv4 reqwest client")
                })
                .await
        }
        IpFamily::Ipv6Only => {
            CLIENT_V6
                .get_or_init(|| async {
                    ensure_rustls_ring_provider();
                    Client::builder()
                        .timeout(Duration::from_secs(5))
                        .local_address(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED))
                        .build()
                        .expect("Failed to build IPv6 reqwest client")
                })
                .await
        }
    }
}

// 通用 Get
async fn fetch_text(url: &str, family: IpFamily) -> Option<String> {
    let client = get_client(family).await;
    client
        .get(url)
        .header(USER_AGENT, "curl/8.7.1")
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()
}

// Parsers
fn parse_ipinfo_json(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("ip")?.as_str().map(ToOwned::to_owned)
}

fn parse_cloudflare_trace(body: &str) -> Option<String> {
    body.lines()
        .find(|line| line.starts_with("ip="))
        .map(|line| line.replace("ip=", ""))
}

// --- Providers ---

pub async fn ip_ipinfo() -> IPInfo {
    // IPv4 Task
    let ipv4: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
        let body = fetch_text("https://ipinfo.io", IpFamily::Ipv4Only).await?;
        let ip_str = parse_ipinfo_json(&body)?;
        Ipv4Addr::from_str(&ip_str).ok()
    });

    // IPv6 Task
    let ipv6: JoinHandle<Option<Ipv6Addr>> = tokio::spawn(async move {
        let body = fetch_text("https://6.ipinfo.io", IpFamily::Ipv6Only).await?;
        let ip_str = parse_ipinfo_json(&body)?;
        Ipv6Addr::from_str(&ip_str).ok()
    });

    let ip_info = IPInfo {
        ipv4: ipv4.await.unwrap_or(None),
        ipv6: ipv6.await.unwrap_or(None),
    };

    trace!("IP (ipinfo) retrieved: {ip_info:?}");
    ip_info
}

pub async fn ip_cloudflare() -> IPInfo {
    // IPv4 Task
    let ipv4: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
        let body = fetch_text(
            "https://www.cloudflare.com/cdn-cgi/trace",
            IpFamily::Ipv4Only,
        )
        .await?;
        let ip_str = parse_cloudflare_trace(&body)?;
        Ipv4Addr::from_str(&ip_str).ok()
    });

    // IPv6 Task
    let ipv6: JoinHandle<Option<Ipv6Addr>> = tokio::spawn(async move {
        let body = fetch_text(
            "https://www.cloudflare.com/cdn-cgi/trace",
            IpFamily::Ipv6Only,
        )
        .await?;
        let ip_str = parse_cloudflare_trace(&body)?;
        Ipv6Addr::from_str(&ip_str).ok()
    });

    let ip_info = IPInfo {
        ipv4: ipv4.await.unwrap_or(None),
        ipv6: ipv6.await.unwrap_or(None),
    };

    trace!("IP (cloudflare) retrieved: {ip_info:?}");
    ip_info
}
