fn final_wave_field(
    wave_reports: &[serde_json::Value],
    field: &str,
) -> anyhow::Result<serde_json::Value> {
    wave_reports
        .last()
        .and_then(|wave| wave.get(field))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("final strategic wave is missing {field}"))
}

fn strategic_smoke_digest<T: serde::Serialize + ?Sized>(value: &T) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};

    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

fn strategic_smoke_bytes_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn recomposed_model_receipt_set_digest(value: &serde_json::Value) -> anyhow::Result<String> {
    let receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt> =
        serde_json::from_value(value.clone()).map_err(|error| {
            anyhow::anyhow!("newspaper recomposition has invalid model receipts: {error}")
        })?;
    strategic_smoke_digest(&receipts)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalWorldNewspaperIssueV2 {
    schema: String,
    id: String,
    title: String,
    edition_label: String,
    at: chrono::DateTime<chrono::Utc>,
    source_world_revision: u64,
    lead_article_id: Option<String>,
    articles: Vec<HistoricalWorldNewspaperArticleV2>,
    editorial_receipt_ids: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalWorldNewspaperArticleV2 {
    id: String,
    section: String,
    headline: String,
    deck: String,
    byline: String,
    dateline: Option<String>,
    paragraphs: Vec<String>,
    source_news_ids: Vec<String>,
    source_channels: Vec<String>,
    source_reliability: Vec<String>,
    event_ids: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn validate_completed_newspaper_recomposition_receipt(
    recomposition: &serde_json::Value,
    wave_index: u64,
    current_recovery_start_wave: usize,
    world_revision: u64,
    source_campaign_digest: &str,
    editorial_contract_digest: &str,
    reader_copy: &str,
    audit_copy: &str,
) -> anyhow::Result<()> {
    let recorded_recovery_start_wave =
        recomposition["recovery_start_wave"]
            .as_u64()
            .ok_or_else(|| {
                anyhow::anyhow!("completed wave newspaper recomposition has no recovery boundary")
            })?;
    let recomposition_schema = recomposition["schema"].as_str();
    if !matches!(
        recomposition_schema,
        Some("ghostlight.newspaper_wave_recomposition.v1")
            | Some("ghostlight.newspaper_wave_recomposition.v2")
            | Some("ghostlight.newspaper_wave_recomposition.v3")
    ) || recomposition["wave_index"].as_u64() != Some(wave_index)
        || wave_index < current_recovery_start_wave as u64
        || recorded_recovery_start_wave == 0
        || recorded_recovery_start_wave > wave_index
        || recomposition["world_revision"].as_u64() != Some(world_revision)
        || recomposition["source_campaign_digest"] != source_campaign_digest
        || recomposition["editorial_contract_digest"] != editorial_contract_digest
        || recomposition["issue"].is_null()
        || matches!(
            recomposition_schema,
            Some("ghostlight.newspaper_wave_recomposition.v1")
                | Some("ghostlight.newspaper_wave_recomposition.v2")
        ) && recomposition["newspaper_grounding"]["accepted"] != true
        || recomposition_schema == Some("ghostlight.newspaper_wave_recomposition.v2")
            && recomposition["newspaper_editorial"]["accepted"] != true
        || recomposition_schema == Some("ghostlight.newspaper_wave_recomposition.v3")
            && (recomposition["newspaper_copy_desk"].is_null()
                || recomposition["newspaper_press_close"].is_null())
        || recomposition["issue_file"] != format!("newspaper-wave-{wave_index:02}.md")
        || recomposition["issue_audit_file"] != format!("newspaper-wave-{wave_index:02}.audit.md")
    {
        anyhow::bail!(
            "completed wave newspaper recomposition does not bind its exact accepted issue"
        )
    }

    let (issue_digest, current_schema) = match recomposition["issue"]["schema"].as_str() {
        Some("ghostlight.world_newspaper_issue.v3") => {
            let issue: ghostlight_dungeon::newspaper::WorldNewspaperIssue = serde_json::from_value(
                recomposition["issue"].clone(),
            )
            .map_err(|error| {
                anyhow::anyhow!(
                    "completed wave newspaper recomposition has an invalid current issue: {error}"
                )
            })?;
            if reader_copy != ghostlight_dungeon::newspaper::render_world_newspaper_markdown(&issue)
                || audit_copy
                    != ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(&issue)
            {
                anyhow::bail!(
                    "completed wave newspaper recomposition current rendering differs from its artifact"
                )
            }
            (strategic_smoke_digest(&issue)?, true)
        }
        Some("ghostlight.world_newspaper_issue.v2") => {
            let issue: HistoricalWorldNewspaperIssueV2 =
                serde_json::from_value(recomposition["issue"].clone()).map_err(|error| {
                    anyhow::anyhow!(
                        "completed wave newspaper recomposition has an invalid historical issue: {error}"
                    )
                })?;
            (strategic_smoke_digest(&issue)?, false)
        }
        Some(schema) => anyhow::bail!(
            "completed wave newspaper recomposition uses unsupported issue schema {schema}"
        ),
        None => anyhow::bail!("completed wave newspaper recomposition issue has no schema"),
    };
    if recomposition["issue_digest"] != issue_digest
        || recomposition["model_receipt_set_digest"]
            != recomposed_model_receipt_set_digest(&recomposition["newspaper_model_receipts"])?
        || recomposition["reader_copy_digest"]
            != strategic_smoke_bytes_digest(reader_copy.as_bytes())
        || recomposition["audit_copy_digest"] != strategic_smoke_bytes_digest(audit_copy.as_bytes())
    {
        anyhow::bail!("completed wave newspaper recomposition artifact differs from its receipt")
    }
    if current_schema && recomposition_schema == Some("ghostlight.newspaper_wave_recomposition.v3")
    {
        let copy_desk: ghostlight_dungeon::newspaper::WorldNewspaperCopyDeskReport =
            serde_json::from_value(recomposition["newspaper_copy_desk"].clone()).map_err(
                |error| {
                    anyhow::anyhow!(
                        "completed wave newspaper recomposition has an invalid copy-desk report: {error}"
                    )
                },
            )?;
        let press_close: ghostlight_dungeon::newspaper::WorldNewspaperPressClose =
            serde_json::from_value(recomposition["newspaper_press_close"].clone()).map_err(
                |error| {
                    anyhow::anyhow!(
                        "completed wave newspaper recomposition has an invalid press close: {error}"
                    )
                },
            )?;
        if recomposition["copy_desk_digest"] != strategic_smoke_digest(&copy_desk)?
            || recomposition["press_close_digest"] != strategic_smoke_digest(&press_close)?
        {
            anyhow::bail!(
                "completed wave newspaper recomposition close evidence differs from its receipt"
            )
        }
    } else if current_schema {
        let grounding: ghostlight_dungeon::newspaper::WorldNewspaperGroundingVerdict =
            serde_json::from_value(recomposition["newspaper_grounding"].clone()).map_err(
                |error| {
                    anyhow::anyhow!(
                        "completed wave newspaper recomposition has an invalid current grounding verdict: {error}"
                    )
                },
            )?;
        if recomposition["grounding_digest"] != strategic_smoke_digest(&grounding)? {
            anyhow::bail!(
                "completed wave newspaper recomposition grounding differs from its receipt"
            )
        }
        if recomposition_schema == Some("ghostlight.newspaper_wave_recomposition.v2") {
            let editorial: ghostlight_dungeon::newspaper::WorldNewspaperEditorialVerdict =
                serde_json::from_value(recomposition["newspaper_editorial"].clone()).map_err(
                    |error| {
                        anyhow::anyhow!(
                            "completed wave newspaper recomposition has an invalid editorial verdict: {error}"
                        )
                    },
                )?;
            if recomposition["editorial_digest"] != strategic_smoke_digest(&editorial)? {
                anyhow::bail!(
                    "completed wave newspaper recomposition editorial verdict differs from its receipt"
                )
            }
        }
    }
    Ok(())
}

fn completed_wave_issue_campaign(
    wave_reports: &[serde_json::Value],
    report_index: usize,
) -> anyhow::Result<ghostlight_dungeon::domain::Campaign> {
    let report = wave_reports
        .get(report_index)
        .ok_or_else(|| anyhow::anyhow!("completed wave report is absent"))?;
    let wave_index = report["wave_index"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("completed wave report has no wave index"))?;
    if wave_index != report_index as u64 + 1 {
        anyhow::bail!("completed wave reports are not a contiguous prefix")
    }
    let mut campaign: ghostlight_dungeon::domain::Campaign =
        serde_json::from_value(report["commit"]["campaign"].clone()).map_err(|error| {
            anyhow::anyhow!("completed wave has no committed campaign: {error}")
        })?;
    if campaign.revision != report["world_revision_after"].as_u64().unwrap_or_default() {
        anyhow::bail!("completed wave campaign disagrees with its committed revision")
    }
    let previous_campaign: ghostlight_dungeon::domain::Campaign = if report_index == 0 {
        anyhow::bail!(
            "a missing first-wave newspaper cannot recover its exact pre-wave news boundary"
        )
    } else {
        serde_json::from_value(wave_reports[report_index - 1]["commit"]["campaign"].clone())
            .map_err(|error| {
                anyhow::anyhow!("prior completed wave has no committed campaign: {error}")
            })?
    };
    let previous_news_count = previous_campaign.news.len();
    if previous_news_count > campaign.news.len() {
        anyhow::bail!("completed wave news ledger is shorter than its prior committed prefix")
    }
    if campaign.news[..previous_news_count] != previous_campaign.news {
        anyhow::bail!("completed wave news ledger does not preserve its exact prior prefix")
    }
    campaign.news = campaign.news[previous_news_count..].to_vec();
    if campaign.news.is_empty() {
        anyhow::bail!("completed wave has no recoverable gated news")
    }
    Ok(campaign)
}

fn missing_newspaper_report_indices(
    wave_reports: &[serde_json::Value],
    recovery_start_wave: usize,
) -> anyhow::Result<Vec<usize>> {
    let mut indices = Vec::new();
    for (report_index, report) in wave_reports.iter().enumerate() {
        let wave_index = report["wave_index"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("completed wave report has no wave index"))?;
        if wave_index != report_index as u64 + 1 {
            anyhow::bail!("completed wave reports are not a contiguous prefix")
        }
        if wave_index >= recovery_start_wave as u64 && report["issue"].is_null() {
            indices.push(report_index);
        }
    }
    Ok(indices)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompilerCheckpoint {
    description: String,
    preview: ghostlight_dungeon::domain::WorldCompilePreview,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FoundationCheckpoint {
    location_id: String,
    location_name: String,
    request: String,
    preview: ghostlight_dungeon::domain::DestinationCompilationPreview,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NewspaperReconciliationImportEnvelope {
    schema: String,
    wave_index: u64,
    recovery_start_wave: usize,
    world_revision: u64,
    import: ghostlight_dungeon::newspaper::WorldNewspaperReconciliationImport,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockBindingProposalCheckpoint {
    schema: String,
    admission: ghostlight_dungeon::clock::ClockConsequenceBindingAdmission,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockBindingCheckpoint {
    schema: String,
    binding_receipt: ghostlight_dungeon::clock::ClockConsequenceBindingReceipt,
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletedInvocationCheckpoint {
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    proposal: ghostlight_dungeon::elaboration::WorldElaborationProposal,
    model_receipt_hashes: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FailedInvocationCheckpoint {
    dispatch: Option<ghostlight_dungeon::elaboration::ElaborationDispatch>,
    diagnostic: String,
    model_receipt_hashes: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledFailureCheckpoint {
    schema: String,
    location_id: String,
    location_name: String,
    request: String,
    wave: Option<ghostlight_dungeon::elaboration::ElaborationWaveBinding>,
    schedule: Option<ghostlight_dungeon::elaboration::ElaborationScheduleReceipt>,
    completed_invocations: Vec<CompletedInvocationCheckpoint>,
    invocation_failures: Vec<FailedInvocationCheckpoint>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledPreviewCheckpoint {
    schema: String,
    location_id: String,
    location_name: String,
    request: String,
    wave: ghostlight_dungeon::elaboration::ElaborationWaveBinding,
    schedule: ghostlight_dungeon::elaboration::ElaborationScheduleReceipt,
    accepted_operations: Vec<ghostlight_dungeon::elaboration::AdmittedWorldElaborationOperation>,
    rejections: Vec<ghostlight_dungeon::elaboration::WorldElaborationRejection>,
    candidate: Option<ghostlight_dungeon::domain::LocalityElaboration>,
    candidate_diagnostic: Option<String>,
    model_receipt_hashes: Vec<String>,
    #[serde(default)]
    resumed_from: Option<std::path::PathBuf>,
    #[serde(default)]
    retried_dispatch_ordinals: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledMutationProof {
    schema: String,
    authority_id: String,
    authority_digest: String,
    batch_id: String,
    batch_digest: String,
    mutation_receipt_id: String,
    intended_effect_digest: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledCommitCheckpoint {
    schema: String,
    location_id: String,
    location_name: String,
    world_revision_before: u64,
    world_revision_after: u64,
    wave: ghostlight_dungeon::elaboration::ElaborationWaveBinding,
    schedule: ghostlight_dungeon::elaboration::ElaborationScheduleReceipt,
    admission_digest: Option<String>,
    verifier_receipt_hash: String,
    model_receipt_hashes: Vec<String>,
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
    mutation_proof: TitledMutationProof,
    legacy_inferred: bool,
}

fn read_checkpoint<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> anyhow::Result<T> {
    let bytes = std::fs::read(path)
        .map_err(|error| anyhow::anyhow!("cannot read checkpoint {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| anyhow::anyhow!("cannot decode checkpoint {}: {error}", path.display()))
}

fn latest_partial_wave_checkpoint<T: serde::de::DeserializeOwned>(
    root: &std::path::Path,
    wave_index: usize,
    maximum_generation: usize,
) -> anyhow::Result<(usize, Option<T>)> {
    let mut latest_generation = 0;
    let mut latest = None;
    for generation in 1..=maximum_generation {
        let path = root.join(format!(
            "wave-{wave_index:02}-partial-pulse-{generation:02}.json"
        ));
        if path.is_file() {
            latest = Some(read_checkpoint(&path)?);
            latest_generation = generation;
        }
    }
    Ok((latest_generation, latest))
}

fn publish_immutable_checkpoint(
    path: &std::path::Path,
    value: &impl serde::Serialize,
) -> anyhow::Result<()> {
    use std::io::Write;

    if path.exists() {
        anyhow::bail!("immutable checkpoint already exists: {}", path.display())
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("checkpoint path has no UTF-8 file name"))?;
    let temporary_path = path.with_file_name(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> anyhow::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(&serde_json::to_vec_pretty(value)?)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)?;
        #[cfg(unix)]
        std::fs::File::open(
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("checkpoint path has no parent"))?,
        )?
        .sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
}

fn titled_failure_checkpoint_paths(
    root: &std::path::Path,
    index: usize,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let original_name = format!("titled-elaboration-{index:02}-terminal-failure.json");
    let resume_prefix = format!("titled-elaboration-{index:02}-resume-");
    let mut paths = Vec::new();
    let original = root.join(original_name);
    if original.is_file() {
        paths.push(original);
    }
    let mut generations = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let generation = name
                .strip_prefix(&resume_prefix)?
                .strip_suffix("-terminal-failure.json")?
                .parse::<u32>()
                .ok()?;
            Some((generation, entry.path()))
        })
        .collect::<Vec<_>>();
    generations.sort_by_key(|(generation, _)| *generation);
    paths.extend(generations.into_iter().map(|(_, path)| path));
    Ok(paths)
}

fn load_checkpoint_receipts(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    hashes: &[String],
) -> anyhow::Result<Vec<ghostlight_dungeon::model::ModelStageReceipt>> {
    hashes
        .iter()
        .map(|hash| {
            store
                .load::<ghostlight_dungeon::model::ModelStageReceipt>(
                    "persona_stage_receipt.v1",
                    hash,
                )?
                .map(|(_, receipt)| receipt)
                .ok_or_else(|| anyhow::anyhow!("checkpoint model receipt is missing: {hash}"))
        })
        .collect()
}

fn latest_clock_binding_receipt(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign_id: uuid::Uuid,
) -> anyhow::Result<Option<ghostlight_dungeon::clock::ClockConsequenceBindingReceipt>> {
    Ok(store
        .load_all::<ghostlight_dungeon::clock::ClockConsequenceBindingReceipt>(
            "clock_consequence_binding_receipt.v1",
        )?
        .into_iter()
        .filter(|receipt| receipt.campaign_id == campaign_id)
        .max_by_key(|receipt| receipt.revision))
}

fn validate_clock_binding_receipt_projection(
    campaign: &ghostlight_dungeon::domain::Campaign,
    receipt: &ghostlight_dungeon::clock::ClockConsequenceBindingReceipt,
) -> anyhow::Result<()> {
    if receipt.schema != "ghostlight.clock_consequence_binding_receipt.v1"
        || receipt.campaign_id != campaign.id
        || receipt.previous_revision.saturating_add(1) != receipt.revision
        || receipt.revision > campaign.revision
        || receipt.bindings.iter().any(|binding| {
            campaign
                .clocks
                .get(&binding.clock_id)
                .is_none_or(|clock| clock.consequence_scope != binding.scope)
        })
        || receipt
            .emitted_event_ids
            .iter()
            .any(|id| !campaign.events.iter().any(|event| &event.id == id))
        || receipt
            .emitted_news_ids
            .iter()
            .any(|id| !campaign.news.iter().any(|news| &news.id == id))
    {
        anyhow::bail!("canonical clock consequence binding receipt does not match the campaign")
    }
    Ok(())
}

fn recover_committed_clock_binding(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    checkpoint_path: &std::path::Path,
) -> anyhow::Result<Option<ghostlight_dungeon::clock::ClockConsequenceBindingReceipt>> {
    let Some(binding_receipt) = latest_clock_binding_receipt(store, campaign.id)? else {
        if checkpoint_path.is_file() {
            anyhow::bail!("clock binding checkpoint has no canonical CultCache receipt")
        }
        return Ok(None);
    };
    validate_clock_binding_receipt_projection(campaign, &binding_receipt)?;
    let commit_receipt = store
        .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
            "world_commit_receipt.v1",
            &format!("{}-{}", campaign.id, binding_receipt.revision),
        )?
        .map(|(_, receipt)| receipt)
        .ok_or_else(|| anyhow::anyhow!("clock binding world commit receipt is missing"))?;
    let checkpoint = ClockBindingCheckpoint {
        schema: "ghostlight.clock_consequence_binding_checkpoint.v2".into(),
        binding_receipt: binding_receipt.clone(),
        commit_receipt,
    };
    if checkpoint_path.is_file() {
        let persisted: ClockBindingCheckpoint = read_checkpoint(checkpoint_path)?;
        if persisted.schema != checkpoint.schema
            || persisted.binding_receipt != checkpoint.binding_receipt
            || persisted.commit_receipt != checkpoint.commit_receipt
        {
            anyhow::bail!("clock binding checkpoint disagrees with canonical CultCache state")
        }
    } else {
        publish_immutable_checkpoint(checkpoint_path, &checkpoint)?;
    }
    Ok(Some(binding_receipt))
}

fn rehydrate_titled_failure(
    checkpoint: TitledFailureCheckpoint,
    store: &ghostlight_dungeon::persistence::CampaignStore,
) -> anyhow::Result<
    ghostlight_dungeon::elaboration::ElaborationWaveFailure<
        ghostlight_dungeon::elaboration::WorldElaborationProposal,
    >,
> {
    if checkpoint.schema != "ghostlight.titled_elaboration_failure.v1" {
        anyhow::bail!("titled elaboration checkpoint schema is unsupported")
    }
    let completed_invocations = checkpoint
        .completed_invocations
        .into_iter()
        .map(|invocation| {
            Ok(ghostlight_dungeon::elaboration::ElaborationInvocation {
                wave: checkpoint
                    .wave
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("checkpoint has no wave binding"))?,
                dispatch: invocation.dispatch,
                proposal: invocation.proposal,
                model_stage_receipts: load_checkpoint_receipts(
                    store,
                    &invocation.model_receipt_hashes,
                )?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let invocation_failures = checkpoint
        .invocation_failures
        .into_iter()
        .map(|failure| {
            Ok(
                ghostlight_dungeon::elaboration::ElaborationInvocationFailure {
                    dispatch: failure.dispatch,
                    diagnostic: failure.diagnostic,
                    model_stage_receipts: load_checkpoint_receipts(
                        store,
                        &failure.model_receipt_hashes,
                    )?,
                },
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(ghostlight_dungeon::elaboration::ElaborationWaveFailure {
        wave: checkpoint.wave,
        schedule: checkpoint.schedule,
        completed_invocations,
        invocation_failures,
    })
}

fn civic_manifest_preserves(
    current: &ghostlight_dungeon::domain::CivicSystemManifest,
    checkpoint: &ghostlight_dungeon::domain::CivicSystemManifest,
) -> bool {
    current.schema == checkpoint.schema
        && current.jurisdiction_location_id == checkpoint.jurisdiction_location_id
        && current.version >= checkpoint.version
        && current
            .governing_institution_ids
            .is_superset(&checkpoint.governing_institution_ids)
        && current
            .resident_population_ids
            .is_superset(&checkpoint.resident_population_ids)
        && current
            .public_authority_fact_ids
            .is_superset(&checkpoint.public_authority_fact_ids)
        && current
            .public_selection_fact_ids
            .is_superset(&checkpoint.public_selection_fact_ids)
        && current
            .public_resource_fact_ids
            .is_superset(&checkpoint.public_resource_fact_ids)
        && current
            .public_redress_fact_ids
            .is_superset(&checkpoint.public_redress_fact_ids)
        && current
            .political_relation_ids
            .is_superset(&checkpoint.political_relation_ids)
        && !current.semantic_verification_receipt_id.is_empty()
}

fn civic_manifest_is_committed_candidate(
    current: &ghostlight_dungeon::domain::CivicSystemManifest,
    candidate: &ghostlight_dungeon::domain::CivicSystemManifest,
) -> bool {
    let mut expected = candidate.clone();
    expected.semantic_verification_receipt_id = current.semantic_verification_receipt_id.clone();
    !current.semantic_verification_receipt_id.is_empty() && current == &expected
}

fn finalized_titled_expansion(
    candidate: &ghostlight_dungeon::domain::LocalityElaboration,
    verifier_receipt_hash: &str,
) -> anyhow::Result<ghostlight_dungeon::domain::RegionExpansion> {
    if verifier_receipt_hash.trim().is_empty() {
        anyhow::bail!("titled elaboration verifier receipt hash is empty")
    }
    let mut expansion = candidate.expansion.clone();
    let civic = expansion
        .civic_system
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("titled elaboration candidate has no civic system"))?;
    if !civic.semantic_verification_receipt_id.is_empty() {
        anyhow::bail!("titled elaboration candidate already claims verifier authority")
    }
    civic.semantic_verification_receipt_id = verifier_receipt_hash.into();
    Ok(expansion)
}

fn committed_elaboration_mutation_proof(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    world_receipt: &ghostlight_dungeon::domain::WorldCommitReceipt,
    expansion: &ghostlight_dungeon::domain::RegionExpansion,
) -> anyhow::Result<TitledMutationProof> {
    use ghostlight_dungeon::transition::{
        MutationAuthorityEnvelope, WorldMutationBatch, WorldMutationReceipt, mutation_digest,
        validate_batch_structure,
    };

    let intended_effect_digest =
        ghostlight_dungeon::legacy_transition::digest_serializable(expansion)?;
    let mut batches = store
        .load_all::<WorldMutationBatch>("world_mutation_batch.v1")?
        .into_iter()
        .filter(|batch| {
            batch.campaign_id == world_receipt.campaign_id
                && batch.expected_world_revision == world_receipt.previous_revision
                && batch.intended_effect_digest.as_deref() == Some(&intended_effect_digest)
        })
        .collect::<Vec<_>>();
    if batches.len() != 1 {
        anyhow::bail!(
            "titled world commit has {} exact candidate mutation batches",
            batches.len()
        )
    }
    let batch = batches.pop().expect("one exact batch was required");
    let mut authorities = store
        .load_all::<MutationAuthorityEnvelope>("mutation_authority_envelope.v1")?
        .into_iter()
        .filter(|authority| {
            authority.campaign_id == world_receipt.campaign_id
                && authority.world_revision == world_receipt.previous_revision
                && authority.digest == batch.authority_envelope_digest
        })
        .collect::<Vec<_>>();
    if authorities.len() != 1 {
        anyhow::bail!(
            "titled world commit has {} exact mutation authorities",
            authorities.len()
        )
    }
    let authority = authorities
        .pop()
        .expect("one exact mutation authority was required");
    validate_batch_structure(&authority, &batch, world_receipt.committed_at)?;

    let mut mutation_receipts = store
        .load_all::<WorldMutationReceipt>("world_mutation_receipt.v1")?
        .into_iter()
        .filter(|receipt| {
            receipt.campaign_id == world_receipt.campaign_id
                && receipt.previous_world_revision == world_receipt.previous_revision
                && receipt.world_revision == world_receipt.revision
                && receipt.batch_digest == batch.digest
                && receipt.authority_envelope_digest == authority.digest
        })
        .collect::<Vec<_>>();
    if mutation_receipts.len() != 1 {
        anyhow::bail!(
            "titled world commit has {} exact mutation receipts",
            mutation_receipts.len()
        )
    }
    let mutation_receipt = mutation_receipts
        .pop()
        .expect("one exact mutation receipt was required");
    let expected_mutation_digests = batch
        .mutations
        .iter()
        .map(mutation_digest)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let expected_source_receipt_id = format!(
        "region-expansion:{}",
        intended_effect_digest.trim_start_matches("sha256:")
    );
    if world_receipt.schema != "ghostlight.world_commit_receipt.v1"
        || world_receipt.command_kind != "elaborate_locality"
        || world_receipt.previous_revision.saturating_add(1) != world_receipt.revision
        || batch.schema != "ghostlight.world_mutation_batch.v1"
        || batch.source_receipt_id != expected_source_receipt_id
        || batch.expected_resolution_epoch.is_some()
        || batch.mutations.is_empty()
        || authority.schema != "ghostlight.mutation_authority_envelope.v1"
        || authority.resolution_epoch.is_some()
        || authority.source_subject.is_some()
        || authority.procedure
            != ghostlight_dungeon::transition::MutationProcedure::CompilerAdmission
        || authority.outcome
            != ghostlight_dungeon::transition::MutationOutcomeBinding::Deterministic
        || mutation_receipt.schema != "ghostlight.world_mutation_receipt.v1"
        || mutation_receipt.id != format!("mutation:{}", batch.id)
        || mutation_receipt.committed_at != world_receipt.committed_at
        || mutation_receipt.mutation_digests != expected_mutation_digests
    {
        anyhow::bail!("titled mutation proof does not bind one compiler admission commit")
    }

    Ok(TitledMutationProof {
        schema: "ghostlight.titled_elaboration_mutation_proof.v1".into(),
        authority_id: authority.id,
        authority_digest: authority.digest,
        batch_id: batch.id,
        batch_digest: batch.digest,
        mutation_receipt_id: mutation_receipt.id,
        intended_effect_digest,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        compiler::DestinationCompilationFailure,
        domain::{TickSource, WorldCommand},
        elaboration::{
            ElaborationScheduler, ElaboratorTitle, ModelWorldElaborationWorker,
            admit_world_elaboration_wave, dispatch_elaboration_wave, finalize_world_elaboration,
            resume_elaboration_wave, world_elaboration_wave_binding,
        },
        kernel::{CommandResult, WorldKernel},
        model_runtime::ModelRuntimeSelection,
        persistence::CampaignStore,
        scheduler::{
            ResolutionWaveCheckpoint, ResolutionWavePipelineFailure, propose_resolution_wave,
            resume_resolution_wave,
        },
        turn::SnapshotPermit,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let runtime_root = std::env::var_os("GHOSTLIGHT_DUNGEON_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(default_runtime_root);
    let model_selection = ModelRuntimeSelection::from_environment(&runtime_root)?;
    let scenario_id = std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO")
        .unwrap_or_else(|_| "strategic-default".into());
    let pressure = std::env::var("GHOSTLIGHT_STRATEGIC_PRESSURE").unwrap_or_else(|_| {
        "The sovereign deep-hold diverted the White Root aquifer. Two tithe caravans have vanished, the charcoal guilds threaten secession, and somebody pawned the regent's rain seal."
            .into()
    });
    let wave_count = bounded_environment_usize("GHOSTLIGHT_STRATEGIC_WAVES", 1, 1, 32)?;
    let newspaper_recovery_start_wave = bounded_environment_usize(
        "GHOSTLIGHT_STRATEGIC_NEWSPAPER_RECOVERY_START_WAVE",
        1,
        1,
        wave_count + 1,
    )?;
    let max_rejected_pulses_per_wave =
        bounded_environment_usize("GHOSTLIGHT_STRATEGIC_MAX_REJECTED_PULSES_PER_WAVE", 2, 0, 4)?;
    let root = std::env::var_os("GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "strategic-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                uuid::Uuid::new_v4()
            ))
        });
    std::fs::create_dir_all(&root)?;
    let resume = matches!(
        std::env::var("GHOSTLIGHT_LIVE_FIRE_RESUME").ok().as_deref(),
        Some("1" | "true")
    );
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    let model = model_selection.open()?.ok_or_else(|| {
        anyhow::anyhow!(
            "{} credential is unavailable at {}",
            model_selection.provider,
            model_selection.credential_path.display()
        )
    })?;
    let public_channel = admitted_public_channel(
        &std::env::var("GHOSTLIGHT_STRATEGIC_PUBLIC_CHANNEL")
            .unwrap_or_else(|_| "root-wire broadsheet".into()),
    )?;
    let compiled = std::env::var("GHOSTLIGHT_WORLD_DESCRIPTION")
        .ok()
        .filter(|description| !description.trim().is_empty());
    let (
        mut campaign,
        seed_evidence_receipts,
        seed_model_receipts,
        mut world_compile,
        initial_seed_location_ids,
    ) = if resume {
        let checkpoint: CompilerCheckpoint = read_checkpoint(&root.join("compiler-preview.json"))?;
        let description = compiled
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("resume requires GHOSTLIGHT_WORLD_DESCRIPTION"))?;
        if description != checkpoint.description {
            anyhow::bail!("resume world description differs from its compiler checkpoint")
        }
        let campaign_keys = store.keys("campaign.v1")?;
        if campaign_keys.len() != 1 {
            anyhow::bail!("resume requires exactly one persisted campaign")
        }
        let campaign = store
            .load::<ghostlight_dungeon::domain::Campaign>("campaign.v1", &campaign_keys[0])?
            .map(|(_, campaign)| campaign)
            .ok_or_else(|| anyhow::anyhow!("resume campaign checkpoint is missing"))?;
        let initial_location_ids = checkpoint
            .preview
            .campaign
            .locations
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let pressure_matches = campaign
            .events
            .iter()
            .any(|event| event.kind == "strategic_pressure" && event.summary == pressure);
        let channel_matches = campaign
            .news
            .iter()
            .any(|issue| issue.channel == public_channel);
        if !pressure_matches || !channel_matches {
            anyhow::bail!("resume pressure or public channel differs from persisted world state")
        }
        (
            campaign,
            Vec::new(),
            Vec::new(),
            Some(serde_json::json!({
                "description":checkpoint.description,
                "preview":checkpoint.preview,
                "model_receipts":checkpoint.model_receipts,
                "preview_path":root.join("compiler-preview.json"),
                "resumed":true,
            })),
            initial_location_ids,
        )
    } else if let Some(description) = compiled.as_deref() {
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_status.v1",
                "state":"compiling_world",
                "waves_completed":0,
                "waves_requested":wave_count,
                "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                "updated_at":Utc::now(),
            }))?,
        )?;
        let (preview, receipts) =
            compile_strategic_campaign(model.clone(), description, &pressure, &public_channel)
                .await?;
        std::fs::write(
            root.join("compiler-preview.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "description":description,
                "preview":&preview,
                "model_receipts":&receipts,
            }))?,
        )?;
        let evidence = preview.evidence_receipts.clone();
        let campaign = preview.campaign.clone();
        let initial_location_ids = campaign.locations.keys().cloned().collect::<Vec<_>>();
        (
            campaign,
            evidence,
            receipts.clone(),
            Some(serde_json::json!({
                "description":description,
                "preview":preview,
                "model_receipts":receipts,
                "preview_path":root.join("compiler-preview.json"),
            })),
            initial_location_ids,
        )
    } else {
        let campaign = strategic_campaign();
        let initial_location_ids = campaign.locations.keys().cloned().collect::<Vec<_>>();
        (campaign, vec![], vec![], None, initial_location_ids)
    };
    let newspaper_reconciliation_import_path =
        std::env::var_os("GHOSTLIGHT_STRATEGIC_NEWSPAPER_RECONCILIATION_IMPORT")
            .map(std::path::PathBuf::from);
    let mut newspaper_reconciliation_import_consumed = false;
    if resume {
        ghostlight_dungeon::compiler::validate_campaign_runtime(&campaign)?;
    } else {
        ghostlight_dungeon::compiler::validate_campaign_seed(&campaign)?;
    }
    if !resume {
        let pressure_event = ghostlight_dungeon::domain::Event {
            id: format!("pressure-{}", uuid::Uuid::new_v4()),
            at: campaign.world_time,
            kind: "strategic_pressure".into(),
            summary: pressure.clone(),
            actor_ids: campaign
                .actors
                .keys()
                .filter(|actor_id| **actor_id != campaign.player_actor_id)
                .cloned()
                .collect(),
            institution_ids: campaign.institutions.keys().cloned().collect(),
            gestalt_ids: campaign.gestalts.keys().cloned().collect(),
            location_ids: campaign.locations.keys().cloned().collect(),
            public_channels: vec![public_channel.clone()],
        };
        ghostlight_dungeon::domain::append_event_with_publications(&mut campaign, pressure_event);
        store.create_unadmitted_fixture_campaign(
            &campaign,
            &seed_evidence_receipts,
            &seed_model_receipts,
        )?;
    }
    let player_before = campaign.actors[&campaign.player_actor_id].clone();
    let kernel = WorldKernel::start(store.clone());
    let clock_binding_path = root.join("clock-consequence-binding.json");
    let clock_binding_proposal_path = root.join("clock-consequence-binding-proposal.json");
    let mut pending_clock_news_start = None;
    if resume
        && campaign
            .clocks
            .values()
            .any(|clock| clock.consequence_scope.is_unbound())
    {
        if clock_binding_path.is_file() {
            anyhow::bail!(
                "clock consequence binding checkpoint exists but the persisted campaign is still unbound"
            )
        }
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_status.v1",
                "state":"binding_clock_consequences",
                "waves_completed":campaign.strategic_tick_count,
                "waves_requested":wave_count,
                "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                "world_revision":campaign.revision,
                "updated_at":Utc::now(),
            }))?,
        )?;
        let proposal = if clock_binding_proposal_path.is_file() {
            read_checkpoint::<ClockBindingProposalCheckpoint>(&clock_binding_proposal_path)?
        } else {
            let binding_run = ghostlight_dungeon::clock::propose_clock_consequence_bindings(
                model.as_ref(),
                &campaign,
            )
            .await
            .map_err(anyhow::Error::new)?;
            let proposal = ClockBindingProposalCheckpoint {
                schema: "ghostlight.clock_consequence_binding_proposal_checkpoint.v1".into(),
                admission: binding_run.output,
                model_receipts: binding_run.receipts,
            };
            publish_immutable_checkpoint(&clock_binding_proposal_path, &proposal)?;
            proposal
        };
        if proposal.schema != "ghostlight.clock_consequence_binding_proposal_checkpoint.v1" {
            anyhow::bail!("clock consequence binding proposal checkpoint is unsupported")
        }
        ghostlight_dungeon::clock::validate_binding_receipts(
            &campaign,
            &proposal.admission,
            &proposal.model_receipts,
        )?;
        let CommandResult::Committed {
            campaign: bound_campaign,
            receipt,
        } = kernel
            .command(WorldCommand::BindClockConsequences {
                expected_revision: campaign.revision,
                admission: proposal.admission,
                model_stage_receipts: proposal.model_receipts,
            })
            .await?
        else {
            anyhow::bail!("clock consequence binding did not commit")
        };
        let binding_receipt = store
            .load::<ghostlight_dungeon::clock::ClockConsequenceBindingReceipt>(
                "clock_consequence_binding_receipt.v1",
                &format!("{}-{}", bound_campaign.id, bound_campaign.revision),
            )?
            .map(|(_, receipt)| receipt)
            .ok_or_else(|| anyhow::anyhow!("clock binding commit lacks its canonical receipt"))?;
        validate_clock_binding_receipt_projection(&bound_campaign, &binding_receipt)?;
        publish_immutable_checkpoint(
            &clock_binding_path,
            &ClockBindingCheckpoint {
                schema: "ghostlight.clock_consequence_binding_checkpoint.v2".into(),
                binding_receipt: binding_receipt.clone(),
                commit_receipt: receipt,
            },
        )?;
        pending_clock_news_start = Some((
            binding_receipt.next_wave_index,
            binding_receipt.news_count_before,
        ));
        campaign = bound_campaign;
    } else if resume
        && let Some(binding_receipt) =
            recover_committed_clock_binding(&store, &campaign, &clock_binding_path)?
    {
        if campaign.strategic_tick_count.saturating_add(1) as usize
            == binding_receipt.next_wave_index
        {
            pending_clock_news_start = Some((
                binding_receipt.next_wave_index,
                binding_receipt.news_count_before,
            ));
        }
    }
    let elaboration_passes = if compiled.is_some() {
        bounded_environment_usize("GHOSTLIGHT_WORLD_ELABORATION_PASSES", 0, 0, 8)?
    } else {
        0
    };
    let initial_location_ids = initial_seed_location_ids
        .into_iter()
        .take(elaboration_passes)
        .collect::<Vec<_>>();
    let mut elaboration_reports = Vec::with_capacity(initial_location_ids.len());
    if let Some(description) = compiled.as_deref()
        && !initial_location_ids.is_empty()
    {
        let compiler =
            strategic_world_compiler(model.clone(), description, &strategic_world_when());
        let titled_profile = strategic_world_elaboration_profile();
        let titled_invocation_budget = titled_profile
            .controls
            .iter()
            .map(|control| u32::from(control.weight))
            .sum::<u32>();
        let titled_parallelism =
            bounded_environment_usize("GHOSTLIGHT_WORLD_ELABORATION_PARALLELISM", 8, 1, 32)?;
        let titled_eligible = ElaboratorTitle::ALL.into_iter().collect();
        let mut titled_scheduler = ElaborationScheduler::new(&titled_profile)?;
        for (index, location_id) in initial_location_ids.iter().enumerate() {
            let location_name = campaign.locations[location_id].name.clone();
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"elaborating_world",
                    "elaborations_completed":index,
                    "elaborations_requested":initial_location_ids.len(),
                    "current_location_id":location_id,
                    "waves_completed":0,
                    "waves_requested":wave_count,
                    "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                    "world_revision":campaign.revision,
                    "updated_at":Utc::now(),
                }))?,
            )?;
            let foundation_request =
                strategic_locality_request(&location_name, location_id, &pressure);
            let preview_path = root.join(format!("elaboration-{:02}-preview.json", index + 1));
            let (elaborated, receipts) = if resume && preview_path.is_file() {
                let checkpoint: FoundationCheckpoint = read_checkpoint(&preview_path)?;
                let (expected_revision, expected_civic) = match &checkpoint.preview {
                    ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(
                        preview,
                    ) if preview.elaboration.target_location_id == *location_id => {
                        (
                            preview.expected_revision,
                            preview
                                .elaboration
                                .expansion
                                .civic_system
                                .as_ref()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "foundation checkpoint has no civic system for {location_id}"
                                    )
                                })?
                                .clone(),
                        )
                    }
                    _ => anyhow::bail!(
                        "foundation checkpoint does not target {location_id} as a locality"
                    ),
                };
                if checkpoint.location_id != *location_id
                    || checkpoint.location_name != location_name
                    || checkpoint.request != foundation_request
                    || campaign.revision <= expected_revision
                    || campaign
                        .civic_systems
                        .get(location_id)
                        .is_none_or(|current| !civic_manifest_preserves(current, &expected_civic))
                {
                    anyhow::bail!(
                        "foundation checkpoint for {location_id} is not committed in the resumed campaign"
                    )
                }
                (campaign.clone(), checkpoint.model_receipts)
            } else {
                let compilation = compiler
                    .compile_destination(&campaign, location_id, &foundation_request)
                    .await;
                let (preview, receipts) = match compilation {
                    Ok(compilation) => compilation,
                    Err(error) => {
                        let failure_receipts = error
                            .downcast_ref::<DestinationCompilationFailure>()
                            .map(|failure| failure.model_receipts.clone())
                            .unwrap_or_default();
                        let receipt_hashes = failure_receipts
                            .iter()
                            .map(|receipt| receipt.storage_key().to_owned())
                            .collect::<Vec<_>>();
                        let persistence_error = if failure_receipts.is_empty() {
                            None
                        } else {
                            store
                                .persist_model_stage_receipts(&failure_receipts)
                                .err()
                                .map(|error| error.to_string())
                        };
                        std::fs::write(
                            root.join(format!(
                                "elaboration-{:02}-terminal-failure.json",
                                index + 1
                            )),
                            serde_json::to_vec_pretty(&serde_json::json!({
                                "schema":"ghostlight.strategic_elaboration_failure.v1",
                                "location_id":location_id,
                                "location_name":location_name,
                                "request":foundation_request,
                                "error":error.to_string(),
                                "model_receipt_hashes":receipt_hashes,
                                "receipt_persistence_error":&persistence_error,
                            }))?,
                        )?;
                        if let Some(persistence_error) = persistence_error {
                            return Err(anyhow::anyhow!(
                                "{error}; model-receipt persistence failed: {persistence_error}"
                            ));
                        }
                        return Err(error);
                    }
                };
                let command = match &preview {
                ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(
                    preview,
                ) => WorldCommand::ElaborateLocality {
                    expected_revision: preview.expected_revision,
                    elaboration: preview.elaboration.clone(),
                    evidence_receipts: preview.evidence_receipts.clone(),
                    canon_candidates: preview.canon_candidates.clone(),
                    model_stage_receipts: receipts.clone(),
                },
                ghostlight_dungeon::domain::DestinationCompilationPreview::RegionExpansion(_) => {
                    anyhow::bail!(
                        "strategic elaboration resolved existing location {location_id} as a new destination"
                    )
                }
            };
                std::fs::write(
                    &preview_path,
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "location_id":location_id,
                        "location_name":location_name,
                        "request":foundation_request,
                        "preview":&preview,
                        "model_receipts":&receipts,
                    }))?,
                )?;
                let committed = kernel.command(command).await?;
                let CommandResult::Committed {
                    campaign: elaborated,
                    ..
                } = committed
                else {
                    anyhow::bail!("strategic locality elaboration did not commit")
                };
                (elaborated, receipts)
            };
            campaign = elaborated;
            let titled_preview_path =
                root.join(format!("titled-elaboration-{:02}-preview.json", index + 1));
            let titled_commit_path =
                root.join(format!("titled-elaboration-{:02}-commit.json", index + 1));
            if resume && titled_preview_path.is_file() {
                let titled: TitledPreviewCheckpoint = read_checkpoint(&titled_preview_path)?;
                let candidate = titled.candidate.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("committed titled checkpoint has no candidate")
                })?;
                let candidate_civic =
                    candidate.expansion.civic_system.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("committed titled checkpoint has no civic candidate")
                    })?;
                if titled.schema != "ghostlight.titled_elaboration_preview.v1"
                    || titled.location_id != *location_id
                    || titled.location_name != location_name
                    || titled.request
                        != strategic_titled_locality_request(&location_name, location_id, &pressure)
                    || candidate.target_location_id != *location_id
                    || titled.candidate_diagnostic.is_some()
                    || !titled.rejections.is_empty()
                    || titled.accepted_operations.len() != titled.schedule.dispatches.len()
                {
                    anyhow::bail!(
                        "committed titled checkpoint for {location_id} is internally inconsistent"
                    )
                }
                let civic = campaign.civic_systems.get(location_id).ok_or_else(|| {
                    anyhow::anyhow!("resumed campaign lacks civic system for {location_id}")
                })?;
                if !civic_manifest_is_committed_candidate(civic, candidate_civic) {
                    anyhow::bail!(
                        "titled preview for {location_id} is not the civic system committed in CultCache"
                    )
                }
                let verifier_receipt = store
                    .load::<ghostlight_dungeon::model::ModelStageReceipt>(
                        "persona_stage_receipt.v1",
                        &civic.semantic_verification_receipt_id,
                    )?
                    .map(|(_, receipt)| receipt)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "committed titled verifier receipt is missing for {location_id}"
                        )
                    })?;
                let proposal_receipts =
                    load_checkpoint_receipts(&store, &titled.model_receipt_hashes)?;
                let expected_foundation_revision = match &read_checkpoint::<FoundationCheckpoint>(
                    &preview_path,
                )?
                .preview
                {
                    ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(
                        preview,
                    ) => preview.expected_revision,
                    _ => unreachable!("foundation checkpoint was validated above"),
                };
                let world_revision_before = expected_foundation_revision.saturating_add(1);
                let world_revision_after = expected_foundation_revision.saturating_add(2);
                let persisted_commit_receipt = store
                    .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
                        "world_commit_receipt.v1",
                        &format!("{}-{world_revision_after}", campaign.id),
                    )?
                    .map(|(_, receipt)| receipt)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "titled world commit receipt is missing for {location_id} revision {world_revision_after}"
                        )
                    })?;
                let finalized_expansion =
                    finalized_titled_expansion(candidate, verifier_receipt.storage_key())?;
                let mutation_proof = committed_elaboration_mutation_proof(
                    &store,
                    &persisted_commit_receipt,
                    &finalized_expansion,
                )?;
                let commit_checkpoint = if titled_commit_path.is_file() {
                    read_checkpoint::<TitledCommitCheckpoint>(&titled_commit_path)?
                } else {
                    let checkpoint = TitledCommitCheckpoint {
                        schema: "ghostlight.titled_elaboration_commit.v1".into(),
                        location_id: location_id.clone(),
                        location_name: location_name.clone(),
                        world_revision_before,
                        world_revision_after,
                        wave: titled.wave.clone(),
                        schedule: titled.schedule.clone(),
                        admission_digest: None,
                        verifier_receipt_hash: verifier_receipt.storage_key().into(),
                        model_receipt_hashes: titled.model_receipt_hashes.clone(),
                        commit_receipt: persisted_commit_receipt.clone(),
                        mutation_proof: mutation_proof.clone(),
                        legacy_inferred: true,
                    };
                    publish_immutable_checkpoint(&titled_commit_path, &checkpoint)?;
                    checkpoint
                };
                if commit_checkpoint.schema != "ghostlight.titled_elaboration_commit.v1"
                    || commit_checkpoint.location_id != *location_id
                    || commit_checkpoint.location_name != location_name
                    || commit_checkpoint.world_revision_before != world_revision_before
                    || commit_checkpoint.world_revision_after != world_revision_after
                    || commit_checkpoint.wave != titled.wave
                    || commit_checkpoint.schedule != titled.schedule
                    || commit_checkpoint.verifier_receipt_hash != verifier_receipt.storage_key()
                    || commit_checkpoint.model_receipt_hashes != titled.model_receipt_hashes
                    || commit_checkpoint.commit_receipt != persisted_commit_receipt
                    || commit_checkpoint.mutation_proof != mutation_proof
                    || commit_checkpoint.commit_receipt.command_kind != "elaborate_locality"
                    || (!commit_checkpoint.legacy_inferred
                        && commit_checkpoint
                            .admission_digest
                            .as_deref()
                            .is_none_or(str::is_empty))
                {
                    anyhow::bail!(
                        "titled completion checkpoint for {location_id} is not bound to its canonical commit"
                    )
                }
                titled_scheduler = ElaborationScheduler::from_state(
                    &titled_profile,
                    titled.schedule.final_state.clone(),
                )?;
                elaboration_reports.push(serde_json::json!({
                    "location_id":location_id,
                    "location_name":location_name,
                    "world_revision":world_revision_after,
                    "preview_path":preview_path,
                    "titled_preview_path":titled_preview_path,
                    "titled_commit_path":titled_commit_path,
                    "titled_model_receipts":proposal_receipts,
                    "titled_semantic_verifier_receipt":verifier_receipt,
                    "model_receipts":receipts,
                    "resumed_committed_checkpoint":true,
                    "original_wave":titled.wave,
                    "original_resume_source":titled.resumed_from,
                    "original_retried_dispatch_ordinals":titled.retried_dispatch_ordinals,
                }));
                continue;
            }
            let titled_wave = world_elaboration_wave_binding(&campaign, location_id)?;
            let titled_worker = Arc::new(ModelWorldElaborationWorker::new(
                model.clone(),
                Arc::new(campaign.clone()),
                location_id.clone(),
                strategic_titled_locality_request(&location_name, location_id, &pressure),
            )?);
            let original_failure_path = root.join(format!(
                "titled-elaboration-{:02}-terminal-failure.json",
                index + 1
            ));
            let failure_checkpoint_paths = titled_failure_checkpoint_paths(&root, index + 1)?;
            let checkpoint_path = failure_checkpoint_paths.last().cloned();
            let next_failure_path = if checkpoint_path.is_some() {
                root.join(format!(
                    "titled-elaboration-{:02}-resume-{:02}-terminal-failure.json",
                    index + 1,
                    failure_checkpoint_paths.len()
                ))
            } else {
                original_failure_path
            };
            let mut retried_dispatch_ordinals = Vec::new();
            let titled_result = if resume {
                if let Some(checkpoint_path) = checkpoint_path.as_ref() {
                    let checkpoint: TitledFailureCheckpoint = read_checkpoint(checkpoint_path)?;
                    if checkpoint.location_id != *location_id
                        || checkpoint.location_name != location_name
                        || checkpoint.request != titled_worker.task_request()
                        || checkpoint.wave.as_ref() != Some(&titled_wave)
                    {
                        anyhow::bail!(
                            "titled failure checkpoint for {location_id} does not bind the current frozen wave"
                        )
                    }
                    retried_dispatch_ordinals = checkpoint
                        .invocation_failures
                        .iter()
                        .filter_map(|failure| {
                            failure.dispatch.as_ref().map(|dispatch| dispatch.ordinal)
                        })
                        .collect();
                    let scheduler_state = checkpoint
                        .schedule
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("resume checkpoint has no schedule"))?
                        .final_state
                        .clone();
                    titled_scheduler =
                        ElaborationScheduler::from_state(&titled_profile, scheduler_state)?;
                    resume_elaboration_wave(
                        rehydrate_titled_failure(checkpoint, &store)?,
                        titled_parallelism,
                        titled_worker.clone(),
                    )
                    .await
                } else {
                    dispatch_elaboration_wave(
                        &mut titled_scheduler,
                        titled_wave,
                        &titled_eligible,
                        titled_invocation_budget,
                        titled_parallelism,
                        titled_worker.clone(),
                    )
                    .await
                }
            } else {
                dispatch_elaboration_wave(
                    &mut titled_scheduler,
                    titled_wave,
                    &titled_eligible,
                    titled_invocation_budget,
                    titled_parallelism,
                    titled_worker.clone(),
                )
                .await
            };
            let titled_run = match titled_result {
                Ok(run) => run,
                Err(failure) => {
                    let receipts = failure
                        .completed_invocations
                        .iter()
                        .flat_map(|invocation| invocation.model_stage_receipts.iter())
                        .chain(
                            failure
                                .invocation_failures
                                .iter()
                                .flat_map(|failure| failure.model_stage_receipts.iter()),
                        )
                        .cloned()
                        .collect::<Vec<_>>();
                    if !receipts.is_empty() {
                        store.persist_model_stage_receipts(&receipts)?;
                    }
                    let failure_path = next_failure_path;
                    publish_immutable_checkpoint(
                        &failure_path,
                        &serde_json::json!({
                            "schema":"ghostlight.titled_elaboration_failure.v1",
                            "location_id":location_id,
                            "location_name":location_name,
                            "request":titled_worker.task_request(),
                            "wave":failure.wave,
                            "schedule":failure.schedule,
                            "completed_invocations":failure.completed_invocations.iter().map(|invocation|serde_json::json!({
                                "dispatch":invocation.dispatch,
                                "proposal":invocation.proposal,
                                "model_receipt_hashes":invocation.model_stage_receipts.iter().map(|receipt|receipt.storage_key()).collect::<Vec<_>>(),
                            })).collect::<Vec<_>>(),
                            "invocation_failures":failure.invocation_failures.iter().map(|failure|serde_json::json!({
                                "dispatch":failure.dispatch,
                                "diagnostic":failure.diagnostic,
                                "model_receipt_hashes":failure.model_stage_receipts.iter().map(|receipt|receipt.storage_key()).collect::<Vec<_>>(),
                            })).collect::<Vec<_>>(),
                        }),
                    )?;
                    anyhow::bail!(
                        "titled elaboration wave failed for {location_id}; exact receipt at {}",
                        failure_path.display()
                    );
                }
            };
            let proposal_receipts = titled_run
                .invocations()
                .iter()
                .flat_map(|invocation| invocation.model_stage_receipts.iter())
                .cloned()
                .collect::<Vec<_>>();
            store.persist_model_stage_receipts(&proposal_receipts)?;
            let admission = admit_world_elaboration_wave(&campaign, location_id, titled_run)?;
            std::fs::write(
                &titled_preview_path,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.titled_elaboration_preview.v1",
                    "location_id":location_id,
                    "location_name":location_name,
                    "request":titled_worker.task_request(),
                    "wave":admission.wave(),
                    "schedule":admission.schedule(),
                    "accepted_operations":admission.accepted_operations(),
                    "rejections":admission.rejections(),
                    "candidate":admission.candidate(),
                    "candidate_diagnostic":admission.candidate_diagnostic(),
                    "model_receipt_hashes":admission.model_stage_receipts().iter().map(|receipt|receipt.storage_key()).collect::<Vec<_>>(),
                    "resumed_from":checkpoint_path,
                    "retried_dispatch_ordinals":retried_dispatch_ordinals,
                }))?,
            )?;
            let candidate = admission.candidate().cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "titled elaboration produced no candidate: {}",
                    admission.candidate_diagnostic().unwrap_or("no diagnostic")
                )
            })?;
            if let Some(diagnostic) = admission.candidate_diagnostic() {
                anyhow::bail!("titled elaboration candidate requires reconciliation: {diagnostic}");
            }
            let admission_digest = admission.digest().to_owned();
            let admitted_wave = admission.wave().clone();
            let admitted_schedule = admission.schedule().clone();
            let admitted_model_receipt_hashes = admission
                .model_stage_receipts()
                .iter()
                .map(|receipt| receipt.storage_key().to_owned())
                .collect::<Vec<_>>();
            let causal_receipt_ids = admission
                .model_stage_receipts()
                .iter()
                .map(|receipt| receipt.storage_key().to_owned())
                .collect::<Vec<_>>();
            let verifier_receipt = match compiler
                .verify_titled_locality_elaboration(
                    &campaign,
                    titled_worker.task_request(),
                    description,
                    &candidate,
                    &causal_receipt_ids,
                )
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    if let Some(failure) = error.downcast_ref::<
                        ghostlight_dungeon::compiler::CivicElaborationVerificationFailure,
                    >() {
                        store.persist_model_stage_receipts(&failure.model_receipts)?;
                    }
                    return Err(error);
                }
            };
            store.persist_model_stage_receipts(std::slice::from_ref(&verifier_receipt))?;
            let finalized_expansion =
                finalized_titled_expansion(&candidate, verifier_receipt.storage_key())?;
            let finalized =
                finalize_world_elaboration(&campaign, admission, verifier_receipt.clone())?;
            let CommandResult::Committed {
                campaign: titled_elaborated,
                receipt: titled_commit_receipt,
            } = kernel
                .commit_elaboration(finalized)
                .await
                .map_err(anyhow::Error::new)?
            else {
                anyhow::bail!("titled locality elaboration did not commit")
            };
            let mutation_proof = committed_elaboration_mutation_proof(
                &store,
                &titled_commit_receipt,
                &finalized_expansion,
            )?;
            let titled_commit_checkpoint = TitledCommitCheckpoint {
                schema: "ghostlight.titled_elaboration_commit.v1".into(),
                location_id: location_id.clone(),
                location_name: location_name.clone(),
                world_revision_before: campaign.revision,
                world_revision_after: titled_elaborated.revision,
                wave: admitted_wave,
                schedule: admitted_schedule,
                admission_digest: Some(admission_digest),
                verifier_receipt_hash: verifier_receipt.storage_key().into(),
                model_receipt_hashes: admitted_model_receipt_hashes,
                commit_receipt: titled_commit_receipt,
                mutation_proof,
                legacy_inferred: false,
            };
            publish_immutable_checkpoint(&titled_commit_path, &titled_commit_checkpoint)?;
            campaign = titled_elaborated;
            elaboration_reports.push(serde_json::json!({
                "location_id":location_id,
                "location_name":location_name,
                "world_revision":campaign.revision,
                "preview_path":preview_path,
                "titled_preview_path":titled_preview_path,
                "titled_commit_path":titled_commit_path,
                "titled_model_receipts":proposal_receipts,
                "titled_semantic_verifier_receipt":verifier_receipt,
                "model_receipts":receipts,
            }));
        }
    }
    if let Some(metadata) = world_compile
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        metadata.insert(
            "elaborations".into(),
            serde_json::Value::Array(elaboration_reports),
        );
    }
    let newspaper_title = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_TITLE")
        .unwrap_or_else(|_| "The Underdeep Clarion".into());
    let newspaper_voice = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_VOICE")
        .unwrap_or_else(|_| {
            "A sharp regional broadsheet for readers who already understand guild politics: skeptical of every throne, attentive to labor and material consequences, formally reported, and capable of one dry local barb without becoming satire."
                .into()
        });
    let started = Instant::now();
    let mut wave_reports = Vec::with_capacity(wave_count);
    if resume {
        for wave_index in 1..=wave_count {
            let checkpoint_path = root.join(format!("wave-{wave_index:02}-checkpoint.json"));
            if !checkpoint_path.is_file() {
                break;
            }
            wave_reports.push(read_checkpoint::<serde_json::Value>(&checkpoint_path)?);
        }
        if campaign.strategic_tick_count != wave_reports.len() as u64 {
            anyhow::bail!(
                "persisted strategic tick count {} does not match {} durable wave checkpoints; refusing to replay a committed wave",
                campaign.strategic_tick_count,
                wave_reports.len()
            )
        }
    }
    let completed_wave_count = wave_reports.len();
    for wave_index in completed_wave_count + 1..=wave_count {
        let previous_news_count = pending_clock_news_start
            .filter(|(next_wave_index, _)| *next_wave_index == wave_index)
            .map(|(_, news_count)| news_count)
            .unwrap_or_else(|| {
                if wave_index == 1 {
                    0
                } else {
                    campaign.news.len()
                }
            });
        let mut rejected_pulses = Vec::new();
        let mut rejected_pulse_count = 0;
        for pulse in 1..=max_rejected_pulses_per_wave + 1 {
            let path = root.join(format!(
                "wave-{wave_index:02}-rejected-pulse-{pulse:02}.json"
            ));
            if !path.is_file() {
                continue;
            }
            rejected_pulses.push(read_checkpoint::<serde_json::Value>(&path)?);
            rejected_pulse_count = pulse;
        }
        let terminal_cell_checkpoint_path = root.join(format!(
            "wave-{wave_index:02}-cell-terminal-checkpoint.json"
        ));
        let mut partial_checkpoint = if terminal_cell_checkpoint_path.is_file() {
            Some(read_checkpoint::<ResolutionWaveCheckpoint>(
                &terminal_cell_checkpoint_path,
            )?)
        } else {
            let (latest_generation, latest) =
                latest_partial_wave_checkpoint::<ResolutionWaveCheckpoint>(
                    &root,
                    wave_index,
                    max_rejected_pulses_per_wave + 1,
                )?;
            rejected_pulse_count = rejected_pulse_count.max(latest_generation);
            latest
        };
        let output = loop {
            let permit = Arc::new(SnapshotPermit::new_resolution(
                store.clone(),
                campaign.id,
                campaign.revision,
                campaign.resolution_policy.resolution_epoch,
            ));
            let attempt = match partial_checkpoint.clone() {
                Some(checkpoint) => {
                    resume_resolution_wave(model.clone(), permit, &campaign, checkpoint).await
                }
                None => propose_resolution_wave(model.clone(), permit, &campaign).await,
            };
            match attempt {
                Ok(output) => break output,
                Err(error) => {
                    let pulse = rejected_pulse_count + 1;
                    let mut resume_checkpoint_path = None;
                    let rejected_stage_receipt_hashes = error
                        .downcast_ref::<ResolutionWavePipelineFailure>()
                        .map(|failure| {
                            store.persist_model_stage_receipts(&failure.stage_receipts)?;
                            if let Some(checkpoint) = &failure.checkpoint {
                                let path = root.join(format!(
                                    "wave-{wave_index:02}-partial-pulse-{pulse:02}.json"
                                ));
                                publish_immutable_checkpoint(&path, checkpoint)?;
                                partial_checkpoint = Some(checkpoint.clone());
                                resume_checkpoint_path = Some(path);
                            }
                            Ok::<_, anyhow::Error>(
                                failure
                                    .stage_receipts
                                    .iter()
                                    .map(|receipt| receipt.storage_key().to_owned())
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .transpose()?
                        .unwrap_or_default();
                    std::fs::write(
                        root.join(format!(
                            "wave-{wave_index:02}-rejected-pulse-{pulse:02}.txt"
                        )),
                        error.to_string(),
                    )?;
                    let rejected_pulse = serde_json::json!({
                        "pulse":pulse,
                        "world_revision":campaign.revision,
                        "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                        "error":error.to_string(),
                        "rejected_stage_receipt_hashes":rejected_stage_receipt_hashes,
                        "resume_checkpoint":resume_checkpoint_path,
                    });
                    publish_immutable_checkpoint(
                        &root.join(format!(
                            "wave-{wave_index:02}-rejected-pulse-{pulse:02}.json"
                        )),
                        &rejected_pulse,
                    )?;
                    if rejected_pulse_count < max_rejected_pulses_per_wave {
                        rejected_pulses.push(rejected_pulse);
                        rejected_pulse_count = pulse;
                    } else {
                        std::fs::write(
                            root.join(format!("wave-{wave_index:02}-terminal-failure.json")),
                            serde_json::to_vec_pretty(&serde_json::json!({
                                "rejected_pulses":rejected_pulses,
                                "terminal_failure":rejected_pulse,
                            }))?,
                        )?;
                        return Err(error);
                    }
                }
            }
        };
        if terminal_cell_checkpoint_path.is_file() {
            let persisted: serde_json::Value = read_checkpoint(&terminal_cell_checkpoint_path)?;
            if persisted != serde_json::to_value(&output.checkpoint)? {
                anyhow::bail!(
                    "persisted terminal cell checkpoint disagrees with resumed scheduler output"
                )
            }
        } else {
            publish_immutable_checkpoint(&terminal_cell_checkpoint_path, &output.checkpoint)?;
        }
        std::fs::write(
            root.join(format!("wave-{wave_index:02}-preflight.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "wave_index":wave_index,
                "world_revision":campaign.revision,
                "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                "cover":&output.wave.cover,
                "appraisals":&output.wave.appraisals,
                "activity_outcomes":&output.wave.activity_outcomes,
                "private_cell_traces":&output.private_cell_traces,
                "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
                "rejected_pulses":&rejected_pulses,
            }))?,
        )?;
        let plan =
            ghostlight_dungeon::resolution::validate_and_resolve_wave(&campaign, &output.wave)?;
        let material_activity_outcomes = plan
            .activity_outcomes
            .iter()
            .filter(|outcome| {
                !matches!(
                    outcome.effect,
                    ghostlight_dungeon::domain::StrategicOutcomeEffect::NoMaterialChange { .. }
                )
            })
            .count();
        if plan.institution_actions.is_empty()
            && plan.gestalt_actions.is_empty()
            && plan.gestalt_migrations.is_empty()
            && plan.actor_moves.is_empty()
            && plan.member_migrations.is_empty()
            && material_activity_outcomes == 0
        {
            anyhow::bail!(
                "strategic wave {wave_index} resolved no material offscreen change: direct transitions were empty and every selected activity outcome was no_material_change"
            );
        }
        store.persist_model_stage_receipts(
            &output
                .stages
                .iter()
                .map(|stage| stage.receipt.clone())
                .collect::<Vec<_>>(),
        )?;
        let committed = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(output.aggregate_receipt_hash.clone()),
                resolution_wave: Some(output.wave.clone()),
            })
            .await?;
        let CommandResult::Committed {
            campaign: advanced, ..
        } = &committed
        else {
            anyhow::bail!("strategic wave {wave_index} did not commit")
        };
        if advanced.actors[&advanced.player_actor_id] != player_before {
            anyhow::bail!("strategic wave {wave_index} puppeted the absent player")
        }
        let mut issue_campaign = advanced.clone();
        issue_campaign.news = advanced.news[previous_news_count..].to_vec();
        if issue_campaign.news.is_empty() {
            anyhow::bail!("strategic wave {wave_index} produced no gated news")
        }
        let issue_composition = compose_persisted_newspaper(
            model.as_ref(),
            &issue_campaign,
            format!("{newspaper_title} — Issue {wave_index}"),
            &newspaper_voice,
            5,
            &store,
        )
        .await;
        let (
            issue,
            newspaper_copy_desk,
            newspaper_press_close,
            newspaper_model_receipts,
            issue_path,
            issue_audit_path,
            newspaper_error,
            newspaper_reconciliation_checkpoint,
        ) = match issue_composition {
            Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Accepted { composition }) => {
                let issue_path = root.join(format!("newspaper-wave-{wave_index:02}.md"));
                let issue_audit_path =
                    root.join(format!("newspaper-wave-{wave_index:02}.audit.md"));
                std::fs::write(
                    &issue_path,
                    ghostlight_dungeon::newspaper::render_world_newspaper_markdown(
                        &composition.issue,
                    ),
                )?;
                std::fs::write(
                    &issue_audit_path,
                    ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(
                        &composition.issue,
                    ),
                )?;
                (
                    Some(composition.issue),
                    Some(composition.copy_desk),
                    Some(composition.press_close),
                    composition.model_receipts,
                    Some(issue_path),
                    Some(issue_audit_path),
                    None,
                    None,
                )
            }
            Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Pending {
                checkpoint,
                model_receipts,
            }) => (
                None,
                None,
                None,
                model_receipts,
                None::<std::path::PathBuf>,
                None::<std::path::PathBuf>,
                None,
                Some(checkpoint),
            ),
            Err(error) => {
                let Some(failure) = error.downcast_ref::<
                    ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure,
                >() else {
                    return Err(error);
                };
                (
                    None,
                    None,
                    None,
                    failure.model_receipts.clone(),
                    None::<std::path::PathBuf>,
                    None::<std::path::PathBuf>,
                    Some(error.to_string()),
                    None,
                )
            }
        };
        let wave_report = serde_json::json!({
            "wave_index":wave_index,
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "world_revision_before":campaign.revision,
            "world_revision_after":advanced.revision,
            "model_receipt_hash":output.aggregate_receipt_hash,
            "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
            "rejected_pulses":rejected_pulses,
            "plan":plan,
            "commit":committed,
            "issue":issue,
            "newspaper_copy_desk":newspaper_copy_desk,
            "newspaper_press_close":newspaper_press_close,
            "newspaper_model_receipts":newspaper_model_receipts,
            "newspaper_error":newspaper_error,
            "newspaper_reconciliation_checkpoint":newspaper_reconciliation_checkpoint,
            "issue_path":issue_path,
            "issue_audit_path":issue_audit_path,
        });
        publish_immutable_checkpoint(
            &root.join(format!("wave-{wave_index:02}-checkpoint.json")),
            &wave_report,
        )?;
        wave_reports.push(wave_report);
        campaign = advanced.clone();
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_status.v1",
                "state":"running",
                "waves_completed":wave_index,
                "waves_requested":wave_count,
                "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                "world_revision":campaign.revision,
                "event_count":campaign.events.len(),
                "news_count":campaign.news.len(),
                "updated_at":Utc::now(),
            }))?,
        )?;
    }
    if resume {
        for report_index in
            missing_newspaper_report_indices(&wave_reports, newspaper_recovery_start_wave)?
        {
            let wave_index = wave_reports[report_index]["wave_index"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("completed wave report has no wave index"))?;
            let issue_campaign = completed_wave_issue_campaign(&wave_reports, report_index)?;
            let issue_title = format!("{newspaper_title} — Issue {wave_index}");
            let source_campaign_digest = strategic_smoke_digest(&issue_campaign)?;
            let newsroom = ghostlight_dungeon::newspaper::canopy_ledger_newsroom();
            let editorial_contract_digest = strategic_smoke_digest(&serde_json::json!({
                "title":&issue_title,
                "editorial_voice":&newspaper_voice,
                "newsroom":&newsroom,
                "max_articles":5,
            }))?;
            let recomposition_path =
                root.join(format!("newspaper-wave-{wave_index:02}-recomposition.json"));
            let recomposition = if recomposition_path.is_file() {
                read_checkpoint::<serde_json::Value>(&recomposition_path)?
            } else {
                if let Some(import_path) = newspaper_reconciliation_import_path.as_deref() {
                    let import =
                        read_checkpoint::<NewspaperReconciliationImportEnvelope>(import_path)?;
                    if import.wave_index == wave_index {
                        if import.schema
                            != "ghostlight.strategic_newspaper_reconciliation_import.v1"
                            || import.recovery_start_wave != newspaper_recovery_start_wave
                            || import.world_revision != issue_campaign.revision
                        {
                            anyhow::bail!(
                                "strategic newspaper reconciliation import does not bind this missing issue"
                            )
                        }
                        ghostlight_dungeon::newspaper::admit_world_newspaper_reconciliation_import(
                            &issue_campaign,
                            &issue_title,
                            &newspaper_voice,
                            &newsroom,
                            5,
                            &store,
                            import.import,
                        )?;
                        newspaper_reconciliation_import_consumed = true;
                    }
                }
                let composition = match compose_persisted_newspaper(
                    model.as_ref(),
                    &issue_campaign,
                    &issue_title,
                    &newspaper_voice,
                    5,
                    &store,
                )
                .await
                {
                    Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Accepted {
                        composition,
                    }) => composition,
                    Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Pending {
                        checkpoint,
                        model_receipts,
                    }) => {
                        let checkpoint_id = checkpoint.id().to_owned();
                        let checkpoint_digest = strategic_smoke_digest(&checkpoint)?;
                        let checkpoint_suffix = checkpoint_digest
                            .strip_prefix("sha256:")
                            .ok_or_else(|| anyhow::anyhow!("invalid checkpoint digest"))?;
                        publish_immutable_checkpoint(
                            &root.join(format!(
                                "newspaper-wave-{wave_index:02}-recomposition-pending-{checkpoint_suffix}.json"
                            )),
                            &serde_json::json!({
                                "schema":"ghostlight.newspaper_wave_recomposition_pending.v1",
                                "wave_index":wave_index,
                                "recovery_start_wave":newspaper_recovery_start_wave,
                                "world_revision":issue_campaign.revision,
                                "source_campaign_digest":source_campaign_digest,
                                "editorial_contract_digest":editorial_contract_digest,
                                "checkpoint_digest":checkpoint_digest,
                                "checkpoint":checkpoint,
                                "model_receipt_set_digest":strategic_smoke_digest(&model_receipts)?,
                                "model_receipts":model_receipts,
                            }),
                        )?;
                        anyhow::bail!(
                            "newspaper reconciliation is pending at checkpoint {}",
                            checkpoint_id
                        )
                    }
                    Err(error) => {
                        let model_receipts = error
                            .downcast_ref::<
                                ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure,
                            >()
                            .map(|failure| failure.model_receipts.clone())
                            .unwrap_or_default();
                        publish_immutable_checkpoint(
                            &root.join(format!(
                                "newspaper-wave-{wave_index:02}-recomposition-terminal-failure.json"
                            )),
                            &serde_json::json!({
                            "schema":"ghostlight.newspaper_wave_recomposition_failure.v1",
                            "wave_index":wave_index,
                            "recovery_start_wave":newspaper_recovery_start_wave,
                            "world_revision":issue_campaign.revision,
                                            "error":error.to_string(),
                                            "model_receipts":model_receipts,
                                        }),
                        )?;
                        return Err(error);
                    }
                };
                let issue_path = root.join(format!("newspaper-wave-{wave_index:02}.md"));
                let issue_audit_path =
                    root.join(format!("newspaper-wave-{wave_index:02}.audit.md"));
                let reader_copy = ghostlight_dungeon::newspaper::render_world_newspaper_markdown(
                    &composition.issue,
                );
                let audit_copy =
                    ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(
                        &composition.issue,
                    );
                std::fs::write(&issue_path, &reader_copy)?;
                std::fs::write(&issue_audit_path, &audit_copy)?;
                let checkpoint = serde_json::json!({
                    "schema":"ghostlight.newspaper_wave_recomposition.v3",
                    "wave_index":wave_index,
                    "recovery_start_wave":newspaper_recovery_start_wave,
                    "world_revision":issue_campaign.revision,
                    "source_campaign_digest":source_campaign_digest,
                    "editorial_contract_digest":editorial_contract_digest,
                    "issue_digest":strategic_smoke_digest(&composition.issue)?,
                    "copy_desk_digest":strategic_smoke_digest(&composition.copy_desk)?,
                    "press_close_digest":strategic_smoke_digest(&composition.press_close)?,
                    "model_receipt_set_digest":strategic_smoke_digest(&composition.model_receipts)?,
                    "reader_copy_digest":strategic_smoke_bytes_digest(reader_copy.as_bytes()),
                    "audit_copy_digest":strategic_smoke_bytes_digest(audit_copy.as_bytes()),
                    "issue":composition.issue,
                    "newspaper_copy_desk":composition.copy_desk,
                    "newspaper_press_close":composition.press_close,
                    "newspaper_model_receipts":composition.model_receipts,
                    "issue_file":format!("newspaper-wave-{wave_index:02}.md"),
                    "issue_audit_file":format!("newspaper-wave-{wave_index:02}.audit.md"),
                });
                publish_immutable_checkpoint(&recomposition_path, &checkpoint)?;
                checkpoint
            };
            let issue_path = root.join(format!("newspaper-wave-{wave_index:02}.md"));
            let issue_audit_path = root.join(format!("newspaper-wave-{wave_index:02}.audit.md"));
            if !issue_path.is_file() || !issue_audit_path.is_file() {
                anyhow::bail!("completed wave newspaper recomposition lost its rendered artifact")
            }
            let reader_copy = std::fs::read_to_string(&issue_path)?;
            let audit_copy = std::fs::read_to_string(&issue_audit_path)?;
            validate_completed_newspaper_recomposition_receipt(
                &recomposition,
                wave_index,
                newspaper_recovery_start_wave,
                wave_reports[report_index]["world_revision_after"]
                    .as_u64()
                    .ok_or_else(|| {
                        anyhow::anyhow!("completed wave report has no committed revision")
                    })?,
                &source_campaign_digest,
                &editorial_contract_digest,
                &reader_copy,
                &audit_copy,
            )?;
            let report = wave_reports[report_index]
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("completed wave report is not an object"))?;
            for field in [
                "issue",
                "newspaper_grounding",
                "newspaper_editorial",
                "newspaper_copy_desk",
                "newspaper_press_close",
                "newspaper_model_receipts",
            ] {
                report.insert(field.into(), recomposition[field].clone());
            }
            report.insert("issue_path".into(), serde_json::to_value(issue_path)?);
            report.insert(
                "issue_audit_path".into(),
                serde_json::to_value(issue_audit_path)?,
            );
            report.insert("newspaper_error".into(), serde_json::Value::Null);
        }
        if newspaper_reconciliation_import_path.is_some()
            && !newspaper_reconciliation_import_consumed
        {
            anyhow::bail!("configured newspaper reconciliation import was not consumed")
        }
    }
    let final_plan = final_wave_field(&wave_reports, "plan")?;
    let final_commit = final_wave_field(&wave_reports, "commit")?;
    let final_model_receipt_hash = final_wave_field(&wave_reports, "model_receipt_hash")?;
    let model_stage_receipts = wave_reports
        .iter()
        .flat_map(|wave| {
            wave["model_stage_receipts"]
                .as_array()
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    let newspaper_composition = match compose_persisted_newspaper(
        model.as_ref(),
        &campaign,
        &newspaper_title,
        &newspaper_voice,
        6,
        &store,
    )
    .await
    {
        Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Accepted { composition }) => {
            composition
        }
        Ok(ghostlight_dungeon::newspaper::WorldNewspaperAdvance::Pending {
            checkpoint,
            model_receipts,
        }) => {
            let checkpoint_id = checkpoint.id().to_owned();
            let pending_result = serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_pending.v1",
                "scenario_id":scenario_id,
                "pressure":pressure,
                "wave_count":wave_count,
                "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                "campaign_id":campaign.id,
                "elapsed_seconds":started.elapsed().as_secs_f64(),
                "model_runtime":model_selection.status("configured"),
                "world_compile":&world_compile,
                "model_receipt_hash":&final_model_receipt_hash,
                "model_stage_receipts":&model_stage_receipts,
                "plan":&final_plan,
                "commit":&final_commit,
                "waves":&wave_reports,
                "event_count":campaign.events.len(),
                "news_count":campaign.news.len(),
                "final_newspaper_checkpoint":checkpoint,
                "final_newspaper_model_receipts":model_receipts,
                "player_location_unchanged":true,
                "player_state_unchanged":true,
                "store":root.join("campaign.cc")
            });
            let result_path = root.join("result.json");
            std::fs::write(&result_path, serde_json::to_vec_pretty(&pending_result)?)?;
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"pending_newspaper_reconciliation",
                    "waves_completed":wave_count,
                    "waves_requested":wave_count,
                    "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                    "world_revision":campaign.revision,
                    "event_count":campaign.events.len(),
                    "news_count":campaign.news.len(),
                    "updated_at":Utc::now(),
                    "result_path":result_path,
                    "newspaper_checkpoint_id":checkpoint_id,
                }))?,
            )?;
            anyhow::bail!("final newspaper reconciliation is pending at checkpoint {checkpoint_id}")
        }
        Err(error) => {
            let final_newspaper_model_receipts = error
                .downcast_ref::<ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure>()
                .map(|failure| failure.model_receipts.clone())
                .unwrap_or_default();
            let failed_result = serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_failure.v1",
                "scenario_id":scenario_id,
                "pressure":pressure,
                "wave_count":wave_count,
                "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                "campaign_id":campaign.id,
                "elapsed_seconds":started.elapsed().as_secs_f64(),
                "model_runtime":model_selection.status("configured"),
                "world_compile":&world_compile,
                "model_receipt_hash":&final_model_receipt_hash,
                "model_stage_receipts":&model_stage_receipts,
                "plan":&final_plan,
                "commit":&final_commit,
                "waves":&wave_reports,
                "event_count":campaign.events.len(),
                "news_count":campaign.news.len(),
                "final_newspaper_error":error.to_string(),
                "final_newspaper_model_receipts":final_newspaper_model_receipts,
                "player_location_unchanged":true,
                "player_state_unchanged":true,
                "store":root.join("campaign.cc")
            });
            let result_path = root.join("result.json");
            std::fs::write(&result_path, serde_json::to_vec_pretty(&failed_result)?)?;
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"failed",
                    "waves_completed":wave_count,
                    "waves_requested":wave_count,
                    "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
                    "world_revision":campaign.revision,
                    "event_count":campaign.events.len(),
                    "news_count":campaign.news.len(),
                    "updated_at":Utc::now(),
                    "result_path":result_path,
                    "newspaper_error":error.to_string(),
                }))?,
            )?;
            return Err(error);
        }
    };
    let newspaper_path = root.join("newspaper.md");
    let newspaper_audit_path = root.join("newspaper.audit.md");
    std::fs::write(
        &newspaper_path,
        ghostlight_dungeon::newspaper::render_world_newspaper_markdown(
            &newspaper_composition.issue,
        ),
    )?;
    std::fs::write(
        &newspaper_audit_path,
        ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(
            &newspaper_composition.issue,
        ),
    )?;
    let result = serde_json::json!({
        "schema":"ghostlight.live_strategic_smoke.v3",
        "scenario_id":scenario_id,
        "pressure":pressure,
        "wave_count":wave_count,
        "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
        "campaign_id":campaign.id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "model_runtime":model_selection.status("configured"),
        "world_compile":world_compile,
        "model_receipt_hash":final_model_receipt_hash,
        "model_stage_receipts":model_stage_receipts,
        "plan":final_plan,
        "commit":final_commit,
        "waves":wave_reports,
        "event_count":campaign.events.len(),
        "news_count":campaign.news.len(),
        "newspaper":newspaper_composition.issue,
        "newspaper_copy_desk":newspaper_composition.copy_desk,
        "newspaper_press_close":newspaper_composition.press_close,
        "newspaper_model_receipts":newspaper_composition.model_receipts,
        "newspaper_path":newspaper_path,
        "newspaper_audit_path":newspaper_audit_path,
        "player_location_unchanged":true,
        "player_state_unchanged":true,
        "store":root.join("campaign.cc")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    std::fs::write(
        root.join("status.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema":"ghostlight.live_strategic_smoke_status.v1",
            "state":"complete",
            "waves_completed":wave_count,
            "waves_requested":wave_count,
            "newspaper_recovery_start_wave":newspaper_recovery_start_wave,
            "world_revision":campaign.revision,
            "event_count":campaign.events.len(),
            "news_count":campaign.news.len(),
            "updated_at":Utc::now(),
            "result_path":root.join("result.json"),
            "newspaper_path":&newspaper_path,
        }))?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn compose_persisted_newspaper(
    model: &dyn ghostlight_dungeon::model::ModelPort,
    campaign: &ghostlight_dungeon::domain::Campaign,
    title: impl Into<String>,
    editorial_voice: &str,
    max_articles: usize,
    store: &ghostlight_dungeon::persistence::CampaignStore,
) -> anyhow::Result<ghostlight_dungeon::newspaper::WorldNewspaperAdvance> {
    ghostlight_dungeon::newspaper::advance_world_newspaper(
        model,
        campaign,
        title,
        editorial_voice,
        &ghostlight_dungeon::newspaper::canopy_ledger_newsroom(),
        max_articles,
        store,
    )
    .await
}

async fn compile_strategic_campaign(
    model: std::sync::Arc<dyn ghostlight_dungeon::model::ModelPort>,
    description: &str,
    pressure: &str,
    public_channel: &str,
) -> anyhow::Result<(
    ghostlight_dungeon::domain::WorldCompilePreview,
    Vec<ghostlight_dungeon::model::ModelStageReceipt>,
)> {
    use ghostlight_dungeon::compiler::{CustomStart, validate_campaign_seed};

    let description = description.trim();
    if description.chars().count() > 8_000 {
        anyhow::bail!("GHOSTLIGHT_WORLD_DESCRIPTION accepts at most 8,000 characters")
    }
    let world_name = std::env::var("GHOSTLIGHT_WORLD_NAME")
        .unwrap_or_else(|_| "The Elven Realms Beyond the Greathold".into());
    let who = std::env::var("GHOSTLIGHT_WORLD_PLAYER").unwrap_or_else(|_| {
        "The player-controlled Greathold, represented by a boundary observer; its sovereign choices remain external to the autonomous world simulation."
            .into()
    });
    let where_ = std::env::var("GHOSTLIGHT_WORLD_WHERE").unwrap_or_else(|_| {
        "the inhabited realms immediately beyond the Greathold boundary described by the supplied setting source"
            .into()
    });
    let when = strategic_world_when();
    let goal = format!(
        "Observe without ruling while the autonomous world responds to this new external pressure from the Greathold: {pressure}"
    );
    let compiler = strategic_world_compiler(model, description, &when);
    let (mut preview, receipts) = compiler
        .compile_custom(CustomStart {
            campaign_name: world_name,
            who,
            where_,
            when,
            goal,
        })
        .await?;
    let campaign = &mut preview.campaign;
    campaign.resolution_policy.active_cell_budget = bounded_environment_usize(
        "GHOSTLIGHT_STRATEGIC_CELL_BUDGET",
        200,
        ghostlight_dungeon::resolution::MIN_ACTIVE_CELL_BUDGET as usize,
        ghostlight_dungeon::resolution::MAX_ACTIVE_CELL_BUDGET as usize,
    )? as u8;
    ghostlight_dungeon::resolution::ensure_agency_profiles(campaign);
    for (subject_id, profile) in &mut campaign.agency_profiles {
        profile.simulation_eligible = subject_id != &campaign.player_actor_id;
        profile.information_channels.insert(public_channel.into());
    }
    for clock in campaign.clocks.values_mut() {
        if clock.consequence_scope.public_channels.is_empty() {
            clock
                .consequence_scope
                .public_channels
                .push(public_channel.into());
        }
    }
    validate_campaign_seed(campaign)?;
    Ok((preview, receipts))
}

fn strategic_world_when() -> String {
    std::env::var("GHOSTLIGHT_WORLD_WHEN").unwrap_or_else(|_| {
        "a strained late age before any single realm has secured hegemony".into()
    })
}

fn strategic_world_compiler(
    model: std::sync::Arc<dyn ghostlight_dungeon::model::ModelPort>,
    description: &str,
    temporal_scope: &str,
) -> ghostlight_dungeon::compiler::WorldCompiler {
    use ghostlight_dungeon::{domain::SourceWitness, vault::FixtureVault};
    use sha2::{Digest, Sha256};

    let witness = SourceWitness {
        source_id: "consumer-setting-description".into(),
        exact_locator: "consumer://setting-description".into(),
        content_hash: format!("sha256:{:x}", Sha256::digest(description.as_bytes())),
        excerpt: description.into(),
        authority_lane: "consumer.setting_description".into(),
        temporal_scope: temporal_scope.into(),
    };
    ghostlight_dungeon::compiler::WorldCompiler::new(
        std::sync::Arc::new(FixtureVault {
            witnesses: vec![witness],
        }),
        model,
        ghostlight_dungeon::model::MODEL_FAST,
        ghostlight_dungeon::model::MODEL_CAPABLE,
    )
}

fn strategic_locality_request(location_name: &str, location_id: &str, pressure: &str) -> String {
    let pressure = pressure.chars().take(60).collect::<String>();
    let request = format!(
        "Elaborate canonical locality {location_name:?} (ID {location_id}) as a politically inhabited jurisdiction. Crisis: {pressure}. Add exactly four non-overlapping resident population leaves and exactly six distinct institutions, never more. Invent authority, succession, revenue, redress, leverage, and opposed interests. Give every new subject a concrete public notice or report channel."
    );
    request.chars().take(500).collect()
}

fn strategic_titled_locality_request(
    location_name: &str,
    location_id: &str,
    pressure: &str,
) -> String {
    let pressure = pressure.chars().take(60).collect::<String>();
    let request = format!(
        "Deepen canonical locality {location_name:?} (ID {location_id}) after its civic foundation has been admitted. Current pressure: {pressure}. Add independently authored texture, material pressure, ordinary life, political leverage, secrets, active instability, and numinous meaning without rewriting the admitted apparatus."
    );
    request.chars().take(500).collect()
}

fn strategic_world_elaboration_profile() -> ghostlight_dungeon::elaboration::WorldElaborationProfile
{
    use ghostlight_dungeon::elaboration::{
        ElaboratorControl, ElaboratorTitle, WorldElaborationProfile,
    };

    WorldElaborationProfile {
        schema: "ghostlight.world_elaboration_profile.v1".into(),
        controls: ElaboratorTitle::ALL
            .into_iter()
            .map(|title| ElaboratorControl {
                title,
                // Patina's bounded child place requires an outward and return
                // route. Every other title receives two operations per pass.
                weight: if title == ElaboratorTitle::Patina {
                    3
                } else {
                    2
                },
            })
            .collect(),
    }
}

fn admitted_public_channel(value: &str) -> anyhow::Result<String> {
    let channel = value.trim();
    if !ghostlight_dungeon::resolution::information_channel_is_concrete(channel) {
        anyhow::bail!("GHOSTLIGHT_STRATEGIC_PUBLIC_CHANNEL is not a concrete information route")
    }
    Ok(channel.into())
}

fn bounded_environment_usize(
    name: &str,
    default: usize,
    minimum: usize,
    maximum: usize,
) -> anyhow::Result<usize> {
    let value = std::env::var(name)
        .ok()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| anyhow::anyhow!("{name} is not an integer: {error}"))?
        .unwrap_or(default);
    if !(minimum..=maximum).contains(&value) {
        anyhow::bail!("{name} must be between {minimum} and {maximum}")
    }
    Ok(value)
}

fn default_runtime_root() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::path::PathBuf::from(r"F:\GameCult\GhostlightDungeon")
    }
    #[cfg(not(windows))]
    {
        std::path::PathBuf::from("/var/lib/gamecult/ghostlight-dungeon")
    }
}

fn strategic_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::{Duration, Utc};
    use ghostlight_dungeon::domain::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn actor(id: &str, name: &str, location_id: &str, goal: &str) -> ActorState {
        ActorState {
            id: id.into(),
            name: name.into(),
            location_id: location_id.into(),
            capabilities: BTreeSet::from(["ordinary travel".into()]),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![goal.into()],
            memories: vec![],
        }
    }
    let now = Utc::now();
    let player = actor(
        "player",
        "Deep-hold Envoy",
        "room",
        "observe without ruling",
    );
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id: uuid::Uuid::new_v4(),
        name: "The Rainless Marches".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations: BTreeMap::from([
            (
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Greathold Boundary Cairn".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            ),
            (
                "depot".into(),
                Location {
                    id: "depot".into(),
                    name: "Rootvault Granary".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "yard".into(),
                        Route {
                            destination_id: "yard".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec![
                        "dry aquifer gauges".into(),
                        "a rain seal's empty reliquary".into(),
                    ],
                },
            ),
            (
                "yard".into(),
                Location {
                    id: "yard".into(),
                    name: "Thornweald Assembly".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "depot".into(),
                        Route {
                            destination_id: "depot".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec![
                        "charcoal tithe scales".into(),
                        "three freshly painted secession banners".into(),
                    ],
                },
            ),
        ]),
        actors: BTreeMap::from([
            ("player".into(), player),
            (
                "runner".into(),
                actor(
                    "runner",
                    "Ilyra Quill",
                    "depot",
                    "find who sold the rain seal",
                ),
            ),
        ]),
        institutions: BTreeMap::from([
            (
                "board".into(),
                InstitutionState {
                    id: "board".into(),
                    name: "Mossglass Regency".into(),
                    resources: vec!["empty rain-seal reliquary".into()],
                    goals: vec!["survive the seal scandal without surrendering the throne".into()],
                    posture: "blaming unnamed caravan clerks".into(),
                },
            ),
            (
                "synod".into(),
                InstitutionState {
                    id: "synod".into(),
                    name: "Copper Synod".into(),
                    resources: vec!["tithe ledgers".into(), "three armed auditors".into()],
                    goals: vec!["make the regency pay for the vanished caravans".into()],
                    posture: "quietly pricing a replacement monarch".into(),
                },
            ),
        ]),
        clocks: BTreeMap::from([(
            "shortage".into(),
            WorldClock {
                id: "shortage".into(),
                label: "White Root succession crisis".into(),
                progress: 1,
                threshold: 4,
                consequence: "the charcoal guilds declare the regent ritually rainless".into(),
                consequence_scope: WorldEventScope {
                    actor_ids: Vec::new(),
                    institution_ids: vec!["board".into(), "synod".into()],
                    gestalt_ids: vec!["workers".into()],
                    location_ids: vec!["yard".into()],
                    public_channels: Vec::new(),
                },
            },
        )]),
        facts: BTreeMap::new(),
        civic_systems: BTreeMap::new(),
        transcript: vec![],
        last_player_activity: now - Duration::hours(2),
        pending_ticks: 1,
        away_ticks_processed: 0,
        events: vec![],
        news: vec![],
        canon_candidates: BTreeMap::new(),
        gestalts: BTreeMap::from([(
            "workers".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "workers".into(),
                name: "Thornweald Charcoal Guilds".into(),
                version: 0,
                home_location_id: "yard".into(),
                shared_capabilities: BTreeSet::from(["close every forest kiln at once".into()]),
                shared_knowledge: BTreeSet::from([
                    "two tithe caravans vanished after the aquifer diversion".into(),
                    "the regent's rain seal is missing".into(),
                ]),
                resources: BTreeSet::from(["assembly grove".into()]),
                goals: vec!["replace tithe tribute with an elected water compact".into()],
                pressures: vec!["three guilds have already painted secession banners".into()],
            },
        )]),
        gestalt_members: BTreeMap::new(),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::from([
            (
                "regency-synod-rivalry".into(),
                AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: "regency-synod-rivalry".into(),
                    from_subject_id: "board".into(),
                    to_subject_id: "synod".into(),
                    kind: AgencyRelationKind::Rivalry,
                    strength: 86,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            ),
            (
                "synod-guild-command".into(),
                AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: "synod-guild-command".into(),
                    from_subject_id: "synod".into(),
                    to_subject_id: "workers".into(),
                    kind: AgencyRelationKind::Command,
                    strength: 63,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            ),
        ]),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: Default::default(),
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        nemesis_attention_history: Vec::new(),
        strategic_tick_count: 0,
    };
    ghostlight_dungeon::resolution::ensure_agency_profiles(&mut campaign);
    for profile in campaign.agency_profiles.values_mut() {
        profile
            .information_channels
            .insert("root-wire broadsheet".into());
    }
    for institution_id in ["board", "synod"] {
        let profile = campaign
            .agency_profiles
            .get_mut(institution_id)
            .expect("fixture institution has a profile");
        profile.location_ids.extend(["depot".into(), "yard".into()]);
        profile
            .facets
            .entry(AgencyAxis::Geography)
            .or_default()
            .extend(["depot".into(), "yard".into()]);
    }
    campaign
}

#[cfg(test)]
mod tests {
    use super::{
        HistoricalWorldNewspaperArticleV2, HistoricalWorldNewspaperIssueV2,
        admitted_public_channel, civic_manifest_is_committed_candidate,
        committed_elaboration_mutation_proof, completed_wave_issue_campaign, final_wave_field,
        latest_partial_wave_checkpoint, missing_newspaper_report_indices,
        publish_immutable_checkpoint, recomposed_model_receipt_set_digest,
        recover_committed_clock_binding, strategic_campaign, strategic_locality_request,
        strategic_smoke_bytes_digest, strategic_smoke_digest, strategic_titled_locality_request,
        titled_failure_checkpoint_paths, validate_completed_newspaper_recomposition_receipt,
    };

    fn civic_manifest(
        version: u64,
        verifier: &str,
    ) -> ghostlight_dungeon::domain::CivicSystemManifest {
        use std::collections::BTreeSet;

        ghostlight_dungeon::domain::CivicSystemManifest {
            schema: "ghostlight.civic_system_manifest.v1".into(),
            version,
            jurisdiction_location_id: "room".into(),
            governing_institution_ids: BTreeSet::from(["council".into()]),
            resident_population_ids: BTreeSet::from(["residents".into()]),
            public_authority_fact_ids: BTreeSet::from(["authority".into()]),
            public_selection_fact_ids: BTreeSet::from(["selection".into()]),
            public_resource_fact_ids: BTreeSet::from(["resources".into()]),
            public_redress_fact_ids: BTreeSet::from(["redress".into()]),
            political_relation_ids: BTreeSet::from(["relation".into()]),
            semantic_verification_receipt_id: verifier.into(),
        }
    }

    #[test]
    fn top_level_projection_uses_the_final_wave_head() {
        let waves = vec![
            serde_json::json!({"commit":{"campaign":{"revision":1}}}),
            serde_json::json!({"commit":{"campaign":{"revision":2}}}),
        ];

        let commit = final_wave_field(&waves, "commit").unwrap();
        assert_eq!(commit["campaign"]["revision"], 2);
    }

    #[test]
    fn newspaper_recovery_boundary_skips_history_and_successful_issues() {
        let reports = vec![
            serde_json::json!({"wave_index":1,"issue":null}),
            serde_json::json!({"wave_index":2,"issue":{"id":"accepted"}}),
            serde_json::json!({"wave_index":3,"issue":null}),
        ];

        assert_eq!(
            missing_newspaper_report_indices(&reports, 1).unwrap(),
            vec![0, 2]
        );
        assert_eq!(
            missing_newspaper_report_indices(&reports, 2).unwrap(),
            vec![2]
        );
        assert_eq!(
            missing_newspaper_report_indices(&reports, 3).unwrap(),
            vec![2]
        );
        assert!(
            missing_newspaper_report_indices(&reports, 4)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn recomposition_receipt_rehashes_typed_model_receipts() {
        let receipts = vec![ghostlight_dungeon::model::ModelStageReceipt {
            schema: "ghostlight.model_stage_receipt.v1".into(),
            receipt_hash: "sha256:receipt".into(),
            provider: "fixture".into(),
            model: "fixture-model".into(),
            stage: "newspaper_editor".into(),
            snapshot_binding: "sha256:snapshot".into(),
            request_hash: "sha256:request".into(),
            output_hash: "sha256:output".into(),
            source_receipt_ids: vec!["sha256:source".into()],
            latency_ms: 1,
            validation_result: "accepted".into(),
            local_validation_error: None,
            input_chars: 10,
            output_chars: 20,
            provider_attempts: Vec::new(),
        }];
        let value = serde_json::to_value(&receipts).unwrap();

        assert_eq!(
            recomposed_model_receipt_set_digest(&value).unwrap(),
            strategic_smoke_digest(&receipts).unwrap()
        );
        assert!(recomposed_model_receipt_set_digest(&serde_json::json!({})).is_err());
    }

    #[test]
    fn historical_recomposition_keeps_its_original_boundary_and_schema() {
        let issue = HistoricalWorldNewspaperIssueV2 {
            schema: "ghostlight.world_newspaper_issue.v2".into(),
            id: "issue:18".into(),
            title: "The Canopy Ledger — Issue 18".into(),
            edition_label: "Late Edition".into(),
            at: chrono::DateTime::parse_from_rfc3339("2026-08-29T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            source_world_revision: 27,
            lead_article_id: Some("article:lead".into()),
            articles: vec![HistoricalWorldNewspaperArticleV2 {
                id: "article:lead".into(),
                section: "Front Page".into(),
                headline: "The roots answer".into(),
                deck: "A historical accepted issue remains historical.".into(),
                byline: "Staff".into(),
                dateline: Some("Sinkroot".into()),
                paragraphs: vec!["The exact old copy survives.".into()],
                source_news_ids: vec!["news:18".into()],
                source_channels: vec!["canopy wire broadsheet".into()],
                source_reliability: vec!["committed public channel".into()],
                event_ids: vec!["event:18".into()],
            }],
            editorial_receipt_ids: vec!["sha256:editor".into()],
        };
        let reader_copy = "# The Canopy Ledger — Issue 18\n";
        let audit_copy = "# Provenance\n";
        let receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt> = Vec::new();
        let recomposition = serde_json::json!({
            "schema":"ghostlight.newspaper_wave_recomposition.v1",
            "wave_index":18,
            "recovery_start_wave":13,
            "world_revision":27,
            "source_campaign_digest":"sha256:campaign",
            "editorial_contract_digest":"sha256:contract",
            "issue_digest":strategic_smoke_digest(&issue).unwrap(),
            "model_receipt_set_digest":strategic_smoke_digest(&receipts).unwrap(),
            "reader_copy_digest":strategic_smoke_bytes_digest(reader_copy.as_bytes()),
            "audit_copy_digest":strategic_smoke_bytes_digest(audit_copy.as_bytes()),
            "issue":issue,
            "newspaper_grounding":{"accepted":true},
            "newspaper_model_receipts":receipts,
            "issue_file":"newspaper-wave-18.md",
            "issue_audit_file":"newspaper-wave-18.audit.md",
        });

        validate_completed_newspaper_recomposition_receipt(
            &recomposition,
            18,
            18,
            27,
            "sha256:campaign",
            "sha256:contract",
            reader_copy,
            audit_copy,
        )
        .unwrap();

        let mut changed_copy = reader_copy.to_owned();
        changed_copy.push_str("tampered");
        assert!(
            validate_completed_newspaper_recomposition_receipt(
                &recomposition,
                18,
                18,
                27,
                "sha256:campaign",
                "sha256:contract",
                &changed_copy,
                audit_copy,
            )
            .is_err()
        );
    }

    #[test]
    fn v2_recomposition_requires_both_historical_review_verdicts_and_rerenders() {
        let issue = ghostlight_dungeon::newspaper::WorldNewspaperIssue {
            schema: "ghostlight.world_newspaper_issue.v3".into(),
            id: "issue:current".into(),
            title: "The Canopy Ledger".into(),
            edition_label: "Final Edition".into(),
            at: chrono::DateTime::parse_from_rfc3339("2026-08-29T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            source_world_revision: 27,
            lead_article_id: None,
            editorial_agenda: None,
            articles: Vec::new(),
            editorial_receipt_ids: vec!["sha256:editor".into()],
        };
        let reader_copy = ghostlight_dungeon::newspaper::render_world_newspaper_markdown(&issue);
        let audit_copy =
            ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(&issue);
        let receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt> = Vec::new();
        let grounding = ghostlight_dungeon::newspaper::WorldNewspaperGroundingVerdict {
            accepted: true,
            assessment: "Exact current grounding".into(),
            findings: Vec::new(),
        };
        let editorial = ghostlight_dungeon::newspaper::WorldNewspaperEditorialVerdict {
            accepted: true,
            assessment: "Exact current editorial acceptance".into(),
            findings: Vec::new(),
        };
        let recomposition = serde_json::json!({
            "schema":"ghostlight.newspaper_wave_recomposition.v2",
            "wave_index":18,
            "recovery_start_wave":18,
            "world_revision":27,
            "source_campaign_digest":"sha256:campaign",
            "editorial_contract_digest":"sha256:contract",
            "issue_digest":strategic_smoke_digest(&issue).unwrap(),
            "grounding_digest":strategic_smoke_digest(&grounding).unwrap(),
            "editorial_digest":strategic_smoke_digest(&editorial).unwrap(),
            "model_receipt_set_digest":strategic_smoke_digest(&receipts).unwrap(),
            "reader_copy_digest":strategic_smoke_bytes_digest(reader_copy.as_bytes()),
            "audit_copy_digest":strategic_smoke_bytes_digest(audit_copy.as_bytes()),
            "issue":issue,
            "newspaper_grounding":grounding,
            "newspaper_editorial":editorial,
            "newspaper_model_receipts":receipts,
            "issue_file":"newspaper-wave-18.md",
            "issue_audit_file":"newspaper-wave-18.audit.md",
        });

        validate_completed_newspaper_recomposition_receipt(
            &recomposition,
            18,
            18,
            27,
            "sha256:campaign",
            "sha256:contract",
            &reader_copy,
            &audit_copy,
        )
        .unwrap();
        let mut legacy_current = recomposition.clone();
        legacy_current["schema"] = serde_json::json!("ghostlight.newspaper_wave_recomposition.v1");
        legacy_current
            .as_object_mut()
            .unwrap()
            .remove("editorial_digest");
        legacy_current
            .as_object_mut()
            .unwrap()
            .remove("newspaper_editorial");
        validate_completed_newspaper_recomposition_receipt(
            &legacy_current,
            18,
            18,
            27,
            "sha256:campaign",
            "sha256:contract",
            &reader_copy,
            &audit_copy,
        )
        .unwrap();
        assert!(
            validate_completed_newspaper_recomposition_receipt(
                &recomposition,
                18,
                18,
                27,
                "sha256:campaign",
                "sha256:contract",
                "# A different paper\n",
                &audit_copy,
            )
            .is_err()
        );
        let mut changed_editorial = recomposition.clone();
        changed_editorial["newspaper_editorial"]["assessment"] =
            serde_json::json!("Changed after admission");
        assert!(
            validate_completed_newspaper_recomposition_receipt(
                &changed_editorial,
                18,
                18,
                27,
                "sha256:campaign",
                "sha256:contract",
                &reader_copy,
                &audit_copy,
            )
            .is_err()
        );
        let mut changed_grounding = recomposition.clone();
        changed_grounding["newspaper_grounding"]["assessment"] =
            serde_json::json!("Changed after admission");
        assert!(
            validate_completed_newspaper_recomposition_receipt(
                &changed_grounding,
                18,
                18,
                27,
                "sha256:campaign",
                "sha256:contract",
                &reader_copy,
                &audit_copy,
            )
            .is_err()
        );
    }

    #[test]
    fn v3_recomposition_binds_copy_report_and_press_close_without_a_post_close_verdict() {
        let issue = ghostlight_dungeon::newspaper::WorldNewspaperIssue {
            schema: "ghostlight.world_newspaper_issue.v3".into(),
            id: "issue:v3".into(),
            title: "The Canopy Ledger".into(),
            edition_label: "Current Edition".into(),
            at: chrono::DateTime::parse_from_rfc3339("2026-08-29T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            source_world_revision: 27,
            lead_article_id: None,
            editorial_agenda: None,
            articles: Vec::new(),
            editorial_receipt_ids: Vec::new(),
        };
        let copy_desk = ghostlight_dungeon::newspaper::WorldNewspaperCopyDeskReport {
            assessment: "The copy desk marked no factual queries.".into(),
            queries: Vec::new(),
        };
        let press_close = ghostlight_dungeon::newspaper::WorldNewspaperPressClose {
            schema: "ghostlight.world_newspaper_press_close.v1".into(),
            source_checkpoint_id: "newspaper-close:fixture".into(),
            copy_desk_receipt_id: "sha256:copy".into(),
            night_editor_receipt_id: "sha256:night".into(),
            night_editor_action_applied: true,
            addressed_query_indices: Vec::new(),
            changed_article_indices: Vec::new(),
            source_page_digest: format!("sha256:{}", "a".repeat(64)),
            printed_page_digest: format!("sha256:{}", "a".repeat(64)),
        };
        let reader_copy = ghostlight_dungeon::newspaper::render_world_newspaper_markdown(&issue);
        let audit_copy =
            ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(&issue);
        let receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt> = Vec::new();
        let recomposition = serde_json::json!({
            "schema":"ghostlight.newspaper_wave_recomposition.v3",
            "wave_index":18,
            "recovery_start_wave":18,
            "world_revision":27,
            "source_campaign_digest":"sha256:campaign",
            "editorial_contract_digest":"sha256:contract",
            "issue_digest":strategic_smoke_digest(&issue).unwrap(),
            "copy_desk_digest":strategic_smoke_digest(&copy_desk).unwrap(),
            "press_close_digest":strategic_smoke_digest(&press_close).unwrap(),
            "model_receipt_set_digest":strategic_smoke_digest(&receipts).unwrap(),
            "reader_copy_digest":strategic_smoke_bytes_digest(reader_copy.as_bytes()),
            "audit_copy_digest":strategic_smoke_bytes_digest(audit_copy.as_bytes()),
            "issue":issue,
            "newspaper_copy_desk":copy_desk,
            "newspaper_press_close":press_close,
            "newspaper_model_receipts":receipts,
            "issue_file":"newspaper-wave-18.md",
            "issue_audit_file":"newspaper-wave-18.audit.md",
        });

        validate_completed_newspaper_recomposition_receipt(
            &recomposition,
            18,
            18,
            27,
            "sha256:campaign",
            "sha256:contract",
            &reader_copy,
            &audit_copy,
        )
        .unwrap();
        assert!(recomposition["newspaper_grounding"].is_null());
        assert!(recomposition["newspaper_editorial"].is_null());
        let mut changed_close = recomposition.clone();
        changed_close["newspaper_press_close"]["night_editor_receipt_id"] =
            serde_json::json!("sha256:changed");
        assert!(
            validate_completed_newspaper_recomposition_receipt(
                &changed_close,
                18,
                18,
                27,
                "sha256:campaign",
                "sha256:contract",
                &reader_copy,
                &audit_copy,
            )
            .is_err()
        );
    }

    #[test]
    fn completed_wave_newspaper_resume_uses_only_that_waves_committed_news() {
        let mut first = strategic_campaign();
        first.revision = 1;
        let first_event = ghostlight_dungeon::domain::Event {
            id: "event:first".into(),
            at: first.world_time,
            kind: "fixture".into(),
            summary: "The first event entered the public ledger.".into(),
            actor_ids: Vec::new(),
            institution_ids: vec!["board".into()],
            gestalt_ids: Vec::new(),
            location_ids: vec!["yard".into()],
            public_channels: vec!["root-wire broadsheet".into()],
        };
        ghostlight_dungeon::domain::append_event_with_publications(&mut first, first_event);
        let mut second = first.clone();
        second.revision = 2;
        let second_event = ghostlight_dungeon::domain::Event {
            id: "event:second".into(),
            at: second.world_time,
            kind: "fixture".into(),
            summary: "The second event entered the public ledger.".into(),
            actor_ids: Vec::new(),
            institution_ids: vec!["synod".into()],
            gestalt_ids: Vec::new(),
            location_ids: vec!["depot".into()],
            public_channels: vec!["root-wire broadsheet".into()],
        };
        ghostlight_dungeon::domain::append_event_with_publications(&mut second, second_event);
        let reports = vec![
            serde_json::json!({
                "wave_index":1,
                "world_revision_after":1,
                "commit":{"campaign":first},
            }),
            serde_json::json!({
                "wave_index":2,
                "world_revision_after":2,
                "commit":{"campaign":second},
            }),
        ];

        let issue_campaign = completed_wave_issue_campaign(&reports, 1).unwrap();

        assert_eq!(issue_campaign.revision, 2);
        assert_eq!(issue_campaign.events.len(), 2);
        assert_eq!(issue_campaign.news.len(), 1);
        assert!(
            issue_campaign.news[0]
                .event_ids
                .contains(&"event:second".into())
        );

        let mut tampered_prefix = reports.clone();
        tampered_prefix[0]["commit"]["campaign"]["news"][0]["headline"] =
            "A different earlier publication".into();
        assert!(
            completed_wave_issue_campaign(&tampered_prefix, 1)
                .unwrap_err()
                .to_string()
                .contains("does not preserve its exact prior prefix")
        );
        assert!(
            completed_wave_issue_campaign(&reports, 0)
                .unwrap_err()
                .to_string()
                .contains("cannot recover its exact pre-wave news boundary")
        );
    }

    #[test]
    fn public_channel_requires_one_concrete_information_route() {
        assert_eq!(
            admitted_public_channel("  root-wire broadsheet  ").unwrap(),
            "root-wire broadsheet"
        );
        for invalid in ["", "   ", "unknown"] {
            assert!(admitted_public_channel(invalid).is_err());
        }
        assert!(admitted_public_channel(&"x".repeat(161)).is_err());
    }

    #[test]
    fn missing_post_commit_checkpoint_is_rebuilt_from_cultcache() {
        let directory = tempfile::tempdir().unwrap();
        let store = ghostlight_dungeon::persistence::CampaignStore::open(
            directory.path().join("campaign.cc"),
        )
        .unwrap();
        let mut campaign = strategic_campaign();
        campaign.revision = 15;
        campaign.strategic_tick_count = 6;
        campaign
            .clocks
            .get_mut("shortage")
            .unwrap()
            .consequence_scope
            .public_channels = vec!["root-wire broadsheet".into()];
        let event = ghostlight_dungeon::domain::Event {
            id: "clock-consequence:shortage".into(),
            at: campaign.world_time,
            kind: "clock_consequence".into(),
            summary: campaign.clocks["shortage"].consequence.clone(),
            actor_ids: Vec::new(),
            institution_ids: vec!["board".into(), "synod".into()],
            gestalt_ids: vec!["workers".into()],
            location_ids: vec!["yard".into()],
            public_channels: vec!["root-wire broadsheet".into()],
        };
        ghostlight_dungeon::domain::append_event_with_publications(&mut campaign, event);
        let committed_at = chrono::Utc::now();
        let binding_receipt = ghostlight_dungeon::clock::ClockConsequenceBindingReceipt {
            schema: "ghostlight.clock_consequence_binding_receipt.v1".into(),
            campaign_id: campaign.id,
            previous_revision: 14,
            revision: 15,
            snapshot_binding: "fixture-snapshot".into(),
            binding_batch_digest: "sha256:fixture-batch".into(),
            bindings: vec![ghostlight_dungeon::clock::ClockConsequenceBinding {
                clock_id: "shortage".into(),
                scope: campaign.clocks["shortage"].consequence_scope.clone(),
            }],
            model_receipt_ids: vec!["sha256:fixture-model".into()],
            accepted_model_receipt_id: "sha256:fixture-model".into(),
            emitted_event_ids: vec!["clock-consequence:shortage".into()],
            emitted_news_ids: vec![ghostlight_dungeon::domain::event_publication_id(
                "clock-consequence:shortage",
                "root-wire broadsheet",
            )],
            news_count_before: 0,
            next_wave_index: 7,
            committed_at,
        };
        let world_receipt = ghostlight_dungeon::domain::WorldCommitReceipt {
            schema: "ghostlight.world_commit_receipt.v1".into(),
            campaign_id: campaign.id,
            previous_revision: 14,
            revision: 15,
            command_kind: "bind_clock_consequences".into(),
            committed_at,
            roll: None,
        };
        let key = format!("{}-15", campaign.id);
        store
            .insert(
                "clock_consequence_binding_receipt.v1",
                "ghostlight.clock_consequence_binding_receipt.v1",
                &key,
                &binding_receipt,
            )
            .unwrap();
        store
            .insert(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                &key,
                &world_receipt,
            )
            .unwrap();
        let checkpoint_path = directory.path().join("clock-consequence-binding.json");

        let recovered = recover_committed_clock_binding(&store, &campaign, &checkpoint_path)
            .unwrap()
            .unwrap();

        assert!(checkpoint_path.is_file());
        assert_eq!(recovered.next_wave_index, 7);
        assert_eq!(recovered.news_count_before, 0);
        assert_eq!(
            recovered.emitted_news_ids,
            [ghostlight_dungeon::domain::event_publication_id(
                "clock-consequence:shortage",
                "root-wire broadsheet"
            )]
        );
    }

    #[test]
    fn locality_elaboration_request_names_the_existing_place_and_stays_bounded() {
        let request = strategic_locality_request(
            "Seed Vault",
            "loc-seed-vault",
            &"an intricately witnessed constitutional crisis ".repeat(40),
        );

        assert!(request.contains("Seed Vault"));
        assert!(request.contains("loc-seed-vault"));
        assert!(request.contains("exactly four non-overlapping resident population leaves"));
        assert!(request.contains("exactly six distinct institutions"));
        assert!(request.contains("authority, succession, revenue, redress"));
        assert!(request.contains("public notice or report channel"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn titled_elaboration_request_owns_only_the_additive_pass() {
        let request = strategic_titled_locality_request(
            "Seed Vault",
            "loc-seed-vault",
            &"an intricately witnessed constitutional crisis ".repeat(40),
        );

        assert!(request.contains("Seed Vault"));
        assert!(request.contains("loc-seed-vault"));
        assert!(request.contains("after its civic foundation has been admitted"));
        assert!(request.contains("ordinary life, political leverage, secrets"));
        assert!(!request.contains("exactly four"));
        assert!(!request.contains("exactly six"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn static_fixture_obeys_the_same_channel_and_knowledge_invariant() {
        let campaign = strategic_campaign();
        ghostlight_dungeon::compiler::validate_campaign_seed(&campaign).unwrap();
        let player = &campaign.actors[&campaign.player_actor_id];
        assert!(!player.knowledge.contains("root-wire broadsheet"));
        assert!(
            campaign.agency_profiles[&campaign.player_actor_id]
                .information_channels
                .contains("root-wire broadsheet")
        );
    }

    #[test]
    fn a_precommit_titled_preview_cannot_masquerade_as_a_committed_candidate() {
        let foundation_only = civic_manifest(1, "foundation-verifier");
        let titled_candidate = civic_manifest(2, "");
        let committed_titled = civic_manifest(2, "titled-verifier");

        assert!(!civic_manifest_is_committed_candidate(
            &foundation_only,
            &titled_candidate
        ));
        assert!(civic_manifest_is_committed_candidate(
            &committed_titled,
            &titled_candidate
        ));
    }

    #[test]
    fn legacy_completion_inference_uses_the_kernel_mutation_proof() {
        use ghostlight_dungeon::domain::{Location, RegionExpansion, Route, WorldCommitReceipt};
        use std::collections::BTreeMap;

        let campaign = strategic_campaign();
        let child = ghostlight_dungeon::domain::Location {
            id: "patina-child".into(),
            name: "The Duck Gate".into(),
            container_id: Some("room".into()),
            routes: BTreeMap::from([(
                "back".into(),
                Route {
                    destination_id: "room".into(),
                    distance: "a short path".into(),
                    travel_minutes: 3,
                },
            )]),
            persistent_features: vec![
                "Three petition ribbons tied around its neck.".into(),
                "A bronze duck called Harold.".into(),
            ],
        };
        let expansion = RegionExpansion {
            origin_location_id: "room".into(),
            origin_routes: BTreeMap::from([(
                "to-duck-gate".into(),
                Route {
                    destination_id: child.id.clone(),
                    distance: "a short path".into(),
                    travel_minutes: 3,
                },
            )]),
            locations: vec![child.clone()],
            facts: Vec::new(),
            populations: Vec::new(),
            population_profiles: Vec::new(),
            migration_relations: Vec::new(),
            institutions: Vec::new(),
            institution_profiles: Vec::new(),
            local_relations: Vec::new(),
            civic_system: None,
        };
        let committed_at = chrono::Utc::now();
        let transition = ghostlight_dungeon::legacy_transition::lower_region_expansion(
            &campaign,
            &expansion,
            committed_at + chrono::Duration::minutes(5),
        )
        .unwrap();
        let mut projected = campaign.clone();
        let mutation_receipt =
            ghostlight_dungeon::legacy_transition::apply_lowered_region_expansion(
                &mut projected,
                &expansion,
                &transition,
                committed_at,
            )
            .unwrap();
        assert_ne!(projected.locations[&child.id], child);

        let world_receipt = WorldCommitReceipt {
            schema: "ghostlight.world_commit_receipt.v1".into(),
            campaign_id: campaign.id,
            previous_revision: campaign.revision,
            revision: campaign.revision + 1,
            command_kind: "elaborate_locality".into(),
            committed_at,
            roll: None,
        };
        let directory = tempfile::tempdir().unwrap();
        let store = ghostlight_dungeon::persistence::CampaignStore::open(
            directory.path().join("campaign.cc"),
        )
        .unwrap();
        store
            .insert(
                "mutation_authority_envelope.v1",
                "ghostlight.mutation_authority_envelope.v1",
                &transition.authority.id,
                &transition.authority,
            )
            .unwrap();
        store
            .insert(
                "world_mutation_batch.v1",
                "ghostlight.world_mutation_batch.v1",
                &transition.batch.id,
                &transition.batch,
            )
            .unwrap();
        store
            .insert(
                "world_mutation_receipt.v1",
                "ghostlight.world_mutation_receipt.v1",
                &mutation_receipt.id,
                &mutation_receipt,
            )
            .unwrap();

        let proof = committed_elaboration_mutation_proof(&store, &world_receipt, &expansion)
            .expect("the persisted mutation bundle owns historical completion");
        assert_eq!(proof.batch_id, transition.batch.id);
        assert_eq!(proof.mutation_receipt_id, mutation_receipt.id);

        let mut uncommitted = expansion.clone();
        uncommitted.locations.push(Location {
            id: "uncommitted-place".into(),
            name: "The Imaginary Annex".into(),
            container_id: Some("room".into()),
            routes: Default::default(),
            persistent_features: Vec::new(),
        });
        assert!(
            committed_elaboration_mutation_proof(&store, &world_receipt, &uncommitted).is_err()
        );
    }

    #[test]
    fn checkpoint_publication_is_immutable_and_ignores_unpublished_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory
            .path()
            .join("titled-elaboration-02-terminal-failure.json");
        let resume = directory
            .path()
            .join("titled-elaboration-02-resume-01-terminal-failure.json");
        publish_immutable_checkpoint(&original, &serde_json::json!({"generation":0})).unwrap();
        publish_immutable_checkpoint(&resume, &serde_json::json!({"generation":1})).unwrap();
        std::fs::write(
            directory
                .path()
                .join(".titled-elaboration-02-resume-02-terminal-failure.json.dead.tmp"),
            b"truncated",
        )
        .unwrap();

        let paths = titled_failure_checkpoint_paths(directory.path(), 2).unwrap();

        assert_eq!(paths, vec![original.clone(), resume.clone()]);
        assert!(
            publish_immutable_checkpoint(&resume, &serde_json::json!({"generation":2})).is_err()
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(resume).unwrap()).unwrap(),
            serde_json::json!({"generation":1})
        );
    }

    #[test]
    fn orphan_partial_wave_checkpoint_survives_missing_rejection_summary() {
        let directory = tempfile::tempdir().unwrap();
        let partial = directory.path().join("wave-01-partial-pulse-01.json");
        publish_immutable_checkpoint(
            &partial,
            &serde_json::json!({"typed_cell_terminals":["cell-accepted"]}),
        )
        .unwrap();
        assert!(
            !directory
                .path()
                .join("wave-01-rejected-pulse-01.json")
                .exists()
        );

        let (generation, checkpoint) =
            latest_partial_wave_checkpoint::<serde_json::Value>(directory.path(), 1, 3).unwrap();
        assert_eq!(generation, 1);
        assert_eq!(
            checkpoint.unwrap()["typed_cell_terminals"][0],
            "cell-accepted"
        );
    }
}
