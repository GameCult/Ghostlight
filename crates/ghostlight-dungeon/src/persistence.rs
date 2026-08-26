use crate::consumer::{
    ExternalProposalReceipt, ExternalSnapshotReceipt, ExternalSubjectAuthority,
    ExternalWorldProposal, WorldSeedAdmission, WorldSeedAdmissionReceipt,
};
use crate::domain::{
    Campaign, CampaignLifecycleReceipt, GestaltMaterializationReceipt, ResolutionControlReceipt,
    ResolutionWaveCommit, StrategicTickReceipt, VaultEvidenceReceipt, VaultManifest,
    WorldCommitReceipt,
};
use crate::session_zero::{
    CellBudgetProposal, GroupTravelProposal, PublishedSessionZeroSeed, TimeAdvanceProposal,
};
use crate::transition::{
    ComponentWorldState, MutationAuthorityEnvelope, WorldMutationBatch, WorldMutationReceipt,
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
        let row = self.load_envelope(kind, key)?;
        row.map(|r| {
            let value = rmp_serde::from_slice(&r.payload).context("decode CultCache row")?;
            Ok((r, value))
        })
        .transpose()
    }

    fn load_envelope(&self, kind: &str, key: &str) -> Result<Option<CultCacheEnvelope>> {
        Ok(self
            .inner
            .pull_all()?
            .into_iter()
            .find(|row| row.r#type == kind && row.key == key))
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

    pub fn create_component_world_state(
        &self,
        state: &ComponentWorldState,
    ) -> Result<CultCacheEnvelope> {
        self.insert(
            "component_world_state.v1",
            "ghostlight.component_world_state.v1",
            &state.campaign_id.to_string(),
            state,
        )
    }

    pub fn commit_world_mutation_batch(
        &self,
        expected: &CultCacheEnvelope,
        next: &ComponentWorldState,
        authority: &MutationAuthorityEnvelope,
        batch: &WorldMutationBatch,
        receipt: &WorldMutationReceipt,
    ) -> Result<CultCacheEnvelope> {
        if expected.r#type != "component_world_state.v1"
            || expected.key != next.campaign_id.to_string()
            || authority.campaign_id != next.campaign_id
            || batch.campaign_id != next.campaign_id
            || receipt.campaign_id != next.campaign_id
            || receipt.world_revision != next.revision
            || receipt.previous_world_revision.saturating_add(1) != next.revision
            || receipt.batch_digest != batch.digest
            || receipt.authority_envelope_digest != authority.digest
        {
            return Err(anyhow!("world mutation persistence bundle is inconsistent"));
        }
        let next_row = envelope(
            "component_world_state.v1",
            "ghostlight.component_world_state.v1",
            &next.campaign_id.to_string(),
            next,
        )?;
        let rows = vec![
            next_row.clone(),
            envelope(
                "mutation_authority_envelope.v1",
                "ghostlight.mutation_authority_envelope.v1",
                &authority.id,
                authority,
            )?,
            envelope(
                "world_mutation_batch.v1",
                "ghostlight.world_mutation_batch.v1",
                &batch.id,
                batch,
            )?,
            envelope(
                "world_mutation_receipt.v1",
                "ghostlight.world_mutation_receipt.v1",
                &receipt.id,
                receipt,
            )?,
        ];
        if !self
            .inner
            .compare_and_swap_batch(std::slice::from_ref(expected), rows)?
        {
            return Err(anyhow!("stale component world snapshot"));
        }
        Ok(next_row)
    }

    #[doc(hidden)]
    pub fn create_unadmitted_fixture_campaign(
        &self,
        campaign: &Campaign,
        receipts: &[VaultEvidenceReceipt],
        model_receipts: &[crate::model::ModelStageReceipt],
    ) -> Result<CultCacheEnvelope> {
        self.create_campaign_rows(campaign, receipts, model_receipts, None, None)
    }

    pub fn create_admitted_campaign(
        &self,
        campaign: &Campaign,
        receipts: &[VaultEvidenceReceipt],
        model_receipts: &[crate::model::ModelStageReceipt],
        admission: &WorldSeedAdmission,
        admission_receipt: &WorldSeedAdmissionReceipt,
        session_zero_publication: Option<&PublishedSessionZeroSeed>,
    ) -> Result<CultCacheEnvelope> {
        if admission.campaign_id != campaign.id
            || admission_receipt.campaign_id != campaign.id
            || admission.seed_digest != admission_receipt.seed_digest
            || admission.producer_id != admission_receipt.producer_id
            || admission.idempotency_key != admission_receipt.idempotency_key
        {
            return Err(anyhow!(
                "world seed admission persistence bundle is inconsistent"
            ));
        }
        self.create_campaign_rows(
            campaign,
            receipts,
            model_receipts,
            Some((admission, admission_receipt)),
            session_zero_publication,
        )
    }

    #[cfg(test)]
    pub(crate) fn create_session_zero_campaign(
        &self,
        campaign: &Campaign,
        receipts: &[VaultEvidenceReceipt],
        model_receipts: &[crate::model::ModelStageReceipt],
        publication: &PublishedSessionZeroSeed,
    ) -> Result<CultCacheEnvelope> {
        if publication.membership.campaign_id != campaign.id
            || publication.governance.campaign_id != campaign.id
        {
            return Err(anyhow!("Session Zero publication targets another campaign"));
        }
        self.create_campaign_rows(campaign, receipts, model_receipts, None, Some(publication))
    }

    fn create_campaign_rows(
        &self,
        campaign: &Campaign,
        receipts: &[VaultEvidenceReceipt],
        model_receipts: &[crate::model::ModelStageReceipt],
        admission: Option<(&WorldSeedAdmission, &WorldSeedAdmissionReceipt)>,
        publication: Option<&PublishedSessionZeroSeed>,
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
        if let Some((admission, receipt)) = admission {
            rows.push(envelope(
                "world_seed_admission.v1",
                "ghostlight.world_seed_admission.v1",
                &campaign.id.to_string(),
                admission,
            )?);
            rows.push(envelope(
                "world_seed_admission_receipt.v1",
                "ghostlight.world_seed_admission_receipt.v1",
                &campaign.id.to_string(),
                receipt,
            )?);
            for authority in &admission.external_subjects {
                rows.push(envelope(
                    "external_subject_authority.v1",
                    "ghostlight.external_subject_authority.v1",
                    &authority.id,
                    authority,
                )?);
            }
        }
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
        rows.push(envelope(
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
        )?);
        if let Some(publication) = publication {
            rows.push(envelope(
                "session_zero_publication.v1",
                "ghostlight.published_session_zero_seed.v1",
                &campaign.id.to_string(),
                publication,
            )?);
            rows.push(envelope(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &campaign.id.to_string(),
                &publication.membership,
            )?);
            rows.push(envelope(
                "campaign_contract.v1",
                "ghostlight.campaign_contract.v1",
                &campaign.id.to_string(),
                &publication.contract,
            )?);
            rows.push(envelope(
                "campaign_governance.v1",
                "ghostlight.campaign_governance.v1",
                &campaign.id.to_string(),
                &publication.governance,
            )?);
            rows.push(envelope(
                "campaign_dm_persona.v1",
                "ghostlight.campaign_dm_persona.v1",
                &publication.dm_persona.id,
                &publication.dm_persona,
            )?);
            rows.push(envelope(
                "approved_campaign_brief.v1",
                "ghostlight.approved_campaign_brief.v1",
                &campaign.id.to_string(),
                &publication.approved_brief,
            )?);
            for approval in &publication.approvals {
                rows.push(envelope(
                    "session_zero_approval.v1",
                    "ghostlight.session_zero_approval.v1",
                    &approval.member_id,
                    approval,
                )?);
            }
            for boundary in &publication.boundaries {
                rows.push(envelope(
                    "content_boundary.v1",
                    "ghostlight.content_boundary.v1",
                    &boundary.id,
                    boundary,
                )?);
            }
        }
        if !self.inner.compare_and_swap_batch(&[], rows)? {
            return Err(anyhow!("campaign store is not empty"));
        }
        Ok(campaign_row)
    }

    pub fn append_external_snapshot(
        &self,
        expected_campaign: &CultCacheEnvelope,
        expected_authority: &CultCacheEnvelope,
        next_campaign: &Campaign,
        next_authority: &ExternalSubjectAuthority,
        receipt: &ExternalSnapshotReceipt,
    ) -> Result<CultCacheEnvelope> {
        if expected_campaign.key != next_campaign.id.to_string()
            || expected_authority.key != next_authority.id
            || receipt.campaign_id != next_campaign.id
            || receipt.authority_id != next_authority.id
            || receipt.subject_id != next_authority.subject_id
            || receipt.world_revision != next_campaign.revision
            || receipt.previous_world_revision.saturating_add(1) != next_campaign.revision
        {
            return Err(anyhow!(
                "external snapshot persistence bundle is inconsistent"
            ));
        }
        let next_campaign_row = envelope(
            "campaign.v1",
            "ghostlight.campaign.v1",
            &next_campaign.id.to_string(),
            next_campaign,
        )?;
        let rows = vec![
            next_campaign_row.clone(),
            envelope(
                "external_subject_authority.v1",
                "ghostlight.external_subject_authority.v1",
                &next_authority.id,
                next_authority,
            )?,
            envelope(
                "external_snapshot_receipt.v1",
                "ghostlight.external_snapshot_receipt.v1",
                &format!("{}:{}", receipt.authority_id, receipt.source_revision),
                receipt,
            )?,
        ];
        if !self.inner.compare_and_swap_batch(
            &[expected_campaign.clone(), expected_authority.clone()],
            rows,
        )? {
            return Err(anyhow!("stale external snapshot or authority watermark"));
        }
        Ok(next_campaign_row)
    }

    pub fn append_external_proposals(
        &self,
        rows: &mut Vec<CultCacheEnvelope>,
        proposals: &[ExternalWorldProposal],
    ) -> Result<()> {
        for proposal in proposals {
            rows.push(envelope(
                "external_world_proposal.v1",
                "ghostlight.external_world_proposal.v1",
                &proposal.id,
                proposal,
            )?);
        }
        Ok(())
    }

    pub fn record_external_proposal_receipt(
        &self,
        receipt: &ExternalProposalReceipt,
    ) -> Result<ExternalProposalReceipt> {
        let key = receipt.proposal_id.clone();
        if let Some((_, existing)) =
            self.load::<ExternalProposalReceipt>("external_proposal_receipt.v1", &key)?
        {
            return if existing == *receipt {
                Ok(existing)
            } else {
                Err(anyhow!("external proposal receipt idempotency conflict"))
            };
        }
        self.insert(
            "external_proposal_receipt.v1",
            "ghostlight.external_proposal_receipt.v1",
            &key,
            receipt,
        )?;
        Ok(receipt.clone())
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

    pub fn append_world_transition_with_mutation<T: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        next_schema: &str,
        next: &T,
        receipt_key: &str,
        receipt: &WorldCommitReceipt,
        authority: &MutationAuthorityEnvelope,
        batch: &WorldMutationBatch,
        mutation_receipt: &WorldMutationReceipt,
    ) -> Result<CultCacheEnvelope> {
        if mutation_receipt.previous_world_revision != receipt.previous_revision
            || mutation_receipt.world_revision != receipt.revision
            || mutation_receipt.campaign_id != receipt.campaign_id
            || mutation_receipt.batch_digest != batch.digest
            || mutation_receipt.authority_envelope_digest != authority.digest
        {
            return Err(anyhow!(
                "world and mutation receipts do not describe one transition"
            ));
        }
        let next_row = envelope(&expected.r#type, next_schema, &expected.key, next)?;
        let mut rows = vec![
            next_row.clone(),
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                receipt_key,
                receipt,
            )?,
            envelope(
                "mutation_authority_envelope.v1",
                "ghostlight.mutation_authority_envelope.v1",
                &authority.id,
                authority,
            )?,
            envelope(
                "world_mutation_batch.v1",
                "ghostlight.world_mutation_batch.v1",
                &batch.id,
                batch,
            )?,
            envelope(
                "world_mutation_receipt.v1",
                "ghostlight.world_mutation_receipt.v1",
                &mutation_receipt.id,
                mutation_receipt,
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

    pub fn commit_time_advance(
        &self,
        expected_campaign: &CultCacheEnvelope,
        next_campaign: &Campaign,
        expected_proposal: &CultCacheEnvelope,
        next_proposal: &TimeAdvanceProposal,
        receipt: &WorldCommitReceipt,
        mutation: (
            &MutationAuthorityEnvelope,
            &WorldMutationBatch,
            &WorldMutationReceipt,
        ),
    ) -> Result<CultCacheEnvelope> {
        let next_campaign_row = envelope(
            &expected_campaign.r#type,
            "ghostlight.campaign.v1",
            &expected_campaign.key,
            next_campaign,
        )?;
        let next_proposal_row = envelope(
            &expected_proposal.r#type,
            "ghostlight.time_advance_proposal.v1",
            &expected_proposal.key,
            next_proposal,
        )?;
        let mut rows = vec![
            next_campaign_row.clone(),
            next_proposal_row,
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                &format!("{}-{}", next_campaign.id, next_campaign.revision),
                receipt,
            )?,
        ];
        append_mutation_proof(&mut rows, receipt, mutation)?;
        if !self.inner.compare_and_swap_batch(
            &[expected_campaign.clone(), expected_proposal.clone()],
            rows,
        )? {
            return Err(anyhow!("stale campaign or time-advance proposal"));
        }
        Ok(next_campaign_row)
    }

    pub fn commit_group_travel(
        &self,
        expected_campaign: &CultCacheEnvelope,
        next_campaign: &Campaign,
        expected_proposal: &CultCacheEnvelope,
        next_proposal: &GroupTravelProposal,
        receipt: &WorldCommitReceipt,
        mutation: (
            &MutationAuthorityEnvelope,
            &WorldMutationBatch,
            &WorldMutationReceipt,
        ),
    ) -> Result<CultCacheEnvelope> {
        let next_campaign_row = envelope(
            &expected_campaign.r#type,
            "ghostlight.campaign.v1",
            &expected_campaign.key,
            next_campaign,
        )?;
        let next_proposal_row = envelope(
            &expected_proposal.r#type,
            "ghostlight.group_travel_proposal.v1",
            &expected_proposal.key,
            next_proposal,
        )?;
        let mut rows = vec![
            next_campaign_row.clone(),
            next_proposal_row,
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                &format!("{}-{}", next_campaign.id, next_campaign.revision),
                receipt,
            )?,
        ];
        append_mutation_proof(&mut rows, receipt, mutation)?;
        if !self.inner.compare_and_swap_batch(
            &[expected_campaign.clone(), expected_proposal.clone()],
            rows,
        )? {
            return Err(anyhow!("stale campaign or group-travel proposal"));
        }
        Ok(next_campaign_row)
    }

    pub fn commit_cell_budget(
        &self,
        expected_campaign: &CultCacheEnvelope,
        next_campaign: &Campaign,
        expected_proposal: &CultCacheEnvelope,
        next_proposal: &CellBudgetProposal,
        receipt: &ResolutionControlReceipt,
    ) -> Result<CultCacheEnvelope> {
        let next_campaign_row = envelope(
            &expected_campaign.r#type,
            "ghostlight.campaign.v1",
            &expected_campaign.key,
            next_campaign,
        )?;
        let rows = vec![
            next_campaign_row.clone(),
            envelope(
                &expected_proposal.r#type,
                "ghostlight.cell_budget_proposal.v1",
                &expected_proposal.key,
                next_proposal,
            )?,
            envelope(
                "resolution_control_receipt.v1",
                "ghostlight.resolution_control_receipt.v1",
                &format!(
                    "{}:{}:{}",
                    receipt.campaign_id, receipt.operation, receipt.resolution_epoch
                ),
                receipt,
            )?,
        ];
        if !self.inner.compare_and_swap_batch(
            &[expected_campaign.clone(), expected_proposal.clone()],
            rows,
        )? {
            return Err(anyhow!("stale campaign or cell-budget proposal"));
        }
        Ok(next_campaign_row)
    }

    pub fn commit_contract_review(
        &self,
        expected_campaign: &CultCacheEnvelope,
        next_campaign: &Campaign,
        publication: &PublishedSessionZeroSeed,
        receipt: &WorldCommitReceipt,
    ) -> Result<CultCacheEnvelope> {
        let campaign_key = next_campaign.id.to_string();
        let required = [
            "session_zero_publication.v1",
            "campaign_contract.v1",
            "campaign_membership.v1",
            "campaign_governance.v1",
            "approved_campaign_brief.v1",
        ];
        let mut expected_rows = vec![expected_campaign.clone()];
        for kind in required {
            let row = self
                .load_envelope(kind, &campaign_key)?
                .ok_or_else(|| anyhow!("contract review requires {kind}"))?;
            expected_rows.push(row);
        }
        let dm_row = self
            .load_envelope("campaign_dm_persona.v1", &publication.dm_persona.id)?
            .ok_or_else(|| anyhow!("contract review requires campaign DM state"))?;
        expected_rows.push(dm_row);

        let next_campaign_row = envelope(
            &expected_campaign.r#type,
            "ghostlight.campaign.v1",
            &expected_campaign.key,
            next_campaign,
        )?;
        let mut rows = vec![
            next_campaign_row.clone(),
            envelope(
                "session_zero_publication.v1",
                "ghostlight.published_session_zero_seed.v1",
                &campaign_key,
                publication,
            )?,
            envelope(
                "campaign_contract.v1",
                "ghostlight.campaign_contract.v1",
                &campaign_key,
                &publication.contract,
            )?,
            envelope(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &campaign_key,
                &publication.membership,
            )?,
            envelope(
                "campaign_governance.v1",
                "ghostlight.campaign_governance.v1",
                &campaign_key,
                &publication.governance,
            )?,
            envelope(
                "campaign_dm_persona.v1",
                "ghostlight.campaign_dm_persona.v1",
                &publication.dm_persona.id,
                &publication.dm_persona,
            )?,
            envelope(
                "approved_campaign_brief.v1",
                "ghostlight.approved_campaign_brief.v1",
                &campaign_key,
                &publication.approved_brief,
            )?,
            envelope(
                "world_commit_receipt.v1",
                "ghostlight.world_commit_receipt.v1",
                &format!("{}-{}", next_campaign.id, next_campaign.revision),
                receipt,
            )?,
        ];
        for approval in &publication.approvals {
            rows.push(envelope(
                "session_zero_approval.v1",
                "ghostlight.session_zero_approval.v1",
                &format!("{}:{}", publication.session_zero_id, approval.member_id),
                approval,
            )?);
        }
        if !self.inner.compare_and_swap_batch(&expected_rows, rows)? {
            return Err(anyhow!("stale campaign contract review"));
        }
        Ok(next_campaign_row)
    }

    pub fn append_world_commit<T: Serialize>(
        &self,
        expected: &CultCacheEnvelope,
        next: &T,
        receipt_key: &str,
        receipt: &WorldCommitReceipt,
        evidence: &[VaultEvidenceReceipt],
        candidates: &[crate::domain::CanonCandidate],
        model_receipts: &[crate::model::ModelStageReceipt],
        mutation: Option<(
            &MutationAuthorityEnvelope,
            &WorldMutationBatch,
            &WorldMutationReceipt,
        )>,
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
        let mut expected_rows = vec![expected.clone()];
        if let Some((row, _)) = existing_manifest {
            expected_rows.push(row);
        }
        for item in evidence {
            self.append_idempotent_companion(
                &mut expected_rows,
                &mut rows,
                envelope(
                    "vault_evidence_receipt.v1",
                    "ghostlight.vault_evidence_receipt.v1",
                    &item.id,
                    item,
                )?,
            )?;
        }
        for item in candidates {
            self.append_idempotent_companion(
                &mut expected_rows,
                &mut rows,
                envelope(
                    "canon_candidate.v1",
                    "ghostlight.canon_candidate.v1",
                    &item.id,
                    item,
                )?,
            )?;
        }
        for item in model_receipts {
            self.append_idempotent_companion(
                &mut expected_rows,
                &mut rows,
                envelope(
                    "persona_stage_receipt.v1",
                    "ghostlight.persona_stage_receipt.v1",
                    item.storage_key(),
                    item,
                )?,
            )?;
        }
        if let Some(mutation) = mutation {
            append_mutation_proof(&mut rows, receipt, mutation)?;
        }
        if !self.inner.compare_and_swap_batch(&expected_rows, rows)? {
            return Err(anyhow!("stale CultCache snapshot"));
        }
        Ok(next_row)
    }

    fn append_idempotent_companion(
        &self,
        expected: &mut Vec<CultCacheEnvelope>,
        replacements: &mut Vec<CultCacheEnvelope>,
        row: CultCacheEnvelope,
    ) -> Result<()> {
        if let Some(existing) = self.load_envelope(&row.r#type, &row.key)? {
            if existing != row {
                return Err(anyhow!(
                    "immutable world-commit companion conflict: {}/{}",
                    row.r#type,
                    row.key
                ));
            }
            expected.push(existing);
        }
        replacements.push(row);
        Ok(())
    }

    pub fn append_strategic_tick(
        &self,
        expected: &CultCacheEnvelope,
        next: &Campaign,
        receipt_key: &str,
        world_receipt: &WorldCommitReceipt,
        strategic_receipt: &StrategicTickReceipt,
        resolution_wave: Option<&ResolutionWaveCommit>,
        external_proposals: &[ExternalWorldProposal],
        mutation: Option<(
            &MutationAuthorityEnvelope,
            &WorldMutationBatch,
            &WorldMutationReceipt,
        )>,
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
            for outcome in &wave.activity_outcomes {
                rows.push(envelope(
                    "strategic_activity_outcome.v1",
                    "ghostlight.strategic_activity_outcome.v1",
                    &format!(
                        "{}:{}:{}",
                        next.revision, wave.resolution_epoch, outcome.action_digest
                    ),
                    outcome,
                )?);
            }
            for individuation in &wave.strategic_individuations {
                rows.push(envelope(
                    "strategic_gestalt_individuation.v1",
                    "ghostlight.strategic_gestalt_individuation.v1",
                    &format!(
                        "{}:{}:{}",
                        next.revision,
                        wave.resolution_epoch,
                        crate::domain::canonical_gestalt_member_local_id(
                            &individuation.individuation.member.id,
                        )
                    ),
                    individuation,
                )?);
            }
        }
        self.append_external_proposals(&mut rows, external_proposals)?;
        if let Some((authority, batch, mutation_receipt)) = mutation {
            if mutation_receipt.previous_world_revision != world_receipt.previous_revision
                || mutation_receipt.world_revision != world_receipt.revision
                || mutation_receipt.batch_digest != batch.digest
                || mutation_receipt.authority_envelope_digest != authority.digest
            {
                return Err(anyhow!(
                    "strategic and mutation receipts do not describe one transition"
                ));
            }
            rows.push(envelope(
                "mutation_authority_envelope.v1",
                "ghostlight.mutation_authority_envelope.v1",
                &authority.id,
                authority,
            )?);
            rows.push(envelope(
                "world_mutation_batch.v1",
                "ghostlight.world_mutation_batch.v1",
                &batch.id,
                batch,
            )?);
            rows.push(envelope(
                "world_mutation_receipt.v1",
                "ghostlight.world_mutation_receipt.v1",
                &mutation_receipt.id,
                mutation_receipt,
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

fn append_mutation_proof(
    rows: &mut Vec<CultCacheEnvelope>,
    world_receipt: &WorldCommitReceipt,
    (authority, batch, mutation_receipt): (
        &MutationAuthorityEnvelope,
        &WorldMutationBatch,
        &WorldMutationReceipt,
    ),
) -> Result<()> {
    if mutation_receipt.previous_world_revision != world_receipt.previous_revision
        || mutation_receipt.world_revision != world_receipt.revision
        || mutation_receipt.campaign_id != world_receipt.campaign_id
        || mutation_receipt.batch_digest != batch.digest
        || mutation_receipt.authority_envelope_digest != authority.digest
    {
        return Err(anyhow!(
            "world and mutation receipts do not describe one transition"
        ));
    }
    rows.push(envelope(
        "mutation_authority_envelope.v1",
        "ghostlight.mutation_authority_envelope.v1",
        &authority.id,
        authority,
    )?);
    rows.push(envelope(
        "world_mutation_batch.v1",
        "ghostlight.world_mutation_batch.v1",
        &batch.id,
        batch,
    )?);
    rows.push(envelope(
        "world_mutation_receipt.v1",
        "ghostlight.world_mutation_receipt.v1",
        &mutation_receipt.id,
        mutation_receipt,
    )?);
    Ok(())
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
