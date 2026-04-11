use tracing::info;

pub async fn run() {
    super::init_or_skip_super_token().await;
    info!(target: "server", "Initialization completed, exiting");
}
