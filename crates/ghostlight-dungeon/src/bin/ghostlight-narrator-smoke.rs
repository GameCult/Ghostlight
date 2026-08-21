#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live narrator smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::Campaign,
        model::{DeepSeekPort, ModelPort},
        narrator::Narrator,
        persistence::CampaignStore,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let store_path = std::env::var_os("GHOSTLIGHT_NARRATOR_STORE")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("GHOSTLIGHT_NARRATOR_STORE is required"))?;
    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let store = CampaignStore::open(&store_path)?;
    let campaign_id = store
        .keys("campaign.v1")?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("narrator smoke store has no campaign"))?;
    let campaign = store
        .load::<Campaign>("campaign.v1", &campaign_id)?
        .map(|(_, value)| value)
        .ok_or_else(|| anyhow::anyhow!("campaign disappeared"))?;
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_runtime_secret(secret)?);
    let narrator = Narrator {
        model,
        model_name: "deepseek-v4-pro".into(),
    };
    let started = Instant::now();
    let (projection, receipt) = narrator.project(&store, &campaign).await?;
    store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        receipt.storage_key(),
        &receipt,
    )?;
    let result_path = store_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(format!(
            "narrator-repeat-{}.json",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
    let result = serde_json::json!({
        "schema":"ghostlight.narrator_smoke.v1",
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "projection":projection,
        "model_stage_receipt":receipt,
        "campaign_revision":campaign.revision,
        "result_path":result_path
    });
    std::fs::write(&result_path, serde_json::to_vec_pretty(&result)?)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
