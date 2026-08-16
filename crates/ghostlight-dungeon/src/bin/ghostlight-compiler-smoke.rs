#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live compiler smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use ghostlight_dungeon::{
        compiler::{CustomStart, WorldCompiler},
        model::{DeepSeekPort, ModelPort},
        vault::VoidBotMcpVault,
    };
    use std::{path::PathBuf, sync::Arc};

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
    let (preview, receipt) = compiler.compile_custom(CustomStart {
        campaign_name: "Compiler acceptance smoke".into(),
        who: "A low-status maintenance worker with local access but no institutional authority".into(),
        where_: "an obscure inhabited Aetheria location supported by the retrieved sources".into(),
        when: "a source-supported pre-Elysium period".into(),
        goal: "prevent an approaching institutional failure without inventing expertise or geography".into(),
    }).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":"ghostlight.world_compile_smoke.v1",
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
            "model_receipt":receipt
        }))?
    );
    Ok(())
}
