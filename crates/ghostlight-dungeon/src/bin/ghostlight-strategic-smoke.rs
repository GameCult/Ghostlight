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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalWorldNewspaperGroundingVerdict {
    accepted: bool,
    assessment: String,
    findings: Vec<ghostlight_dungeon::newspaper::WorldNewspaperGroundingFinding>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalWorldNewspaperEditorialVerdict {
    accepted: bool,
    assessment: String,
    findings: Vec<HistoricalWorldNewspaperEditorialFinding>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalWorldNewspaperEditorialFinding {
    article_index: u16,
    category: HistoricalWorldNewspaperEditorialCategory,
    passage: String,
    reason: String,
    rewrite_goal: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum HistoricalWorldNewspaperEditorialCategory {
    BuriedLede,
    ProceduralForeground,
    ThroughlineDropped,
    StakesAbstracted,
    TensionFlattened,
    RepetitiveUpdate,
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
        let grounding: HistoricalWorldNewspaperGroundingVerdict =
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
            let editorial: HistoricalWorldNewspaperEditorialVerdict =
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

#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct FoundationCommitCheckpoint {
    schema: String,
    location_id: String,
    location_name: String,
    request: String,
    world_revision_before: u64,
    world_revision_after: u64,
    model_receipt_hashes: Vec<String>,
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
    mutation_proof: TitledMutationProof,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldRegionPlanCheckpoint {
    schema: String,
    origin_location_id: String,
    requests: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldRegionPreviewCheckpoint {
    schema: String,
    request: String,
    preview: ghostlight_dungeon::domain::RegionExpansionPreview,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldRegionCommitCheckpoint {
    schema: String,
    request: String,
    origin_location_id: String,
    jurisdiction_location_id: String,
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
    model_receipt_hashes: Vec<String>,
    mutation_proof: TitledMutationProof,
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
    #[serde(default)]
    semantic_retry_diagnostic: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledReadyCheckpoint {
    schema: String,
    location_id: String,
    location_name: String,
    request: String,
    finalized: ghostlight_dungeon::elaboration::FinalizedWorldElaboration,
    #[serde(default)]
    resumed_from: Option<std::path::PathBuf>,
    #[serde(default)]
    retried_dispatch_ordinals: Vec<u64>,
    #[serde(default)]
    semantic_retry_diagnostic: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledSemanticFailureCheckpoint {
    schema: String,
    attempt: u8,
    location_id: String,
    location_name: String,
    base_request: String,
    verification_request: String,
    repair_request: Option<String>,
    diagnostic: String,
    admission: ghostlight_dungeon::elaboration::WorldElaborationAdmission,
    wave: ghostlight_dungeon::elaboration::ElaborationWaveBinding,
    schedule: ghostlight_dungeon::elaboration::ElaborationScheduleReceipt,
    model_receipt_hashes: Vec<String>,
    verifier_receipt_hashes: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TitledVerifierExecutionFailureCheckpoint {
    schema: String,
    semantic_attempt: u8,
    generation: u32,
    location_id: String,
    location_name: String,
    verification_request: String,
    diagnostic: String,
    attempts: u8,
    admission: ghostlight_dungeon::elaboration::WorldElaborationAdmission,
    model_receipt_hashes: Vec<String>,
    verifier_receipt_hashes: Vec<String>,
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

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComplexityPreviewInvocation {
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    parent_binding: String,
    proposal: ghostlight_dungeon::elaboration::WorldComplexityProposal,
    model_receipt_hashes: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComplexityRoundPreviewCheckpoint {
    schema: String,
    round: u32,
    demand: ghostlight_dungeon::elaboration::WorldElaborationDemand,
    frozen_world_revision: u64,
    wave: ghostlight_dungeon::elaboration::ElaborationWaveBinding,
    schedule: ghostlight_dungeon::elaboration::ElaborationScheduleReceipt,
    invocations: Vec<ComplexityPreviewInvocation>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComplexityFailedInvocation {
    dispatch: Option<ghostlight_dungeon::elaboration::ElaborationDispatch>,
    diagnostic: String,
    model_receipt_hashes: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComplexityRoundFailureCheckpoint {
    schema: String,
    round: u32,
    demand: ghostlight_dungeon::elaboration::WorldElaborationDemand,
    parent_gestalt_ids: Vec<String>,
    wave: Option<ghostlight_dungeon::elaboration::ElaborationWaveBinding>,
    schedule: Option<ghostlight_dungeon::elaboration::ElaborationScheduleReceipt>,
    completed_invocations: Vec<ComplexityPreviewInvocation>,
    invocation_failures: Vec<ComplexityFailedInvocation>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ComplexityMutationCheckpoint {
    schema: String,
    round: u32,
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    parent_gestalt_id: String,
    mutation_kind: String,
    affected_subject_ids: Vec<String>,
    model_receipt_hashes: Vec<String>,
    #[serde(default)]
    semantic_summary: String,
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ComplexityPreparedMutationCheckpoint {
    schema: String,
    round: u32,
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    expected_revision: u64,
    proposal: ghostlight_dungeon::elaboration::WorldComplexityProposal,
    parent_gestalt_id: String,
    mutation_kind: String,
    affected_subject_ids: Vec<String>,
    model_receipt_hashes: Vec<String>,
    semantic_summary: String,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ComplexitySupersededInvocationCheckpoint {
    schema: String,
    round: u32,
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    retained_dispatch_ordinal: u64,
    canonical_subject_ids: Vec<String>,
    public_identity_keys: Vec<String>,
    diagnostic: String,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ComplexitySemanticRejectionCheckpoint {
    schema: String,
    round: u32,
    dispatch: ghostlight_dungeon::elaboration::ElaborationDispatch,
    parent_gestalt_id: String,
    proposal: ghostlight_dungeon::elaboration::WorldComplexityProposal,
    verifier_receipt_hash: String,
    diagnostic: String,
}

fn complexity_semantic_rejection_diagnostic(
    verdict: &ghostlight_dungeon::elaboration::WorldComplexitySemanticVerification,
) -> anyhow::Result<String> {
    Ok(serde_json::to_string(verdict)?)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComplexityRoundCheckpoint {
    schema: String,
    round: u32,
    demand_before: ghostlight_dungeon::elaboration::WorldElaborationDemand,
    actionable_subjects_after: u32,
    schedule: ghostlight_dungeon::elaboration::ElaborationScheduleReceipt,
    mutation_checkpoints: Vec<std::path::PathBuf>,
    #[serde(default)]
    superseded_invocation_checkpoints: Vec<std::path::PathBuf>,
    session_checkpoints: std::collections::BTreeMap<
        String,
        ghostlight_dungeon::elaboration::ElaboratorSessionCheckpoint,
    >,
}

fn complexity_affected_subject_ids(
    proposal: &ghostlight_dungeon::elaboration::WorldComplexityProposal,
) -> Vec<String> {
    match proposal {
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission { preview, .. } => {
            std::iter::once(preview.parent_gestalt_id.clone())
                .chain(preview.children.iter().map(|child| child.id.clone()))
                .collect()
        }
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
            individuation,
            ..
        } => vec![
            individuation.gestalt_id.clone(),
            ghostlight_dungeon::domain::gestalt_member_subject_id(&individuation.member.id),
        ],
    }
}

fn complexity_session_journal_summary(
    proposal: &ghostlight_dungeon::elaboration::WorldComplexityProposal,
) -> anyhow::Result<String> {
    let semantic_delta = match proposal {
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission { preview, .. } => {
            let parent = preview
                .children
                .iter()
                .find(|child| child.id == preview.residual_child_id)
                .ok_or_else(|| anyhow::anyhow!("complexity fission summary lost its residual"))?;
            serde_json::json!({
                "specific_children":preview.children.iter()
                    .filter(|child|child.id != preview.residual_child_id)
                    .map(|child|serde_json::json!({
                        "name":child.name,
                        "capabilities":child.shared_capabilities.difference(&parent.shared_capabilities).collect::<Vec<_>>(),
                        "knowledge":child.shared_knowledge.difference(&parent.shared_knowledge).collect::<Vec<_>>(),
                        "goals":child.goals.iter().filter(|goal|!parent.goals.contains(goal)).collect::<Vec<_>>(),
                        "pressures":child.pressures.iter().filter(|pressure|!parent.pressures.contains(pressure)).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>()
            })
        }
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
            individuation,
            ..
        } => serde_json::json!({
            "person":individuation.member.name,
            "capabilities":individuation.member.capability_additions,
            "knowledge":individuation.member.knowledge_additions,
            "obligations":individuation.member.obligations,
            "relationships":individuation.member.relationships,
            "goals":individuation.member.goals,
            "memories":individuation.member.memories,
        }),
    };
    Ok(bounded_prompt_excerpt(
        &format!(
            "Applied {} to {}. Semantic delta: {}",
            proposal.mutation_kind(),
            proposal.parent_gestalt_id(),
            semantic_delta
        ),
        900,
    ))
}

fn validate_complexity_round_session_checkpoints(
    campaign: &ghostlight_dungeon::domain::Campaign,
    previous: &std::collections::BTreeMap<
        String,
        ghostlight_dungeon::elaboration::ElaboratorSessionCheckpoint,
    >,
    current: &std::collections::BTreeMap<
        String,
        ghostlight_dungeon::elaboration::ElaboratorSessionCheckpoint,
    >,
    session_routes: &std::collections::BTreeMap<
        String,
        (ghostlight_dungeon::elaboration::ElaboratorTitle, String),
    >,
    journals: &std::collections::BTreeMap<
        String,
        Vec<ghostlight_dungeon::elaboration::ElaboratorSessionJournalEntry>,
    >,
    rejection_findings: &std::collections::BTreeMap<String, Vec<String>>,
    through_world_revision: u64,
) -> anyhow::Result<()> {
    let touched = journals
        .keys()
        .chain(rejection_findings.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let expected_ids = previous
        .keys()
        .chain(touched.iter())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if current
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        != expected_ids
    {
        anyhow::bail!(
            "completed complexity round session checkpoint set does not match prior memory plus exact invocation journals"
        )
    }
    for session_id in expected_ids {
        let checkpoint = current
            .get(&session_id)
            .expect("the exact session set was checked above");
        if !touched.contains(&session_id) {
            if previous.get(&session_id) != Some(checkpoint) {
                anyhow::bail!("completed complexity round changed an untouched session checkpoint")
            }
            continue;
        }
        let (title, location_id) = session_routes
            .get(&session_id)
            .ok_or_else(|| anyhow::anyhow!("complexity session journal has no exact route"))?;
        checkpoint.validate_for(campaign, location_id, *title)?;
        let expected_receipt_ids = journals
            .get(&session_id)
            .into_iter()
            .flatten()
            .map(|entry| entry.commit_receipt_id.clone())
            .collect::<Vec<_>>();
        let expected_rejections = rejection_findings
            .get(&session_id)
            .cloned()
            .unwrap_or_default();
        let expected_prior_digest = previous
            .get(&session_id)
            .map(|checkpoint| checkpoint.digest.clone());
        let expected_generation = previous
            .get(&session_id)
            .map_or(1, |checkpoint| checkpoint.generation.saturating_add(1));
        if checkpoint.through_world_revision != through_world_revision
            || checkpoint.generation != expected_generation
            || checkpoint.recent_commit_receipt_ids != expected_receipt_ids
            || checkpoint.recent_rejection_findings != expected_rejections
            || checkpoint.prior_checkpoint_digest != expected_prior_digest
        {
            anyhow::bail!(
                "completed complexity round session checkpoint is not bound to its exact journal, rejection findings, and prior memory"
            )
        }
    }
    Ok(())
}

fn complexity_proposal_is_committed(
    campaign: &ghostlight_dungeon::domain::Campaign,
    proposal: &ghostlight_dungeon::elaboration::WorldComplexityProposal,
) -> bool {
    match proposal {
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission { preview, .. } => {
            let child_ids = preview
                .children
                .iter()
                .map(|child| child.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let actionable =
                ghostlight_dungeon::elaboration::canonical_actionable_subject_ids(campaign);
            campaign
                .gestalt_lineages
                .get(&preview.parent_gestalt_id)
                .is_some_and(|lineage| {
                    lineage.child_gestalt_ids
                        == preview
                            .children
                            .iter()
                            .map(|child| child.id.clone())
                            .collect::<Vec<_>>()
                        && lineage.partition_axis == preview.partition_axis
                        && lineage.partition_values == preview.child_partition_values
                        && lineage.residual_child_id == preview.residual_child_id
                        && lineage.source_revision == preview.expected_world_revision
                        && preview.children.iter().all(|expected| {
                            campaign.gestalts.get(&expected.id) == Some(expected)
                                && campaign.agency_profiles.get(&expected.id).is_some_and(
                                    |profile| {
                                        profile.parent_subject_id.as_deref()
                                            == Some(preview.parent_gestalt_id.as_str())
                                            && profile.active_leaf
                                            && profile.location_ids
                                                == std::collections::BTreeSet::from([expected
                                                    .home_location_id
                                                    .clone()])
                                            && profile.simulation_eligible
                                                == (expected.id != preview.residual_child_id)
                                    },
                                )
                                && actionable.contains(&expected.id)
                                    == (expected.id != preview.residual_child_id)
                        })
                        && campaign
                            .agency_profiles
                            .get(&preview.parent_gestalt_id)
                            .is_some_and(|profile| {
                                !profile.active_leaf && !profile.simulation_eligible
                            })
                        && !actionable.contains(&preview.parent_gestalt_id)
                        && preview
                            .member_child_assignments
                            .iter()
                            .all(|(member_id, child_id)| {
                                campaign
                                    .gestalt_members
                                    .get(member_id)
                                    .is_some_and(|member| member.gestalt_id == *child_id)
                            })
                        && campaign.gestalt_members.values().all(|member| {
                            member.gestalt_id != preview.parent_gestalt_id
                                && (!child_ids.contains(member.gestalt_id.as_str())
                                    || preview.member_child_assignments.contains_key(&member.id)
                                    || member.gestalt_id == preview.residual_child_id)
                        })
                        && campaign.civic_systems.values().all(|system| {
                            !system
                                .resident_population_ids
                                .contains(&preview.parent_gestalt_id)
                        })
                        && campaign.agency_relations.values().all(|relation| {
                            !relation.active
                                || (relation.from_subject_id != preview.parent_gestalt_id
                                    && relation.to_subject_id != preview.parent_gestalt_id)
                        })
                })
        }
        ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
            individuation,
            ..
        } => {
            let member_id = ghostlight_dungeon::domain::canonical_gestalt_member_local_id(
                &individuation.member.id,
            );
            let actor_id = ghostlight_dungeon::domain::gestalt_member_subject_id(&member_id);
            let mut expected_member = individuation.member.clone();
            expected_member.id = member_id.clone();
            expected_member.version = 1;
            expected_member.last_location_id = Some(individuation.location_id.clone());
            expected_member.materialized_actor_id = Some(actor_id.clone());
            expected_member.last_relevant_revision = proposal.expected_world_revision();
            expected_member.relevance_lease_until_revision =
                proposal.expected_world_revision().saturating_add(2);
            let Some(gestalt) = campaign.gestalts.get(&individuation.gestalt_id) else {
                return false;
            };
            let overlay =
                |base: &std::collections::BTreeSet<String>,
                 additions: &std::collections::BTreeSet<String>,
                 removals: &std::collections::BTreeSet<String>| {
                    base.difference(removals)
                        .cloned()
                        .chain(additions.iter().cloned())
                        .collect::<std::collections::BTreeSet<_>>()
                };
            let expected_actor = ghostlight_dungeon::domain::ActorState {
                id: actor_id.clone(),
                name: expected_member.name.clone(),
                location_id: individuation.location_id.clone(),
                capabilities: overlay(
                    &gestalt.shared_capabilities,
                    &expected_member.capability_additions,
                    &expected_member.capability_removals,
                ),
                knowledge: overlay(
                    &gestalt.shared_knowledge,
                    &expected_member.knowledge_additions,
                    &expected_member.knowledge_removals,
                ),
                equipment: expected_member.equipment.clone(),
                conditions: expected_member.conditions.clone(),
                obligations: expected_member.obligations.clone(),
                relationships: expected_member.relationships.clone(),
                goals: if expected_member.goals.is_empty() {
                    gestalt.goals.clone()
                } else {
                    expected_member.goals.clone()
                },
                memories: expected_member.memories.clone(),
            };
            campaign.gestalt_members.get(&member_id) == Some(&expected_member)
                && campaign.actors.get(&actor_id) == Some(&expected_actor)
                && gestalt.version == individuation.expected_gestalt_version
                && campaign
                    .agency_profiles
                    .get(&actor_id)
                    .is_some_and(|profile| {
                        profile.subject_kind == ghostlight_dungeon::domain::AgencySubjectKind::Actor
                            && profile.active_leaf
                            && profile.simulation_eligible
                            && profile.location_ids
                                == std::collections::BTreeSet::from([individuation
                                    .location_id
                                    .clone()])
                    })
                && ghostlight_dungeon::elaboration::canonical_actionable_subject_ids(campaign)
                    .contains(&actor_id)
        }
    }
}

fn retain_unique_complexity_invocations(
    round: u32,
    invocations: &[ComplexityPreviewInvocation],
) -> (
    Vec<&ComplexityPreviewInvocation>,
    Vec<ComplexitySupersededInvocationCheckpoint>,
) {
    let mut retained = Vec::new();
    let mut superseded = Vec::new();
    let mut retained_subject_ids = BTreeMap::<String, u64>::new();
    let mut retained_person_names = BTreeMap::<String, u64>::new();
    let mut retained_population_names = BTreeMap::<String, u64>::new();
    let mut ordered = invocations.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|invocation| invocation.dispatch.ordinal);
    for invocation in ordered {
        let (canonical_subject_ids, public_identity_keys, population_identity_keys) =
            match &invocation.proposal {
                ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
                    individuation,
                    ..
                } => (
                    vec![ghostlight_dungeon::domain::gestalt_member_subject_id(
                        &individuation.member.id,
                    )],
                    vec![ghostlight_dungeon::resolution::public_identity_key(
                        &individuation.member.name,
                    )],
                    Vec::new(),
                ),
                ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission {
                    preview,
                    ..
                } => (
                    preview
                        .children
                        .iter()
                        .map(|child| child.id.clone())
                        .collect(),
                    Vec::new(),
                    preview
                        .children
                        .iter()
                        .filter(|child| child.id != preview.residual_child_id)
                        .map(|child| {
                            ghostlight_dungeon::resolution::public_identity_key(&child.name)
                        })
                        .collect(),
                ),
            };
        let retained_ordinal = canonical_subject_ids
            .iter()
            .filter_map(|id| retained_subject_ids.get(id))
            .chain(
                public_identity_keys
                    .iter()
                    .filter_map(|key| retained_person_names.get(key)),
            )
            .chain(
                population_identity_keys
                    .iter()
                    .filter_map(|key| retained_population_names.get(key)),
            )
            .copied()
            .min();
        if retained_ordinal.is_none() {
            for id in &canonical_subject_ids {
                retained_subject_ids.insert(id.clone(), invocation.dispatch.ordinal);
            }
            for key in &public_identity_keys {
                retained_person_names.insert(key.clone(), invocation.dispatch.ordinal);
            }
            for key in &population_identity_keys {
                retained_population_names.insert(key.clone(), invocation.dispatch.ordinal);
            }
            retained.push(invocation);
        } else {
            superseded.push(ComplexitySupersededInvocationCheckpoint {
                schema: "ghostlight.complexity_superseded_invocation.v1".into(),
                round,
                dispatch: invocation.dispatch.clone(),
                retained_dispatch_ordinal: retained_ordinal.expect("duplicate has an owner"),
                canonical_subject_ids,
                public_identity_keys: public_identity_keys
                    .into_iter()
                    .chain(population_identity_keys)
                    .collect(),
                diagnostic:
                    "parallel complexity proposals duplicated a canonical subject ID or normalized public identity; the earliest dispatch owns the proposal"
                        .into(),
            });
        }
    }
    (retained, superseded)
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

fn titled_verifier_execution_failure_checkpoint_paths(
    root: &std::path::Path,
    index: usize,
    semantic_attempt: u8,
) -> anyhow::Result<Vec<(u32, std::path::PathBuf)>> {
    let prefix = format!(
        "titled-elaboration-{index:02}-verifier-execution-failure-{semantic_attempt:02}-generation-"
    );
    let mut generations = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let generation = name
                .strip_prefix(&prefix)?
                .strip_suffix(".json")?
                .parse::<u32>()
                .ok()?;
            Some((generation, entry.path()))
        })
        .collect::<Vec<_>>();
    generations.sort_by_key(|(generation, _)| *generation);
    if generations
        .iter()
        .enumerate()
        .any(|(index, (generation, _))| *generation != index as u32 + 1)
    {
        anyhow::bail!("titled verifier-execution failure checkpoint generations are not contiguous")
    }
    Ok(generations)
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComplexityCommitValidationMode {
    CurrentCampaignEffect,
    HistoricalCommit,
}

fn validate_complexity_semantic_rejection_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    round: u32,
    invocation: &ComplexityPreviewInvocation,
    path: &std::path::Path,
) -> anyhow::Result<ComplexitySemanticRejectionCheckpoint> {
    use ghostlight_dungeon::elaboration::{
        WorldComplexityProposal, WorldComplexitySemanticVerification,
        validate_world_complexity_semantic_qualification_shape,
    };

    if !path.is_file() {
        anyhow::bail!(
            "completed complexity round references a missing semantic-rejection checkpoint: {}",
            path.display()
        )
    }
    let rejection = read_checkpoint::<ComplexitySemanticRejectionCheckpoint>(path)?;
    let rejected_verdict =
        serde_json::from_str::<WorldComplexitySemanticVerification>(&rejection.diagnostic)?;
    if rejection.schema != "ghostlight.complexity_semantic_rejection.v2"
        || rejection.round != round
        || rejection.dispatch != invocation.dispatch
        || rejection.parent_gestalt_id != invocation.proposal.parent_gestalt_id()
        || rejection.proposal.parent_gestalt_id() != invocation.proposal.parent_gestalt_id()
        || rejected_verdict.accepted()
    {
        anyhow::bail!("complexity semantic-rejection checkpoint is inconsistent")
    }
    let receipt = store
        .load::<ghostlight_dungeon::model::ModelStageReceipt>(
            "persona_stage_receipt.v1",
            &rejection.verifier_receipt_hash,
        )?
        .map(|(_, receipt)| receipt)
        .ok_or_else(|| {
            anyhow::anyhow!("complexity semantic rejection lost its verifier receipt")
        })?;
    let semantic = match &rejection.proposal {
        WorldComplexityProposal::Fission { qualification, .. } => &qualification.semantic,
        WorldComplexityProposal::Individuate { qualification, .. } => &qualification.semantic,
    };
    validate_world_complexity_semantic_qualification_shape(&rejection.proposal, semantic)?;
    let expected_sources = invocation
        .model_receipt_hashes
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let actual_sources = receipt
        .source_receipt_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let expected_error = format!(
        "world-complexity semantic verifier rejected the candidate: {}",
        rejection.diagnostic
    );
    if receipt.storage_key() != rejection.verifier_receipt_hash
        || receipt.stage != "world-complexity-semantic-verification"
        || receipt.model != ghostlight_dungeon::model::MODEL_BALANCED
        || receipt.validation_result == "valid"
        || receipt.local_validation_error.as_deref() != Some(expected_error.as_str())
        || receipt.snapshot_binding != semantic.semantic_verification_binding
        || semantic.frozen_campaign_id != campaign.id
        || semantic.frozen_world_revision > campaign.revision
        || rejection.proposal.expected_world_revision() != semantic.frozen_world_revision
        || expected_sources.is_empty()
        || expected_sources.len() != invocation.model_receipt_hashes.len()
        || actual_sources.len() != receipt.source_receipt_ids.len()
        || actual_sources != expected_sources
    {
        anyhow::bail!(
            "complexity semantic rejection is not backed by its exact invalid verifier receipt"
        )
    }
    Ok(rejection)
}

fn committed_complexity_fission_mutation_proof(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    world_receipt: &ghostlight_dungeon::domain::WorldCommitReceipt,
    preview: &ghostlight_dungeon::domain::GestaltFissionPreview,
) -> anyhow::Result<TitledMutationProof> {
    let intended_effect_digest =
        ghostlight_dungeon::legacy_transition::digest_serializable(preview)?;
    let mut exact_batches = store
        .load_all::<ghostlight_dungeon::transition::WorldMutationBatch>("world_mutation_batch.v1")?
        .into_iter()
        .filter(|batch| {
            batch.campaign_id == world_receipt.campaign_id
                && batch.expected_world_revision == world_receipt.previous_revision
                && batch.intended_effect_digest.as_deref() == Some(&intended_effect_digest)
        })
        .collect::<Vec<_>>();
    if exact_batches.len() != 1 {
        anyhow::bail!(
            "complexity fission world commit has {} exact candidate mutation batches",
            exact_batches.len()
        )
    }
    let expected_resolution_epoch = exact_batches
        .pop()
        .expect("one exact fission batch was required")
        .expected_resolution_epoch;
    committed_compiler_mutation_proof(
        store,
        world_receipt,
        intended_effect_digest,
        "elaborate_gestalt_fission",
        "fission-preview",
        expected_resolution_epoch,
    )
}

fn validate_complexity_mutation_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    round: u32,
    invocation: &ComplexityPreviewInvocation,
    prepared: &ComplexityPreparedMutationCheckpoint,
    checkpoint: &ComplexityMutationCheckpoint,
    mode: ComplexityCommitValidationMode,
) -> anyhow::Result<()> {
    use ghostlight_dungeon::elaboration::WorldComplexityProposal;

    let derived_affected_subject_ids = complexity_affected_subject_ids(&prepared.proposal);
    let derived_semantic_summary = complexity_session_journal_summary(&prepared.proposal)?;
    if checkpoint.schema != "ghostlight.complexity_mutation_checkpoint.v1"
        || checkpoint.round != round
        || checkpoint.dispatch != invocation.dispatch
        || prepared.schema != "ghostlight.complexity_prepared_mutation.v1"
        || prepared.round != round
        || prepared.dispatch != invocation.dispatch
        || prepared.parent_gestalt_id != invocation.proposal.parent_gestalt_id()
        || prepared.mutation_kind != invocation.proposal.mutation_kind()
        || checkpoint.parent_gestalt_id != prepared.parent_gestalt_id
        || checkpoint.mutation_kind != prepared.mutation_kind
        || checkpoint.affected_subject_ids != derived_affected_subject_ids
        || prepared.affected_subject_ids != derived_affected_subject_ids
        || checkpoint.semantic_summary != derived_semantic_summary
        || prepared.semantic_summary != derived_semantic_summary
        || checkpoint.model_receipt_hashes != prepared.model_receipt_hashes
        || !prepared
            .model_receipt_hashes
            .starts_with(&invocation.model_receipt_hashes)
        || prepared.model_receipt_hashes.len()
            != invocation.model_receipt_hashes.len().saturating_add(1)
        || checkpoint.commit_receipt.schema != "ghostlight.world_commit_receipt.v1"
        || checkpoint.commit_receipt.campaign_id != campaign.id
        || checkpoint.commit_receipt.previous_revision != prepared.expected_revision
        || checkpoint.commit_receipt.revision != prepared.expected_revision.saturating_add(1)
        || checkpoint.commit_receipt.revision > campaign.revision
        || checkpoint.commit_receipt.command_kind != checkpoint.mutation_kind
    {
        anyhow::bail!("complexity mutation checkpoint is inconsistent")
    }
    if mode == ComplexityCommitValidationMode::CurrentCampaignEffect
        && !complexity_proposal_is_committed(campaign, &prepared.proposal)
    {
        anyhow::bail!(
            "complexity mutation checkpoint is present but its committed proposal effect is absent from the campaign"
        )
    }

    let model_receipts = load_checkpoint_receipts(store, &prepared.model_receipt_hashes)?;
    let semantic = match &prepared.proposal {
        WorldComplexityProposal::Fission { qualification, .. } => &qualification.semantic,
        WorldComplexityProposal::Individuate { qualification, .. } => &qualification.semantic,
    };
    ghostlight_dungeon::elaboration::validate_world_complexity_semantic_receipt_provenance(
        &prepared.proposal,
        semantic,
        &model_receipts,
    )?;
    if semantic.frozen_campaign_id != campaign.id
        || semantic.frozen_world_revision != prepared.expected_revision
        || prepared.proposal.expected_world_revision() != prepared.expected_revision
    {
        anyhow::bail!("prepared complexity qualification is bound to another campaign revision")
    }

    let canonical_receipt = store
        .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
            "world_commit_receipt.v1",
            &format!(
                "{}-{}",
                checkpoint.commit_receipt.campaign_id, checkpoint.commit_receipt.revision
            ),
        )?
        .map(|(_, receipt)| receipt)
        .ok_or_else(|| anyhow::anyhow!("complexity mutation lacks its canonical world receipt"))?;
    if canonical_receipt != checkpoint.commit_receipt {
        anyhow::bail!("complexity checkpoint disagrees with its canonical world receipt")
    }

    match &prepared.proposal {
        WorldComplexityProposal::Fission { preview, .. } => {
            committed_complexity_fission_mutation_proof(store, &canonical_receipt, preview)?;
        }
        WorldComplexityProposal::Individuate { individuation, .. } => {
            let presence = store
                .load::<ghostlight_dungeon::domain::GestaltMaterializationReceipt>(
                    "gestalt_materialization_receipt.v1",
                    &format!(
                        "{}-{}",
                        canonical_receipt.campaign_id, canonical_receipt.revision
                    ),
                )?
                .map(|(_, receipt)| receipt)
                .ok_or_else(|| {
                    anyhow::anyhow!("complexity individuation lacks its canonical presence receipt")
                })?;
            let member_id = ghostlight_dungeon::domain::canonical_gestalt_member_local_id(
                &individuation.member.id,
            );
            let actor_id = ghostlight_dungeon::domain::gestalt_member_subject_id(&member_id);
            if presence.schema != "ghostlight.gestalt_materialization_receipt.v1"
                || presence.campaign_id != canonical_receipt.campaign_id
                || presence.previous_revision != prepared.expected_revision
                || presence.revision != canonical_receipt.revision
                || presence.previous_resolution_epoch.saturating_add(1) != presence.resolution_epoch
                || presence.reason != "model-qualified world-complexity individuation"
                || presence.committed_at != canonical_receipt.committed_at
                || presence.changes.len() != 1
                || presence.changes[0].operation != "materialized"
                || presence.changes[0].gestalt_id != individuation.gestalt_id
                || presence.changes[0].member_id != member_id
                || presence.changes[0].actor_id != actor_id
                || presence.changes[0].gestalt_version != individuation.expected_gestalt_version
                || presence.changes[0].member_version != 1
            {
                anyhow::bail!(
                    "complexity individuation found a different canonical presence change"
                )
            }
        }
    }
    Ok(())
}

fn load_and_validate_complexity_mutation_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    round: u32,
    invocation: &ComplexityPreviewInvocation,
    prepared_path: &std::path::Path,
    commit_path: &std::path::Path,
    mode: ComplexityCommitValidationMode,
) -> anyhow::Result<(
    ComplexityPreparedMutationCheckpoint,
    ComplexityMutationCheckpoint,
)> {
    if !prepared_path.is_file() {
        anyhow::bail!(
            "complexity commit references a missing prepared-mutation checkpoint: {}",
            prepared_path.display()
        )
    }
    if !commit_path.is_file() {
        anyhow::bail!(
            "completed complexity round references a missing mutation checkpoint: {}",
            commit_path.display()
        )
    }
    let prepared = read_checkpoint::<ComplexityPreparedMutationCheckpoint>(prepared_path)?;
    let checkpoint = read_checkpoint::<ComplexityMutationCheckpoint>(commit_path)?;
    validate_complexity_mutation_checkpoint(
        store,
        campaign,
        round,
        invocation,
        &prepared,
        &checkpoint,
        mode,
    )?;
    Ok((prepared, checkpoint))
}

fn validate_completed_complexity_round_checkpoint(
    root: &std::path::Path,
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    checkpoint: &ComplexityRoundCheckpoint,
    round: u32,
) -> anyhow::Result<()> {
    if checkpoint.schema != "ghostlight.complexity_round_checkpoint.v1"
        || checkpoint.round != round
        || checkpoint.actionable_subjects_after
            > ghostlight_dungeon::elaboration::canonical_actionable_subject_count(campaign)
    {
        anyhow::bail!("complexity round checkpoint is stale or malformed")
    }
    for path in &checkpoint.mutation_checkpoints {
        if !path.is_file() {
            anyhow::bail!(
                "completed complexity round references a missing mutation checkpoint: {}",
                path.display()
            )
        }
    }
    for path in &checkpoint.superseded_invocation_checkpoints {
        if !path.is_file() {
            anyhow::bail!(
                "completed complexity round references a missing supersession or rejection checkpoint: {}",
                path.display()
            )
        }
    }

    let preview_path = root.join(format!("complexity-round-{round:03}-preview.json"));
    if !preview_path.is_file() {
        anyhow::bail!(
            "completed complexity round has no immutable preview checkpoint: {}",
            preview_path.display()
        )
    }
    let preview = read_checkpoint::<ComplexityRoundPreviewCheckpoint>(&preview_path)?;
    if preview.schema != "ghostlight.complexity_round_preview.v1"
        || preview.round != round
        || preview.invocations.is_empty()
        || preview.frozen_world_revision > campaign.revision
        || preview.demand != checkpoint.demand_before
        || preview.schedule != checkpoint.schedule
    {
        anyhow::bail!("completed complexity round preview is stale or malformed")
    }
    let previous_session_checkpoints = if round == 1 {
        std::collections::BTreeMap::new()
    } else {
        let previous_path = root.join(format!(
            "complexity-round-{:03}-checkpoint.json",
            round.saturating_sub(1)
        ));
        let previous = read_checkpoint::<ComplexityRoundCheckpoint>(&previous_path)?;
        if previous.schema != "ghostlight.complexity_round_checkpoint.v1"
            || previous.round != round.saturating_sub(1)
        {
            anyhow::bail!("completed complexity round has no exact prior session checkpoint")
        }
        previous.session_checkpoints
    };

    let (retained_invocations, superseded_invocations) =
        retain_unique_complexity_invocations(round, &preview.invocations);
    let mut expected_superseded_paths = Vec::new();
    for superseded in superseded_invocations {
        let path = root.join(format!(
            "complexity-round-{round:03}-superseded-{:04}.json",
            superseded.dispatch.ordinal
        ));
        let persisted = read_checkpoint::<ComplexitySupersededInvocationCheckpoint>(&path)?;
        if persisted != superseded {
            anyhow::bail!("completed complexity supersession checkpoint is inconsistent")
        }
        expected_superseded_paths.push(path);
    }

    let mut expected_mutation_paths = Vec::new();
    let mut expected_revision = preview.frozen_world_revision;
    let mut expected_actionable_subjects = checkpoint.demand_before.current_actionable_subjects;
    let mut session_routes = std::collections::BTreeMap::new();
    let mut session_journals = std::collections::BTreeMap::<
        String,
        Vec<ghostlight_dungeon::elaboration::ElaboratorSessionJournalEntry>,
    >::new();
    let mut session_rejection_findings = std::collections::BTreeMap::<String, Vec<String>>::new();
    for invocation in retained_invocations {
        let location_id = match &invocation.proposal {
            ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission {
                qualification,
                ..
            } => qualification.jurisdiction_location_id.clone(),
            ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
                qualification,
                ..
            } => qualification.jurisdiction_location_id.clone(),
        };
        let session_id = ghostlight_dungeon::elaboration::elaborator_session_id(
            invocation.dispatch.title,
            &location_id,
        );
        session_routes.insert(session_id.clone(), (invocation.dispatch.title, location_id));
        let commit_path = root.join(format!(
            "complexity-round-{round:03}-commit-{:04}.json",
            invocation.dispatch.ordinal
        ));
        let rejection_path = root.join(format!(
            "complexity-round-{round:03}-semantic-rejection-{:04}.json",
            invocation.dispatch.ordinal
        ));
        match (commit_path.is_file(), rejection_path.is_file()) {
            (true, false) => {
                let prepared_path = root.join(format!(
                    "complexity-round-{round:03}-prepared-{:04}.json",
                    invocation.dispatch.ordinal
                ));
                let (prepared, mutation_checkpoint) =
                    load_and_validate_complexity_mutation_checkpoint(
                        store,
                        campaign,
                        round,
                        invocation,
                        &prepared_path,
                        &commit_path,
                        ComplexityCommitValidationMode::HistoricalCommit,
                    )?;
                if prepared.expected_revision != expected_revision {
                    anyhow::bail!(
                        "completed complexity commits do not form the exact historical revision chain"
                    )
                }
                expected_revision = expected_revision.saturating_add(1);
                expected_actionable_subjects =
                    expected_actionable_subjects.saturating_add(match &prepared.proposal {
                        ghostlight_dungeon::elaboration::WorldComplexityProposal::Fission {
                            qualification,
                            ..
                        } => u32::from(qualification.target_actionable_gain),
                        ghostlight_dungeon::elaboration::WorldComplexityProposal::Individuate {
                            ..
                        } => 1,
                    });
                session_journals
                    .entry(session_id.clone())
                    .or_default()
                    .push(
                        ghostlight_dungeon::elaboration::ElaboratorSessionJournalEntry {
                            world_revision: mutation_checkpoint.commit_receipt.revision,
                            commit_receipt_id: format!(
                                "{}-{}",
                                mutation_checkpoint.commit_receipt.campaign_id,
                                mutation_checkpoint.commit_receipt.revision
                            ),
                            mutation_kind: mutation_checkpoint.mutation_kind,
                            affected_subject_ids: mutation_checkpoint.affected_subject_ids,
                            summary: mutation_checkpoint.semantic_summary,
                        },
                    );
                expected_mutation_paths.push(commit_path);
            }
            (false, true) => {
                let rejection = validate_complexity_semantic_rejection_checkpoint(
                    store,
                    campaign,
                    round,
                    invocation,
                    &rejection_path,
                )?;
                session_rejection_findings
                    .entry(session_id)
                    .or_default()
                    .push(bounded_prompt_excerpt(&rejection.diagnostic, 800));
                expected_superseded_paths.push(rejection_path);
            }
            _ => anyhow::bail!(
                "completed complexity invocation must have exactly one commit or semantic rejection checkpoint"
            ),
        }
    }
    if checkpoint.mutation_checkpoints != expected_mutation_paths
        || checkpoint.superseded_invocation_checkpoints != expected_superseded_paths
        || checkpoint.actionable_subjects_after != expected_actionable_subjects
    {
        anyhow::bail!(
            "completed complexity round does not exactly cover its mutations, supersessions, rejections, and admitted complexity gain"
        )
    }
    validate_complexity_round_session_checkpoints(
        campaign,
        &previous_session_checkpoints,
        &checkpoint.session_checkpoints,
        &session_routes,
        &session_journals,
        &session_rejection_findings,
        expected_revision,
    )?;
    Ok(())
}

fn validate_titled_semantic_failure_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    checkpoint: &TitledSemanticFailureCheckpoint,
    expected_attempt: u8,
    location_id: &str,
    location_name: &str,
    base_request: &str,
    expected_verification_request: &str,
    wave: &ghostlight_dungeon::elaboration::ElaborationWaveBinding,
) -> anyhow::Result<()> {
    wave.validate()?;
    if checkpoint.schema != "ghostlight.titled_elaboration_semantic_failure.v2"
        || checkpoint.attempt != expected_attempt
        || checkpoint.location_id != location_id
        || checkpoint.location_name != location_name
        || checkpoint.base_request != base_request
        || checkpoint.verification_request != expected_verification_request
        || &checkpoint.wave != wave
        || checkpoint.schedule.schema != "ghostlight.elaboration_schedule_receipt.v1"
        || checkpoint.schedule.dispatches.is_empty()
        || checkpoint.model_receipt_hashes.is_empty()
        || checkpoint.verifier_receipt_hashes.len() != 1
        || (expected_attempt == 1
            && checkpoint
                .repair_request
                .as_deref()
                .is_none_or(str::is_empty))
        || (expected_attempt == 2 && checkpoint.repair_request.is_some())
    {
        anyhow::bail!("titled semantic-failure checkpoint is malformed or stale")
    }
    let candidate = checkpoint.admission.valid_candidate(campaign)?;
    let expected_model_receipt_hashes = checkpoint
        .admission
        .model_stage_receipts()
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    if checkpoint.admission.target_location_id() != location_id
        || checkpoint.admission.wave() != wave
        || checkpoint.admission.schedule() != &checkpoint.schedule
        || candidate.target_location_id != location_id
        || checkpoint.model_receipt_hashes != expected_model_receipt_hashes
    {
        anyhow::bail!(
            "titled semantic-failure checkpoint admission is not its exact derived candidate"
        )
    }
    let loaded_model_receipts = load_checkpoint_receipts(store, &checkpoint.model_receipt_hashes)?;
    if loaded_model_receipts != checkpoint.admission.model_stage_receipts() {
        anyhow::bail!("titled semantic-failure checkpoint model ancestry was substituted")
    }
    let verifier_receipts = load_checkpoint_receipts(store, &checkpoint.verifier_receipt_hashes)?;
    let expected_verifier_binding =
        ghostlight_dungeon::compiler::titled_civic_verifier_binding(campaign, &candidate)?;
    let expected_sources = checkpoint
        .model_receipt_hashes
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if verifier_receipts.iter().any(|receipt| {
        let actual_sources = receipt
            .source_receipt_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        receipt.stage != "destination_civic_verification"
            || receipt.model != ghostlight_dungeon::model::MODEL_CAPABLE
            || receipt.validation_result == "valid"
            || receipt.local_validation_error.is_none()
            || receipt.snapshot_binding != expected_verifier_binding
            || actual_sources.len() != receipt.source_receipt_ids.len()
            || actual_sources != expected_sources
    }) {
        anyhow::bail!(
            "titled semantic-failure checkpoint lacks its exact invalid civic verifier ancestry"
        )
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_titled_verifier_execution_failure_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    campaign: &ghostlight_dungeon::domain::Campaign,
    checkpoint: &TitledVerifierExecutionFailureCheckpoint,
    expected_semantic_attempt: u8,
    expected_generation: u32,
    location_id: &str,
    location_name: &str,
    expected_verification_request: &str,
) -> anyhow::Result<()> {
    if checkpoint.schema != "ghostlight.titled_verifier_execution_failure.v2"
        || !matches!(expected_semantic_attempt, 1 | 2)
        || checkpoint.semantic_attempt != expected_semantic_attempt
        || checkpoint.generation != expected_generation
        || checkpoint.location_id != location_id
        || checkpoint.location_name != location_name
        || checkpoint.verification_request != expected_verification_request
        || checkpoint.diagnostic.trim().is_empty()
        || checkpoint.attempts == 0
        || checkpoint.attempts > 3
    {
        anyhow::bail!("titled verifier-execution failure checkpoint is malformed or stale")
    }
    let candidate = checkpoint.admission.valid_candidate(campaign)?;
    let expected_model_receipt_hashes = checkpoint
        .admission
        .model_stage_receipts()
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    if checkpoint.admission.target_location_id() != location_id
        || checkpoint.model_receipt_hashes != expected_model_receipt_hashes
    {
        anyhow::bail!(
            "titled verifier-execution failure admission is not its exact derived candidate"
        )
    }
    let loaded_model_receipts = load_checkpoint_receipts(store, &checkpoint.model_receipt_hashes)?;
    if loaded_model_receipts != checkpoint.admission.model_stage_receipts() {
        anyhow::bail!("titled verifier-execution failure model ancestry was substituted")
    }
    let verifier_receipts = load_checkpoint_receipts(store, &checkpoint.verifier_receipt_hashes)?;
    let expected_binding =
        ghostlight_dungeon::compiler::titled_civic_verifier_binding(campaign, &candidate)?;
    let expected_sources = checkpoint
        .model_receipt_hashes
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let unique_verifier_hashes = checkpoint
        .verifier_receipt_hashes
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique_verifier_hashes.len() != checkpoint.verifier_receipt_hashes.len()
        || verifier_receipts.iter().any(|receipt| {
            let actual_sources = receipt
                .source_receipt_ids
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            receipt.stage != "destination_civic_verification"
                || receipt.model != ghostlight_dungeon::model::MODEL_CAPABLE
                || receipt.validation_result == "valid"
                || receipt.local_validation_error.is_none()
                || receipt.snapshot_binding != expected_binding
                || actual_sources.len() != receipt.source_receipt_ids.len()
                || actual_sources != expected_sources
        })
    {
        anyhow::bail!("titled verifier-execution failure lacks its exact failed verifier ancestry")
    }
    Ok(())
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

fn complexity_failure_checkpoint_paths(
    root: &std::path::Path,
    round: u32,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let original = root.join(format!("complexity-round-{round:03}-terminal-failure.json"));
    let prefix = format!("complexity-round-{round:03}-resume-");
    let mut paths = original
        .is_file()
        .then_some(original)
        .into_iter()
        .collect::<Vec<_>>();
    let mut resumed = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let generation = name
                .to_str()?
                .strip_prefix(&prefix)?
                .strip_suffix("-terminal-failure.json")?
                .parse::<u32>()
                .ok()?;
            Some((generation, entry.path()))
        })
        .collect::<Vec<_>>();
    resumed.sort_by_key(|(generation, _)| *generation);
    paths.extend(resumed.into_iter().map(|(_, path)| path));
    Ok(paths)
}

fn rehydrate_complexity_failure(
    checkpoint: ComplexityRoundFailureCheckpoint,
    store: &ghostlight_dungeon::persistence::CampaignStore,
) -> anyhow::Result<
    ghostlight_dungeon::elaboration::ElaborationWaveFailure<
        ghostlight_dungeon::elaboration::WorldComplexityProposal,
    >,
> {
    if checkpoint.schema != "ghostlight.complexity_round_failure.v1" {
        anyhow::bail!("complexity failure checkpoint schema is unsupported")
    }
    let wave = checkpoint.wave.clone();
    let completed_invocations = checkpoint
        .completed_invocations
        .into_iter()
        .map(|invocation| {
            Ok(ghostlight_dungeon::elaboration::ElaborationInvocation {
                wave: wave
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("complexity checkpoint has no wave"))?,
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
        wave,
        schedule: checkpoint.schedule,
        completed_invocations,
        invocation_failures,
    })
}

fn civic_manifest_preserves(
    campaign: &ghostlight_dungeon::domain::Campaign,
    current: &ghostlight_dungeon::domain::CivicSystemManifest,
    checkpoint: &ghostlight_dungeon::domain::CivicSystemManifest,
) -> bool {
    current.schema == checkpoint.schema
        && current.jurisdiction_location_id == checkpoint.jurisdiction_location_id
        && current.version >= checkpoint.version
        && current
            .governing_institution_ids
            .is_superset(&checkpoint.governing_institution_ids)
        && checkpoint
            .resident_population_ids
            .iter()
            .all(|resident_id| {
                fission_population_binding_is_present(
                    campaign,
                    &current.resident_population_ids,
                    resident_id,
                )
            })
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
        && checkpoint.political_relation_ids.iter().all(|relation_id| {
            fission_relation_binding_is_present(
                campaign,
                &current.political_relation_ids,
                relation_id,
            )
        })
        && !current.semantic_verification_receipt_id.is_empty()
}

fn fission_population_binding_is_present(
    campaign: &ghostlight_dungeon::domain::Campaign,
    current_ids: &BTreeSet<String>,
    expected_id: &str,
) -> bool {
    fn visit(
        campaign: &ghostlight_dungeon::domain::Campaign,
        current_ids: &BTreeSet<String>,
        expected_id: &str,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if current_ids.contains(expected_id) {
            return true;
        }
        if !visited.insert(expected_id.to_owned()) {
            return false;
        }
        campaign
            .gestalt_lineages
            .get(expected_id)
            .is_some_and(|lineage| {
                !lineage.child_gestalt_ids.is_empty()
                    && lineage
                        .child_gestalt_ids
                        .iter()
                        .all(|child_id| visit(campaign, current_ids, child_id, visited))
            })
    }
    visit(campaign, current_ids, expected_id, &mut BTreeSet::new())
}

fn fission_relation_binding_is_present(
    campaign: &ghostlight_dungeon::domain::Campaign,
    current_ids: &BTreeSet<String>,
    expected_id: &str,
) -> bool {
    fn visit(
        campaign: &ghostlight_dungeon::domain::Campaign,
        current_ids: &BTreeSet<String>,
        expected_id: &str,
    ) -> bool {
        if current_ids.contains(expected_id) {
            return true;
        }
        campaign.gestalt_lineages.values().any(|lineage| {
            if lineage.child_gestalt_ids.is_empty() {
                return false;
            }
            let child_relation_ids = lineage
                .child_gestalt_ids
                .iter()
                .map(|child_id| format!("{expected_id}:fission:{child_id}"))
                .collect::<Vec<_>>();
            child_relation_ids.iter().all(|candidate| {
                current_ids.contains(candidate)
                    || current_ids
                        .iter()
                        .any(|current| current.starts_with(&format!("{candidate}:fission:")))
            }) && child_relation_ids
                .iter()
                .all(|candidate| visit(campaign, current_ids, candidate))
        })
    }
    visit(campaign, current_ids, expected_id)
}

#[cfg(test)]
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
    committed_compiler_mutation_proof(
        store,
        world_receipt,
        ghostlight_dungeon::legacy_transition::digest_serializable(expansion)?,
        "elaborate_locality",
        "region-expansion",
        None,
    )
}

fn committed_region_mutation_proof(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    world_receipt: &ghostlight_dungeon::domain::WorldCommitReceipt,
    expansion: &ghostlight_dungeon::domain::RegionExpansion,
) -> anyhow::Result<TitledMutationProof> {
    committed_compiler_mutation_proof(
        store,
        world_receipt,
        ghostlight_dungeon::legacy_transition::digest_serializable(expansion)?,
        "expand_region",
        "region-expansion",
        None,
    )
}

fn expected_foundation_commit_checkpoint(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    location_id: &str,
    location_name: &str,
    request: &str,
    preview: &ghostlight_dungeon::domain::LocalityElaborationPreview,
    model_receipts: &[ghostlight_dungeon::model::ModelStageReceipt],
    commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt,
) -> anyhow::Result<FoundationCommitCheckpoint> {
    if commit_receipt.command_kind != "elaborate_locality"
        || commit_receipt.previous_revision != preview.expected_revision
        || commit_receipt.revision != preview.expected_revision.saturating_add(1)
        || preview.elaboration.target_location_id != location_id
    {
        anyhow::bail!("foundation world receipt does not bind the prepared locality")
    }
    Ok(FoundationCommitCheckpoint {
        schema: "ghostlight.foundation_commit_checkpoint.v1".into(),
        location_id: location_id.into(),
        location_name: location_name.into(),
        request: request.into(),
        world_revision_before: preview.expected_revision,
        world_revision_after: preview.expected_revision.saturating_add(1),
        model_receipt_hashes: model_receipts
            .iter()
            .map(|receipt| receipt.storage_key().to_owned())
            .collect(),
        mutation_proof: committed_elaboration_mutation_proof(
            store,
            &commit_receipt,
            &preview.elaboration.expansion,
        )?,
        commit_receipt,
    })
}

fn committed_compiler_mutation_proof(
    store: &ghostlight_dungeon::persistence::CampaignStore,
    world_receipt: &ghostlight_dungeon::domain::WorldCommitReceipt,
    intended_effect_digest: String,
    expected_command_kind: &str,
    source_receipt_kind: &str,
    expected_resolution_epoch: Option<u64>,
) -> anyhow::Result<TitledMutationProof> {
    use ghostlight_dungeon::transition::{
        MutationAuthorityEnvelope, WorldMutationBatch, WorldMutationReceipt, mutation_digest,
        validate_batch_structure,
    };

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
            "world commit has {} exact candidate mutation batches",
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
            "world commit has {} exact mutation authorities",
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
            "world commit has {} exact mutation receipts",
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
        "{source_receipt_kind}:{}",
        intended_effect_digest.trim_start_matches("sha256:")
    );
    if world_receipt.schema != "ghostlight.world_commit_receipt.v1"
        || world_receipt.command_kind != expected_command_kind
        || world_receipt.previous_revision.saturating_add(1) != world_receipt.revision
        || batch.schema != "ghostlight.world_mutation_batch.v1"
        || batch.source_receipt_id != expected_source_receipt_id
        || batch.expected_resolution_epoch != expected_resolution_epoch
        || batch.mutations.is_empty()
        || authority.schema != "ghostlight.mutation_authority_envelope.v1"
        || authority.resolution_epoch != expected_resolution_epoch
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
        anyhow::bail!("mutation proof does not bind one compiler admission commit")
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
            ElaborationScheduler, ElaboratorSessionCheckpoint, ElaboratorSessionJournalEntry,
            ElaboratorTitle, ModelWorldComplexityWorker, ModelWorldElaborationWorker,
            WorldComplexityProposal, WorldScaleIntent, admit_world_elaboration_wave,
            canonical_actionable_subject_count, compact_elaborator_session,
            derive_world_elaboration_demand, dispatch_elaboration_wave, elaborator_session_id,
            finalize_world_elaboration, qualify_world_complexity_proposal_semantics,
            rebase_world_complexity_proposal, resume_elaboration_wave,
            world_complexity_parent_binding, world_elaboration_wave_binding,
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
    use std::{
        collections::{BTreeMap, BTreeSet},
        path::PathBuf,
        sync::Arc,
        time::Instant,
    };

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
        mut initial_seed_location_ids,
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
    if !resume {
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
    let civic_reconciliation_path = root.join("fission-civic-reconciliation.json");
    if resume
        && ghostlight_dungeon::legacy_transition::fission_civic_reconciliation_required(&campaign)
    {
        if civic_reconciliation_path.is_file() {
            anyhow::bail!(
                "fission civic reconciliation checkpoint exists but canonical repair is absent"
            )
        }
        let committed = kernel
            .command(WorldCommand::ReconcileFissionCivicBindings {
                expected_revision: campaign.revision,
            })
            .await?;
        let CommandResult::Committed {
            campaign: advanced,
            receipt,
        } = committed
        else {
            anyhow::bail!("fission civic reconciliation did not commit")
        };
        publish_immutable_checkpoint(
            &civic_reconciliation_path,
            &serde_json::json!({
                "schema":"ghostlight.fission_civic_reconciliation_checkpoint.v1",
                "receipt":receipt,
            }),
        )?;
        campaign = advanced;
    } else if civic_reconciliation_path.is_file() {
        let checkpoint: serde_json::Value = read_checkpoint(&civic_reconciliation_path)?;
        let revision = checkpoint["receipt"]["revision"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("fission civic reconciliation receipt is malformed"))?;
        if checkpoint["schema"] != "ghostlight.fission_civic_reconciliation_checkpoint.v1"
            || checkpoint["receipt"]["command_kind"] != "reconcile_fission_civic_bindings"
            || revision > campaign.revision
        {
            anyhow::bail!("fission civic reconciliation checkpoint disagrees with campaign")
        }
    }
    if resume {
        ghostlight_dungeon::compiler::validate_campaign_runtime(&campaign)?;
    }
    let region_requests = std::env::var("GHOSTLIGHT_WORLD_REGION_REQUESTS")
        .ok()
        .map(|value| {
            value
                .split("||")
                .map(str::trim)
                .filter(|request| !request.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if region_requests.len() > 8 {
        anyhow::bail!("GHOSTLIGHT_WORLD_REGION_REQUESTS accepts at most 8 requests")
    }
    let region_plan_path = root.join("world-regions-plan.json");
    if region_plan_path.is_file() {
        let plan: WorldRegionPlanCheckpoint = read_checkpoint(&region_plan_path)?;
        if plan.schema != "ghostlight.world_region_plan.v1" || plan.requests != region_requests {
            anyhow::bail!("world-region request plan differs from its frozen checkpoint")
        }
    }
    if !region_requests.is_empty() {
        let description = compiled.as_deref().ok_or_else(|| {
            anyhow::anyhow!("world region requests require GHOSTLIGHT_WORLD_DESCRIPTION")
        })?;
        let compiler =
            strategic_world_compiler(model.clone(), description, &strategic_world_when());
        let origin_location_id = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        if region_plan_path.is_file() {
            let plan: WorldRegionPlanCheckpoint = read_checkpoint(&region_plan_path)?;
            if plan.origin_location_id != origin_location_id {
                anyhow::bail!("world-region origin differs from its frozen checkpoint")
            }
        } else {
            publish_immutable_checkpoint(
                &region_plan_path,
                &WorldRegionPlanCheckpoint {
                    schema: "ghostlight.world_region_plan.v1".into(),
                    origin_location_id: origin_location_id.clone(),
                    requests: region_requests.clone(),
                },
            )?;
        }
        let mut expanded_jurisdictions = Vec::new();
        for (index, request) in region_requests.iter().enumerate() {
            let compile_request = strategic_region_request(request);
            let preview_path = root.join(format!("world-region-{:02}-preview.json", index + 1));
            let commit_path = root.join(format!("world-region-{:02}-checkpoint.json", index + 1));
            if resume && commit_path.is_file() {
                let checkpoint: WorldRegionCommitCheckpoint = read_checkpoint(&commit_path)?;
                let prepared: WorldRegionPreviewCheckpoint = read_checkpoint(&preview_path)?;
                if checkpoint.schema != "ghostlight.world_region_expansion_checkpoint.v2"
                    || checkpoint.request != *request
                    || checkpoint.origin_location_id != origin_location_id
                    || checkpoint.commit_receipt.command_kind != "expand_region"
                    || prepared.schema != "ghostlight.world_region_expansion_preview.v1"
                    || prepared.request != *request
                    || prepared.preview.expansion.origin_location_id != origin_location_id
                {
                    anyhow::bail!("world-region checkpoint differs at index {}", index + 1)
                }
                validate_strategic_region_expansion_shape(&prepared.preview.expansion)?;
                let expected_jurisdiction_id = prepared
                    .preview
                    .expansion
                    .civic_system
                    .as_ref()
                    .map(|civic| civic.jurisdiction_location_id.clone())
                    .or_else(|| {
                        prepared
                            .preview
                            .expansion
                            .locations
                            .first()
                            .map(|location| location.id.clone())
                    })
                    .ok_or_else(|| anyhow::anyhow!("world-region expansion created no locality"))?;
                let expected_model_receipt_hashes = prepared
                    .model_receipts
                    .iter()
                    .map(|receipt| receipt.storage_key().to_owned())
                    .collect::<Vec<_>>();
                let canonical_receipt = store
                    .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
                        "world_commit_receipt.v1",
                        &format!("{}-{}", campaign.id, checkpoint.commit_receipt.revision),
                    )?
                    .map(|(_, receipt)| receipt)
                    .ok_or_else(|| anyhow::anyhow!("world-region commit receipt is missing"))?;
                let mutation_proof = committed_region_mutation_proof(
                    &store,
                    &canonical_receipt,
                    &prepared.preview.expansion,
                )?;
                if canonical_receipt != checkpoint.commit_receipt
                    || checkpoint.commit_receipt.previous_revision
                        != prepared.preview.expected_revision
                    || checkpoint.jurisdiction_location_id != expected_jurisdiction_id
                    || checkpoint.model_receipt_hashes != expected_model_receipt_hashes
                    || checkpoint.mutation_proof != mutation_proof
                    || load_checkpoint_receipts(&store, &checkpoint.model_receipt_hashes)?
                        != prepared.model_receipts
                {
                    anyhow::bail!(
                        "world-region checkpoint differs from its exact canonical expansion proof"
                    )
                }
                expanded_jurisdictions.push(checkpoint.jurisdiction_location_id);
                continue;
            }
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"expanding_world",
                    "regions_completed":index,
                    "regions_requested":region_requests.len(),
                    "current_region_request":request,
                    "waves_completed":0,
                    "waves_requested":wave_count,
                    "world_revision":campaign.revision,
                    "updated_at":Utc::now(),
                }))?,
            )?;
            let preview_checkpoint = if resume && preview_path.is_file() {
                let checkpoint: WorldRegionPreviewCheckpoint = read_checkpoint(&preview_path)?;
                if checkpoint.schema != "ghostlight.world_region_expansion_preview.v1"
                    || checkpoint.request != *request
                    || checkpoint.preview.expansion.origin_location_id != origin_location_id
                {
                    anyhow::bail!("world-region preview differs at index {}", index + 1)
                }
                checkpoint
            } else {
                let (preview, receipts) = compiler
                    .compile_destination(&campaign, &origin_location_id, &compile_request)
                    .await?;
                let ghostlight_dungeon::domain::DestinationCompilationPreview::RegionExpansion(
                    preview,
                ) = preview
                else {
                    anyhow::bail!(
                        "world-region request resolved to an existing destination: {request}"
                    )
                };
                let checkpoint = WorldRegionPreviewCheckpoint {
                    schema: "ghostlight.world_region_expansion_preview.v1".into(),
                    request: request.clone(),
                    preview,
                    model_receipts: receipts,
                };
                publish_immutable_checkpoint(&preview_path, &checkpoint)?;
                checkpoint
            };
            let WorldRegionPreviewCheckpoint {
                preview,
                model_receipts: receipts,
                ..
            } = preview_checkpoint;
            validate_strategic_region_expansion_shape(&preview.expansion)?;
            if !preview.gaps.is_empty() {
                publish_immutable_checkpoint(
                    &root.join(format!(
                        "world-region-{:02}-terminal-failure.json",
                        index + 1
                    )),
                    &serde_json::json!({
                        "schema":"ghostlight.world_region_expansion_failure.v1",
                        "request":request,
                        "preview":preview,
                        "model_receipts":receipts,
                    }),
                )?;
                anyhow::bail!(
                    "world-region expansion has unresolved canon gaps at index {}",
                    index + 1
                )
            }
            let jurisdiction_id = preview
                .expansion
                .civic_system
                .as_ref()
                .map(|civic| civic.jurisdiction_location_id.clone())
                .or_else(|| {
                    preview
                        .expansion
                        .locations
                        .first()
                        .map(|location| location.id.clone())
                })
                .ok_or_else(|| anyhow::anyhow!("world-region expansion created no locality"))?;
            let receipt = if campaign.revision == preview.expected_revision {
                let CommandResult::Committed {
                    campaign: expanded,
                    receipt,
                } = kernel
                    .command(WorldCommand::ExpandRegion {
                        expected_revision: preview.expected_revision,
                        expansion: preview.expansion.clone(),
                        evidence_receipts: preview.evidence_receipts.clone(),
                        canon_candidates: preview.canon_candidates.clone(),
                        model_stage_receipts: receipts.clone(),
                    })
                    .await?
                else {
                    anyhow::bail!("world-region expansion did not commit")
                };
                campaign = expanded;
                receipt
            } else if campaign.revision > preview.expected_revision {
                store
                    .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
                        "world_commit_receipt.v1",
                        &format!("{}-{}", campaign.id, preview.expected_revision + 1),
                    )?
                    .map(|(_, receipt)| receipt)
                    .filter(|receipt| {
                        receipt.command_kind == "expand_region"
                            && receipt.previous_revision == preview.expected_revision
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!("committed world region lacks its canonical receipt")
                    })?
            } else {
                anyhow::bail!("world-region preview no longer matches canonical campaign state")
            };
            let mutation_proof =
                committed_region_mutation_proof(&store, &receipt, &preview.expansion)?;
            publish_immutable_checkpoint(
                &commit_path,
                &WorldRegionCommitCheckpoint {
                    schema: "ghostlight.world_region_expansion_checkpoint.v2".into(),
                    request: request.clone(),
                    origin_location_id: origin_location_id.clone(),
                    jurisdiction_location_id: jurisdiction_id.clone(),
                    commit_receipt: receipt,
                    model_receipt_hashes: receipts
                        .iter()
                        .map(|receipt| receipt.storage_key().to_owned())
                        .collect(),
                    mutation_proof,
                },
            )?;
            expanded_jurisdictions.push(jurisdiction_id);
        }
        initial_seed_location_ids = expanded_jurisdictions;
    }
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
            let foundation_commit_path =
                root.join(format!("elaboration-{:02}-commit.json", index + 1));
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
                    || campaign.revision < expected_revision
                {
                    anyhow::bail!(
                        "foundation checkpoint for {location_id} does not bind the resumed campaign"
                    )
                }
                if campaign.revision == expected_revision {
                    validate_strategic_foundation_civic_shape(&expected_civic)?;
                    let command = match &checkpoint.preview {
                        ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(
                            preview,
                        ) => WorldCommand::ElaborateLocality {
                            expected_revision: preview.expected_revision,
                            elaboration: preview.elaboration.clone(),
                            evidence_receipts: preview.evidence_receipts.clone(),
                            canon_candidates: preview.canon_candidates.clone(),
                            model_stage_receipts: checkpoint.model_receipts.clone(),
                        },
                        _ => unreachable!("foundation checkpoint target was validated above"),
                    };
                    let CommandResult::Committed {
                        campaign: elaborated,
                        receipt,
                    } = kernel.command(command).await?
                    else {
                        anyhow::bail!("prepared foundation did not commit")
                    };
                    let locality_preview = match &checkpoint.preview {
                        ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(preview) => preview,
                        _ => unreachable!("foundation checkpoint target was validated above"),
                    };
                    let expected_commit = expected_foundation_commit_checkpoint(
                        &store,
                        location_id,
                        &location_name,
                        &foundation_request,
                        locality_preview,
                        &checkpoint.model_receipts,
                        receipt,
                    )?;
                    if foundation_commit_path.is_file() {
                        if read_checkpoint::<FoundationCommitCheckpoint>(&foundation_commit_path)?
                            != expected_commit
                        {
                            anyhow::bail!(
                                "foundation completion checkpoint for {location_id} differs from its exact canonical proof"
                            )
                        }
                    } else {
                        publish_immutable_checkpoint(&foundation_commit_path, &expected_commit)?;
                    }
                    (elaborated, checkpoint.model_receipts)
                } else {
                    let locality_preview = match &checkpoint.preview {
                        ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(preview) => preview,
                        _ => unreachable!("foundation checkpoint target was validated above"),
                    };
                    let receipt = store
                        .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
                            "world_commit_receipt.v1",
                            &format!("{}-{}", campaign.id, expected_revision.saturating_add(1)),
                        )?
                        .map(|(_, receipt)| receipt)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "foundation checkpoint for {location_id} lacks its canonical world receipt"
                            )
                        })?;
                    let expected_commit = expected_foundation_commit_checkpoint(
                        &store,
                        location_id,
                        &location_name,
                        &foundation_request,
                        locality_preview,
                        &checkpoint.model_receipts,
                        receipt,
                    )?;
                    if foundation_commit_path.is_file() {
                        if read_checkpoint::<FoundationCommitCheckpoint>(&foundation_commit_path)?
                            != expected_commit
                        {
                            anyhow::bail!(
                                "foundation completion checkpoint for {location_id} differs from its exact canonical proof"
                            )
                        }
                    } else {
                        publish_immutable_checkpoint(&foundation_commit_path, &expected_commit)?;
                    }
                    if load_checkpoint_receipts(&store, &expected_commit.model_receipt_hashes)?
                        != checkpoint.model_receipts
                    {
                        anyhow::bail!("foundation checkpoint model ancestry was substituted")
                    }
                    (campaign.clone(), checkpoint.model_receipts)
                }
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
                ) => {
                    validate_strategic_foundation_civic_shape(
                        preview
                            .elaboration
                            .expansion
                            .civic_system
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!(
                                "strategic locality foundation has no civic manifest"
                            ))?,
                    )?;
                    WorldCommand::ElaborateLocality {
                        expected_revision: preview.expected_revision,
                        elaboration: preview.elaboration.clone(),
                        evidence_receipts: preview.evidence_receipts.clone(),
                        canon_candidates: preview.canon_candidates.clone(),
                        model_stage_receipts: receipts.clone(),
                    }
                },
                ghostlight_dungeon::domain::DestinationCompilationPreview::RegionExpansion(_) => {
                    anyhow::bail!(
                        "strategic elaboration resolved existing location {location_id} as a new destination"
                    )
                }
            };
                publish_immutable_checkpoint(
                    &preview_path,
                    &serde_json::json!({
                        "location_id":location_id,
                        "location_name":location_name,
                        "request":foundation_request,
                        "preview":&preview,
                        "model_receipts":&receipts,
                    }),
                )?;
                let committed = kernel.command(command).await?;
                let CommandResult::Committed {
                    campaign: elaborated,
                    receipt,
                } = committed
                else {
                    anyhow::bail!("strategic locality elaboration did not commit")
                };
                let locality_preview = match &preview {
                    ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(preview) => preview,
                    _ => unreachable!("foundation command target was validated above"),
                };
                let expected_commit = expected_foundation_commit_checkpoint(
                    &store,
                    location_id,
                    &location_name,
                    &foundation_request,
                    locality_preview,
                    &receipts,
                    receipt,
                )?;
                publish_immutable_checkpoint(&foundation_commit_path, &expected_commit)?;
                (elaborated, receipts)
            };
            campaign = elaborated;
            let titled_preview_path =
                root.join(format!("titled-elaboration-{:02}-preview.json", index + 1));
            let titled_commit_path =
                root.join(format!("titled-elaboration-{:02}-commit.json", index + 1));
            if resume && titled_preview_path.is_file() {
                let titled: TitledReadyCheckpoint = read_checkpoint(&titled_preview_path)?;
                let admission = titled.finalized.admission();
                let candidate = admission.candidate().ok_or_else(|| {
                    anyhow::anyhow!("prepared titled checkpoint has no candidate")
                })?;
                let candidate_civic =
                    candidate.expansion.civic_system.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("prepared titled checkpoint has no civic candidate")
                    })?;
                if titled.schema != "ghostlight.titled_elaboration_ready.v1"
                    || titled.location_id != *location_id
                    || titled.location_name != location_name
                    || titled.request
                        != strategic_titled_locality_request(&location_name, location_id, &pressure)
                    || candidate.target_location_id != *location_id
                    || admission.target_location_id() != *location_id
                {
                    anyhow::bail!(
                        "prepared titled checkpoint for {location_id} is internally inconsistent"
                    )
                }
                let verifier_receipt = titled.finalized.semantic_verifier_receipt().clone();
                let world_revision_before = admission.expected_revision();
                let world_revision_after = world_revision_before.saturating_add(1);
                let finalized_expansion =
                    finalized_titled_expansion(candidate, verifier_receipt.storage_key())?;
                let (titled_elaborated, persisted_commit_receipt) = if campaign.revision
                    == world_revision_before
                {
                    let CommandResult::Committed {
                        campaign: advanced,
                        receipt,
                    } = kernel
                        .commit_elaboration(titled.finalized.clone())
                        .await
                        .map_err(anyhow::Error::new)?
                    else {
                        anyhow::bail!("prepared titled elaboration did not commit")
                    };
                    (advanced, receipt)
                } else if campaign.revision == world_revision_after {
                    let civic = campaign.civic_systems.get(location_id).ok_or_else(|| {
                        anyhow::anyhow!("resumed campaign lacks civic system for {location_id}")
                    })?;
                    if !civic_manifest_preserves(&campaign, civic, candidate_civic)
                        || civic.semantic_verification_receipt_id != verifier_receipt.storage_key()
                    {
                        anyhow::bail!(
                            "prepared titled elaboration for {location_id} is not the canonical committed civic system"
                        )
                    }
                    let receipt = store
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
                    (campaign.clone(), receipt)
                } else {
                    anyhow::bail!(
                        "prepared titled elaboration for {location_id} cannot recover from world revision {}",
                        campaign.revision
                    )
                };
                let mutation_proof = committed_elaboration_mutation_proof(
                    &store,
                    &persisted_commit_receipt,
                    &finalized_expansion,
                )?;
                let admitted_model_receipt_hashes = admission
                    .model_stage_receipts()
                    .iter()
                    .map(|receipt| receipt.storage_key().to_owned())
                    .collect::<Vec<_>>();
                let expected_commit_checkpoint = TitledCommitCheckpoint {
                    schema: "ghostlight.titled_elaboration_commit.v1".into(),
                    location_id: location_id.clone(),
                    location_name: location_name.clone(),
                    world_revision_before,
                    world_revision_after,
                    wave: admission.wave().clone(),
                    schedule: admission.schedule().clone(),
                    admission_digest: Some(admission.digest().into()),
                    verifier_receipt_hash: verifier_receipt.storage_key().into(),
                    model_receipt_hashes: admitted_model_receipt_hashes.clone(),
                    commit_receipt: persisted_commit_receipt.clone(),
                    mutation_proof: mutation_proof.clone(),
                    legacy_inferred: false,
                };
                let commit_checkpoint = if titled_commit_path.is_file() {
                    read_checkpoint::<TitledCommitCheckpoint>(&titled_commit_path)?
                } else {
                    publish_immutable_checkpoint(&titled_commit_path, &expected_commit_checkpoint)?;
                    expected_commit_checkpoint.clone()
                };
                if commit_checkpoint != expected_commit_checkpoint {
                    anyhow::bail!(
                        "titled completion checkpoint for {location_id} is not bound to its canonical commit"
                    )
                }
                titled_scheduler = ElaborationScheduler::from_state(
                    &titled_profile,
                    admission.schedule().final_state.clone(),
                )?;
                campaign = titled_elaborated;
                elaboration_reports.push(serde_json::json!({
                    "location_id":location_id,
                    "location_name":location_name,
                    "world_revision":world_revision_after,
                    "preview_path":preview_path,
                    "titled_preview_path":titled_preview_path,
                    "titled_commit_path":titled_commit_path,
                    "titled_model_receipts":admission.model_stage_receipts(),
                    "titled_semantic_verifier_receipt":verifier_receipt,
                    "model_receipts":receipts,
                    "resumed_committed_checkpoint":true,
                    "original_wave":admission.wave(),
                    "original_resume_source":titled.resumed_from,
                    "original_retried_dispatch_ordinals":titled.retried_dispatch_ordinals,
                }));
                continue;
            }
            let titled_wave = world_elaboration_wave_binding(&campaign, location_id)?;
            let base_titled_request =
                strategic_titled_locality_request(&location_name, location_id, &pressure);
            let semantic_failure_one_path = root.join(format!(
                "titled-elaboration-{:02}-semantic-failure-01.json",
                index + 1
            ));
            let semantic_failure_two_path = root.join(format!(
                "titled-elaboration-{:02}-semantic-failure-02.json",
                index + 1
            ));
            if resume && semantic_failure_two_path.is_file() {
                if !semantic_failure_one_path.is_file() {
                    anyhow::bail!(
                        "terminal titled semantic failure for {location_id} lost its first-attempt authority"
                    )
                }
                let first: TitledSemanticFailureCheckpoint =
                    read_checkpoint(&semantic_failure_one_path)?;
                validate_titled_semantic_failure_checkpoint(
                    &store,
                    &campaign,
                    &first,
                    1,
                    location_id,
                    &location_name,
                    &base_titled_request,
                    &base_titled_request,
                    &titled_wave,
                )?;
                let repair_request = first.repair_request.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("first titled semantic failure has no repair request")
                })?;
                let terminal: TitledSemanticFailureCheckpoint =
                    read_checkpoint(&semantic_failure_two_path)?;
                validate_titled_semantic_failure_checkpoint(
                    &store,
                    &campaign,
                    &terminal,
                    2,
                    location_id,
                    &location_name,
                    &base_titled_request,
                    repair_request,
                    &titled_wave,
                )?;
                anyhow::bail!(
                    "titled semantic repair was already rejected for {location_id}: {}",
                    terminal.diagnostic
                )
            }
            let semantic_resume = if resume && semantic_failure_one_path.is_file() {
                let checkpoint: TitledSemanticFailureCheckpoint =
                    read_checkpoint(&semantic_failure_one_path)?;
                validate_titled_semantic_failure_checkpoint(
                    &store,
                    &campaign,
                    &checkpoint,
                    1,
                    location_id,
                    &location_name,
                    &base_titled_request,
                    &base_titled_request,
                    &titled_wave,
                )?;
                Some(checkpoint)
            } else {
                None
            };
            let original_failure_path = root.join(format!(
                "titled-elaboration-{:02}-terminal-failure.json",
                index + 1
            ));
            let failure_checkpoint_paths = titled_failure_checkpoint_paths(&root, index + 1)?;
            let mut checkpoint_path = failure_checkpoint_paths.last().cloned();
            let mut checkpoint_for_resume = checkpoint_path
                .as_ref()
                .map(|path| read_checkpoint::<TitledFailureCheckpoint>(path))
                .transpose()?;
            if let Some(semantic) = semantic_resume.as_ref()
                && checkpoint_for_resume.as_ref().is_some_and(|checkpoint| {
                    checkpoint.request != semantic.repair_request.as_deref().unwrap_or_default()
                })
            {
                checkpoint_path = None;
                checkpoint_for_resume = None;
            }
            if let (Some(semantic), Some(checkpoint)) =
                (semantic_resume.as_ref(), checkpoint_for_resume.as_ref())
                && checkpoint.semantic_retry_diagnostic.as_deref()
                    != Some(semantic.diagnostic.as_str())
            {
                anyhow::bail!("titled repair failure checkpoint lost its semantic retry diagnostic")
            }
            if checkpoint_for_resume.is_none()
                && let Some(semantic) = semantic_resume.as_ref()
            {
                titled_scheduler = ElaborationScheduler::from_state(
                    &titled_profile,
                    semantic.schedule.final_state.clone(),
                )?;
            }
            let active_titled_request = checkpoint_for_resume
                .as_ref()
                .map(|checkpoint| checkpoint.request.clone())
                .or_else(|| {
                    semantic_resume
                        .as_ref()
                        .and_then(|checkpoint| checkpoint.repair_request.clone())
                })
                .unwrap_or_else(|| base_titled_request.clone());
            let initial_semantic_retry_diagnostic = semantic_resume
                .as_ref()
                .map(|checkpoint| checkpoint.diagnostic.clone());
            let mut verification_request = active_titled_request.clone();
            let verifier_semantic_attempt = if semantic_resume.is_some() { 2 } else { 1 };
            let verifier_execution_paths = titled_verifier_execution_failure_checkpoint_paths(
                &root,
                index + 1,
                verifier_semantic_attempt,
            )?;
            let verifier_execution_resume = if resume {
                let mut latest = None;
                let mut prior_admission_digest = None::<String>;
                let mut prior_verifier_hashes = std::collections::BTreeSet::<String>::new();
                for (generation, path) in &verifier_execution_paths {
                    let checkpoint =
                        read_checkpoint::<TitledVerifierExecutionFailureCheckpoint>(path)?;
                    validate_titled_verifier_execution_failure_checkpoint(
                        &store,
                        &campaign,
                        &checkpoint,
                        verifier_semantic_attempt,
                        *generation,
                        location_id,
                        &location_name,
                        &verification_request,
                    )?;
                    let verifier_hashes = checkpoint
                        .verifier_receipt_hashes
                        .iter()
                        .cloned()
                        .collect::<std::collections::BTreeSet<_>>();
                    if let Some(prior_digest) = prior_admission_digest.as_deref()
                        && (checkpoint.admission.digest() != prior_digest
                            || !verifier_hashes.is_superset(&prior_verifier_hashes))
                    {
                        anyhow::bail!(
                            "titled verifier-execution recovery generations changed admission or lost failed verifier ancestry"
                        )
                    }
                    prior_admission_digest = Some(checkpoint.admission.digest().into());
                    prior_verifier_hashes = verifier_hashes;
                    latest = Some((path.clone(), checkpoint));
                }
                latest
            } else {
                None
            };
            if let Some((path, _)) = verifier_execution_resume.as_ref() {
                checkpoint_path = Some(path.clone());
            }
            let next_failure_path = if failure_checkpoint_paths.is_empty() {
                original_failure_path
            } else {
                root.join(format!(
                    "titled-elaboration-{:02}-resume-{:02}-terminal-failure.json",
                    index + 1,
                    failure_checkpoint_paths.len()
                ))
            };
            let mut retried_dispatch_ordinals = Vec::new();
            let (mut proposal_receipts, mut admission, mut verifier_execution_receipt_hashes) =
                if let Some((_, checkpoint)) = verifier_execution_resume {
                    titled_scheduler = ElaborationScheduler::from_state(
                        &titled_profile,
                        checkpoint.admission.schedule().final_state.clone(),
                    )?;
                    let proposal_receipts = checkpoint.admission.model_stage_receipts().to_vec();
                    (
                        proposal_receipts,
                        checkpoint.admission,
                        checkpoint.verifier_receipt_hashes,
                    )
                } else {
                    let titled_worker = Arc::new(ModelWorldElaborationWorker::new(
                        model.clone(),
                        Arc::new(campaign.clone()),
                        location_id.clone(),
                        active_titled_request,
                    )?);
                    let titled_result = if resume {
                        if let Some(checkpoint) = checkpoint_for_resume {
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
                                .ok_or_else(|| {
                                    anyhow::anyhow!("resume checkpoint has no schedule")
                                })?
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
                                titled_wave.clone(),
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
                            titled_wave.clone(),
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
                                    "semantic_retry_diagnostic":initial_semantic_retry_diagnostic,
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
                    let admission =
                        admit_world_elaboration_wave(&campaign, location_id, titled_run)?;
                    (proposal_receipts, admission, Vec::new())
                };
            let mut semantic_retry_diagnostic = initial_semantic_retry_diagnostic;
            let mut verifier_execution_failures = 0_u8;
            let (candidate, verifier_receipt) = loop {
                let candidate = admission.candidate().cloned().ok_or_else(|| {
                    anyhow::anyhow!(
                        "titled elaboration produced no candidate: {}",
                        admission.candidate_diagnostic().unwrap_or("no diagnostic")
                    )
                })?;
                if let Some(diagnostic) = admission.candidate_diagnostic() {
                    anyhow::bail!(
                        "titled elaboration candidate requires reconciliation: {diagnostic}"
                    );
                }
                let causal_receipt_ids = admission
                    .model_stage_receipts()
                    .iter()
                    .map(|receipt| receipt.storage_key().to_owned())
                    .collect::<Vec<_>>();
                match compiler
                    .verify_titled_locality_elaboration(
                        &campaign,
                        &verification_request,
                        description,
                        &candidate,
                        &causal_receipt_ids,
                    )
                    .await
                {
                    Ok(receipt) => break (candidate, receipt),
                    Err(error) => {
                        let failure = error.downcast_ref::<
                            ghostlight_dungeon::compiler::CivicElaborationVerificationFailure,
                        >();
                        if let Some(failure) = failure {
                            store.persist_model_stage_receipts(&failure.model_receipts)?;
                            verifier_execution_receipt_hashes.extend(
                                failure
                                    .model_receipts
                                    .iter()
                                    .map(|receipt| receipt.storage_key().to_owned()),
                            );
                        }
                        if failure
                            .and_then(|failure| failure.semantic_diagnostic.as_ref())
                            .is_none()
                        {
                            verifier_execution_failures =
                                verifier_execution_failures.saturating_add(1);
                            if failure.is_some() && verifier_execution_failures < 3 {
                                continue;
                            }
                            verifier_execution_receipt_hashes.sort();
                            verifier_execution_receipt_hashes.dedup();
                            let diagnostic = failure
                                .map(|failure| failure.message.clone())
                                .unwrap_or_else(|| error.to_string());
                            let semantic_attempt = if semantic_retry_diagnostic.is_some() {
                                2
                            } else {
                                1
                            };
                            let prior_failures =
                                titled_verifier_execution_failure_checkpoint_paths(
                                    &root,
                                    index + 1,
                                    semantic_attempt,
                                )?;
                            let generation = prior_failures
                                .last()
                                .map_or(1, |(generation, _)| generation.saturating_add(1));
                            let failure_path = root.join(format!(
                                "titled-elaboration-{:02}-verifier-execution-failure-{semantic_attempt:02}-generation-{generation:03}.json",
                                index + 1,
                            ));
                            publish_immutable_checkpoint(
                                &failure_path,
                                &TitledVerifierExecutionFailureCheckpoint {
                                    schema: "ghostlight.titled_verifier_execution_failure.v2"
                                        .into(),
                                    semantic_attempt,
                                    generation,
                                    location_id: location_id.clone(),
                                    location_name: location_name.clone(),
                                    verification_request: verification_request.clone(),
                                    diagnostic: diagnostic.clone(),
                                    attempts: verifier_execution_failures,
                                    admission: admission.clone(),
                                    model_receipt_hashes: admission
                                        .model_stage_receipts()
                                        .iter()
                                        .map(|receipt| receipt.storage_key().to_owned())
                                        .collect(),
                                    verifier_receipt_hashes: verifier_execution_receipt_hashes
                                        .clone(),
                                },
                            )?;
                            anyhow::bail!(
                                "titled civic verifier execution failed without rejecting the candidate for {location_id}; exact retry authority at {}: {diagnostic}",
                                failure_path.display()
                            )
                        }
                        let diagnostic = failure
                            .and_then(|failure| failure.semantic_diagnostic.clone())
                            .expect("semantic rejection was established above");
                        let repair_request = strategic_titled_repair_request(
                            &location_name,
                            location_id,
                            &pressure,
                            &diagnostic,
                        );
                        let semantic_failure = TitledSemanticFailureCheckpoint {
                            schema: "ghostlight.titled_elaboration_semantic_failure.v2".into(),
                            attempt: if semantic_retry_diagnostic.is_some() {
                                2
                            } else {
                                1
                            },
                            location_id: location_id.clone(),
                            location_name: location_name.clone(),
                            base_request: base_titled_request.clone(),
                            verification_request: verification_request.clone(),
                            repair_request: semantic_retry_diagnostic
                                .is_none()
                                .then_some(repair_request.clone()),
                            diagnostic: diagnostic.clone(),
                            admission: admission.clone(),
                            wave: admission.wave().clone(),
                            schedule: admission.schedule().clone(),
                            model_receipt_hashes: admission
                                .model_stage_receipts()
                                .iter()
                                .map(|receipt| receipt.storage_key().to_owned())
                                .collect(),
                            verifier_receipt_hashes: failure
                                .into_iter()
                                .flat_map(|failure| failure.model_receipts.iter())
                                .map(|receipt| receipt.storage_key().to_owned())
                                .collect(),
                        };
                        let semantic_failure_path = if semantic_retry_diagnostic.is_some() {
                            &semantic_failure_two_path
                        } else {
                            &semantic_failure_one_path
                        };
                        publish_immutable_checkpoint(semantic_failure_path, &semantic_failure)?;
                        if semantic_retry_diagnostic.is_some() {
                            return Err(error);
                        }
                        semantic_retry_diagnostic = Some(diagnostic.clone());
                        let repair_worker = Arc::new(ModelWorldElaborationWorker::new(
                            model.clone(),
                            Arc::new(campaign.clone()),
                            location_id.clone(),
                            repair_request.clone(),
                        )?);
                        let repair_result = dispatch_elaboration_wave(
                            &mut titled_scheduler,
                            titled_wave.clone(),
                            &titled_eligible,
                            titled_invocation_budget,
                            titled_parallelism,
                            repair_worker,
                        )
                        .await;
                        let repair_run = match repair_result {
                            Ok(run) => run,
                            Err(failure) => {
                                let receipts = failure
                                    .completed_invocations
                                    .iter()
                                    .flat_map(|invocation| invocation.model_stage_receipts.iter())
                                    .chain(
                                        failure.invocation_failures.iter().flat_map(|failure| {
                                            failure.model_stage_receipts.iter()
                                        }),
                                    )
                                    .cloned()
                                    .collect::<Vec<_>>();
                                if !receipts.is_empty() {
                                    store.persist_model_stage_receipts(&receipts)?;
                                }
                                publish_immutable_checkpoint(
                                    &next_failure_path,
                                    &serde_json::json!({
                                        "schema":"ghostlight.titled_elaboration_failure.v1",
                                        "location_id":location_id,
                                        "location_name":location_name,
                                        "request":repair_request,
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
                                        "semantic_retry_diagnostic":semantic_retry_diagnostic,
                                    }),
                                )?;
                                anyhow::bail!(
                                    "titled semantic retry wave failed for {location_id}; exact receipt at {}",
                                    next_failure_path.display()
                                )
                            }
                        };
                        let repair_receipts = repair_run
                            .invocations()
                            .iter()
                            .flat_map(|invocation| invocation.model_stage_receipts.iter())
                            .cloned()
                            .collect::<Vec<_>>();
                        store.persist_model_stage_receipts(&repair_receipts)?;
                        proposal_receipts.extend(repair_receipts);
                        verification_request = repair_request;
                        admission =
                            admit_world_elaboration_wave(&campaign, location_id, repair_run)?;
                        verifier_execution_failures = 0;
                        verifier_execution_receipt_hashes.clear();
                    }
                }
            };
            let admission_digest = admission.digest().to_owned();
            let admitted_wave = admission.wave().clone();
            let admitted_schedule = admission.schedule().clone();
            let admitted_model_receipt_hashes = admission
                .model_stage_receipts()
                .iter()
                .map(|receipt| receipt.storage_key().to_owned())
                .collect::<Vec<_>>();
            store.persist_model_stage_receipts(std::slice::from_ref(&verifier_receipt))?;
            let finalized_expansion =
                finalized_titled_expansion(&candidate, verifier_receipt.storage_key())?;
            let finalized =
                finalize_world_elaboration(&campaign, admission, verifier_receipt.clone())?;
            publish_immutable_checkpoint(
                &titled_preview_path,
                &TitledReadyCheckpoint {
                    schema: "ghostlight.titled_elaboration_ready.v1".into(),
                    location_id: location_id.clone(),
                    location_name: location_name.clone(),
                    request: base_titled_request.clone(),
                    finalized: finalized.clone(),
                    resumed_from: checkpoint_path,
                    retried_dispatch_ordinals,
                    semantic_retry_diagnostic,
                },
            )?;
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
    let target_cover_basis_points =
        bounded_environment_usize("GHOSTLIGHT_WORLD_TARGET_COVER_BASIS_POINTS", 0, 0, 10_000)?
            as u16;
    let mut complexity_reports = Vec::new();
    if target_cover_basis_points > 0 {
        if initial_location_ids.is_empty() {
            anyhow::bail!("world complexity scaling requires admitted elaboration jurisdictions")
        }
        let complexity_parallelism =
            bounded_environment_usize("GHOSTLIGHT_WORLD_COMPLEXITY_PARALLELISM", 8, 1, 32)?;
        let maximum_complexity_rounds =
            bounded_environment_usize("GHOSTLIGHT_WORLD_COMPLEXITY_MAX_ROUNDS", 32, 1, 128)?;
        let scale_intent = WorldScaleIntent {
            schema: "ghostlight.world_scale_intent.v1".into(),
            target_active_cover_basis_points: target_cover_basis_points,
        };
        let realm_weights = initial_location_ids
            .iter()
            .cloned()
            .map(|location_id| (location_id, 1_u32))
            .collect::<BTreeMap<_, _>>();
        let complexity_profile = strategic_world_elaboration_profile();
        let eligible_titles = ElaboratorTitle::ALL.into_iter().collect::<BTreeSet<_>>();
        let mut complexity_scheduler = ElaborationScheduler::new(&complexity_profile)?;
        let mut session_checkpoints = BTreeMap::<String, ElaboratorSessionCheckpoint>::new();
        let mut completed_complexity_rounds = 0_u32;
        let mut last_completed_complexity_count = None;
        if resume {
            for round in 1..=maximum_complexity_rounds as u32 {
                let path = root.join(format!("complexity-round-{round:03}-checkpoint.json"));
                if !path.is_file() {
                    break;
                }
                let checkpoint: ComplexityRoundCheckpoint = read_checkpoint(&path)?;
                validate_completed_complexity_round_checkpoint(
                    &root,
                    &store,
                    &campaign,
                    &checkpoint,
                    round,
                )?;
                complexity_scheduler = ElaborationScheduler::from_state(
                    &complexity_profile,
                    checkpoint.schedule.final_state.clone(),
                )?;
                for (session_id, session) in &checkpoint.session_checkpoints {
                    if session.session_id != *session_id
                        || elaborator_session_id(session.title, &session.target_location_id)
                            != *session_id
                    {
                        anyhow::bail!("complexity session checkpoint is misrouted")
                    }
                    session.validate_for(&campaign, &session.target_location_id, session.title)?;
                }
                session_checkpoints = checkpoint.session_checkpoints.clone();
                completed_complexity_rounds = round;
                last_completed_complexity_count = Some(checkpoint.actionable_subjects_after);
                complexity_reports.push(serde_json::to_value(checkpoint)?);
            }
        }
        for round in completed_complexity_rounds + 1..=maximum_complexity_rounds as u32 {
            let preview_path = root.join(format!("complexity-round-{round:03}-preview.json"));
            let resumed_preview = if resume && preview_path.is_file() {
                Some(read_checkpoint::<ComplexityRoundPreviewCheckpoint>(
                    &preview_path,
                )?)
            } else {
                None
            };
            let demand = if let Some(checkpoint) = resumed_preview.as_ref() {
                checkpoint.demand.clone()
            } else {
                derive_world_elaboration_demand(
                    u16::from(campaign.resolution_policy.active_cell_budget),
                    canonical_actionable_subject_count(&campaign),
                    &scale_intent,
                    realm_weights.clone(),
                )?
            };
            let actionable_before = demand.current_actionable_subjects;
            if demand.actionable_subject_deficit == 0 {
                break;
            }
            let preview = if let Some(checkpoint) = resumed_preview {
                if checkpoint.schema != "ghostlight.complexity_round_preview.v1"
                    || checkpoint.round != round
                    || checkpoint.invocations.is_empty()
                {
                    anyhow::bail!("complexity round preview checkpoint is stale or malformed")
                }
                complexity_scheduler = ElaborationScheduler::from_state(
                    &complexity_profile,
                    checkpoint.schedule.final_state.clone(),
                )?;
                checkpoint
            } else {
                let parents = complexity_parent_candidates(
                    &campaign,
                    &demand,
                    usize::try_from(demand.round_mutation_budget).unwrap_or(usize::MAX),
                );
                if parents.is_empty() {
                    anyhow::bail!(
                        "world complexity remains {} subjects short but no active Gestalt can be subdivided",
                        demand.actionable_subject_deficit
                    )
                }
                let failure_paths = complexity_failure_checkpoint_paths(&root, round)?;
                let latest_failure = failure_paths.last().cloned();
                let first_dispatch_ordinal = if let Some(path) = latest_failure.as_ref() {
                    let checkpoint: ComplexityRoundFailureCheckpoint = read_checkpoint(path)?;
                    if checkpoint.round != round
                        || checkpoint.demand != demand
                        || checkpoint.parent_gestalt_ids != parents
                    {
                        anyhow::bail!("complexity failure checkpoint is stale or malformed")
                    }
                    checkpoint
                        .schedule
                        .as_ref()
                        .and_then(|schedule| schedule.dispatches.first())
                        .map(|dispatch| dispatch.ordinal)
                        .ok_or_else(|| anyhow::anyhow!("complexity failure has no dispatches"))?
                } else {
                    complexity_scheduler
                        .state()
                        .total_dispatches
                        .saturating_add(1)
                };
                let parent_jurisdiction_ids = parents
                    .iter()
                    .map(|parent_id| {
                        let profile = campaign.agency_profiles.get(parent_id).ok_or_else(|| {
                            anyhow::anyhow!("complexity parent has no agency profile")
                        })?;
                        let jurisdiction_id = complexity_realm_for_profile(
                            &campaign, profile, &demand,
                        )
                        .ok_or_else(|| {
                            anyhow::anyhow!("complexity parent has no demanded realm jurisdiction")
                        })?;
                        Ok((parent_id.clone(), jurisdiction_id))
                    })
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
                let worker = Arc::new(ModelWorldComplexityWorker::new(
                    model.clone(),
                    Arc::new(campaign.clone()),
                    first_dispatch_ordinal,
                    parents.clone(),
                    parent_jurisdiction_ids,
                    demand.actionable_subject_deficit,
                    session_checkpoints.clone(),
                )?);
                let result = if let Some(path) = latest_failure.as_ref() {
                    let checkpoint: ComplexityRoundFailureCheckpoint = read_checkpoint(path)?;
                    let scheduler_state = checkpoint
                        .schedule
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("complexity failure has no schedule"))?
                        .final_state
                        .clone();
                    complexity_scheduler =
                        ElaborationScheduler::from_state(&complexity_profile, scheduler_state)?;
                    resume_elaboration_wave(
                        rehydrate_complexity_failure(checkpoint, &store)?,
                        complexity_parallelism,
                        worker,
                    )
                    .await
                } else {
                    dispatch_elaboration_wave(
                        &mut complexity_scheduler,
                        worker.wave().clone(),
                        &eligible_titles,
                        u32::try_from(parents.len()).unwrap_or(u32::MAX),
                        complexity_parallelism,
                        worker,
                    )
                    .await
                };
                let run = match result {
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
                        let failure_path = if latest_failure.is_some() {
                            root.join(format!(
                                "complexity-round-{round:03}-resume-{:02}-terminal-failure.json",
                                failure_paths.len()
                            ))
                        } else {
                            root.join(format!("complexity-round-{round:03}-terminal-failure.json"))
                        };
                        let checkpoint = ComplexityRoundFailureCheckpoint {
                            schema: "ghostlight.complexity_round_failure.v1".into(),
                            round,
                            demand: demand.clone(),
                            parent_gestalt_ids: parents.clone(),
                            wave: failure.wave,
                            schedule: failure.schedule,
                            completed_invocations: failure
                                .completed_invocations
                                .iter()
                                .map(|invocation| {
                                    Ok(ComplexityPreviewInvocation {
                                        dispatch: invocation.dispatch.clone(),
                                        parent_binding: world_complexity_parent_binding(
                                            &campaign,
                                            invocation.proposal.parent_gestalt_id(),
                                        )?,
                                        proposal: invocation.proposal.clone(),
                                        model_receipt_hashes: invocation
                                            .model_stage_receipts
                                            .iter()
                                            .map(|receipt| receipt.storage_key().to_owned())
                                            .collect(),
                                    })
                                })
                                .collect::<anyhow::Result<Vec<_>>>()?,
                            invocation_failures: failure
                                .invocation_failures
                                .iter()
                                .map(|failure| ComplexityFailedInvocation {
                                    dispatch: failure.dispatch.clone(),
                                    diagnostic: failure.diagnostic.clone(),
                                    model_receipt_hashes: failure
                                        .model_stage_receipts
                                        .iter()
                                        .map(|receipt| receipt.storage_key().to_owned())
                                        .collect(),
                                })
                                .collect(),
                        };
                        publish_immutable_checkpoint(&failure_path, &checkpoint)?;
                        anyhow::bail!(
                            "world complexity round {round} failed; exact checkpoint at {}",
                            failure_path.display()
                        )
                    }
                };
                let receipts = run
                    .invocations()
                    .iter()
                    .flat_map(|invocation| invocation.model_stage_receipts.iter())
                    .cloned()
                    .collect::<Vec<_>>();
                store.persist_model_stage_receipts(&receipts)?;
                let checkpoint = ComplexityRoundPreviewCheckpoint {
                    schema: "ghostlight.complexity_round_preview.v1".into(),
                    round,
                    demand: demand.clone(),
                    frozen_world_revision: campaign.revision,
                    wave: run.wave().clone(),
                    schedule: run.schedule().clone(),
                    invocations: run
                        .invocations()
                        .iter()
                        .map(|invocation| {
                            Ok(ComplexityPreviewInvocation {
                                dispatch: invocation.dispatch.clone(),
                                parent_binding: world_complexity_parent_binding(
                                    &campaign,
                                    invocation.proposal.parent_gestalt_id(),
                                )?,
                                proposal: invocation.proposal.clone(),
                                model_receipt_hashes: invocation
                                    .model_stage_receipts
                                    .iter()
                                    .map(|receipt| receipt.storage_key().to_owned())
                                    .collect(),
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?,
                };
                publish_immutable_checkpoint(&preview_path, &checkpoint)?;
                checkpoint
            };
            let (retained_invocations, superseded_invocations) =
                retain_unique_complexity_invocations(round, &preview.invocations);
            let mut superseded_paths = Vec::new();
            for superseded in superseded_invocations {
                let path = root.join(format!(
                    "complexity-round-{round:03}-superseded-{:04}.json",
                    superseded.dispatch.ordinal
                ));
                if path.is_file() {
                    let checkpoint: ComplexitySupersededInvocationCheckpoint =
                        read_checkpoint(&path)?;
                    if checkpoint != superseded {
                        anyhow::bail!("complexity supersession checkpoint is inconsistent")
                    }
                } else {
                    publish_immutable_checkpoint(&path, &superseded)?;
                }
                superseded_paths.push(path);
            }
            let mut mutation_paths = Vec::new();
            let mut session_routes = BTreeMap::<String, (ElaboratorTitle, String)>::new();
            let mut invocation_sessions = BTreeMap::<u64, String>::new();
            for invocation in &retained_invocations {
                let location_id = match &invocation.proposal {
                    WorldComplexityProposal::Fission { qualification, .. } => {
                        qualification.jurisdiction_location_id.clone()
                    }
                    WorldComplexityProposal::Individuate { qualification, .. } => {
                        qualification.jurisdiction_location_id.clone()
                    }
                };
                let session_id = elaborator_session_id(invocation.dispatch.title, &location_id);
                invocation_sessions.insert(invocation.dispatch.ordinal, session_id.clone());
                session_routes.insert(session_id, (invocation.dispatch.title, location_id));
            }
            let mut journals = BTreeMap::<String, Vec<ElaboratorSessionJournalEntry>>::new();
            let mut recent_rejection_findings = BTreeMap::<String, Vec<String>>::new();
            let mut accepted_round_semantic_deltas = Vec::<String>::new();
            for invocation in &retained_invocations {
                let commit_path = root.join(format!(
                    "complexity-round-{round:03}-commit-{:04}.json",
                    invocation.dispatch.ordinal
                ));
                let semantic_rejection_path = root.join(format!(
                    "complexity-round-{round:03}-semantic-rejection-{:04}.json",
                    invocation.dispatch.ordinal
                ));
                let prepared_path = root.join(format!(
                    "complexity-round-{round:03}-prepared-{:04}.json",
                    invocation.dispatch.ordinal
                ));
                if semantic_rejection_path.is_file() {
                    let rejection = validate_complexity_semantic_rejection_checkpoint(
                        &store,
                        &campaign,
                        round,
                        invocation,
                        &semantic_rejection_path,
                    )?;
                    let session_id = invocation_sessions
                        .get(&invocation.dispatch.ordinal)
                        .ok_or_else(|| {
                            anyhow::anyhow!("complexity rejection has no session route")
                        })?;
                    recent_rejection_findings
                        .entry(session_id.clone())
                        .or_default()
                        .push(bounded_prompt_excerpt(&rejection.diagnostic, 800));
                    superseded_paths.push(semantic_rejection_path);
                    continue;
                }
                let checkpoint = if commit_path.is_file() {
                    read_checkpoint::<ComplexityMutationCheckpoint>(&commit_path)?
                } else {
                    let prepared = if prepared_path.is_file() {
                        let prepared = read_checkpoint::<ComplexityPreparedMutationCheckpoint>(
                            &prepared_path,
                        )?;
                        let derived_affected_subject_ids =
                            complexity_affected_subject_ids(&prepared.proposal);
                        let derived_semantic_summary =
                            complexity_session_journal_summary(&prepared.proposal)?;
                        if prepared.schema != "ghostlight.complexity_prepared_mutation.v1"
                            || prepared.round != round
                            || prepared.dispatch != invocation.dispatch
                            || prepared.parent_gestalt_id != invocation.proposal.parent_gestalt_id()
                            || prepared.mutation_kind != invocation.proposal.mutation_kind()
                            || !prepared
                                .model_receipt_hashes
                                .starts_with(&invocation.model_receipt_hashes)
                            || prepared.model_receipt_hashes.len()
                                != invocation.model_receipt_hashes.len().saturating_add(1)
                            || prepared.affected_subject_ids != derived_affected_subject_ids
                            || prepared.semantic_summary != derived_semantic_summary
                        {
                            anyhow::bail!("complexity prepared-mutation checkpoint is inconsistent")
                        }
                        prepared
                    } else {
                        let proposal = rebase_world_complexity_proposal(
                            &invocation.parent_binding,
                            &campaign,
                            invocation.proposal.clone(),
                        )?;
                        let semantic_summary = complexity_session_journal_summary(&proposal)?;
                        let parent_gestalt_id = proposal.parent_gestalt_id().to_owned();
                        let mutation_kind = proposal.mutation_kind().to_owned();
                        let source_model_receipts =
                            load_checkpoint_receipts(&store, &invocation.model_receipt_hashes)?;
                        let source_receipt_ids = source_model_receipts
                            .iter()
                            .map(|receipt| receipt.storage_key().to_owned())
                            .collect::<Vec<_>>();
                        let prior_world_sessions =
                            session_checkpoints.values().cloned().collect::<Vec<_>>();
                        let semantic_context = serde_json::json!({
                            "assigned_title":invocation.dispatch.title,
                            "all_prior_round_title_jurisdiction_sessions":&prior_world_sessions,
                            "accepted_earlier_this_round":&accepted_round_semantic_deltas,
                        });
                        let (proposal, semantic_verdict, semantic_receipt) =
                            qualify_world_complexity_proposal_semantics(
                                model.as_ref(),
                                &campaign,
                                &semantic_context,
                                proposal,
                                source_receipt_ids,
                            )
                            .await?;
                        store.persist_model_stage_receipts(std::slice::from_ref(
                            &semantic_receipt,
                        ))?;
                        if !semantic_verdict.accepted() {
                            let rejection = ComplexitySemanticRejectionCheckpoint {
                                schema: "ghostlight.complexity_semantic_rejection.v2".into(),
                                round,
                                dispatch: invocation.dispatch.clone(),
                                parent_gestalt_id,
                                proposal,
                                verifier_receipt_hash: semantic_receipt.storage_key().into(),
                                diagnostic: complexity_semantic_rejection_diagnostic(
                                    &semantic_verdict,
                                )?,
                            };
                            publish_immutable_checkpoint(&semantic_rejection_path, &rejection)?;
                            let session_id = invocation_sessions
                                .get(&invocation.dispatch.ordinal)
                                .ok_or_else(|| {
                                    anyhow::anyhow!("complexity rejection has no session route")
                                })?;
                            recent_rejection_findings
                                .entry(session_id.clone())
                                .or_default()
                                .push(bounded_prompt_excerpt(&rejection.diagnostic, 800));
                            superseded_paths.push(semantic_rejection_path);
                            continue;
                        }
                        let affected_subject_ids = complexity_affected_subject_ids(&proposal);
                        let mut model_receipts = source_model_receipts;
                        model_receipts.push(semantic_receipt);
                        let committed_model_receipt_hashes = model_receipts
                            .iter()
                            .map(|receipt| receipt.storage_key().to_owned())
                            .collect::<Vec<_>>();
                        let prepared = ComplexityPreparedMutationCheckpoint {
                            schema: "ghostlight.complexity_prepared_mutation.v1".into(),
                            round,
                            dispatch: invocation.dispatch.clone(),
                            expected_revision: campaign.revision,
                            proposal,
                            parent_gestalt_id,
                            mutation_kind,
                            affected_subject_ids,
                            model_receipt_hashes: committed_model_receipt_hashes,
                            semantic_summary,
                        };
                        publish_immutable_checkpoint(&prepared_path, &prepared)?;
                        prepared
                    };
                    let model_receipts =
                        load_checkpoint_receipts(&store, &prepared.model_receipt_hashes)?;
                    let semantic = match &prepared.proposal {
                        WorldComplexityProposal::Fission { qualification, .. } => {
                            &qualification.semantic
                        }
                        WorldComplexityProposal::Individuate { qualification, .. } => {
                            &qualification.semantic
                        }
                    };
                    ghostlight_dungeon::elaboration::validate_world_complexity_semantic_receipt_provenance(
                        &prepared.proposal,
                        semantic,
                        &model_receipts,
                    )?;
                    if semantic.frozen_campaign_id != campaign.id
                        || semantic.frozen_world_revision != prepared.expected_revision
                        || prepared.proposal.expected_world_revision() != prepared.expected_revision
                    {
                        anyhow::bail!(
                            "prepared complexity qualification is bound to another campaign revision"
                        )
                    }
                    let (advanced, receipt) = if campaign.revision == prepared.expected_revision {
                        let committed = match prepared.proposal.clone() {
                            WorldComplexityProposal::Fission {
                                preview,
                                qualification,
                            } => {
                                kernel
                                    .command(WorldCommand::ElaborateGestaltFission {
                                        expected_revision: campaign.revision,
                                        preview,
                                        qualification,
                                        model_stage_receipts: model_receipts,
                                    })
                                    .await?
                            }
                            WorldComplexityProposal::Individuate {
                                individuation,
                                qualification,
                            } => {
                                kernel
                                    .command(WorldCommand::ElaborateGestaltIndividuation {
                                        expected_revision: campaign.revision,
                                        individuation,
                                        qualification,
                                        model_stage_receipts: model_receipts,
                                    })
                                    .await?
                            }
                        };
                        let CommandResult::Committed {
                            campaign: advanced,
                            receipt,
                        } = committed
                        else {
                            anyhow::bail!("complexity mutation did not commit")
                        };
                        (advanced, receipt)
                    } else if campaign.revision == prepared.expected_revision.saturating_add(1)
                        && complexity_proposal_is_committed(&campaign, &prepared.proposal)
                    {
                        let receipt = store
                            .load::<ghostlight_dungeon::domain::WorldCommitReceipt>(
                                "world_commit_receipt.v1",
                                &format!("{}-{}", campaign.id, campaign.revision),
                            )?
                            .map(|(_, receipt)| receipt)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "prepared complexity mutation lacks its canonical world receipt"
                                )
                            })?;
                        if receipt.schema != "ghostlight.world_commit_receipt.v1"
                            || receipt.campaign_id != campaign.id
                            || receipt.previous_revision != prepared.expected_revision
                            || receipt.revision != campaign.revision
                            || receipt.command_kind != prepared.mutation_kind
                        {
                            anyhow::bail!(
                                "prepared complexity mutation found a different canonical world receipt"
                            )
                        }
                        match &prepared.proposal {
                            WorldComplexityProposal::Fission { preview, .. } => {
                                committed_compiler_mutation_proof(
                                    &store,
                                    &receipt,
                                    ghostlight_dungeon::legacy_transition::digest_serializable(
                                        preview,
                                    )?,
                                    "elaborate_gestalt_fission",
                                    "fission-preview",
                                    Some(
                                        campaign
                                            .resolution_policy
                                            .resolution_epoch
                                            .saturating_sub(1),
                                    ),
                                )?;
                            }
                            WorldComplexityProposal::Individuate { individuation, .. } => {
                                let presence = store
                                    .load::<ghostlight_dungeon::domain::GestaltMaterializationReceipt>(
                                        "gestalt_materialization_receipt.v1",
                                        &format!("{}-{}", campaign.id, campaign.revision),
                                    )?
                                    .map(|(_, receipt)| receipt)
                                    .ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "prepared individuation lacks its canonical presence receipt"
                                        )
                                    })?;
                                let member_id =
                                    ghostlight_dungeon::domain::canonical_gestalt_member_local_id(
                                        &individuation.member.id,
                                    );
                                let actor_id =
                                    ghostlight_dungeon::domain::gestalt_member_subject_id(
                                        &member_id,
                                    );
                                if presence.schema
                                    != "ghostlight.gestalt_materialization_receipt.v1"
                                    || presence.campaign_id != campaign.id
                                    || presence.previous_revision != prepared.expected_revision
                                    || presence.revision != campaign.revision
                                    || presence.reason
                                        != "model-qualified world-complexity individuation"
                                    || presence.changes.len() != 1
                                    || presence.changes[0].operation != "materialized"
                                    || presence.changes[0].gestalt_id != individuation.gestalt_id
                                    || presence.changes[0].member_id != member_id
                                    || presence.changes[0].actor_id != actor_id
                                    || presence.changes[0].gestalt_version
                                        != individuation.expected_gestalt_version
                                    || presence.changes[0].member_version != 1
                                {
                                    anyhow::bail!(
                                        "prepared individuation found a different canonical presence change"
                                    )
                                }
                            }
                        }
                        (campaign.clone(), receipt)
                    } else {
                        anyhow::bail!(
                            "prepared complexity mutation cannot recover from world revision {}",
                            campaign.revision
                        )
                    };
                    let checkpoint = ComplexityMutationCheckpoint {
                        schema: "ghostlight.complexity_mutation_checkpoint.v1".into(),
                        round,
                        dispatch: invocation.dispatch.clone(),
                        parent_gestalt_id: prepared.parent_gestalt_id,
                        mutation_kind: prepared.mutation_kind,
                        affected_subject_ids: prepared.affected_subject_ids,
                        model_receipt_hashes: prepared.model_receipt_hashes,
                        semantic_summary: prepared.semantic_summary,
                        commit_receipt: receipt,
                    };
                    publish_immutable_checkpoint(&commit_path, &checkpoint)?;
                    campaign = advanced;
                    checkpoint
                };
                let prepared_authority =
                    read_checkpoint::<ComplexityPreparedMutationCheckpoint>(&prepared_path)?;
                validate_complexity_mutation_checkpoint(
                    &store,
                    &campaign,
                    round,
                    invocation,
                    &prepared_authority,
                    &checkpoint,
                    ComplexityCommitValidationMode::CurrentCampaignEffect,
                )?;
                let session_id = invocation_sessions
                    .get(&checkpoint.dispatch.ordinal)
                    .ok_or_else(|| anyhow::anyhow!("complexity invocation has no session route"))?;
                journals.entry(session_id.clone()).or_default().push(
                    ElaboratorSessionJournalEntry {
                        world_revision: checkpoint.commit_receipt.revision,
                        commit_receipt_id: format!(
                            "{}-{}",
                            checkpoint.commit_receipt.campaign_id,
                            checkpoint.commit_receipt.revision
                        ),
                        mutation_kind: checkpoint.mutation_kind.clone(),
                        affected_subject_ids: checkpoint.affected_subject_ids.clone(),
                        summary: checkpoint.semantic_summary.clone(),
                    },
                );
                accepted_round_semantic_deltas.push(checkpoint.semantic_summary.clone());
                mutation_paths.push(commit_path);
            }
            let compaction_limit = Arc::new(tokio::sync::Semaphore::new(complexity_parallelism));
            let mut compactions = tokio::task::JoinSet::new();
            let sessions_to_compact = journals
                .keys()
                .chain(recent_rejection_findings.keys())
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            for session_id in sessions_to_compact {
                let journal = journals.remove(&session_id).unwrap_or_default();
                let rejection_findings = recent_rejection_findings
                    .remove(&session_id)
                    .unwrap_or_default();
                let (title, location_id) = session_routes
                    .get(&session_id)
                    .ok_or_else(|| anyhow::anyhow!("complexity journal has no session route"))?;
                let session_digest = strategic_smoke_digest(&session_id)?;
                let session_suffix = session_digest
                    .strip_prefix("sha256:")
                    .unwrap_or(&session_digest)
                    .chars()
                    .take(12)
                    .collect::<String>();
                let path = root.join(format!(
                    "complexity-round-{round:03}-session-{}-{session_suffix}.json",
                    title.display_name().to_ascii_lowercase(),
                ));
                let previous = session_checkpoints.get(&session_id).cloned();
                if path.is_file() {
                    let checkpoint = read_checkpoint::<ElaboratorSessionCheckpoint>(&path)?;
                    checkpoint.validate_for(&campaign, location_id, *title)?;
                    let expected_commit_receipt_ids = journal
                        .iter()
                        .map(|entry| entry.commit_receipt_id.clone())
                        .collect::<Vec<_>>();
                    if checkpoint.through_world_revision != campaign.revision
                        || checkpoint.recent_commit_receipt_ids != expected_commit_receipt_ids
                        || checkpoint.recent_rejection_findings != rejection_findings
                        || checkpoint.prior_checkpoint_digest
                            != previous
                                .as_ref()
                                .map(|checkpoint| checkpoint.digest.clone())
                    {
                        anyhow::bail!(
                            "complexity session checkpoint is not bound to its exact admitted journal and rejection findings"
                        )
                    }
                    session_checkpoints.insert(session_id, checkpoint);
                    continue;
                }
                let model = model.clone();
                let campaign = Arc::new(campaign.clone());
                let title = *title;
                let location_id = location_id.clone();
                let compaction_limit = compaction_limit.clone();
                compactions.spawn(async move {
                    let _permit = compaction_limit
                        .acquire_owned()
                        .await
                        .map_err(|_| anyhow::anyhow!("complexity compaction limiter closed"))?;
                    let result = compact_elaborator_session(
                        model.as_ref(),
                        campaign.as_ref(),
                        &location_id,
                        title,
                        &session_id,
                        previous.as_ref(),
                        &journal,
                        rejection_findings,
                    )
                    .await;
                    Ok::<_, anyhow::Error>((session_id, title, location_id, path, campaign, result))
                });
            }
            let mut first_compaction_error = None;
            while let Some(joined) = compactions.join_next().await {
                match joined {
                    Err(error) => {
                        first_compaction_error.get_or_insert_with(|| anyhow::Error::new(error));
                    }
                    Ok(Err(error)) => {
                        first_compaction_error.get_or_insert(error);
                    }
                    Ok(Ok((session_id, title, location_id, path, campaign, result))) => {
                        match result {
                            Err(error) => {
                                first_compaction_error.get_or_insert(error);
                            }
                            Ok((checkpoint, receipts)) => {
                                store.persist_model_stage_receipts(&receipts)?;
                                publish_immutable_checkpoint(&path, &checkpoint)?;
                                checkpoint.validate_for(campaign.as_ref(), &location_id, title)?;
                                session_checkpoints.insert(session_id, checkpoint);
                            }
                        }
                    }
                }
            }
            if let Some(error) = first_compaction_error {
                return Err(error);
            }
            let actionable_after = canonical_actionable_subject_count(&campaign);
            if actionable_after <= actionable_before {
                anyhow::bail!(
                    "world complexity round {round} committed without increasing admitted complexity"
                )
            }
            let checkpoint = ComplexityRoundCheckpoint {
                schema: "ghostlight.complexity_round_checkpoint.v1".into(),
                round,
                demand_before: demand,
                actionable_subjects_after: actionable_after,
                schedule: preview.schedule,
                mutation_checkpoints: mutation_paths,
                superseded_invocation_checkpoints: superseded_paths,
                session_checkpoints: session_checkpoints.clone(),
            };
            let path = root.join(format!("complexity-round-{round:03}-checkpoint.json"));
            publish_immutable_checkpoint(&path, &checkpoint)?;
            last_completed_complexity_count = Some(actionable_after);
            complexity_reports.push(serde_json::to_value(&checkpoint)?);
        }
        let final_count = last_completed_complexity_count
            .unwrap_or_else(|| canonical_actionable_subject_count(&campaign));
        let final_demand = derive_world_elaboration_demand(
            u16::from(campaign.resolution_policy.active_cell_budget),
            final_count,
            &scale_intent,
            realm_weights,
        )?;
        validate_exact_world_scale_count(
            final_count,
            final_demand.target_actionable_subjects,
            maximum_complexity_rounds,
        )?;
        validate_qualified_horizontal_spread(&campaign, &initial_location_ids)?;
        if let Some(metadata) = world_compile
            .as_mut()
            .and_then(serde_json::Value::as_object_mut)
        {
            metadata.insert(
                "complexity_rounds".into(),
                serde_json::Value::Array(complexity_reports.clone()),
            );
            metadata.insert(
                "actionable_subject_count".into(),
                serde_json::json!(final_count),
            );
        }
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
        ) = match issue_composition {
            Ok(composition) => {
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
                )
            }
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
                    Ok(composition) => composition,
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
        Ok(composition) => composition,
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
) -> anyhow::Result<ghostlight_dungeon::newspaper::WorldNewspaperComposition> {
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

fn strategic_region_request(request: &str) -> String {
    bounded_strategic_request(
        "Create one jurisdiction: exactly 4 resident populations, 6 governing institutions, 3+ connected internal places, homes in 3+ places, and one mixed/cross-border population. Give opposed interests executable means. Preserve every Region constraint. Region: ".into(),
        request,
    )
}

fn expansion_location_is_within(
    expansion: &ghostlight_dungeon::domain::RegionExpansion,
    location_id: &str,
    jurisdiction_id: &str,
) -> bool {
    let locations = expansion
        .locations
        .iter()
        .map(|location| (location.id.as_str(), location))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut current = Some(location_id);
    for _ in 0..=locations.len() {
        let Some(id) = current else {
            return false;
        };
        if id == jurisdiction_id {
            return true;
        }
        current = locations
            .get(id)
            .and_then(|location| location.container_id.as_deref());
    }
    false
}

fn validate_strategic_region_expansion_shape(
    expansion: &ghostlight_dungeon::domain::RegionExpansion,
) -> anyhow::Result<()> {
    let civic = expansion
        .civic_system
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("strategic region has no civic apparatus"))?;
    if civic.resident_population_ids.len() != 4
        || civic.governing_institution_ids.len() != 6
        || expansion.populations.len() != 4
        || expansion.institutions.len() != 6
    {
        anyhow::bail!(
            "strategic region requires exactly four resident populations and six governing institutions before foundation; candidate supplied {}/{} civic IDs and {}/{} records",
            civic.resident_population_ids.len(),
            civic.governing_institution_ids.len(),
            expansion.populations.len(),
            expansion.institutions.len()
        )
    }
    let admitted_locations = expansion
        .locations
        .iter()
        .filter(|location| {
            expansion_location_is_within(expansion, &location.id, &civic.jurisdiction_location_id)
        })
        .map(|location| location.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let resident_homes = expansion
        .populations
        .iter()
        .filter(|population| civic.resident_population_ids.contains(&population.id))
        .map(|population| population.home_location_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if admitted_locations.len() < 3
        || resident_homes.len() < 3
        || !resident_homes.is_subset(&admitted_locations)
    {
        anyhow::bail!(
            "strategic region needs at least three admitted locations and resident homes inside its jurisdiction; candidate supplied {} locations and {} resident homes",
            admitted_locations.len(),
            resident_homes.len()
        )
    }
    Ok(())
}

fn validate_qualified_horizontal_spread(
    campaign: &ghostlight_dungeon::domain::Campaign,
    jurisdiction_ids: &[String],
) -> anyhow::Result<()> {
    let actionable = ghostlight_dungeon::elaboration::canonical_actionable_subject_ids(campaign);
    for jurisdiction_id in jurisdiction_ids {
        let admitted =
            ghostlight_dungeon::elaboration::locations_in_jurisdiction(campaign, jurisdiction_id)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for subject_id in &actionable {
            let Some(profile) = campaign.agency_profiles.get(subject_id) else {
                continue;
            };
            let homes = profile
                .location_ids
                .intersection(&admitted)
                .cloned()
                .collect::<Vec<_>>();
            if homes.len() == 1 {
                *counts.entry(homes[0].clone()).or_default() += 1;
            }
        }
        let total = counts.values().sum::<usize>();
        let largest = counts.values().copied().max().unwrap_or_default();
        if counts.len() < 3 || total == 0 || largest.saturating_mul(5) > total.saturating_mul(4) {
            anyhow::bail!(
                "qualified complexity in jurisdiction {jurisdiction_id} is not horizontally distributed: {total} subjects across {} occupied places, largest place {largest}",
                counts.len()
            )
        }
    }
    Ok(())
}

fn validate_exact_world_scale_count(
    qualified_subject_count: u32,
    target_qualified_subject_count: u32,
    maximum_complexity_rounds: usize,
) -> anyhow::Result<()> {
    if qualified_subject_count != target_qualified_subject_count {
        anyhow::bail!(
            "world complexity stopped at {qualified_subject_count} of exact target {target_qualified_subject_count} admitted subjects after {maximum_complexity_rounds} rounds"
        )
    }
    Ok(())
}

fn strategic_locality_request(location_name: &str, location_id: &str, pressure: &str) -> String {
    let location_name = bounded_prompt_excerpt(location_name, 48);
    let prefix = format!(
        "Elaborate {location_name:?} [{location_id}]. Exactly 4 resident populations and 6 institutions. Include one mixed/diasporic/cross-border body; ancestry is not a civic partition. Give opposed interests executable means. State authority, succession, revenue, redress, and a public notice channel. Pressure: "
    );
    bounded_strategic_request(prefix, pressure)
}

fn validate_strategic_foundation_civic_shape(
    civic: &ghostlight_dungeon::domain::CivicSystemManifest,
) -> anyhow::Result<()> {
    if civic.resident_population_ids.len() != 4 || civic.governing_institution_ids.len() != 6 {
        anyhow::bail!(
            "strategic locality foundation requires exactly four resident populations and six governing institutions; candidate supplied {} and {}",
            civic.resident_population_ids.len(),
            civic.governing_institution_ids.len()
        )
    }
    Ok(())
}

fn strategic_titled_locality_request(
    location_name: &str,
    location_id: &str,
    pressure: &str,
) -> String {
    let location_name = bounded_prompt_excerpt(location_name, 48);
    let prefix = format!(
        "Deepen canonical locality {location_name:?} [{location_id}] after civic admission. Add intact ordinary life, material pressure, political leverage, secrets, instability, and numinous meaning. Prefer executable means, cross-border ties, and non-civic ecology over another filing procedure. Pressure: "
    );
    bounded_strategic_request(prefix, pressure)
}

fn strategic_titled_repair_request(
    location_name: &str,
    location_id: &str,
    pressure: &str,
    diagnostic: &str,
) -> String {
    const COMPILER_REQUEST_LIMIT: usize = 500;
    const PRESSURE_RESERVE: usize = 48;
    let location_name = bounded_prompt_excerpt(location_name, 48);
    let prefix = format!(
        "Redo {location_name:?} [{location_id}] on the same frozen world: fresh complete deepening, civic authority preserved, with material/ecological/political means. Verifier finding: "
    );
    let pressure_label = " Pressure: ";
    let diagnostic_budget = COMPILER_REQUEST_LIMIT
        .saturating_sub(prefix.chars().count())
        .saturating_sub(pressure_label.chars().count())
        .saturating_sub(PRESSURE_RESERVE);
    let diagnostic = bounded_prompt_excerpt(diagnostic, diagnostic_budget);
    bounded_strategic_request(format!("{prefix}{diagnostic}{pressure_label}"), pressure)
}

fn bounded_strategic_request(prefix: String, pressure: &str) -> String {
    const COMPILER_REQUEST_LIMIT: usize = 500;
    let prefix_chars = prefix.chars().count();
    assert!(
        prefix_chars < COMPILER_REQUEST_LIMIT,
        "strategic request identity exceeds compiler request budget"
    );
    let pressure = bounded_prompt_excerpt(pressure, COMPILER_REQUEST_LIMIT - prefix_chars);
    let request = format!("{prefix}{pressure}");
    debug_assert!(request.chars().count() <= COMPILER_REQUEST_LIMIT);
    request
}

fn bounded_prompt_excerpt(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if max_chars == 0 {
        return String::new();
    }
    if trimmed.chars().count() <= max_chars {
        return trimmed.into();
    }
    let mut excerpt = String::new();
    for word in trimmed.split_whitespace() {
        let separator = usize::from(!excerpt.is_empty());
        if excerpt.chars().count() + separator + word.chars().count() + 1 > max_chars {
            break;
        }
        if !excerpt.is_empty() {
            excerpt.push(' ');
        }
        excerpt.push_str(word);
    }
    if excerpt.is_empty() {
        excerpt.extend(trimmed.chars().take(max_chars.saturating_sub(1)));
    }
    excerpt.push('…');
    excerpt
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
                weight: match title {
                    ElaboratorTitle::Patina => 9,
                    ElaboratorTitle::Tangle => 4,
                    _ => 2,
                },
            })
            .collect(),
    }
}

fn complexity_parent_candidates(
    campaign: &ghostlight_dungeon::domain::Campaign,
    demand: &ghostlight_dungeon::elaboration::WorldElaborationDemand,
    limit: usize,
) -> Vec<String> {
    let qualified_ids = ghostlight_dungeon::elaboration::canonical_actionable_subject_ids(campaign);
    let mut current_by_realm = demand
        .realm_subject_targets
        .keys()
        .cloned()
        .map(|realm| (realm, 0_u32))
        .collect::<BTreeMap<_, _>>();
    for profile in campaign
        .agency_profiles
        .values()
        .filter(|profile| qualified_ids.contains(&profile.subject_id))
    {
        if let Some(realm) = complexity_realm_for_profile(campaign, profile, demand) {
            *current_by_realm.entry(realm).or_default() += 1;
        }
    }
    let realm_pressure = demand
        .realm_subject_targets
        .iter()
        .map(|(realm, target)| {
            (
                realm.clone(),
                target.saturating_sub(current_by_realm.get(realm).copied().unwrap_or(0)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut by_realm = BTreeMap::<String, Vec<(std::cmp::Reverse<u64>, String)>>::new();
    for (id, profile) in campaign.gestalts.keys().filter_map(|id| {
        let profile = campaign.agency_profiles.get(id)?;
        qualified_ids.contains(id).then_some((id, profile))
    }) {
        if let Some(realm) = complexity_realm_for_profile(campaign, profile, demand)
            && realm_pressure.get(&realm).copied().unwrap_or(0) > 0
        {
            by_realm
                .entry(realm)
                .or_default()
                .push((std::cmp::Reverse(profile.detail_debt), id.clone()));
        }
    }
    for candidates in by_realm.values_mut() {
        candidates.sort();
        candidates.reverse();
    }
    let mut selected_per_realm = BTreeMap::<String, u32>::new();
    let mut selected = Vec::new();
    while selected.len() < limit {
        let next_realm = by_realm
            .iter()
            .filter(|(_, candidates)| !candidates.is_empty())
            .min_by(|(left, _), (right, _)| {
                let left_selected = selected_per_realm.get(*left).copied().unwrap_or(0);
                let right_selected = selected_per_realm.get(*right).copied().unwrap_or(0);
                let left_pressure = realm_pressure.get(*left).copied().unwrap_or(1).max(1);
                let right_pressure = realm_pressure.get(*right).copied().unwrap_or(1).max(1);
                left_selected
                    .saturating_mul(right_pressure)
                    .cmp(&right_selected.saturating_mul(left_pressure))
                    .then_with(|| left.cmp(right))
            })
            .map(|(realm, _)| realm.clone());
        let Some(realm) = next_realm else {
            break;
        };
        let (_, id) = by_realm
            .get_mut(&realm)
            .and_then(Vec::pop)
            .expect("selected realm has a candidate");
        *selected_per_realm.entry(realm).or_default() += 1;
        selected.push(id);
    }
    selected
}

fn complexity_realm_for_profile(
    campaign: &ghostlight_dungeon::domain::Campaign,
    profile: &ghostlight_dungeon::domain::AgencyProfile,
    demand: &ghostlight_dungeon::elaboration::WorldElaborationDemand,
) -> Option<String> {
    ghostlight_dungeon::elaboration::unique_containing_jurisdiction(
        campaign,
        &profile.location_ids,
        &demand.realm_subject_targets.keys().cloned().collect(),
    )
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
        ComplexityCommitValidationMode, ComplexityMutationCheckpoint,
        ComplexityPreparedMutationCheckpoint, ComplexityPreviewInvocation,
        ComplexityRoundCheckpoint, HistoricalWorldNewspaperArticleV2,
        HistoricalWorldNewspaperEditorialVerdict, HistoricalWorldNewspaperGroundingVerdict,
        HistoricalWorldNewspaperIssueV2, admitted_public_channel, bounded_prompt_excerpt,
        civic_manifest_is_committed_candidate, civic_manifest_preserves,
        committed_elaboration_mutation_proof, completed_wave_issue_campaign,
        complexity_affected_subject_ids, complexity_realm_for_profile,
        complexity_semantic_rejection_diagnostic, complexity_session_journal_summary,
        final_wave_field, fission_population_binding_is_present,
        fission_relation_binding_is_present, latest_partial_wave_checkpoint,
        load_and_validate_complexity_mutation_checkpoint, missing_newspaper_report_indices,
        publish_immutable_checkpoint, recomposed_model_receipt_set_digest,
        recover_committed_clock_binding, retain_unique_complexity_invocations, strategic_campaign,
        strategic_locality_request, strategic_region_request, strategic_smoke_bytes_digest,
        strategic_smoke_digest, strategic_titled_locality_request, strategic_titled_repair_request,
        titled_failure_checkpoint_paths, validate_completed_complexity_round_checkpoint,
        validate_completed_newspaper_recomposition_receipt,
        validate_complexity_round_session_checkpoints, validate_exact_world_scale_count,
        validate_strategic_foundation_civic_shape,
    };

    #[test]
    fn complexity_realm_is_derived_from_canonical_location_containment() {
        use ghostlight_dungeon::domain::{Location, Route};
        use ghostlight_dungeon::elaboration::{WorldScaleIntent, derive_world_elaboration_demand};
        use std::collections::{BTreeMap, BTreeSet};

        let mut campaign = strategic_campaign();
        campaign.locations.insert(
            "realm".into(),
            Location {
                id: "realm".into(),
                name: "Realm".into(),
                container_id: None,
                routes: BTreeMap::<String, Route>::new(),
                persistent_features: Vec::new(),
            },
        );
        campaign.locations.insert(
            "town".into(),
            Location {
                id: "town".into(),
                name: "Town".into(),
                container_id: Some("realm".into()),
                routes: BTreeMap::new(),
                persistent_features: Vec::new(),
            },
        );
        campaign.locations.insert(
            "ward".into(),
            Location {
                id: "ward".into(),
                name: "Ward".into(),
                container_id: Some("town".into()),
                routes: BTreeMap::new(),
                persistent_features: Vec::new(),
            },
        );
        let profile_id = campaign.agency_profiles.keys().next().unwrap().clone();
        let profile = {
            let profile = campaign.agency_profiles.get_mut(&profile_id).unwrap();
            profile.location_ids = BTreeSet::from(["ward".into()]);
            profile.clone()
        };
        let demand = derive_world_elaboration_demand(
            240,
            1,
            &WorldScaleIntent::ten_percent(),
            BTreeMap::from([("realm".into(), 1)]),
        )
        .unwrap();

        assert_eq!(
            complexity_realm_for_profile(&campaign, &profile, &demand).as_deref(),
            Some("realm")
        );
        assert_eq!(profile.location_ids, BTreeSet::from(["ward".into()]));
    }

    #[test]
    fn completed_complexity_round_rejects_missing_referenced_mutation_despite_sufficient_count() {
        use ghostlight_dungeon::elaboration::{
            ElaborationDispatchState, ElaborationScheduleReceipt, WorldElaborationDemand,
            canonical_actionable_subject_count,
        };
        use std::collections::{BTreeMap, BTreeSet};

        let directory = tempfile::tempdir().unwrap();
        let store = ghostlight_dungeon::persistence::CampaignStore::open(
            directory.path().join("campaign.cc"),
        )
        .unwrap();
        let campaign = strategic_campaign();
        let missing = directory
            .path()
            .join("complexity-round-001-commit-0001.json");
        let checkpoint = ComplexityRoundCheckpoint {
            schema: "ghostlight.complexity_round_checkpoint.v1".into(),
            round: 1,
            demand_before: WorldElaborationDemand {
                schema: "ghostlight.world_elaboration_demand.v1".into(),
                active_cell_budget: 240,
                target_active_cover_basis_points: 2_000,
                target_actionable_subjects: 48,
                current_actionable_subjects: 1,
                actionable_subject_deficit: 47,
                round_mutation_budget: 1,
                realm_complexity_weights: BTreeMap::new(),
                realm_subject_targets: BTreeMap::new(),
            },
            actionable_subjects_after: canonical_actionable_subject_count(&campaign),
            schedule: ElaborationScheduleReceipt {
                schema: "ghostlight.elaboration_schedule_receipt.v1".into(),
                requested_invocations: 0,
                unused_invocations: 0,
                eligible_titles: BTreeSet::new(),
                dispatch_counts: BTreeMap::new(),
                unused_counts: BTreeMap::new(),
                dispatches: Vec::new(),
                final_state: ElaborationDispatchState::default(),
            },
            mutation_checkpoints: vec![missing],
            superseded_invocation_checkpoints: Vec::new(),
            session_checkpoints: BTreeMap::new(),
        };

        let error = validate_completed_complexity_round_checkpoint(
            directory.path(),
            &store,
            &campaign,
            &checkpoint,
            1,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("references a missing mutation checkpoint")
        );
    }

    #[test]
    fn completed_complexity_round_cannot_drop_a_touched_session_checkpoint() {
        use ghostlight_dungeon::elaboration::{
            ElaboratorSessionCheckpoint, ElaboratorSessionCompactionDraft,
            ElaboratorSessionJournalEntry, ElaboratorTitle, elaborator_session_id,
        };
        use std::collections::BTreeMap;

        let campaign = strategic_campaign();
        let title = ElaboratorTitle::Charter;
        let location_id = "yard".to_owned();
        let session_id = elaborator_session_id(title, &location_id);
        let journal = ElaboratorSessionJournalEntry {
            world_revision: campaign.revision,
            commit_receipt_id: format!("{}-{}", campaign.id, campaign.revision),
            mutation_kind: "fission".into(),
            affected_subject_ids: vec!["workers".into()],
            summary: "Applied one distinct charter split.".into(),
        };
        let checkpoint = ElaboratorSessionCheckpoint::bind_compaction(
            &session_id,
            title,
            1,
            campaign.id,
            campaign.revision,
            &location_id,
            ElaboratorSessionCompactionDraft {
                schema: "ghostlight.elaborator_session_compaction_draft.v1".into(),
                frontier_summary: "One charter distinction remains live.".into(),
                unresolved_leads: Vec::new(),
            },
            vec![journal.commit_receipt_id.clone()],
            Vec::new(),
            None,
        )
        .unwrap();
        let routes = BTreeMap::from([(session_id.clone(), (title, location_id))]);
        let journals = BTreeMap::from([(session_id.clone(), vec![journal])]);
        let mut current = BTreeMap::from([(session_id.clone(), checkpoint)]);
        validate_complexity_round_session_checkpoints(
            &campaign,
            &BTreeMap::new(),
            &current,
            &routes,
            &journals,
            &BTreeMap::new(),
            campaign.revision,
        )
        .unwrap();

        current.remove(&session_id);
        let diagnostic = validate_complexity_round_session_checkpoints(
            &campaign,
            &BTreeMap::new(),
            &current,
            &routes,
            &journals,
            &BTreeMap::new(),
            campaign.revision,
        )
        .unwrap_err()
        .to_string();
        assert!(diagnostic.contains("session checkpoint set"));
    }

    #[test]
    fn existing_complexity_commit_checkpoint_rejects_campaign_without_committed_effect() {
        use ghostlight_dungeon::domain::{GestaltIndividuation, GestaltMemberDelta};
        use ghostlight_dungeon::elaboration::{
            ElaborationDispatch, ElaboratorTitle, WorldComplexityIndividuationQualification,
            WorldComplexityProposal, WorldComplexitySemanticQualification,
        };
        use std::collections::{BTreeMap, BTreeSet};

        let directory = tempfile::tempdir().unwrap();
        let store = ghostlight_dungeon::persistence::CampaignStore::open(
            directory.path().join("campaign.cc"),
        )
        .unwrap();
        let mut campaign = strategic_campaign();
        let expected_revision = campaign.revision;
        let mut semantic = WorldComplexitySemanticQualification::default();
        semantic.frozen_campaign_id = campaign.id;
        semantic.frozen_world_revision = expected_revision;
        let proposal = WorldComplexityProposal::Individuate {
            individuation: GestaltIndividuation {
                gestalt_id: "workers".into(),
                expected_gestalt_version: campaign.gestalts["workers"].version,
                location_id: "yard".into(),
                member: GestaltMemberDelta {
                    schema: "ghostlight.gestalt_member_delta.v1".into(),
                    id: "resume-ghost".into(),
                    gestalt_id: "workers".into(),
                    version: 0,
                    name: "Mara Quill".into(),
                    capability_additions: BTreeSet::from(["read pressure gauges".into()]),
                    capability_removals: BTreeSet::new(),
                    knowledge_additions: BTreeSet::from(["night sluice timings".into()]),
                    knowledge_removals: BTreeSet::new(),
                    equipment: BTreeSet::new(),
                    conditions: BTreeSet::new(),
                    obligations: BTreeSet::new(),
                    relationships: BTreeMap::new(),
                    goals: vec!["keep the lower channel open".into()],
                    memories: vec!["the winter gauge fracture".into()],
                    last_location_id: Some("yard".into()),
                    materialized_actor_id: None,
                    last_relevant_revision: expected_revision,
                    relevance_lease_until_revision: expected_revision,
                },
            },
            qualification: WorldComplexityIndividuationQualification {
                schema: "ghostlight.world_complexity_individuation_qualification.v1".into(),
                title: ElaboratorTitle::Veil,
                jurisdiction_location_id: "yard".into(),
                semantic,
            },
        };
        let dispatch = ElaborationDispatch {
            schema: "ghostlight.elaboration_dispatch.v1".into(),
            budget_ordinal: 1,
            ordinal: 1,
            title: ElaboratorTitle::Veil,
            title_weight: 1,
            total_enabled_weight: 1,
            requested_share_millionths: 1_000_000,
            title_dispatch_count: 1,
        };
        let invocation = ComplexityPreviewInvocation {
            dispatch: dispatch.clone(),
            parent_binding: "plausible-frozen-parent-binding".into(),
            proposal: proposal.clone(),
            model_receipt_hashes: vec!["sha256:plausible-generation".into()],
        };
        let semantic_summary = complexity_session_journal_summary(&proposal).unwrap();
        let affected_subject_ids = complexity_affected_subject_ids(&proposal);
        let prepared = ComplexityPreparedMutationCheckpoint {
            schema: "ghostlight.complexity_prepared_mutation.v1".into(),
            round: 1,
            dispatch: dispatch.clone(),
            expected_revision,
            proposal,
            parent_gestalt_id: "workers".into(),
            mutation_kind: "elaborate_gestalt_individuation".into(),
            affected_subject_ids: affected_subject_ids.clone(),
            model_receipt_hashes: vec![
                "sha256:plausible-generation".into(),
                "sha256:plausible-verifier".into(),
            ],
            semantic_summary: semantic_summary.clone(),
        };
        campaign.revision = expected_revision.saturating_add(1);
        let checkpoint = ComplexityMutationCheckpoint {
            schema: "ghostlight.complexity_mutation_checkpoint.v1".into(),
            round: 1,
            dispatch,
            parent_gestalt_id: "workers".into(),
            mutation_kind: "elaborate_gestalt_individuation".into(),
            affected_subject_ids,
            model_receipt_hashes: prepared.model_receipt_hashes.clone(),
            semantic_summary,
            commit_receipt: ghostlight_dungeon::domain::WorldCommitReceipt {
                schema: "ghostlight.world_commit_receipt.v1".into(),
                campaign_id: campaign.id,
                previous_revision: expected_revision,
                revision: campaign.revision,
                command_kind: "elaborate_gestalt_individuation".into(),
                committed_at: chrono::Utc::now(),
                roll: None,
            },
        };
        let prepared_path = directory
            .path()
            .join("complexity-round-001-prepared-0001.json");
        let commit_path = directory
            .path()
            .join("complexity-round-001-commit-0001.json");
        publish_immutable_checkpoint(&prepared_path, &prepared).unwrap();
        publish_immutable_checkpoint(&commit_path, &checkpoint).unwrap();

        let error = load_and_validate_complexity_mutation_checkpoint(
            &store,
            &campaign,
            1,
            &invocation,
            &prepared_path,
            &commit_path,
            ComplexityCommitValidationMode::CurrentCampaignEffect,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("committed proposal effect is absent")
        );
    }

    #[test]
    fn parallel_individuation_retains_one_exact_identity_and_records_supersession() {
        use ghostlight_dungeon::domain::{GestaltIndividuation, GestaltMemberDelta};
        use ghostlight_dungeon::elaboration::{
            ElaborationDispatch, ElaboratorTitle, WorldComplexityIndividuationQualification,
            WorldComplexityProposal, WorldComplexitySemanticQualification,
        };
        use std::collections::{BTreeMap, BTreeSet};

        let invocation = |ordinal: u64, parent: &str, id: &str| ComplexityPreviewInvocation {
            dispatch: ElaborationDispatch {
                schema: "ghostlight.elaboration_dispatch.v1".into(),
                budget_ordinal: ordinal,
                ordinal,
                title: ElaboratorTitle::Veil,
                title_weight: 1,
                total_enabled_weight: 1,
                requested_share_millionths: 1_000_000,
                title_dispatch_count: ordinal,
            },
            parent_binding: format!("binding:{parent}"),
            proposal: WorldComplexityProposal::Individuate {
                individuation: GestaltIndividuation {
                    gestalt_id: parent.into(),
                    expected_gestalt_version: 0,
                    location_id: "yard".into(),
                    member: GestaltMemberDelta {
                        schema: "ghostlight.gestalt_member_delta.v1".into(),
                        id: id.into(),
                        gestalt_id: parent.into(),
                        version: 0,
                        name: "Tarin Vel".into(),
                        capability_additions: BTreeSet::new(),
                        capability_removals: BTreeSet::new(),
                        knowledge_additions: BTreeSet::new(),
                        knowledge_removals: BTreeSet::new(),
                        equipment: BTreeSet::new(),
                        conditions: BTreeSet::new(),
                        obligations: BTreeSet::new(),
                        relationships: BTreeMap::new(),
                        goals: Vec::new(),
                        memories: Vec::new(),
                        last_location_id: Some("yard".into()),
                        materialized_actor_id: None,
                        last_relevant_revision: 0,
                        relevance_lease_until_revision: 0,
                    },
                },
                qualification: WorldComplexityIndividuationQualification {
                    schema: "ghostlight.world_complexity_individuation_qualification.v1".into(),
                    title: ElaboratorTitle::Veil,
                    jurisdiction_location_id: "yard".into(),
                    semantic: WorldComplexitySemanticQualification::default(),
                },
            },
            model_receipt_hashes: vec![format!("receipt:{ordinal}")],
        };
        let invocations = vec![
            invocation(12, "yard-carriers", "tarin-vel"),
            invocation(9, "route-carriers", "tarin-vel"),
            invocation(14, "yard-carriers", "oren-pell"),
        ];
        let (retained, superseded) = retain_unique_complexity_invocations(19, &invocations);

        assert_eq!(
            retained
                .iter()
                .map(|invocation| invocation.dispatch.ordinal)
                .collect::<Vec<_>>(),
            [9]
        );
        assert_eq!(
            superseded
                .iter()
                .map(|checkpoint| checkpoint.dispatch.ordinal)
                .collect::<Vec<_>>(),
            [12, 14]
        );
        assert!(
            superseded
                .iter()
                .all(|checkpoint| checkpoint.retained_dispatch_ordinal == 9)
        );
        assert_eq!(superseded[0].canonical_subject_ids, ["member:tarin-vel"]);
        assert_eq!(superseded[1].canonical_subject_ids, ["member:oren-pell"]);
        assert!(
            superseded
                .iter()
                .all(|checkpoint| checkpoint.public_identity_keys == ["tarinvel"])
        );
    }

    #[test]
    fn parallel_fissions_reserve_every_child_id_and_public_specific_name() {
        use ghostlight_dungeon::domain::{AgencyAxis, GestaltFissionPreview, GestaltPersonaState};
        use ghostlight_dungeon::elaboration::{
            ElaborationDispatch, ElaboratorTitle, WorldComplexityFissionQualification,
            WorldComplexityProposal, WorldComplexitySemanticQualification,
        };
        use std::collections::{BTreeMap, BTreeSet};

        let invocation = |ordinal: u64,
                          parent: &str,
                          specific_id: &str,
                          residual_id: &str,
                          specific_name: &str|
         -> ComplexityPreviewInvocation {
            ComplexityPreviewInvocation {
                dispatch: ElaborationDispatch {
                    schema: "ghostlight.elaboration_dispatch.v1".into(),
                    budget_ordinal: ordinal,
                    ordinal,
                    title: ElaboratorTitle::Charter,
                    title_weight: 1,
                    total_enabled_weight: 1,
                    requested_share_millionths: 1_000_000,
                    title_dispatch_count: ordinal,
                },
                parent_binding: format!("binding:{parent}"),
                proposal: WorldComplexityProposal::Fission {
                    preview: GestaltFissionPreview {
                        schema: "ghostlight.gestalt_fission_preview.v1".into(),
                        campaign_id: uuid::Uuid::nil(),
                        expected_world_revision: 0,
                        parent_gestalt_id: parent.into(),
                        partition_axis: AgencyAxis::Authority,
                        children: vec![
                            GestaltPersonaState {
                                schema: "ghostlight.gestalt_persona.v1".into(),
                                id: specific_id.into(),
                                name: specific_name.into(),
                                version: 0,
                                home_location_id: "yard".into(),
                                shared_capabilities: BTreeSet::new(),
                                shared_knowledge: BTreeSet::new(),
                                resources: BTreeSet::new(),
                                goals: Vec::new(),
                                pressures: Vec::new(),
                            },
                            GestaltPersonaState {
                                schema: "ghostlight.gestalt_persona.v1".into(),
                                id: residual_id.into(),
                                name: format!("{parent} remainder"),
                                version: 0,
                                home_location_id: "yard".into(),
                                shared_capabilities: BTreeSet::new(),
                                shared_knowledge: BTreeSet::new(),
                                resources: BTreeSet::new(),
                                goals: Vec::new(),
                                pressures: Vec::new(),
                            },
                        ],
                        child_partition_values: BTreeMap::from([
                            (specific_id.into(), "licensed".into()),
                            (residual_id.into(), "unresolved".into()),
                        ]),
                        residual_child_id: residual_id.into(),
                        member_child_assignments: BTreeMap::new(),
                        resource_child_assignments: BTreeMap::new(),
                        evidence_receipt_ids: Vec::new(),
                        gaps: Vec::new(),
                        canon_candidates: Vec::new(),
                        requires_approval: true,
                    },
                    qualification: WorldComplexityFissionQualification {
                        schema: "ghostlight.world_complexity_fission_qualification.v1".into(),
                        title: ElaboratorTitle::Charter,
                        jurisdiction_location_id: "yard".into(),
                        target_actionable_gain: 1,
                        semantic: WorldComplexitySemanticQualification::default(),
                    },
                },
                model_receipt_hashes: vec![format!("receipt:{ordinal}")],
            }
        };
        let invocations = vec![
            invocation(3, "yard-carriers", "licensed", "residual", "Ledger Keepers"),
            invocation(
                7,
                "route-carriers",
                "licensed",
                "residual",
                "Route Charter Hands",
            ),
            invocation(
                9,
                "sluice-carriers",
                "distinct-talliers",
                "sluice-residual",
                "Distinct Talliers",
            ),
        ];

        let (retained, superseded) = retain_unique_complexity_invocations(4, &invocations);

        assert_eq!(
            retained
                .iter()
                .map(|invocation| invocation.dispatch.ordinal)
                .collect::<Vec<_>>(),
            [3, 9]
        );
        assert_eq!(superseded.len(), 1);
        assert_eq!(superseded[0].dispatch.ordinal, 7);
        assert_eq!(superseded[0].retained_dispatch_ordinal, 3);
        assert_eq!(
            superseded[0].canonical_subject_ids,
            ["licensed", "residual"]
        );
        assert_eq!(superseded[0].public_identity_keys, ["routecharterhands"]);
    }

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
    fn strategic_foundation_shape_requires_the_requested_exact_population_and_institution_counts() {
        let mut civic = civic_manifest(0, "");
        civic.resident_population_ids = (0..4).map(|index| format!("people-{index}")).collect();
        civic.governing_institution_ids = (0..6).map(|index| format!("office-{index}")).collect();
        validate_strategic_foundation_civic_shape(&civic).unwrap();

        civic.resident_population_ids.remove("people-3");
        assert!(validate_strategic_foundation_civic_shape(&civic).is_err());
    }

    #[test]
    fn resumed_foundation_bindings_follow_complete_fission_lineage() {
        use std::collections::{BTreeMap, BTreeSet};

        let mut campaign = strategic_campaign();
        campaign.gestalt_lineages.insert(
            "residents".into(),
            ghostlight_dungeon::domain::GestaltLineage {
                schema: "ghostlight.gestalt_lineage.v1".into(),
                parent_gestalt_id: "residents".into(),
                child_gestalt_ids: vec!["east".into(), "west".into()],
                partition_axis: ghostlight_dungeon::domain::AgencyAxis::Geography,
                partition_values: BTreeMap::from([
                    ("east".into(), "east".into()),
                    ("west".into(), "west".into()),
                ]),
                residual_child_id: "west".into(),
                source_revision: 1,
            },
        );
        for index in 0..24 {
            let parent = format!("unrelated-{index}");
            let child = format!("unrelated-{index}-child");
            campaign.gestalt_lineages.insert(
                parent.clone(),
                ghostlight_dungeon::domain::GestaltLineage {
                    schema: "ghostlight.gestalt_lineage.v1".into(),
                    parent_gestalt_id: parent,
                    child_gestalt_ids: vec![child.clone()],
                    partition_axis: ghostlight_dungeon::domain::AgencyAxis::Geography,
                    partition_values: BTreeMap::from([(child.clone(), child.clone())]),
                    residual_child_id: child,
                    source_revision: 1,
                },
            );
        }
        let residents = BTreeSet::from(["east".into(), "west".into()]);
        let relations = BTreeSet::from([
            "relation:fission:east".into(),
            "relation:fission:west".into(),
        ]);

        assert!(fission_population_binding_is_present(
            &campaign,
            &residents,
            "residents"
        ));
        assert!(fission_relation_binding_is_present(
            &campaign, &relations, "relation"
        ));
        assert!(!fission_relation_binding_is_present(
            &campaign,
            &BTreeSet::from(["relation:fission:east".into()]),
            "relation"
        ));

        let checkpoint = civic_manifest(1, "");
        let mut current = checkpoint.clone();
        current.version = 2;
        current.resident_population_ids = residents;
        current.political_relation_ids = relations;
        current.semantic_verification_receipt_id = "fission-repair".into();
        assert!(civic_manifest_preserves(&campaign, &current, &checkpoint));
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
        let grounding = HistoricalWorldNewspaperGroundingVerdict {
            accepted: true,
            assessment: "Exact current grounding".into(),
            findings: Vec::new(),
        };
        let editorial = HistoricalWorldNewspaperEditorialVerdict {
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
        assert!(request.contains("Exactly 4 resident populations and 6 institutions"));
        assert!(request.contains("authority, succession, revenue, redress"));
        assert!(request.contains("public notice channel"));
        assert!(request.contains("Pressure: an intricately witnessed"));
        assert!(request.ends_with('…'));
        assert!(request.chars().count() <= 500);

        let maximum_identity_request = strategic_locality_request(
            &"N".repeat(160),
            &"location-id-".repeat(13),
            &"pressure-word ".repeat(40),
        );
        assert!(maximum_identity_request.chars().count() <= 500);
    }

    #[test]
    fn bounded_prompt_excerpt_never_cuts_through_a_word() {
        let excerpt = bounded_prompt_excerpt(
            "constitutional succession remains disputed across the eastern works",
            32,
        );
        assert_eq!(excerpt, "constitutional succession…");
        assert!(excerpt.chars().count() <= 32);
    }

    #[test]
    fn bounded_prompt_excerpt_preserves_an_overlong_first_token() {
        let pressure = format!("{} meaningful words follow", "x".repeat(80));
        let excerpt = bounded_prompt_excerpt(&pressure, 32);

        assert_eq!(excerpt.chars().count(), 32);
        assert!(excerpt.starts_with(&"x".repeat(31)));
        assert!(excerpt.ends_with('…'));
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
        assert!(request.contains("after civic admission"));
        assert!(request.contains("ordinary life, material pressure, political leverage"));
        assert!(request.contains("Pressure: an intricately witnessed"));
        assert!(request.ends_with('…'));
        assert!(!request.contains("exactly four"));
        assert!(!request.contains("exactly six"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn titled_semantic_repair_request_is_bounded_and_candidate_specific() {
        let location_id = "location-id-".repeat(13);
        let diagnostic = format!(
            "{{\"failed_checks\":[\"constituencies_cross_boundaries_or_local_homogeneity_justified\"],\"rationale\":\"{}\"}}",
            "verifier detail ".repeat(80)
        );
        let request = strategic_titled_repair_request(
            &"N".repeat(160),
            &location_id,
            &"pressure-word ".repeat(80),
            &diagnostic,
        );

        assert!(request.contains(&location_id));
        assert!(request.contains("same frozen world"));
        assert!(request.contains("fresh complete deepening"));
        assert!(request.contains("constituencies_cross_boundaries_or_local_homogeneity_justified"));
        assert!(request.contains("Pressure:"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn world_scale_completion_requires_the_exact_qualified_count() {
        validate_exact_world_scale_count(1_200, 1_200, 128).unwrap();
        for observed in [1_199, 1_201] {
            let diagnostic = validate_exact_world_scale_count(observed, 1_200, 128)
                .unwrap_err()
                .to_string();
            assert!(diagnostic.contains(&observed.to_string()));
            assert!(diagnostic.contains("exact target 1200"));
        }
    }

    #[test]
    fn strategic_region_request_preserves_the_complete_operator_brief() {
        let operator_brief = format!(
            "{} final settlement, route, deep-crisis response, polity, and culture requirements.",
            "regional-detail ".repeat(10)
        );
        assert!(operator_brief.chars().count() <= 241);

        let request = strategic_region_request(&operator_brief);

        assert!(request.ends_with(&operator_brief));
        assert!(request.contains("exactly 4 resident populations"));
        assert!(request.contains("6 governing institutions"));
        assert!(request.contains("3+ connected internal places"));
        assert!(request.contains("one mixed/cross-border population"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn complexity_semantic_feedback_preserves_each_failed_dimension() {
        let verdict = ghostlight_dungeon::elaboration::WorldComplexitySemanticVerification {
            public_names_are_legible_identifiers: true,
            names_do_not_repeat_an_overused_template: false,
            cultural_resemblance_is_grounded_not_quota_cloning: true,
            causal_additions_are_materially_distinct: true,
            causal_additions_do_not_repeat_an_overused_procedural_template: false,
            rationale: "The names share one mold and the causal additions repeat a filing ritual."
                .into(),
        };

        let diagnostic = complexity_semantic_rejection_diagnostic(&verdict).unwrap();
        let compacted = bounded_prompt_excerpt(&diagnostic, 800);

        assert!(compacted.contains("\"names_do_not_repeat_an_overused_template\":false"));
        assert!(
            compacted.contains(
                "\"causal_additions_do_not_repeat_an_overused_procedural_template\":false"
            )
        );
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
use std::collections::{BTreeMap, BTreeSet};
