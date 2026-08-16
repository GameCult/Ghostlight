use crate::domain::{Campaign, VaultEvidenceReceipt};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

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
    ) -> Result<CultCacheEnvelope> {
        let campaign_row = envelope(
            "campaign.v1",
            "ghostlight.campaign.v1",
            &campaign.id.to_string(),
            campaign,
        )?;
        let mut rows = vec![campaign_row.clone()];
        for receipt in receipts {
            rows.push(envelope(
                "vault_evidence_receipt.v1",
                "ghostlight.vault_evidence_receipt.v1",
                &receipt.id,
                receipt,
            )?);
        }
        if !self.inner.compare_and_swap_batch(&[], rows)? {
            return Err(anyhow!("campaign store is not empty"));
        }
        Ok(campaign_row)
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

    pub fn append_with_replace<T: Serialize, R: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        next_schema: &str,
        next: &T,
        receipt_kind: &str,
        receipt_schema: &str,
        receipt_key: &str,
        receipt: &R,
    ) -> Result<CultCacheEnvelope> {
        let next_row = envelope(&expected.r#type, next_schema, &expected.key, next)?;
        let receipt_row = envelope(receipt_kind, receipt_schema, receipt_key, receipt)?;
        if !self.inner.compare_and_swap_batch(
            std::slice::from_ref(expected),
            vec![next_row.clone(), receipt_row],
        )? {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
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
