mod app_session;
mod eve;
mod heimdall;
mod idunn_health;
mod mesh;
mod runtime;
mod world;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    runtime::run().await
}
