use anyhow::{Context, Result};
use ghostlight_dungeon::mesh::MeshPublisher;
use uuid::Uuid;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let store_path = args
        .next()
        .context("usage: ghostlight-mesh-inspect <copied-mesh.cc> <campaign-id>")?;
    let campaign_id = args
        .next()
        .context("usage: ghostlight-mesh-inspect <copied-mesh.cc> <campaign-id>")?
        .parse::<Uuid>()
        .context("campaign-id must be a UUID")?;
    if args.next().is_some() {
        anyhow::bail!("usage: ghostlight-mesh-inspect <copied-mesh.cc> <campaign-id>");
    }

    let mesh = MeshPublisher::open(store_path, None)?;
    let surface = mesh
        .operator_surface(campaign_id)
        .context("copied mesh contains no operator projection for this campaign")?;
    println!("{}", serde_json::to_string_pretty(&surface)?);
    Ok(())
}
