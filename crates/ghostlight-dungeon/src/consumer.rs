use crate::domain::{
    ActorState, AgencyProfile, AgencyRelation, AgencySubjectKind, BranchOrigin, Campaign,
    CanonCandidate, CellActionProposal, GestaltLineage, GestaltMemberDelta, GestaltPersonaState,
    InstitutionState, Location, ResolutionPolicy, StrategicCellEffect, WorldClock, WorldFact,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const WORLD_CONSUMER_SERVICE_ID: &str = "ghostlight.world.consumer";
pub const ADMIT_WORLD_OPERATION: &str = "ghostlight.world.seed.admit";
pub const APPLY_EXTERNAL_SNAPSHOT_OPERATION: &str = "ghostlight.world.external.snapshot.apply";
pub const LIST_EXTERNAL_PROPOSALS_OPERATION: &str = "ghostlight.world.external.proposals.list";
pub const ACKNOWLEDGE_EXTERNAL_PROPOSAL_OPERATION: &str =
    "ghostlight.world.external.proposal.acknowledge";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorldSeedProducerKind {
    Compiler,
    Consumer,
}

/// Initial canonical world state before runtime-only revision, transcript,
/// scheduling, and cover state exist.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldSeed {
    pub schema: String,
    pub id: Uuid,
    pub name: String,
    pub branch_origin: BranchOrigin,
    pub world_time: DateTime<Utc>,
    pub tick_hours: u32,
    /// One subject protected from Ghostlight agency. Dungeon supplies its host
    /// actor. A world consumer may supply an externally owned institution.
    pub primary_controlled_subject_id: String,
    pub locations: BTreeMap<String, Location>,
    pub actors: BTreeMap<String, ActorState>,
    pub institutions: BTreeMap<String, InstitutionState>,
    pub clocks: BTreeMap<String, WorldClock>,
    pub facts: BTreeMap<String, WorldFact>,
    #[serde(default)]
    pub canon_candidates: BTreeMap<String, CanonCandidate>,
    #[serde(default)]
    pub gestalts: BTreeMap<String, GestaltPersonaState>,
    #[serde(default)]
    pub gestalt_members: BTreeMap<String, GestaltMemberDelta>,
    #[serde(default)]
    pub agency_profiles: BTreeMap<String, AgencyProfile>,
    #[serde(default)]
    pub agency_relations: BTreeMap<String, AgencyRelation>,
    #[serde(default)]
    pub gestalt_lineages: BTreeMap<String, GestaltLineage>,
    #[serde(default)]
    pub resolution_policy: ResolutionPolicy,
}

impl WorldSeed {
    pub fn from_campaign(campaign: &Campaign) -> Result<Self> {
        if campaign.revision != 0
            || !campaign.transcript.is_empty()
            || !campaign.events.is_empty()
            || !campaign.news.is_empty()
            || !campaign.pending_world_proposals.is_empty()
            || campaign.pending_ticks != 0
            || campaign.away_ticks_processed != 0
            || !campaign.resolution_pins.is_empty()
            || campaign.resolution_cover.is_some()
            || campaign.strategic_tick_count != 0
        {
            return Err(anyhow!(
                "world seed cannot contain already-authoritative runtime state"
            ));
        }
        Ok(Self {
            schema: "ghostlight.world_seed.v1".into(),
            id: campaign.id,
            name: campaign.name.clone(),
            branch_origin: campaign.branch_origin.clone(),
            world_time: campaign.world_time,
            tick_hours: campaign.tick_hours,
            primary_controlled_subject_id: campaign.player_actor_id.clone(),
            locations: campaign.locations.clone(),
            actors: campaign.actors.clone(),
            institutions: campaign.institutions.clone(),
            clocks: campaign.clocks.clone(),
            facts: campaign.facts.clone(),
            canon_candidates: campaign.canon_candidates.clone(),
            gestalts: campaign.gestalts.clone(),
            gestalt_members: campaign.gestalt_members.clone(),
            agency_profiles: campaign.agency_profiles.clone(),
            agency_relations: campaign.agency_relations.clone(),
            gestalt_lineages: campaign.gestalt_lineages.clone(),
            resolution_policy: campaign.resolution_policy.clone(),
        })
    }

    pub fn into_campaign(self) -> Campaign {
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: self.id,
            name: self.name,
            revision: 0,
            branch_origin: self.branch_origin,
            world_time: self.world_time,
            tick_hours: self.tick_hours,
            player_actor_id: self.primary_controlled_subject_id,
            locations: self.locations,
            actors: self.actors,
            institutions: self.institutions,
            clocks: self.clocks,
            facts: self.facts,
            transcript: Vec::new(),
            last_player_activity: self.world_time,
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: Vec::new(),
            news: Vec::new(),
            canon_candidates: self.canon_candidates,
            gestalts: self.gestalts,
            gestalt_members: self.gestalt_members,
            pending_world_proposals: Vec::new(),
            agency_profiles: self.agency_profiles,
            agency_relations: self.agency_relations,
            gestalt_lineages: self.gestalt_lineages,
            resolution_policy: self.resolution_policy,
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        }
    }

    pub fn digest(&self) -> Result<String> {
        crate::legacy_transition::digest_serializable(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExternalSubjectAuthority {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub subject_id: String,
    pub subject_kind: AgencySubjectKind,
    pub owner_id: String,
    pub authority_key_sha256: String,
    #[serde(default)]
    pub last_source_revision: Option<u64>,
    #[serde(default)]
    pub last_payload_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldSeedAdmission {
    pub schema: String,
    pub campaign_id: Uuid,
    pub producer_id: String,
    pub producer_kind: WorldSeedProducerKind,
    pub seed_digest: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub external_subjects: Vec<ExternalSubjectAuthority>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldSeedAdmissionReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub producer_id: String,
    pub seed_digest: String,
    pub idempotency_key: String,
    pub admitted_subject_ids: Vec<String>,
    pub admitted_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldSeedAdmissionRequest {
    pub schema: String,
    pub seed: WorldSeed,
    pub admission: WorldSeedAdmission,
    /// Proves possession of every external authority declared in this request.
    /// The raw value is never persisted.
    pub authority_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExternalInstitutionSnapshot {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub authority_id: String,
    pub owner_id: String,
    pub authority_key: String,
    pub source_revision: u64,
    pub idempotency_key: String,
    pub payload_digest: String,
    pub projection: InstitutionState,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExternalSnapshotReceipt {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub authority_id: String,
    pub subject_id: String,
    pub owner_id: String,
    pub source_revision: u64,
    pub payload_digest: String,
    pub previous_world_revision: u64,
    pub world_revision: u64,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalProposalStatus {
    Pending,
    Accepted,
    PartiallyAccepted,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExternalWorldProposal {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub world_revision: u64,
    pub authority_id: String,
    pub external_subject_id: String,
    pub source_subject_id: String,
    pub intent: String,
    pub intended_effect: String,
    pub action_digest: String,
    pub public_channels: Vec<String>,
    pub state_references: Vec<String>,
    pub status: ExternalProposalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExternalProposalListRequest {
    pub schema: String,
    pub campaign_id: Uuid,
    pub authority_id: String,
    pub owner_id: String,
    pub authority_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExternalProposalList {
    pub schema: String,
    pub campaign_id: Uuid,
    pub authority_id: String,
    pub proposals: Vec<ExternalWorldProposal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExternalProposalAcknowledgement {
    pub schema: String,
    pub campaign_id: Uuid,
    pub authority_id: String,
    pub owner_id: String,
    pub authority_key: String,
    pub proposal_id: String,
    pub idempotency_key: String,
    pub status: ExternalProposalStatus,
    pub result_summary: String,
    pub acknowledged_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExternalProposalReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub authority_id: String,
    pub proposal_id: String,
    pub idempotency_key: String,
    pub status: ExternalProposalStatus,
    pub result_summary: String,
    pub acknowledged_at: DateTime<Utc>,
}

pub fn authority_key_digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn validate_authority_key(authority: &ExternalSubjectAuthority, supplied: &str) -> Result<()> {
    if supplied.is_empty() || authority_key_digest(supplied) != authority.authority_key_sha256 {
        return Err(anyhow!("external subject authority proof is invalid"));
    }
    Ok(())
}

pub fn snapshot_payload_digest(snapshot: &ExternalInstitutionSnapshot) -> Result<String> {
    crate::legacy_transition::digest_serializable(&(
        snapshot.campaign_id,
        &snapshot.authority_id,
        &snapshot.owner_id,
        snapshot.source_revision,
        &snapshot.idempotency_key,
        &snapshot.projection,
    ))
}

pub fn validate_seed_admission(
    seed: &WorldSeed,
    admission: &WorldSeedAdmission,
    authority_key: &str,
) -> Result<Campaign> {
    if seed.schema != "ghostlight.world_seed.v1"
        || admission.schema != "ghostlight.world_seed_admission.v1"
        || seed.id != admission.campaign_id
        || admission.seed_digest != seed.digest()?
        || admission.producer_id.trim().is_empty()
        || admission.idempotency_key.trim().is_empty()
    {
        return Err(anyhow!("world seed admission envelope is inconsistent"));
    }
    let mut campaign = seed.clone().into_campaign();
    crate::resolution::ensure_agency_profiles(&mut campaign);
    crate::compiler::validate_campaign_seed(&campaign)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut seen_subjects = std::collections::BTreeSet::new();
    for authority in &admission.external_subjects {
        if authority.schema != "ghostlight.external_subject_authority.v1"
            || authority.campaign_id != campaign.id
            || authority.owner_id != admission.producer_id
            || !seen.insert(authority.id.as_str())
            || !seen_subjects.insert(authority.subject_id.as_str())
            || authority.last_source_revision.is_some()
            || authority.last_payload_digest.is_some()
        {
            return Err(anyhow!(
                "external subject authority envelope is inconsistent"
            ));
        }
        validate_authority_key(authority, authority_key)?;
        let profile = campaign
            .agency_profiles
            .get_mut(&authority.subject_id)
            .filter(|profile| profile.subject_kind == authority.subject_kind)
            .ok_or_else(|| anyhow!("external authority subject or kind is unknown"))?;
        if authority.subject_kind != AgencySubjectKind::Institution
            || !campaign.institutions.contains_key(&authority.subject_id)
        {
            return Err(anyhow!(
                "the first consumer API slice admits institution-shaped external subjects only"
            ));
        }
        profile.simulation_eligible = false;
    }
    let primary = &campaign.player_actor_id;
    if !campaign.actors.contains_key(primary)
        && !campaign.institutions.contains_key(primary)
        && !campaign.gestalts.contains_key(primary)
    {
        return Err(anyhow!("primary controlled subject is missing"));
    }
    if admission.producer_kind == WorldSeedProducerKind::Consumer
        && !admission
            .external_subjects
            .iter()
            .any(|authority| authority.subject_id == *primary)
    {
        return Err(anyhow!(
            "consumer seed primary controlled subject lacks external authority"
        ));
    }
    Ok(campaign)
}

pub fn proposal_targets(action: &CellActionProposal) -> Vec<&str> {
    action
        .effects
        .iter()
        .flat_map(|effect| match effect {
            StrategicCellEffect::GestaltActivity {
                target_subject_ids, ..
            }
            | StrategicCellEffect::ActorActivity {
                target_subject_ids, ..
            }
            | StrategicCellEffect::MemberActivity {
                target_subject_ids, ..
            } => target_subject_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}
