use crate::domain::{
    Campaign, GestaltMaterializationReceipt, ResolutionControlReceipt, ResolutionWaveCommit,
    StrategicTickReceipt, VaultEvidenceReceipt, VaultManifest, WorldCommitReceipt,
};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use cultcache_legacy::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone)]
pub struct CampaignStore {
    inner: OwnedRedbMessagePackBackingStore,
}

impl CampaignStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            inner: OwnedRedbMessagePackBackingStore::new(path.as_ref())?,
        })
    }
    pub fn identity(&self) -> &str {
        self.inner.file_identity()
    }

    pub fn keys(&self, kind: &str) -> Result<Vec<String>> {
        Ok(self
            .inner
            .pull_all()?
            .into_iter()
            .filter(|r| r.r#type == kind)
            .map(|r| r.key)
            .collect())
    }

    pub fn load_all<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        self.inner
            .pull_all()?
            .into_iter()
            .filter(|row| row.r#type == kind)
            .map(|row| rmp_serde::from_slice(&row.payload).context("decode CultCache row"))
            .collect()
    }

    pub fn load<T: DeserializeOwned>(
        &self,
        kind: &str,
        key: &str,
    ) -> Result<Option<(CultCacheEnvelope, T)>> {
        let row = self
            .inner
            .pull_all()?
            .into_iter()
            .find(|r| r.r#type == kind && r.key == key);
        row.map(|r| {
            let value = rmp_serde::from_slice(&r.payload).context("decode CultCache row")?;
            Ok((r, value))
        })
        .transpose()
    }

    pub fn insert<T: Serialize>(
        &self,
        kind: &str,
        schema: &str,
        key: &str,
        value: &T,
    ) -> Result<CultCacheEnvelope> {
        let row = envelope(kind, schema, key, value)?;
        if !self.inner.insert_entry_if_absent(row.clone())? {
            return Err(anyhow!("row already exists: {kind}/{key}"));
        }
        Ok(row)
    }

    pub fn create_campaign(
        &self,
        campaign: &Campaign,
        receipts: &[VaultEvidenceReceipt],
        model_receipts: &[crate::model::ModelStageReceipt],
    ) -> Result<CultCacheEnvelope> {
        let campaign_row = envelope(
            "campaign.v1",
            "ghostlight.campaign.v1",
            &campaign.id.to_string(),
            campaign,
        )?;
        let mut rows = vec![campaign_row.clone()];
        rows.push(envelope(
            "campaign_seed.v1",
            "ghostlight.campaign.v1",
            &campaign.id.to_string(),
            campaign,
        )?);
        let manifest = merge_vault_manifest(None, receipts);
        rows.push(envelope(
            "vault_manifest.v1",
            "ghostlight.vault_manifest.v1",
            &campaign.id.to_string(),
            &manifest,
        )?);
        for receipt in receipts {
            rows.push(envelope(
                "vault_evidence_receipt.v1",
                "ghostlight.vault_evidence_receipt.v1",
                &receipt.id,
                receipt,
            )?);
        }
        for receipt in model_receipts {
            rows.push(envelope(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                receipt.storage_key(),
                receipt,
            )?);
        }
        for candidate in campaign.canon_candidates.values() {
            rows.push(envelope(
                "canon_candidate.v1",
                "ghostlight.canon_candidate.v1",
                &candidate.id,
                candidate,
            )?);
        }
        if !self.inner.compare_and_swap_batch(&[], rows)? {
            return Err(anyhow!("campaign store is not empty"));
        }
        Ok(campaign_row)
    }

    pub fn snapshot_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let rows = self.inner.pull_all()?;
        let target = OwnedRedbMessagePackBackingStore::new(path.as_ref())?;
        if !target.compare_and_swap_batch(&[], rows)? {
            return Err(anyhow!("export target is not empty"));
        }
        Ok(())
    }

    pub fn replace<T: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        schema: &str,
        value: &T,
    ) -> Result<CultCacheEnvelope> {
        let next = envelope(&expected.r#type, schema, &expected.key, value)?;
        if !self.inner.compare_and_swap_entry(expected, next.clone())? {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next)
    }

    pub fn append_world_transition<T: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        next_schema: &str,
        next: &T,
        receipt_key: &str,
        receipt: &WorldCommitReceipt,
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(&expected.r#type, next_schema, &expected.key, next)?;
        let mut rows = vec![
            next_row.clone(),
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                receipt_key,
                receipt,
            )?,
        ];
        if let Some(roll) = &receipt.roll {
            rows.push(envelope(
                "roll_receipt.v1",
                "ghostlight.roll_receipt.v1",
                &roll.assessment_digest,
                roll,
            )?);
        }
        if !self
            .inner
            .compare_and_swap_batch(std::slice::from_ref(expected), rows)?
        {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }

    pub fn append_world_commit<T: Serialize, R: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        next: &T,
        receipt_key: &str,
        receipt: &R,
        evidence: &[VaultEvidenceReceipt],
        candidates: &[crate::domain::CanonCandidate],
        model_receipts: &[crate::model::ModelStageReceipt],
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(
            &expected.r#type,
            "ghostlight.campaign.v1",
            &expected.key,
            next,
        )?;
        let mut rows = vec![
            next_row.clone(),
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                receipt_key,
                receipt,
            )?,
        ];
        let existing_manifest = self.load::<VaultManifest>("vault_manifest.v1", &expected.key)?;
        let manifest =
            merge_vault_manifest(existing_manifest.as_ref().map(|(_, value)| value), evidence);
        rows.push(envelope(
            "vault_manifest.v1",
            "ghostlight.vault_manifest.v1",
            &expected.key,
            &manifest,
        )?);
        for item in evidence {
            rows.push(envelope(
                "vault_evidence_receipt.v1",
                "ghostlight.vault_evidence_receipt.v1",
                &item.id,
                item,
            )?);
        }
        for item in candidates {
            rows.push(envelope(
                "canon_candidate.v1",
                "ghostlight.canon_candidate.v1",
                &item.id,
                item,
            )?);
        }
        for item in model_receipts {
            rows.push(envelope(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                item.storage_key(),
                item,
            )?);
        }
        let mut expected_rows = vec![expected.clone()];
        if let Some((row, _)) = existing_manifest {
            expected_rows.push(row);
        }
        if !self.inner.compare_and_swap_batch(&expected_rows, rows)? {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }

    pub fn append_strategic_tick(
        &self,
        expected: &CultCacheEnvelope,
        next: &Campaign,
        receipt_key: &str,
        world_receipt: &WorldCommitReceipt,
        strategic_receipt: &StrategicTickReceipt,
        resolution_wave: Option<&ResolutionWaveCommit>,
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(
            &expected.r#type,
            "ghostlight.campaign.v1",
            &expected.key,
            next,
        )?;
        let mut rows = vec![
            next_row.clone(),
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                receipt_key,
                world_receipt,
            )?,
            envelope(
                "strategic_tick.v1",
                "ghostlight.strategic_tick.v1",
                receipt_key,
                strategic_receipt,
            )?,
        ];
        if let Some(wave) = resolution_wave {
            let key = format!(
                "{}:{}:{}",
                next.id, wave.world_revision, wave.resolution_epoch
            );
            rows.push(envelope(
                "resolution_cover.v1",
                "ghostlight.resolution_cover.v1",
                &key,
                &wave.cover,
            )?);
            rows.push(envelope(
                "resolution_plan_receipt.v1",
                "ghostlight.resolution_plan_receipt.v1",
                &key,
                &wave.plan_receipt,
            )?);
            for appraisal in &wave.appraisals {
                rows.push(envelope(
                    "cell_appraisal.v1",
                    "ghostlight.cell_appraisal.v1",
                    &format!(
                        "{}:{}:{}",
                        next.revision, wave.resolution_epoch, appraisal.cell_id
                    ),
                    appraisal,
                )?);
            }
        }
        if !self
            .inner
            .compare_and_swap_batch(std::slice::from_ref(expected), rows)?
        {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }

    pub fn append_resolution_control(
        &self,
        expected: &CultCacheEnvelope,
        next: &Campaign,
        receipt: &ResolutionControlReceipt,
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(
            &expected.r#type,
            "ghostlight.campaign.v1",
            &expected.key,
            next,
        )?;
        let rows = vec![
            next_row.clone(),
            envelope(
                "resolution_control_receipt.v1",
                "ghostlight.resolution_control_receipt.v1",
                &format!("{}:{}", next.id, receipt.resolution_epoch),
                receipt,
            )?,
        ];
        if !self
            .inner
            .compare_and_swap_batch(std::slice::from_ref(expected), rows)?
        {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }

    pub fn append_gestalt_presence(
        &self,
        expected: &CultCacheEnvelope,
        next: &Campaign,
        receipt_key: &str,
        world_receipt: &WorldCommitReceipt,
        gestalt_receipt: &GestaltMaterializationReceipt,
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(
            &expected.r#type,
            "ghostlight.campaign.v1",
            &expected.key,
            next,
        )?;
        let rows = vec![
            next_row.clone(),
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                receipt_key,
                world_receipt,
            )?,
            envelope(
                "gestalt_materialization_receipt.v1",
                "ghostlight.gestalt_materialization_receipt.v1",
                receipt_key,
                gestalt_receipt,
            )?,
        ];
        if !self
            .inner
            .compare_and_swap_batch(std::slice::from_ref(expected), rows)?
        {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }
}

fn merge_vault_manifest(
    existing: Option<&VaultManifest>,
    receipts: &[VaultEvidenceReceipt],
) -> VaultManifest {
    let mut providers = existing
        .map(|manifest| BTreeSet::from([manifest.provider.clone()]))
        .unwrap_or_default();
    let mut source_ids = existing
        .map(|manifest| manifest.source_ids.clone())
        .unwrap_or_default();
    let mut authority_lanes = existing
        .map(|manifest| manifest.authority_lanes.clone())
        .unwrap_or_default();
    let mut temporal_scopes = existing
        .map(|manifest| manifest.temporal_scopes.clone())
        .unwrap_or_default();
    for receipt in receipts {
        providers.insert(receipt.provider.clone());
        for witness in &receipt.witnesses {
            source_ids.insert(witness.source_id.clone());
            authority_lanes.insert(witness.authority_lane.clone());
            temporal_scopes.insert(witness.temporal_scope.clone());
        }
    }
    VaultManifest {
        schema: "ghostlight.vault_manifest.v1".into(),
        provider: if providers.is_empty() {
            "none".into()
        } else {
            providers.into_iter().collect::<Vec<_>>().join("+")
        },
        source_ids,
        authority_lanes,
        temporal_scopes,
    }
}

fn envelope<T: Serialize>(
    kind: &str,
    schema: &str,
    key: &str,
    value: &T,
) -> Result<CultCacheEnvelope> {
    Ok(CultCacheEnvelope {
        key: key.into(),
        r#type: kind.into(),
        payload: rmp_serde::to_vec_named(value)?,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(schema.into()),
    })
}
