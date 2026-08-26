use crate::{
    consumer::{
        ExternalProposalAcknowledgement, ExternalProposalList, ExternalProposalListRequest,
        ExternalProposalReceipt, ExternalProposalStatus, ExternalSubjectAuthority,
        ExternalWorldProposal, WorldNewspaperRequest, WorldSeed, WorldSeedAdmission,
        WorldSeedAdmissionReceipt, WorldSeedAdmissionRequest, WorldSeedProducerKind,
        validate_authority_key, validate_seed_admission,
    },
    domain::{Campaign, CampaignLifecycleReceipt, VaultEvidenceReceipt},
    kernel::WorldKernel,
    model::ModelStageReceipt,
    persistence::CampaignStore,
    session_zero::PublishedSessionZeroSeed,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct CampaignRuntime {
    pub store: CampaignStore,
    pub kernel: WorldKernel,
}

#[derive(Clone)]
pub struct CampaignRegistry {
    root: PathBuf,
    runtimes: Arc<RwLock<BTreeMap<Uuid, CampaignRuntime>>>,
}

impl CampaignRegistry {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            runtimes: Arc::new(RwLock::new(BTreeMap::new())),
        })
    }

    pub async fn load_existing(&self) -> Result<()> {
        let mut found = BTreeMap::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let Ok(id) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let path = entry.path().join("campaign.cc");
            if !path.is_file() {
                continue;
            }
            let store = CampaignStore::open(path)?;
            let keys = store.keys("campaign.v1")?;
            if keys.len() != 1 {
                return Err(anyhow!("campaign store must contain exactly one campaign"));
            }
            if keys[0] != id.to_string() {
                return Err(anyhow!("campaign directory and stored id disagree"));
            }
            let (row, mut campaign) = store
                .load::<Campaign>("campaign.v1", &keys[0])?
                .ok_or_else(|| anyhow!("campaign row vanished during migration"))?;
            let before = campaign.clone();
            let normalized_member_identities =
                crate::resolution::normalize_legacy_gestalt_member_identities(&mut campaign)?;
            crate::resolution::ensure_agency_profiles(&mut campaign);
            if campaign != before {
                let previous_resolution_epoch = campaign.resolution_policy.resolution_epoch;
                campaign.resolution_policy.resolution_epoch =
                    previous_resolution_epoch.saturating_add(1);
                campaign.resolution_cover = None;
                store.append_resolution_control(
                    &row,
                    &campaign,
                    &crate::domain::ResolutionControlReceipt {
                        schema: "ghostlight.resolution_control_receipt.v1".into(),
                        campaign_id: campaign.id,
                        world_revision: campaign.revision,
                        previous_resolution_epoch,
                        resolution_epoch: campaign.resolution_policy.resolution_epoch,
                        provider_configuration_epoch: campaign
                            .resolution_policy
                            .provider_configuration_epoch,
                        operation: if normalized_member_identities {
                            "normalize_gestalt_member_identity".into()
                        } else {
                            "migrate_flat_agency_state".into()
                        },
                        active_cell_budget: campaign.resolution_policy.active_cell_budget,
                        pin_ids: campaign.resolution_pins.keys().cloned().collect(),
                        committed_at: Utc::now(),
                    },
                )?;
            }
            found.insert(
                id,
                CampaignRuntime {
                    kernel: WorldKernel::start(store.clone()),
                    store,
                },
            );
        }
        *self.runtimes.write().await = found;
        Ok(())
    }

    pub async fn runtime(&self, id: Uuid) -> Result<CampaignRuntime> {
        self.runtimes
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("campaign runtime is not loaded"))
    }

    pub async fn list(&self) -> Vec<Uuid> {
        self.runtimes.read().await.keys().copied().collect()
    }

    #[doc(hidden)]
    pub async fn create_unadmitted_fixture(
        &self,
        campaign: Campaign,
        evidence: Vec<VaultEvidenceReceipt>,
        model_receipts: Vec<ModelStageReceipt>,
    ) -> Result<CampaignRuntime> {
        self.create_with_lifecycle(campaign, evidence, model_receipts, None)
            .await
    }

    pub async fn publish_session_zero(
        &self,
        mut campaign: Campaign,
        evidence: Vec<VaultEvidenceReceipt>,
        model_receipts: Vec<ModelStageReceipt>,
        publication: PublishedSessionZeroSeed,
    ) -> Result<CampaignRuntime> {
        crate::resolution::ensure_agency_profiles(&mut campaign);
        crate::compiler::validate_campaign_seed(&campaign)?;
        if publication
            .membership
            .controlled_actor_ids()
            .iter()
            .any(|actor_id| {
                !campaign.actors.contains_key(actor_id)
                    || campaign
                        .agency_profiles
                        .get(actor_id)
                        .is_none_or(|profile| profile.simulation_eligible)
            })
        {
            return Err(anyhow!(
                "every campaign member must bind an existing human-protected actor"
            ));
        }
        let seed = WorldSeed::from_campaign(&campaign)?;
        let admission = WorldSeedAdmission {
            schema: "ghostlight.world_seed_admission.v1".into(),
            campaign_id: campaign.id,
            producer_id: format!("session-zero:{}", publication.session_zero_id),
            producer_kind: WorldSeedProducerKind::Compiler,
            seed_digest: seed.digest()?,
            idempotency_key: publication.approved_seed_digest.clone(),
            external_subjects: Vec::new(),
        };
        self.admit_seed(
            seed,
            admission,
            "",
            evidence,
            model_receipts,
            Some(publication),
        )
        .await
        .map(|(runtime, _)| runtime)
    }

    pub async fn admit_world_seed(
        &self,
        request: WorldSeedAdmissionRequest,
    ) -> Result<(CampaignRuntime, WorldSeedAdmissionReceipt)> {
        if request.schema != "ghostlight.world_seed_admission_request.v1" {
            return Err(anyhow!("world seed admission request schema is invalid"));
        }
        self.admit_seed(
            request.seed,
            request.admission,
            &request.authority_key,
            Vec::new(),
            Vec::new(),
            None,
        )
        .await
    }

    async fn admit_seed(
        &self,
        seed: WorldSeed,
        admission: WorldSeedAdmission,
        authority_key: &str,
        evidence: Vec<VaultEvidenceReceipt>,
        model_receipts: Vec<ModelStageReceipt>,
        publication: Option<PublishedSessionZeroSeed>,
    ) -> Result<(CampaignRuntime, WorldSeedAdmissionReceipt)> {
        let campaign = validate_seed_admission(&seed, &admission, authority_key)?;
        let receipt = WorldSeedAdmissionReceipt {
            schema: "ghostlight.world_seed_admission_receipt.v1".into(),
            campaign_id: campaign.id,
            producer_id: admission.producer_id.clone(),
            seed_digest: admission.seed_digest.clone(),
            idempotency_key: admission.idempotency_key.clone(),
            admitted_subject_ids: admission
                .external_subjects
                .iter()
                .map(|item| item.subject_id.clone())
                .collect(),
            admitted_at: Utc::now(),
        };
        if let Some(existing) = self.open_existing_admission(&admission).await? {
            return Ok(existing);
        }
        let directory = self.root.join(campaign.id.to_string());
        if directory.exists() {
            return Err(anyhow!(
                "campaign directory belongs to another admitted seed"
            ));
        }
        let staging = self
            .root
            .join(format!(".creating-{}-{}", campaign.id, Uuid::new_v4()));
        fs::create_dir(&staging)?;
        let prepared = (|| -> Result<()> {
            let store = CampaignStore::open(staging.join("campaign.cc"))?;
            store.create_admitted_campaign(
                &campaign,
                &evidence,
                &model_receipts,
                &admission,
                &receipt,
                publication.as_ref(),
            )?;
            drop(store);
            fs::rename(&staging, &directory)?;
            Ok(())
        })();
        if let Err(error) = prepared {
            cleanup_staging_directory(&self.root, &staging);
            return Err(error);
        }
        let store = CampaignStore::open(directory.join("campaign.cc"))?;
        let runtime = CampaignRuntime {
            kernel: WorldKernel::start(store.clone()),
            store,
        };
        self.runtimes
            .write()
            .await
            .insert(campaign.id, runtime.clone());
        Ok((runtime, receipt))
    }

    async fn open_existing_admission(
        &self,
        admission: &WorldSeedAdmission,
    ) -> Result<Option<(CampaignRuntime, WorldSeedAdmissionReceipt)>> {
        let runtime = if let Ok(runtime) = self.runtime(admission.campaign_id).await {
            Some(runtime)
        } else {
            let directory = self.root.join(admission.campaign_id.to_string());
            if !directory.exists() {
                None
            } else {
                let store = CampaignStore::open(directory.join("campaign.cc"))?;
                Some(CampaignRuntime {
                    kernel: WorldKernel::start(store.clone()),
                    store,
                })
            }
        };
        let Some(runtime) = runtime else {
            return Ok(None);
        };
        let stored = runtime
            .store
            .load::<WorldSeedAdmission>(
                "world_seed_admission.v1",
                &admission.campaign_id.to_string(),
            )?
            .map(|(_, value)| value);
        let stored_receipt = runtime
            .store
            .load::<WorldSeedAdmissionReceipt>(
                "world_seed_admission_receipt.v1",
                &admission.campaign_id.to_string(),
            )?
            .map(|(_, value)| value);
        match (stored, stored_receipt) {
            (Some(stored), Some(receipt)) if stored == *admission => {
                self.runtimes
                    .write()
                    .await
                    .insert(admission.campaign_id, runtime.clone());
                Ok(Some((runtime, receipt)))
            }
            _ => Err(anyhow!("campaign id is already admitted from another seed")),
        }
    }

    pub async fn apply_external_subject_snapshot(
        &self,
        snapshot: crate::consumer::ExternalSubjectSnapshot,
    ) -> Result<crate::consumer::ExternalSnapshotReceipt> {
        let runtime = self.runtime(snapshot.campaign_id).await?;
        match runtime
            .kernel
            .command(crate::domain::WorldCommand::ApplyExternalSubjectSnapshot { snapshot })
            .await
            .map_err(|error| anyhow!(error.to_string()))?
        {
            crate::kernel::CommandResult::ExternalSnapshotCommitted { receipt, .. } => Ok(receipt),
            _ => Err(anyhow!(
                "kernel returned the wrong external snapshot result"
            )),
        }
    }

    pub async fn list_external_proposals(
        &self,
        request: ExternalProposalListRequest,
    ) -> Result<ExternalProposalList> {
        if request.schema != "ghostlight.external_proposal_list_request.v1" {
            return Err(anyhow!("external proposal list request schema is invalid"));
        }
        let runtime = self.runtime(request.campaign_id).await?;
        let (_, authority) = runtime
            .store
            .load::<ExternalSubjectAuthority>(
                "external_subject_authority.v1",
                &request.authority_id,
            )?
            .ok_or_else(|| anyhow!("external subject authority is unknown"))?;
        if authority.campaign_id != request.campaign_id || authority.owner_id != request.owner_id {
            return Err(anyhow!(
                "external proposal request does not match its authority"
            ));
        }
        validate_authority_key(&authority, &request.authority_key)?;
        let receipts = runtime
            .store
            .load_all::<ExternalProposalReceipt>("external_proposal_receipt.v1")?;
        let acknowledged: std::collections::BTreeSet<_> = receipts
            .into_iter()
            .map(|receipt| receipt.proposal_id)
            .collect();
        let proposals = runtime
            .store
            .load_all::<ExternalWorldProposal>("external_world_proposal.v1")?
            .into_iter()
            .filter(|proposal| {
                proposal.authority_id == request.authority_id
                    && !acknowledged.contains(&proposal.id)
            })
            .collect();
        Ok(ExternalProposalList {
            schema: "ghostlight.external_proposal_list.v1".into(),
            campaign_id: request.campaign_id,
            authority_id: request.authority_id,
            proposals,
        })
    }

    pub async fn compose_world_newspaper(
        &self,
        request: WorldNewspaperRequest,
    ) -> Result<crate::newspaper::WorldNewspaperIssue> {
        if request.schema != "ghostlight.world_newspaper_request.v1"
            || request.max_articles == 0
            || request.max_articles > 64
        {
            return Err(anyhow!("world newspaper request is invalid"));
        }
        let runtime = self.runtime(request.campaign_id).await?;
        let (_, authority) = runtime
            .store
            .load::<ExternalSubjectAuthority>(
                "external_subject_authority.v1",
                &request.authority_id,
            )?
            .ok_or_else(|| anyhow!("external subject authority is unknown"))?;
        if authority.campaign_id != request.campaign_id || authority.owner_id != request.owner_id {
            return Err(anyhow!(
                "world newspaper request does not match its authority"
            ));
        }
        validate_authority_key(&authority, &request.authority_key)?;
        let campaign = runtime
            .store
            .load::<Campaign>("campaign.v1", &request.campaign_id.to_string())?
            .ok_or_else(|| anyhow!("campaign is missing"))?
            .1;
        crate::newspaper::compose_world_newspaper(
            &campaign,
            request.title,
            usize::from(request.max_articles),
        )
    }

    pub async fn acknowledge_external_proposal(
        &self,
        acknowledgement: ExternalProposalAcknowledgement,
    ) -> Result<ExternalProposalReceipt> {
        if acknowledgement.schema != "ghostlight.external_proposal_acknowledgement.v1"
            || acknowledgement.status == ExternalProposalStatus::Pending
            || acknowledgement.idempotency_key.trim().is_empty()
            || acknowledgement.result_summary.trim().is_empty()
            || acknowledgement.result_summary.chars().count() > 1000
        {
            return Err(anyhow!("external proposal acknowledgement is invalid"));
        }
        let runtime = self.runtime(acknowledgement.campaign_id).await?;
        let (_, authority) = runtime
            .store
            .load::<ExternalSubjectAuthority>(
                "external_subject_authority.v1",
                &acknowledgement.authority_id,
            )?
            .ok_or_else(|| anyhow!("external subject authority is unknown"))?;
        if authority.owner_id != acknowledgement.owner_id {
            return Err(anyhow!(
                "external acknowledgement does not match its authority"
            ));
        }
        validate_authority_key(&authority, &acknowledgement.authority_key)?;
        let (_, proposal) = runtime
            .store
            .load::<ExternalWorldProposal>(
                "external_world_proposal.v1",
                &acknowledgement.proposal_id,
            )?
            .ok_or_else(|| anyhow!("external proposal is unknown"))?;
        if proposal.authority_id != acknowledgement.authority_id
            || proposal.campaign_id != acknowledgement.campaign_id
        {
            return Err(anyhow!("external proposal belongs to another authority"));
        }
        runtime
            .store
            .record_external_proposal_receipt(&ExternalProposalReceipt {
                schema: "ghostlight.external_proposal_receipt.v1".into(),
                campaign_id: acknowledgement.campaign_id,
                authority_id: acknowledgement.authority_id,
                proposal_id: acknowledgement.proposal_id,
                idempotency_key: acknowledgement.idempotency_key,
                status: acknowledgement.status,
                result_summary: acknowledgement.result_summary,
                acknowledged_at: acknowledgement.acknowledged_at,
            })
    }

    async fn create_with_lifecycle(
        &self,
        mut campaign: Campaign,
        evidence: Vec<VaultEvidenceReceipt>,
        model_receipts: Vec<ModelStageReceipt>,
        additional_lifecycle: Option<CampaignLifecycleReceipt>,
    ) -> Result<CampaignRuntime> {
        crate::resolution::ensure_agency_profiles(&mut campaign);
        if self.runtimes.read().await.contains_key(&campaign.id) {
            return Err(anyhow!("campaign already exists"));
        }
        let directory = self.root.join(campaign.id.to_string());
        if directory.exists() {
            return Err(anyhow!("campaign directory already exists"));
        }
        let staging = self
            .root
            .join(format!(".creating-{}-{}", campaign.id, Uuid::new_v4()));
        fs::create_dir(&staging)?;
        let prepared = (|| -> Result<()> {
            let store = CampaignStore::open(staging.join("campaign.cc"))?;
            WorldKernel::initialize_campaign(
                &store,
                crate::domain::WorldCommand::CreateCampaign {
                    campaign: campaign.clone(),
                    evidence_receipts: evidence,
                    model_stage_receipts: model_receipts,
                },
            )
            .map_err(|error| anyhow!(error.to_string()))?;
            if let Some(receipt) = &additional_lifecycle {
                store.insert(
                    "campaign_lifecycle_receipt.v1",
                    "ghostlight.campaign_lifecycle_receipt.v1",
                    &format!("{}:{}", receipt.campaign_id, receipt.operation),
                    receipt,
                )?;
            }
            drop(store);
            fs::rename(&staging, &directory)?;
            Ok(())
        })();
        if let Err(error) = prepared {
            cleanup_staging_directory(&self.root, &staging);
            return Err(error);
        }
        let store = match CampaignStore::open(directory.join("campaign.cc")) {
            Ok(store) => store,
            Err(error) => {
                let _ = fs::rename(&directory, &staging);
                cleanup_staging_directory(&self.root, &staging);
                return Err(error);
            }
        };
        let kernel = WorldKernel::start(store.clone());
        let runtime = CampaignRuntime { store, kernel };
        self.runtimes
            .write()
            .await
            .insert(campaign.id, runtime.clone());
        Ok(runtime)
    }

    pub async fn fork(&self, source_id: Uuid, name: String) -> Result<CampaignRuntime> {
        let name = validated_branch_name(name)?;
        let source = self.runtime(source_id).await?;
        let key = source_id.to_string();
        let (_, parent) = source
            .store
            .load::<Campaign>("campaign.v1", &key)?
            .ok_or_else(|| anyhow!("source campaign vanished"))?;
        let mut evidence = Vec::new();
        for id in source.store.keys("vault_evidence_receipt.v1")? {
            if let Some((_, receipt)) = source
                .store
                .load::<VaultEvidenceReceipt>("vault_evidence_receipt.v1", &id)?
            {
                evidence.push(receipt);
            }
        }
        let model_receipts = source
            .store
            .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")?;
        let parent_revision = parent.revision;
        let mut fork = parent;
        fork.id = Uuid::new_v4();
        fork.name = name;
        fork.revision = 0;
        fork.pending_world_proposals.clear();
        fork.resolution_policy.resolution_epoch =
            fork.resolution_policy.resolution_epoch.saturating_add(1);
        fork.resolution_cover = None;
        let runtime = self
            .create_with_lifecycle(
                fork.clone(),
                evidence,
                model_receipts,
                Some(CampaignLifecycleReceipt {
                    schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                    campaign_id: fork.id,
                    operation: "fork".into(),
                    parent_campaign_id: Some(source_id),
                    parent_revision: Some(parent_revision),
                    created_at: Utc::now(),
                }),
            )
            .await?;
        Ok(runtime)
    }

    pub async fn reset(&self, source_id: Uuid, name: String) -> Result<CampaignRuntime> {
        let name = validated_branch_name(name)?;
        let source = self.runtime(source_id).await?;
        let (_, mut seed) = source
            .store
            .load::<Campaign>("campaign_seed.v1", &source_id.to_string())?
            .ok_or_else(|| anyhow!("campaign has no approved seed"))?;
        let parent_revision = source
            .store
            .load::<Campaign>("campaign.v1", &source_id.to_string())?
            .ok_or_else(|| anyhow!("source campaign vanished"))?
            .1
            .revision;
        seed.id = Uuid::new_v4();
        seed.name = name;
        seed.revision = 0;
        seed.pending_world_proposals.clear();
        let mut evidence = Vec::new();
        for id in source.store.keys("vault_evidence_receipt.v1")? {
            if let Some((_, receipt)) = source
                .store
                .load::<VaultEvidenceReceipt>("vault_evidence_receipt.v1", &id)?
            {
                evidence.push(receipt);
            }
        }
        let model_receipts = source
            .store
            .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")?;
        let runtime = self
            .create_with_lifecycle(
                seed.clone(),
                evidence,
                model_receipts,
                Some(CampaignLifecycleReceipt {
                    schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                    campaign_id: seed.id,
                    operation: "reset".into(),
                    parent_campaign_id: Some(source_id),
                    parent_revision: Some(parent_revision),
                    created_at: Utc::now(),
                }),
            )
            .await?;
        Ok(runtime)
    }

    pub async fn export(
        &self,
        campaign_id: Uuid,
        exports_root: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let runtime = self.runtime(campaign_id).await?;
        fs::create_dir_all(exports_root.as_ref())?;
        let path = exports_root.as_ref().join(format!(
            "{}-{}-{}.cc",
            campaign_id,
            Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
            Uuid::new_v4()
        ));
        runtime.store.snapshot_to(&path)?;
        Ok(path)
    }
}

fn cleanup_staging_directory(root: &Path, staging: &Path) {
    let safe = staging.parent() == Some(root)
        && staging
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".creating-"));
    if safe && staging.exists() {
        let _ = fs::remove_dir_all(staging);
    }
}

fn validated_branch_name(name: String) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(anyhow!(
            "campaign name must contain 1 to 80 visible characters"
        ));
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ActorState, BranchOrigin, Location, WorldCommitReceipt};
    use crate::session_zero::{
        ApprovedCampaignBrief, CampaignContract, CampaignDmPersona, CampaignGovernance,
        CampaignMember, CampaignMembership, CharacterDraft,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn seed(name: &str) -> Campaign {
        let actor = ActorState {
            id: "player".into(),
            name: "Player".into(),
            location_id: "room".into(),
            capabilities: BTreeSet::new(),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        };
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: Uuid::new_v4(),
            name: name.into(),
            revision: 0,
            branch_origin: BranchOrigin {
                canon_cutoff: "test".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([(
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Room".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            )]),
            actors: BTreeMap::from([("player".into(), actor)]),
            institutions: BTreeMap::new(),
            clocks: BTreeMap::new(),
            facts: BTreeMap::new(),
            civic_systems: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: Utc::now(),
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: vec![],
            news: vec![],
            canon_candidates: BTreeMap::new(),
            gestalts: BTreeMap::new(),
            gestalt_members: BTreeMap::new(),
            pending_world_proposals: vec![],
            agency_profiles: BTreeMap::new(),
            agency_relations: BTreeMap::new(),
            gestalt_lineages: BTreeMap::new(),
            resolution_policy: Default::default(),
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        }
    }

    fn publication(campaign: &Campaign, seed_digest: &str) -> PublishedSessionZeroSeed {
        let member_id = "member:host".to_string();
        let mut contract = CampaignContract::default();
        contract.campaign_name = campaign.name.clone();
        contract.vault_provider = "fixture".into();
        let character = CharacterDraft {
            schema: "ghostlight.character_draft.v1".into(),
            member_id: member_id.clone(),
            actor_id: campaign.player_actor_id.clone(),
            name: campaign.actors[&campaign.player_actor_id].name.clone(),
            ..CharacterDraft::default()
        };
        let membership = CampaignMembership {
            schema: "ghostlight.campaign_membership.v1".into(),
            campaign_id: campaign.id,
            governance_epoch: 0,
            host_member_id: member_id.clone(),
            members: BTreeMap::from([(
                member_id.clone(),
                CampaignMember {
                    member_id: member_id.clone(),
                    account_hash: "account:host".into(),
                    display_name: "Host".into(),
                    actor_id: campaign.player_actor_id.clone(),
                    is_host: true,
                    active: true,
                    cell_allowance: 8,
                },
            )]),
            extraordinary_permissions: BTreeMap::new(),
        };
        PublishedSessionZeroSeed {
            schema: "ghostlight.published_session_zero_seed.v1".into(),
            session_zero_id: Uuid::new_v4(),
            approved_seed_digest: seed_digest.into(),
            contract: contract.clone(),
            membership: membership.clone(),
            governance: CampaignGovernance {
                schema: "ghostlight.campaign_governance.v1".into(),
                campaign_id: campaign.id,
                governance_epoch: 0,
                time_advance_policy: "unanimous".into(),
                pooled_cell_ceiling: 8,
                cooperative_shared_scene_only: true,
                pvp_enabled: false,
            },
            dm_persona: CampaignDmPersona {
                schema: "ghostlight.campaign_dm_persona.v1".into(),
                id: "dm:test".into(),
                name: "Ghostlight".into(),
                voice: "Candid".into(),
                shared_memories: vec![],
                private_member_memories: BTreeMap::new(),
            },
            approvals: vec![],
            approved_brief: ApprovedCampaignBrief {
                schema: "ghostlight.approved_campaign_brief.v1".into(),
                session_zero_id: Uuid::new_v4(),
                host_member_id: member_id.clone(),
                contract,
                aggregate_boundaries: vec![],
                characters: vec![character],
                member_actor_ids: BTreeMap::from([(member_id, campaign.player_actor_id.clone())]),
                shared_digest: "sha256:shared".into(),
                character_digests: BTreeMap::new(),
            },
            approved_world_preview_digest: "sha256:preview".into(),
            approved_evidence_gaps: vec![],
            approved_branch_assumptions: vec![],
            boundaries: vec![],
        }
    }

    #[tokio::test]
    async fn create_fork_reset_and_export_preserve_isolated_stores() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let original = seed("Original");
        let model_receipt = ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: "sha256:receipt".into(),
            provider: "fixture".into(),
            model: "fixture".into(),
            stage: "world_compile".into(),
            snapshot_binding: "custom-start".into(),
            request_hash: "sha256:request".into(),
            output_hash: "sha256:output".into(),
            source_receipt_ids: vec![],
            latency_ms: 1,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 7,
            output_chars: 7,
            provider_attempts: vec![],
        };
        registry
            .create_unadmitted_fixture(original.clone(), vec![], vec![model_receipt.clone()])
            .await
            .unwrap();
        let fork = registry.fork(original.id, "Fork".into()).await.unwrap();
        let fork_id = fork.store.keys("campaign.v1").unwrap()[0]
            .parse::<Uuid>()
            .unwrap();
        assert_ne!(fork_id, original.id);
        assert_eq!(
            fork.store
                .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")
                .unwrap(),
            vec![model_receipt]
        );
        assert_eq!(
            fork.store.keys("campaign_lifecycle_receipt.v1").unwrap(),
            vec![format!("{}:create", fork_id), format!("{}:fork", fork_id)]
        );
        let reset = registry.reset(original.id, "Reset".into()).await.unwrap();
        let reset_id = reset.store.keys("campaign.v1").unwrap()[0]
            .parse::<Uuid>()
            .unwrap();
        assert_ne!(reset_id, original.id);
        assert_eq!(
            reset.store.keys("campaign_lifecycle_receipt.v1").unwrap(),
            vec![
                format!("{}:create", reset_id),
                format!("{}:reset", reset_id)
            ]
        );
        assert_eq!(registry.list().await.len(), 3);
        let exported = registry
            .export(fork_id, dir.path().join("exports"))
            .await
            .unwrap();
        let second_export = registry
            .export(fork_id, dir.path().join("exports"))
            .await
            .unwrap();
        assert_ne!(exported, second_export);
        let export_store = CampaignStore::open(exported).unwrap();
        let second_export_store = CampaignStore::open(second_export).unwrap();
        assert_eq!(
            export_store.keys("campaign.v1").unwrap(),
            vec![fork_id.to_string()]
        );
        assert_eq!(
            second_export_store.keys("campaign.v1").unwrap(),
            vec![fork_id.to_string()]
        );
        assert_eq!(
            registry
                .runtime(original.id)
                .await
                .unwrap()
                .store
                .keys("campaign.v1")
                .unwrap(),
            vec![original.id.to_string()]
        );
    }

    #[tokio::test]
    async fn fork_and_reset_reject_invalid_names_before_creating_state() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let original = seed("Original");
        registry
            .create_unadmitted_fixture(original.clone(), vec![], vec![])
            .await
            .unwrap();

        for invalid in ["", "   ", "bad\nname"] {
            assert!(registry.fork(original.id, invalid.into()).await.is_err());
            assert!(registry.reset(original.id, invalid.into()).await.is_err());
        }
        let too_long = "x".repeat(81);
        assert!(registry.fork(original.id, too_long.clone()).await.is_err());
        assert!(registry.reset(original.id, too_long).await.is_err());
        assert_eq!(registry.list().await, vec![original.id]);

        let fork = registry
            .fork(original.id, "  A clean branch  ".into())
            .await
            .unwrap();
        let fork_id = fork.store.keys("campaign.v1").unwrap()[0].clone();
        let forked = fork
            .store
            .load::<Campaign>("campaign.v1", &fork_id)
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(forked.name, "A clean branch");
    }

    #[tokio::test]
    async fn failed_creation_never_enters_the_discoverable_campaign_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        let registry = CampaignRegistry::new(&root).unwrap();
        let mut invalid = seed("Invalid");
        invalid
            .actors
            .get_mut(&invalid.player_actor_id)
            .unwrap()
            .location_id = "missing".into();

        assert!(
            registry
                .create_unadmitted_fixture(invalid, vec![], vec![])
                .await
                .is_err()
        );
        assert!(registry.list().await.is_empty());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);

        let valid = seed("Valid");
        let runtime = registry
            .create_unadmitted_fixture(valid.clone(), vec![], vec![])
            .await
            .unwrap();
        assert_eq!(
            runtime.store.keys("campaign_lifecycle_receipt.v1").unwrap(),
            vec![format!("{}:create", valid.id)]
        );
    }

    #[tokio::test]
    async fn failed_additional_lifecycle_write_never_publishes_the_campaign() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        let registry = CampaignRegistry::new(&root).unwrap();
        let campaign = seed("Colliding lifecycle");

        let result = registry
            .create_with_lifecycle(
                campaign.clone(),
                vec![],
                vec![],
                Some(CampaignLifecycleReceipt {
                    schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                    campaign_id: campaign.id,
                    operation: "create".into(),
                    parent_campaign_id: None,
                    parent_revision: None,
                    created_at: Utc::now(),
                }),
            )
            .await;

        assert!(result.is_err());
        assert!(registry.list().await.is_empty());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn session_zero_publication_is_atomic_complete_and_digest_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        let registry = CampaignRegistry::new(&root).unwrap();
        let campaign = seed("Approved seed");
        let approved = publication(&campaign, "sha256:approved");

        let first = registry
            .publish_session_zero(campaign.clone(), vec![], vec![], approved.clone())
            .await
            .unwrap();
        for document_type in [
            "campaign.v1",
            "world_seed_admission.v1",
            "world_seed_admission_receipt.v1",
            "session_zero_publication.v1",
            "campaign_membership.v1",
            "campaign_contract.v1",
            "campaign_governance.v1",
            "campaign_dm_persona.v1",
            "approved_campaign_brief.v1",
        ] {
            assert_eq!(first.store.keys(document_type).unwrap().len(), 1);
        }
        assert_eq!(registry.list().await, vec![campaign.id]);
        assert!(std::fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".creating-")
        }));

        let recovered = registry
            .publish_session_zero(campaign.clone(), vec![], vec![], approved)
            .await
            .unwrap();
        assert_eq!(
            recovered.store.keys("campaign.v1").unwrap(),
            vec![campaign.id.to_string()]
        );

        let conflicting = publication(&campaign, "sha256:different");
        assert!(
            registry
                .publish_session_zero(campaign, vec![], vec![], conflicting)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn contract_review_replaces_all_governance_rows_in_one_cas() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let campaign = seed("Reviewable seed");
        let original_publication = publication(&campaign, "sha256:original");
        let runtime = registry
            .publish_session_zero(
                campaign.clone(),
                vec![],
                vec![],
                original_publication.clone(),
            )
            .await
            .unwrap();
        let (campaign_row, mut next_campaign) = runtime
            .store
            .load::<Campaign>("campaign.v1", &campaign.id.to_string())
            .unwrap()
            .unwrap();
        next_campaign.revision = 1;
        let mut reviewed = original_publication;
        reviewed.approved_seed_digest = "sha256:reviewed".into();
        reviewed.contract.pacing = "deliberate".into();
        reviewed.approved_brief.contract = reviewed.contract.clone();
        reviewed.membership.governance_epoch = 1;
        reviewed.governance.governance_epoch = 1;
        reviewed.dm_persona.voice = "Patient and exact".into();
        let receipt = WorldCommitReceipt {
            schema: "ghostlight.world_commit_receipt.v1".into(),
            campaign_id: campaign.id,
            previous_revision: 0,
            revision: 1,
            command_kind: "unanimous_contract_review".into(),
            committed_at: Utc::now(),
            roll: None,
        };

        runtime
            .store
            .commit_contract_review(&campaign_row, &next_campaign, &reviewed, &receipt)
            .unwrap();
        assert_eq!(
            runtime
                .store
                .load::<Campaign>("campaign.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            1
        );
        assert_eq!(
            runtime
                .store
                .load::<CampaignContract>("campaign_contract.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .pacing,
            "deliberate"
        );
        assert_eq!(
            runtime
                .store
                .load::<CampaignDmPersona>("campaign_dm_persona.v1", "dm:test")
                .unwrap()
                .unwrap()
                .1
                .voice,
            "Patient and exact"
        );

        let mut impossible_second = reviewed;
        impossible_second.contract.pacing = "frantic".into();
        assert!(
            runtime
                .store
                .commit_contract_review(
                    &campaign_row,
                    &next_campaign,
                    &impossible_second,
                    &receipt,
                )
                .is_err()
        );
        assert_eq!(
            runtime
                .store
                .load::<CampaignContract>("campaign_contract.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .pacing,
            "deliberate"
        );
    }

    #[tokio::test]
    async fn load_existing_migrates_flat_campaign_without_advancing_world_revision() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        let flat = seed("Flat legacy");
        let campaign_root = root.join(flat.id.to_string());
        std::fs::create_dir_all(&campaign_root).unwrap();
        let store = CampaignStore::open(campaign_root.join("campaign.cc")).unwrap();
        store
            .create_unadmitted_fixture_campaign(&flat, &[], &[])
            .unwrap();
        drop(store);
        let registry = CampaignRegistry::new(&root).unwrap();
        registry.load_existing().await.unwrap();
        let runtime = registry.runtime(flat.id).await.unwrap();
        let migrated = runtime
            .store
            .load::<Campaign>("campaign.v1", &flat.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(migrated.revision, flat.revision);
        assert_eq!(migrated.world_time, flat.world_time);
        assert!(migrated.agency_profiles.contains_key(&flat.player_actor_id));
        assert_eq!(migrated.resolution_policy.resolution_epoch, 1);
        assert_eq!(
            runtime
                .store
                .keys("resolution_control_receipt.v1")
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn consumer_seed_admission_and_external_snapshot_share_one_authoritative_path() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path()).unwrap();
        let mut campaign = seed("Consumer world");
        campaign.institutions.insert(
            "external-hold".into(),
            crate::domain::InstitutionState {
                id: "external-hold".into(),
                name: "External Hold".into(),
                resources: vec!["ore".into()],
                goals: vec!["remain sovereign".into()],
                posture: "watchful".into(),
            },
        );
        campaign.institutions.insert(
            "inner-court".into(),
            crate::domain::InstitutionState {
                id: "inner-court".into(),
                name: "The Inner Court".into(),
                resources: vec!["sealed minutes".into()],
                goals: vec!["control the grain audit".into()],
                posture: "denying that the missing grain has political significance".into(),
            },
        );
        campaign.gestalts.insert(
            "external-population".into(),
            crate::domain::GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "external-population".into(),
                name: "External Population".into(),
                version: 1,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::from(["farming".into()]),
                shared_knowledge: BTreeSet::from(["old road".into()]),
                resources: BTreeSet::new(),
                goals: vec!["endure".into()],
                pressures: Vec::new(),
            },
        );
        campaign.player_actor_id = "external-population".into();
        let seed = WorldSeed::from_campaign(&campaign).unwrap();
        let authority_key = "test-authority-key";
        let admission = WorldSeedAdmission {
            schema: "ghostlight.world_seed_admission.v1".into(),
            campaign_id: campaign.id,
            producer_id: "fixture-consumer".into(),
            producer_kind: WorldSeedProducerKind::Consumer,
            seed_digest: seed.digest().unwrap(),
            idempotency_key: "admit-once".into(),
            external_subjects: vec![
                ExternalSubjectAuthority {
                    schema: "ghostlight.external_subject_authority.v1".into(),
                    id: "authority:external-hold".into(),
                    campaign_id: campaign.id,
                    subject_id: "external-hold".into(),
                    subject_kind: crate::domain::AgencySubjectKind::Institution,
                    owner_id: "fixture-consumer".into(),
                    authority_key_sha256: crate::consumer::authority_key_digest(authority_key),
                    last_source_revision: None,
                    last_payload_digest: None,
                },
                ExternalSubjectAuthority {
                    schema: "ghostlight.external_subject_authority.v1".into(),
                    id: "authority:external-population".into(),
                    campaign_id: campaign.id,
                    subject_id: "external-population".into(),
                    subject_kind: crate::domain::AgencySubjectKind::Gestalt,
                    owner_id: "fixture-consumer".into(),
                    authority_key_sha256: crate::consumer::authority_key_digest(authority_key),
                    last_source_revision: None,
                    last_payload_digest: None,
                },
            ],
        };
        let request = WorldSeedAdmissionRequest {
            schema: "ghostlight.world_seed_admission_request.v1".into(),
            seed,
            admission,
            authority_key: authority_key.into(),
        };
        let (runtime, receipt) = registry.admit_world_seed(request.clone()).await.unwrap();
        assert_eq!(
            receipt.admitted_subject_ids,
            vec!["external-hold", "external-population"]
        );
        assert!(
            !runtime
                .store
                .load::<Campaign>("campaign.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .agency_profiles["external-hold"]
                .simulation_eligible
        );
        let (_, repeated) = registry.admit_world_seed(request).await.unwrap();
        assert_eq!(repeated.seed_digest, receipt.seed_digest);
        let newspaper_request = crate::consumer::WorldNewspaperRequest {
            schema: "ghostlight.world_newspaper_request.v1".into(),
            campaign_id: campaign.id,
            authority_id: "authority:external-hold".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            title: "The Consumer Gazette".into(),
            max_articles: 12,
        };
        let mut denied_newspaper = newspaper_request.clone();
        denied_newspaper.authority_key = "wrong-key".into();
        assert!(
            registry
                .compose_world_newspaper(denied_newspaper)
                .await
                .is_err()
        );
        let newspaper = registry
            .compose_world_newspaper(newspaper_request.clone())
            .await
            .unwrap();
        assert_eq!(newspaper.source_world_revision, 0);
        assert!(newspaper.articles.is_empty());

        let mut snapshot = crate::consumer::ExternalSubjectSnapshot {
            schema: "ghostlight.external_subject_snapshot.v1".into(),
            campaign_id: campaign.id,
            expected_world_revision: 0,
            authority_id: "authority:external-hold".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            source_revision: 7,
            idempotency_key: "snapshot-seven".into(),
            payload_digest: String::new(),
            projection: crate::consumer::ExternalSubjectProjection::Institution(
                crate::domain::InstitutionState {
                    id: "external-hold".into(),
                    name: "External Hold".into(),
                    resources: vec!["ore".into(), "grain".into()],
                    goals: vec!["remain sovereign".into()],
                    posture: "mobilized".into(),
                },
            ),
        };
        snapshot.payload_digest = crate::consumer::snapshot_payload_digest(&snapshot).unwrap();
        let mut denied = snapshot.clone();
        denied.authority_key = "wrong-key".into();
        assert!(
            registry
                .apply_external_subject_snapshot(denied)
                .await
                .is_err()
        );
        assert_eq!(
            runtime
                .store
                .load::<Campaign>("campaign.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            0
        );
        let snapshot_receipt = registry
            .apply_external_subject_snapshot(snapshot.clone())
            .await
            .unwrap();
        assert_eq!(snapshot_receipt.world_revision, 1);
        let updated = runtime
            .store
            .load::<Campaign>("campaign.v1", &campaign.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(updated.institutions["external-hold"].posture, "mobilized");
        assert!(
            updated.agency_profiles["external-hold"].facets[&crate::domain::AgencyAxis::Authority]
                .contains("mobilized")
        );
        assert!(
            updated.agency_profiles["external-hold"].facets
                [&crate::domain::AgencyAxis::EconomyRole]
                .contains("grain")
        );
        assert_eq!(
            registry
                .apply_external_subject_snapshot(snapshot)
                .await
                .unwrap(),
            snapshot_receipt
        );
        let mut stale_snapshot = crate::consumer::ExternalSubjectSnapshot {
            schema: "ghostlight.external_subject_snapshot.v1".into(),
            campaign_id: campaign.id,
            expected_world_revision: 1,
            authority_id: "authority:external-hold".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            source_revision: 6,
            idempotency_key: "snapshot-six".into(),
            payload_digest: String::new(),
            projection: crate::consumer::ExternalSubjectProjection::Institution(
                campaign.institutions["external-hold"].clone(),
            ),
        };
        stale_snapshot.payload_digest =
            crate::consumer::snapshot_payload_digest(&stale_snapshot).unwrap();
        assert!(
            registry
                .apply_external_subject_snapshot(stale_snapshot)
                .await
                .is_err()
        );
        assert_eq!(
            runtime
                .store
                .load::<Campaign>("campaign.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            1
        );
        let mut invalid_gestalt_snapshot = crate::consumer::ExternalSubjectSnapshot {
            schema: "ghostlight.external_subject_snapshot.v1".into(),
            campaign_id: campaign.id,
            expected_world_revision: 1,
            authority_id: "authority:external-population".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            source_revision: 1,
            idempotency_key: "population-invalid-home".into(),
            payload_digest: String::new(),
            projection: crate::consumer::ExternalSubjectProjection::Gestalt(
                crate::domain::GestaltPersonaState {
                    schema: "ghostlight.gestalt_persona_state.v1".into(),
                    id: "external-population".into(),
                    name: "External Population".into(),
                    version: 2,
                    home_location_id: "invented-place".into(),
                    shared_capabilities: BTreeSet::from(["farming".into()]),
                    shared_knowledge: BTreeSet::from(["old road".into()]),
                    resources: BTreeSet::new(),
                    goals: vec!["endure".into()],
                    pressures: Vec::new(),
                },
            ),
        };
        invalid_gestalt_snapshot.payload_digest =
            crate::consumer::snapshot_payload_digest(&invalid_gestalt_snapshot).unwrap();
        let error = registry
            .apply_external_subject_snapshot(invalid_gestalt_snapshot)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("unknown home location"),
            "{error}"
        );
        assert_eq!(
            runtime
                .store
                .load::<Campaign>("campaign.v1", &campaign.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            1
        );
        let mut gestalt_snapshot = crate::consumer::ExternalSubjectSnapshot {
            schema: "ghostlight.external_subject_snapshot.v1".into(),
            campaign_id: campaign.id,
            expected_world_revision: 1,
            authority_id: "authority:external-population".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            source_revision: 1,
            idempotency_key: "population-one".into(),
            payload_digest: String::new(),
            projection: crate::consumer::ExternalSubjectProjection::Gestalt(
                crate::domain::GestaltPersonaState {
                    schema: "ghostlight.gestalt_persona_state.v1".into(),
                    id: "external-population".into(),
                    name: "External Population".into(),
                    version: 2,
                    home_location_id: "room".into(),
                    shared_capabilities: BTreeSet::from(["farming".into(), "milling".into()]),
                    shared_knowledge: BTreeSet::from(["old road".into(), "new levy".into()]),
                    resources: BTreeSet::new(),
                    goals: vec!["endure".into()],
                    pressures: vec!["taxation".into()],
                },
            ),
        };
        gestalt_snapshot.payload_digest =
            crate::consumer::snapshot_payload_digest(&gestalt_snapshot).unwrap();
        let gestalt_receipt = registry
            .apply_external_subject_snapshot(gestalt_snapshot)
            .await
            .unwrap();
        assert_eq!(gestalt_receipt.world_revision, 2);
        let updated = runtime
            .store
            .load::<Campaign>("campaign.v1", &campaign.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(updated.gestalts["external-population"].version, 2);
        assert!(!updated.agency_profiles["external-population"].simulation_eligible);
        assert!(
            updated.agency_profiles["external-population"].facets
                [&crate::domain::AgencyAxis::EconomyRole]
                .contains("milling")
        );
        runtime
            .kernel
            .command(crate::domain::WorldCommand::AdvanceStrategicTick {
                expected_revision: 2,
                source: crate::domain::TickSource::Scheduler,
                plan: Some(crate::domain::StrategicTickPlan {
                    institution_actions: vec![crate::domain::StrategicInstitutionAction {
                        institution_id: "inner-court".into(),
                        posture:
                            "accusing three granary auditors of being one witch in a long coat"
                                .into(),
                        location_ids: vec!["room".into()],
                        public_channels: vec!["court broadsheet".into()],
                    }],
                    ..Default::default()
                }),
                model_receipt_hash: Some(format!("sha256:{}", "a".repeat(64))),
                resolution_wave: None,
            })
            .await
            .unwrap();
        let newspaper = registry
            .compose_world_newspaper(newspaper_request)
            .await
            .unwrap();
        assert_eq!(newspaper.source_world_revision, 3);
        assert_eq!(newspaper.articles.len(), 1);
        assert_eq!(
            newspaper.articles[0].event_ids,
            ["strategic:3:institution:inner-court"]
        );
        assert!(
            newspaper.articles[0]
                .body
                .contains("one witch in a long coat")
        );
        let proposal = ExternalWorldProposal {
            schema: "ghostlight.external_world_proposal.v1".into(),
            id: "external-proposal:test".into(),
            campaign_id: campaign.id,
            world_revision: 1,
            authority_id: "authority:external-hold".into(),
            external_subject_id: "external-hold".into(),
            source_subject_id: "foreign-court".into(),
            intent: "Offer a treaty.".into(),
            intended_effect: "Open negotiations.".into(),
            action_digest: format!("sha256:{}", "a".repeat(64)),
            public_channels: vec!["diplomatic".into()],
            state_references: Vec::new(),
            status: ExternalProposalStatus::Pending,
            created_at: Utc::now(),
        };
        runtime
            .store
            .insert(
                "external_world_proposal.v1",
                "ghostlight.external_world_proposal.v1",
                &proposal.id,
                &proposal,
            )
            .unwrap();
        let proposals = registry
            .list_external_proposals(ExternalProposalListRequest {
                schema: "ghostlight.external_proposal_list_request.v1".into(),
                campaign_id: campaign.id,
                authority_id: "authority:external-hold".into(),
                owner_id: "fixture-consumer".into(),
                authority_key: authority_key.into(),
            })
            .await
            .unwrap();
        assert_eq!(proposals.proposals, vec![proposal.clone()]);
        let acknowledgement = ExternalProposalAcknowledgement {
            schema: "ghostlight.external_proposal_acknowledgement.v1".into(),
            campaign_id: campaign.id,
            authority_id: "authority:external-hold".into(),
            owner_id: "fixture-consumer".into(),
            authority_key: authority_key.into(),
            proposal_id: proposal.id,
            idempotency_key: "ack-treaty".into(),
            status: ExternalProposalStatus::Rejected,
            result_summary: "The hold declined.".into(),
            acknowledged_at: Utc::now(),
        };
        let acknowledgement_receipt = registry
            .acknowledge_external_proposal(acknowledgement.clone())
            .await
            .unwrap();
        assert_eq!(
            registry
                .acknowledge_external_proposal(acknowledgement)
                .await
                .unwrap(),
            acknowledgement_receipt
        );
        assert!(
            registry
                .list_external_proposals(ExternalProposalListRequest {
                    schema: "ghostlight.external_proposal_list_request.v1".into(),
                    campaign_id: campaign.id,
                    authority_id: "authority:external-hold".into(),
                    owner_id: "fixture-consumer".into(),
                    authority_key: authority_key.into(),
                })
                .await
                .unwrap()
                .proposals
                .is_empty()
        );
    }
}
