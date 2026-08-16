use crate::{
    domain::{Campaign, CampaignLifecycleReceipt, VaultEvidenceReceipt},
    kernel::WorldKernel,
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
            let path = entry.path().join("campaign.cc");
            if !path.is_file() {
                continue;
            }
            let store = CampaignStore::open(path)?;
            let keys = store.keys("campaign.v1")?;
            if keys.len() != 1 {
                return Err(anyhow!("campaign store must contain exactly one campaign"));
            }
            let id = Uuid::parse_str(&keys[0])?;
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
        campaign: Campaign,
        evidence: Vec<VaultEvidenceReceipt>,
    ) -> Result<CampaignRuntime> {
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
        let parent_revision = parent.revision;
        let mut fork = parent;
        fork.id = Uuid::new_v4();
        fork.name = name;
        fork.revision = 0;
        fork.pending_world_proposals.clear();
        let runtime = self.create(fork.clone(), evidence).await?;
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
        let runtime = self.create(seed.clone(), evidence).await?;
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
        }
    }

    #[tokio::test]
    async fn create_fork_reset_and_export_preserve_isolated_stores() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let original = seed("Original");
        registry.create(original.clone(), vec![]).await.unwrap();
        let fork = registry.fork(original.id, "Fork".into()).await.unwrap();
        let fork_id = fork.store.keys("campaign.v1").unwrap()[0]
            .parse::<Uuid>()
            .unwrap();
        assert_ne!(fork_id, original.id);
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
}
