use ghostlight_dungeon::{agency_corpus::seed_agency_attempt_cases, persistence::CampaignStore};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/ghostlight-agency-corpus.cc"));
    if path.exists() {
        anyhow::bail!(
            "agency corpus target already exists; choose a fresh path: {}",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = CampaignStore::open(&path)?;
    let cases = seed_agency_attempt_cases();
    for case in &cases {
        store.insert(
            "agency_attempt_case.v1",
            "ghostlight.agency_attempt_case.v1",
            &case.id,
            case,
        )?;
    }
    println!(
        "materialized {} typed agency cases at {}",
        cases.len(),
        path.display()
    );
    Ok(())
}
