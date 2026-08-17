#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live compiler smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        compiler::{CustomStart, WorldCompiler},
        model::{DeepSeekPort, ModelPort},
        vault::VoidBotMcpVault,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_machine_dpapi(secret)?);
    let compiler = WorldCompiler::new(
        Arc::new(VoidBotMcpVault::starfire_loopback()),
        model,
        "deepseek-v4-flash",
        "deepseek-v4-pro",
    );
    let root = PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance")
        .join(format!("compiler-{}", Utc::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir_all(&root)?;
    let started = Instant::now();
    let (preview, model_receipts) = compiler.compile_custom(CustomStart {
        campaign_name: "Compiler acceptance smoke".into(),
        who: "A low-status maintenance worker with local access but no institutional authority".into(),
        where_: "an obscure inhabited Aetheria location supported by the retrieved sources".into(),
        when: "a source-supported pre-Elysium period".into(),
        goal: "prevent an approaching institutional failure without inventing expertise or geography".into(),
    }).await?;
    let result = serde_json::json!({
            "schema":"ghostlight.world_compile_smoke.v1",
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "title":preview.title,
            "location_count":preview.campaign.locations.len(),
            "actor_count":preview.campaign.actors.len(),
            "institution_count":preview.campaign.institutions.len(),
            "clock_count":preview.campaign.clocks.len(),
            "evidence_receipt_count":preview.evidence_receipts.len(),
            "evidence_witness_count":preview.evidence_receipts.iter().map(|r|r.witnesses.len()).sum::<usize>(),
            "gaps":preview.gaps,
            "branch_assumptions":preview.branch_assumptions,
            "requires_approval":preview.requires_approval,
            "campaign_revision":preview.campaign.revision,
            "preview":preview,
            "model_receipts":model_receipts,
            "result_path":root.join("result.json")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
