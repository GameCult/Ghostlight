#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live outcome-verifier smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
struct ObservedModel {
    inner: ghostlight_dungeon::model::DeepSeekPort,
    calls: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

#[cfg(windows)]
#[async_trait::async_trait]
impl ghostlight_dungeon::model::ModelPort for ObservedModel {
    async fn run(
        &self,
        request: &ghostlight_dungeon::model::ModelStageRequest,
    ) -> anyhow::Result<String> {
        Ok(self.run_observed(request).await?.content)
    }

    async fn run_observed(
        &self,
        request: &ghostlight_dungeon::model::ModelStageRequest,
    ) -> anyhow::Result<ghostlight_dungeon::model::ModelProviderOutput> {
        let started = std::time::Instant::now();
        let result = self.inner.run_observed(request).await;
        let record = match &result {
            Ok(output) => serde_json::json!({
                "stage": request.stage,
                "model": request.model,
                "snapshot_binding": request.snapshot_binding,
                "input": request.lived_stream,
                "output": output.content,
                "provider_request_id": output.provider_request_id,
                "system_fingerprint": output.system_fingerprint,
                "finish_reason": output.finish_reason,
                "token_usage": output.token_usage,
                "latency_ms": started.elapsed().as_millis(),
            }),
            Err(error) => serde_json::json!({
                "stage": request.stage,
                "model": request.model,
                "snapshot_binding": request.snapshot_binding,
                "input": request.lived_stream,
                "error": error.to_string(),
                "latency_ms": started.elapsed().as_millis(),
            }),
        };
        self.calls.lock().expect("outcome trace lock").push(record);
        result
    }

    fn provider(&self) -> &'static str {
        self.inner.provider()
    }
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use ghostlight_dungeon::{
        domain::{
            Campaign, CellActionProposal, StrategicActivityKind, StrategicCellEffect,
            StrategicOutcomeEffect,
        },
        model::{DeepSeekPort, ModelPort},
        outcome::resolve_activity_outcomes,
        resolution::subject_state_references,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let baseline = std::env::var_os("GHOSTLIGHT_OUTCOME_BASELINE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                r"F:\GameCult\GhostlightDungeon\acceptance\outcome-risk-verifier-20260818\result.json",
            )
        });
    let result_root = std::env::var_os("GHOSTLIGHT_OUTCOME_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "forced-high-risk-outcome-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ))
        });
    std::fs::create_dir_all(&result_root)?;
    let baseline_bytes = std::fs::read(&baseline)?;
    let baseline_json: serde_json::Value = serde_json::from_slice(&baseline_bytes)?;
    let campaign: Campaign = serde_json::from_value(
        baseline_json
            .pointer("/commit/campaign")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("baseline result lacks /commit/campaign"))?,
    )?;
    let before = rmp_serde::to_vec_named(&campaign)?;
    let source_id = "dock-guild";
    let target_id = "harbor-neighbors";
    let resource = "west winch rerouting plan";
    let state_references = subject_state_references(&campaign, source_id)?
        .into_iter()
        .filter(|reference| {
            reference == &format!("resource:{resource}")
                || reference.starts_with("location:")
                || reference.starts_with("capability:")
        })
        .collect::<Vec<_>>();
    if !state_references
        .iter()
        .any(|reference| reference == &format!("resource:{resource}"))
    {
        anyhow::bail!("baseline lost the exact transfer resource")
    }
    let location = campaign.gestalts[source_id].home_location_id.clone();
    if campaign.gestalts[target_id].home_location_id != location {
        anyhow::bail!("forced transfer subjects are no longer colocated")
    }
    let proposal = CellActionProposal {
        subject_id: source_id.into(),
        intent: format!("hand the {resource} to Harbor Neighbors"),
        intended_effect: format!(
            "transfer the exact {resource} from Dock Labor Guild custody to Harbor Neighbors custody"
        ),
        priority: 100,
        state_references,
        public_channels: vec![],
        effects: vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: source_id.into(),
            activity: StrategicActivityKind::Trade,
            target_subject_ids: vec![target_id.into()],
            location_ids: vec![location],
        }],
    };
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let model: Arc<dyn ModelPort> = Arc::new(ObservedModel {
        inner: DeepSeekPort::from_runtime_secret(secret)?,
        calls: calls.clone(),
    });
    let started = Instant::now();
    let resolved = resolve_activity_outcomes(model.as_ref(), &campaign, &[proposal.clone()]).await;
    let private_calls = calls.lock().expect("outcome trace lock").clone();
    std::fs::write(
        result_root.join("private-model-calls.json"),
        serde_json::to_vec_pretty(&private_calls)?,
    )?;
    let (outcomes, stages) = resolved?;
    if rmp_serde::to_vec_named(&campaign)? != before {
        anyhow::bail!("outcome resolution mutated its canonical input snapshot")
    }
    if !stages
        .iter()
        .any(|stage| stage.receipt.stage == "strategic_outcome_verifier")
    {
        anyhow::bail!("forced high-risk outcome never reached the independent verifier")
    }
    if !matches!(
        outcomes.as_slice(),
        [outcome]
            if matches!(
                &outcome.effect,
                StrategicOutcomeEffect::ResourceTransferred {
                    from_subject_id,
                    to_subject_id,
                    resource: transferred,
                } if from_subject_id == source_id
                    && to_subject_id == target_id
                    && transferred == resource
            )
    ) {
        anyhow::bail!("provider did not resolve the explicit exact-custody transfer: {outcomes:?}")
    }
    let result = serde_json::json!({
        "schema":"ghostlight.forced_high_risk_outcome_smoke.v1",
        "baseline":baseline,
        "elapsed_ms":started.elapsed().as_millis(),
        "proposal":proposal,
        "outcomes":outcomes,
        "stage_receipts":stages.into_iter().map(|stage| stage.receipt).collect::<Vec<_>>(),
        "private_model_call_count":private_calls.len(),
        "canonical_input_unchanged":true,
        "verifier_exercised":true,
    });
    std::fs::write(
        result_root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
