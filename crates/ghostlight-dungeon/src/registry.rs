use crate::{
    domain::{Campaign, CampaignLifecycleReceipt, VaultEvidenceReceipt},
    kernel::WorldKernel,
    model::ModelStageReceipt,
    persistence::CampaignStore,
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
                        operation: "migrate_flat_agency_state".into(),
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

    pub async fn create(
        &self,
        mut campaign: Campaign,
        evidence: Vec<VaultEvidenceReceipt>,
        model_receipts: Vec<ModelStageReceipt>,
    ) -> Result<CampaignRuntime> {
        crate::resolution::ensure_agency_profiles(&mut campaign);
        if self.runtimes.read().await.contains_key(&campaign.id) {
            return Err(anyhow!("campaign already exists"));
        }
        let directory = self.root.join(campaign.id.to_string());
        fs::create_dir(&directory)?;
        let store = CampaignStore::open(directory.join("campaign.cc"))?;
        let kernel = WorldKernel::start(store.clone());
        kernel
            .command(crate::domain::WorldCommand::CreateCampaign {
                campaign: campaign.clone(),
                evidence_receipts: evidence,
                model_stage_receipts: model_receipts,
            })
            .await?;
        store.insert(
            "campaign_lifecycle_receipt.v1",
            "ghostlight.campaign_lifecycle_receipt.v1",
            &format!("{}:create", campaign.id),
            &CampaignLifecycleReceipt {
                schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                campaign_id: campaign.id,
                operation: "create".into(),
                parent_campaign_id: None,
                parent_revision: None,
                created_at: Utc::now(),
            },
        )?;
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
        let runtime = self.create(fork.clone(), evidence, model_receipts).await?;
        runtime.store.insert(
            "campaign_lifecycle_receipt.v1",
            "ghostlight.campaign_lifecycle_receipt.v1",
            &format!("{}:fork", fork.id),
            &CampaignLifecycleReceipt {
                schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                campaign_id: fork.id,
                operation: "fork".into(),
                parent_campaign_id: Some(source_id),
                parent_revision: Some(parent_revision),
                created_at: Utc::now(),
            },
        )?;
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
        let runtime = self.create(seed.clone(), evidence, model_receipts).await?;
        runtime.store.insert(
            "campaign_lifecycle_receipt.v1",
            "ghostlight.campaign_lifecycle_receipt.v1",
            &format!("{}:reset", seed.id),
            &CampaignLifecycleReceipt {
                schema: "ghostlight.campaign_lifecycle_receipt.v1".into(),
                campaign_id: seed.id,
                operation: "reset".into(),
                parent_campaign_id: Some(source_id),
                parent_revision: Some(parent_revision),
                created_at: Utc::now(),
            },
        )?;
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
            "{}-{}.cc",
            campaign_id,
            Utc::now().format("%Y%m%dT%H%M%SZ")
        ));
        runtime.store.snapshot_to(&path)?;
        Ok(path)
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
    use crate::domain::{ActorState, BranchOrigin, Location};
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
            .create(original.clone(), vec![], vec![model_receipt.clone()])
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
        let reset = registry.reset(original.id, "Reset".into()).await.unwrap();
        let reset_id = reset.store.keys("campaign.v1").unwrap()[0]
            .parse::<Uuid>()
            .unwrap();
        assert_ne!(reset_id, original.id);
        assert_eq!(registry.list().await.len(), 3);
        let exported = registry
            .export(fork_id, dir.path().join("exports"))
            .await
            .unwrap();
        let export_store = CampaignStore::open(exported).unwrap();
        assert_eq!(
            export_store.keys("campaign.v1").unwrap(),
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
            .create(original.clone(), vec![], vec![])
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
    async fn load_existing_migrates_flat_campaign_without_advancing_world_revision() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("campaigns");
        let flat = seed("Flat legacy");
        let campaign_root = root.join(flat.id.to_string());
        std::fs::create_dir_all(&campaign_root).unwrap();
        let store = CampaignStore::open(campaign_root.join("campaign.cc")).unwrap();
        store.create_campaign(&flat, &[], &[]).unwrap();
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
}
