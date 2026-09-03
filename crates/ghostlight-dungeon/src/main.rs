mod app_session;
mod eve;
mod heimdall;
mod idunn_health;
mod mesh;
mod runtime;
mod world;

use anyhow::bail;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let state_root = match args.as_slice() {
        [flag, path] if flag == "--state-root" => Some(PathBuf::from(path)),
        [] => None,
        _ => bail!("usage: ghostlight-dungeon [--state-root PATH]"),
    };
    runtime::run(state_root).await
}
