use crate::{
    domain::{ActorState, Campaign, EvidenceCoverage, WorldCompilePreview},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{RwLock, mpsc, oneshot};
use uuid::Uuid;

pub const MAX_SESSION_ZERO_MEMBERS: usize = 8;
pub const FIXTURE_CELL_ALLOWANCE: u8 = 8;
pub const OPERATOR_CELL_CEILING: u8 = 128;

pub trait EntitlementPort: Send + Sync {
    fn persona_cell_allowance(&self, account_hash: &str) -> u8;
}

#[derive(Default)]
pub struct FixtureEntitlementPort;

impl EntitlementPort for FixtureEntitlementPort {
    fn persona_cell_allowance(&self, _account_hash: &str) -> u8 {
        FIXTURE_CELL_ALLOWANCE
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionZeroStatus {
    Drafting,
    RosterLocked,
    Compiling,
    Review,
    Published,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionZeroChannelKind {
    SharedTable,
    PrivateDm,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionZeroSpeakerKind {
    Player,
    Dm,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryLevel {
    AskFirst,
    Veil,
    Line,
}

impl BoundaryLevel {
    fn severity(&self) -> u8 {
        match self {
            Self::AskFirst => 1,
            Self::Veil => 2,
            Self::Line => 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ContentBoundary {
    pub schema: String,
    pub id: String,
    pub owner_member_id: String,
    pub topic: String,
    pub normalized_topic: String,
    pub level: BoundaryLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AggregatedBoundary {
    pub normalized_topic: String,
    pub display_topic: String,
    pub level: BoundaryLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ActiveContractBoundaryPolicy {
    pub schema: String,
    pub campaign_id: Uuid,
    pub review_session_zero_id: Uuid,
    pub aggregate_boundaries: Vec<AggregatedBoundary>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct CampaignContract {
    pub schema: String,
    pub vault_provider: String,
    pub campaign_name: String,
    pub premise: String,
    pub canon_horizon: String,
    pub starting_where: String,
    pub starting_when: String,
    pub starting_pressure: String,
    pub desired_goal: String,
    pub tone: Vec<String>,
    pub themes: Vec<String>,
    pub pacing: String,
    pub consequence_style: String,
    pub narrative_focus: String,
    pub party_bonds: Vec<String>,
    pub internal_tension: String,
    pub dm_style: String,
    pub time_advance_policy: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExtraordinaryPermission {
    pub schema: String,
    pub id: String,
    pub actor_id: String,
    pub name: String,
    pub reliable_scope: String,
    pub prerequisites: Vec<String>,
    pub costs: Vec<String>,
    pub limits: Vec<String>,
    pub exposure: Vec<String>,
    pub effect_ceiling: String,
    pub evidence_receipt_ids: Vec<String>,
    pub branch_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct CharacterDraft {
    pub schema: String,
    pub member_id: String,
    pub actor_id: String,
    pub name: String,
    pub public_premise: String,
    pub private_history: Vec<String>,
    pub secrets: Vec<String>,
    pub capabilities: Vec<String>,
    pub knowledge: Vec<String>,
    pub equipment: Vec<String>,
    pub relationships: BTreeMap<String, String>,
    pub obligations: Vec<String>,
    pub vulnerabilities: Vec<String>,
    pub goals: Vec<String>,
    pub extraordinary_permissions: Vec<ExtraordinaryPermission>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroMember {
    pub schema: String,
    pub id: String,
    #[schemars(skip)]
    pub account_hash: String,
    pub display_name: String,
    pub is_host: bool,
    pub active: bool,
    pub cell_allowance: u8,
    pub joined_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroChannel {
    pub schema: String,
    pub id: String,
    pub kind: SessionZeroChannelKind,
    pub member_id: Option<String>,
    pub revision: u64,
    pub message_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroMessage {
    pub schema: String,
    pub id: String,
    pub channel_id: String,
    pub author_member_id: Option<String>,
    pub speaker: SessionZeroSpeakerKind,
    pub text: String,
    pub session_revision: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroDecision {
    pub schema: String,
    pub id: String,
    pub owner_member_id: Option<String>,
    pub prompt: String,
    pub proposed_resolution: String,
    #[serde(default)]
    pub proposed_extraordinary_permission: Option<ExtraordinaryPermission>,
    #[serde(default)]
    pub proposed_contract_patch: Option<CampaignContractPatch>,
    #[serde(default)]
    pub proposed_character_patch: Option<CharacterDraftPatch>,
    #[serde(default)]
    pub evidence_receipt_ids: Vec<String>,
    #[serde(default)]
    pub pending_counter: Option<String>,
    pub material: bool,
    pub resolved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroApproval {
    pub schema: String,
    pub member_id: String,
    pub shared_digest: String,
    pub character_digest: String,
    #[serde(default)]
    pub world_preview_digest: String,
    pub approved_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SessionZeroInvite {
    pub schema: String,
    pub id: String,
    #[schemars(skip)]
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_by_member_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CampaignDmPersona {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub voice: String,
    pub shared_memories: Vec<String>,
    pub private_member_memories: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CampaignMember {
    pub member_id: String,
    #[schemars(skip)]
    pub account_hash: String,
    pub display_name: String,
    pub actor_id: String,
    pub is_host: bool,
    pub active: bool,
    pub cell_allowance: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CampaignMembership {
    pub schema: String,
    pub campaign_id: Uuid,
    pub governance_epoch: u64,
    pub host_member_id: String,
    pub members: BTreeMap<String, CampaignMember>,
    pub extraordinary_permissions: BTreeMap<String, Vec<ExtraordinaryPermission>>,
}

impl CampaignMembership {
    pub fn member_for_account(&self, account_hash: &str) -> Option<&CampaignMember> {
        self.members
            .values()
            .find(|member| member.active && member.account_hash == account_hash)
    }

    pub fn controlled_actor_ids(&self) -> BTreeSet<String> {
        self.members
            .values()
            .filter(|member| member.active)
            .map(|member| member.actor_id.clone())
            .collect()
    }

    pub fn pooled_cell_allowance(&self) -> u8 {
        self.members
            .values()
            .filter(|member| member.active)
            .fold(0_u8, |total, member| {
                total.saturating_add(member.cell_allowance)
            })
            .min(OPERATOR_CELL_CEILING)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CampaignGovernance {
    pub schema: String,
    pub campaign_id: Uuid,
    pub governance_epoch: u64,
    pub time_advance_policy: String,
    pub pooled_cell_ceiling: u8,
    pub cooperative_shared_scene_only: bool,
    pub pvp_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TimeAdvanceProposal {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub minutes: u32,
    pub proposer_member_id: String,
    pub approvals: BTreeSet<String>,
    pub committed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GroupTravelProposal {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub origin_location_id: String,
    pub destination_location_id: String,
    pub travel_minutes: u32,
    pub proposer_member_id: String,
    pub approvals: BTreeSet<String>,
    pub committed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CellBudgetProposal {
    pub schema: String,
    pub id: String,
    pub campaign_id: Uuid,
    pub expected_world_revision: u64,
    pub expected_resolution_epoch: u64,
    pub active_cell_budget: u8,
    pub proposer_member_id: String,
    pub approvals: BTreeSet<String>,
    pub committed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PublishedSessionZeroSeed {
    pub schema: String,
    pub session_zero_id: Uuid,
    pub approved_seed_digest: String,
    pub contract: CampaignContract,
    pub membership: CampaignMembership,
    pub governance: CampaignGovernance,
    pub dm_persona: CampaignDmPersona,
    pub approvals: Vec<SessionZeroApproval>,
    pub approved_brief: ApprovedCampaignBrief,
    #[serde(default)]
    pub approved_world_preview_digest: String,
    #[serde(default)]
    pub approved_evidence_gaps: Vec<String>,
    #[serde(default)]
    pub approved_branch_assumptions: Vec<String>,
    #[serde(default)]
    pub boundaries: Vec<ContentBoundary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ApprovedCampaignBrief {
    pub schema: String,
    pub session_zero_id: Uuid,
    pub host_member_id: String,
    pub contract: CampaignContract,
    pub aggregate_boundaries: Vec<AggregatedBoundary>,
    pub characters: Vec<CharacterDraft>,
    pub member_actor_ids: BTreeMap<String, String>,
    pub shared_digest: String,
    pub character_digests: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct CampaignContractPatch {
    pub premise: Option<String>,
    pub canon_horizon: Option<String>,
    pub starting_where: Option<String>,
    pub starting_when: Option<String>,
    pub starting_pressure: Option<String>,
    pub desired_goal: Option<String>,
    pub tone: Option<Vec<String>>,
    pub themes: Option<Vec<String>>,
    pub pacing: Option<String>,
    pub consequence_style: Option<String>,
    pub narrative_focus: Option<String>,
    pub party_bonds: Option<Vec<String>>,
    pub internal_tension: Option<String>,
    pub dm_style: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct CharacterDraftPatch {
    pub name: Option<String>,
    pub public_premise: Option<String>,
    pub private_history_add: Vec<String>,
    pub private_history_remove: Vec<String>,
    pub secrets_add: Vec<String>,
    pub secrets_remove: Vec<String>,
    pub capabilities_add: Vec<String>,
    pub capabilities_remove: Vec<String>,
    pub knowledge_add: Vec<String>,
    pub knowledge_remove: Vec<String>,
    pub equipment_add: Vec<String>,
    pub equipment_remove: Vec<String>,
    pub relationships: BTreeMap<String, String>,
    pub relationship_removals: Vec<String>,
    pub obligations_add: Vec<String>,
    pub obligations_remove: Vec<String>,
    pub vulnerabilities_add: Vec<String>,
    pub vulnerabilities_remove: Vec<String>,
    pub goals_add: Vec<String>,
    pub goals_remove: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct SessionZeroDelta {
    pub contract_patch: CampaignContractPatch,
    pub character_patch: Option<CharacterDraftPatch>,
    pub decisions: Vec<SessionZeroDecision>,
    pub dm_speech: String,
    pub suggested_replies: Vec<String>,
}

/// Model-facing typed extraction. The Persona owns the natural DM utterance;
/// the Interpreter may only propose typed state and reply affordances.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct SessionZeroInterpretation {
    pub contract_patch: CampaignContractPatch,
    pub character_patch: Option<CharacterDraftPatch>,
    pub decisions: Vec<SessionZeroDecision>,
    pub suggested_replies: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FocusedPermissionTerms {
    pub name: String,
    pub reliable_scope: String,
    pub prerequisites: Vec<String>,
    pub costs: Vec<String>,
    pub limits: Vec<String>,
    pub exposure: Vec<String>,
    pub effect_ceiling: String,
    pub branch_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SessionZeroState {
    pub schema: String,
    pub id: Uuid,
    pub name: String,
    pub status: SessionZeroStatus,
    pub revision: u64,
    pub shared_epoch: u64,
    pub boundary_epoch: u64,
    pub host_member_id: String,
    pub roster_locked: bool,
    pub members: BTreeMap<String, SessionZeroMember>,
    pub channels: BTreeMap<String, SessionZeroChannel>,
    pub messages: BTreeMap<String, SessionZeroMessage>,
    pub contract: CampaignContract,
    pub character_drafts: BTreeMap<String, CharacterDraft>,
    pub character_epochs: BTreeMap<String, u64>,
    pub boundaries: BTreeMap<String, ContentBoundary>,
    pub aggregate_boundaries: Vec<AggregatedBoundary>,
    pub decisions: BTreeMap<String, SessionZeroDecision>,
    pub approvals: BTreeMap<String, SessionZeroApproval>,
    pub invites: BTreeMap<String, SessionZeroInvite>,
    pub dm_persona: CampaignDmPersona,
    pub preview: Option<WorldCompilePreview>,
    #[serde(default)]
    pub model_receipts: Vec<ModelStageReceipt>,
    #[serde(default)]
    pub preview_model_receipts: Vec<ModelStageReceipt>,
    pub preview_evidence_coverage: Vec<EvidenceCoverage>,
    pub preview_shared_digest: Option<String>,
    pub preview_character_digests: BTreeMap<String, String>,
    pub published_campaign_id: Option<Uuid>,
    pub published_seed_digest: Option<String>,
    #[serde(default)]
    pub review_campaign_id: Option<Uuid>,
    #[serde(default)]
    pub review_world_revision: Option<u64>,
    #[serde(default)]
    pub inherited_aggregate_boundaries: Vec<AggregatedBoundary>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionZeroState {
    pub fn new(
        name: String,
        vault_provider: String,
        account_hash: String,
        display_name: String,
    ) -> Result<Self> {
        Self::new_with_allowance(
            name,
            vault_provider,
            account_hash,
            display_name,
            FIXTURE_CELL_ALLOWANCE,
        )
    }

    pub fn new_with_allowance(
        name: String,
        vault_provider: String,
        account_hash: String,
        display_name: String,
        cell_allowance: u8,
    ) -> Result<Self> {
        validate_bounded("session name", &name, 1, 80)?;
        validate_bounded("vault provider", &vault_provider, 1, 120)?;
        validate_bounded("display name", &display_name, 1, 80)?;
        if cell_allowance == 0 || cell_allowance > OPERATOR_CELL_CEILING {
            return Err(anyhow!("Persona-cell entitlement is out of range"));
        }
        let id = Uuid::new_v4();
        let member_id = format!("member:{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let mut members = BTreeMap::new();
        members.insert(
            member_id.clone(),
            SessionZeroMember {
                schema: "ghostlight.session_zero_member.v1".into(),
                id: member_id.clone(),
                account_hash,
                display_name: display_name.clone(),
                is_host: true,
                active: true,
                cell_allowance,
                joined_at: now,
            },
        );
        let shared_id = "shared:table".to_string();
        let private_id = format!("private:{member_id}");
        let channels = BTreeMap::from([
            (
                shared_id.clone(),
                SessionZeroChannel {
                    schema: "ghostlight.session_zero_channel.v1".into(),
                    id: shared_id,
                    kind: SessionZeroChannelKind::SharedTable,
                    member_id: None,
                    revision: 0,
                    message_ids: vec![],
                },
            ),
            (
                private_id.clone(),
                SessionZeroChannel {
                    schema: "ghostlight.session_zero_channel.v1".into(),
                    id: private_id,
                    kind: SessionZeroChannelKind::PrivateDm,
                    member_id: Some(member_id.clone()),
                    revision: 0,
                    message_ids: vec![],
                },
            ),
        ]);
        let actor_id = format!("player:{}", Uuid::new_v4().simple());
        Ok(Self {
            schema: "ghostlight.session_zero.v1".into(),
            id,
            name: name.clone(),
            status: SessionZeroStatus::Drafting,
            revision: 0,
            shared_epoch: 0,
            boundary_epoch: 0,
            host_member_id: member_id.clone(),
            roster_locked: false,
            members,
            channels,
            messages: BTreeMap::new(),
            contract: CampaignContract {
                schema: "ghostlight.campaign_contract.v1".into(),
                vault_provider,
                campaign_name: name,
                time_advance_policy: "unanimous".into(),
                ..CampaignContract::default()
            },
            character_drafts: BTreeMap::from([(
                member_id.clone(),
                CharacterDraft {
                    schema: "ghostlight.character_draft.v1".into(),
                    member_id: member_id.clone(),
                    actor_id,
                    name: display_name,
                    ..CharacterDraft::default()
                },
            )]),
            character_epochs: BTreeMap::from([(member_id.clone(), 0)]),
            boundaries: BTreeMap::new(),
            aggregate_boundaries: vec![],
            decisions: BTreeMap::new(),
            approvals: BTreeMap::new(),
            invites: BTreeMap::new(),
            dm_persona: CampaignDmPersona {
                schema: "ghostlight.campaign_dm_persona.v1".into(),
                id: format!("dm:{}", id.simple()),
                name: "Ghostlight".into(),
                voice: "Curious, candid, fiction-first, and willing to negotiate for meaningful stakes.".into(),
                shared_memories: vec![],
                private_member_memories: BTreeMap::new(),
            },
            preview: None,
            model_receipts: vec![],
            preview_model_receipts: vec![],
            preview_evidence_coverage: vec![],
            preview_shared_digest: None,
            preview_character_digests: BTreeMap::new(),
            published_campaign_id: None,
            published_seed_digest: None,
            review_campaign_id: None,
            review_world_revision: None,
            inherited_aggregate_boundaries: vec![],
            created_at: now,
            updated_at: now,
        })
    }

    pub fn for_contract_review(
        campaign: &Campaign,
        membership: &CampaignMembership,
        contract: CampaignContract,
        dm_persona: CampaignDmPersona,
        previous_brief: Option<&ApprovedCampaignBrief>,
        boundaries: Vec<ContentBoundary>,
    ) -> Result<Self> {
        if membership.campaign_id != campaign.id {
            return Err(anyhow!(
                "contract review membership targets another campaign"
            ));
        }
        let id = Uuid::new_v4();
        let now = Utc::now();
        let mut members = BTreeMap::new();
        let mut channels = BTreeMap::from([(
            "shared:table".to_string(),
            SessionZeroChannel {
                schema: "ghostlight.session_zero_channel.v1".into(),
                id: "shared:table".into(),
                kind: SessionZeroChannelKind::SharedTable,
                member_id: None,
                revision: 0,
                message_ids: vec![],
            },
        )]);
        let mut character_drafts = BTreeMap::new();
        let mut character_epochs = BTreeMap::new();
        for member in membership.members.values().filter(|member| member.active) {
            let actor = campaign
                .actors
                .get(&member.actor_id)
                .ok_or_else(|| anyhow!("campaign member actor is missing"))?;
            let previous = previous_brief.and_then(|brief| {
                brief
                    .characters
                    .iter()
                    .find(|draft| draft.actor_id == member.actor_id)
            });
            members.insert(
                member.member_id.clone(),
                SessionZeroMember {
                    schema: "ghostlight.session_zero_member.v1".into(),
                    id: member.member_id.clone(),
                    account_hash: member.account_hash.clone(),
                    display_name: member.display_name.clone(),
                    is_host: member.is_host,
                    active: true,
                    cell_allowance: member.cell_allowance,
                    joined_at: now,
                },
            );
            let channel_id = format!("private:{}", member.member_id);
            channels.insert(
                channel_id.clone(),
                SessionZeroChannel {
                    schema: "ghostlight.session_zero_channel.v1".into(),
                    id: channel_id,
                    kind: SessionZeroChannelKind::PrivateDm,
                    member_id: Some(member.member_id.clone()),
                    revision: 0,
                    message_ids: vec![],
                },
            );
            character_drafts.insert(
                member.member_id.clone(),
                CharacterDraft {
                    schema: "ghostlight.character_draft.v1".into(),
                    member_id: member.member_id.clone(),
                    actor_id: member.actor_id.clone(),
                    name: actor.name.clone(),
                    public_premise: previous
                        .map(|value| value.public_premise.clone())
                        .unwrap_or_default(),
                    private_history: previous
                        .map(|value| value.private_history.clone())
                        .unwrap_or_else(|| actor.memories.clone()),
                    secrets: previous
                        .map(|value| value.secrets.clone())
                        .unwrap_or_default(),
                    capabilities: actor.capabilities.iter().cloned().collect(),
                    knowledge: actor.knowledge.iter().cloned().collect(),
                    equipment: actor.equipment.iter().cloned().collect(),
                    relationships: actor.relationships.clone(),
                    obligations: actor.obligations.iter().cloned().collect(),
                    vulnerabilities: actor.conditions.iter().cloned().collect(),
                    goals: actor.goals.clone(),
                    extraordinary_permissions: membership
                        .extraordinary_permissions
                        .get(&member.actor_id)
                        .cloned()
                        .unwrap_or_default(),
                },
            );
            character_epochs.insert(member.member_id.clone(), 0);
        }
        let exact_boundaries = boundaries
            .into_iter()
            .map(|boundary| (boundary.id.clone(), boundary))
            .collect::<BTreeMap<_, _>>();
        let inherited_aggregate_boundaries = previous_brief
            .map(|brief| brief.aggregate_boundaries.clone())
            .unwrap_or_default();
        let aggregate_boundaries =
            aggregate_boundaries_with_inherited(&exact_boundaries, &inherited_aggregate_boundaries);
        Ok(Self {
            schema: "ghostlight.session_zero.v1".into(),
            id,
            name: format!("{} — Contract Review", campaign.name),
            status: SessionZeroStatus::RosterLocked,
            revision: 0,
            shared_epoch: 0,
            boundary_epoch: 0,
            host_member_id: membership.host_member_id.clone(),
            roster_locked: true,
            members,
            channels,
            messages: BTreeMap::new(),
            contract,
            character_drafts,
            character_epochs,
            boundaries: exact_boundaries,
            aggregate_boundaries,
            decisions: BTreeMap::new(),
            approvals: BTreeMap::new(),
            invites: BTreeMap::new(),
            dm_persona,
            preview: None,
            model_receipts: vec![],
            preview_model_receipts: vec![],
            preview_evidence_coverage: vec![],
            preview_shared_digest: None,
            preview_character_digests: BTreeMap::new(),
            published_campaign_id: None,
            published_seed_digest: None,
            review_campaign_id: Some(campaign.id),
            review_world_revision: Some(campaign.revision),
            inherited_aggregate_boundaries,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn member_for_account(&self, account_hash: &str) -> Option<&SessionZeroMember> {
        self.members
            .values()
            .find(|member| member.active && member.account_hash == account_hash)
    }

    pub fn shared_digest(&self) -> Result<String> {
        digest(&(
            &self.contract,
            &self.aggregate_boundaries,
            self.members
                .values()
                .filter(|member| member.active)
                .map(|member| {
                    let draft = &self.character_drafts[&member.id];
                    (
                        &member.id,
                        &member.display_name,
                        &draft.name,
                        &draft.public_premise,
                    )
                })
                .collect::<Vec<_>>(),
        ))
    }

    pub fn character_digest(&self, member_id: &str) -> Result<String> {
        let draft = self
            .character_drafts
            .get(member_id)
            .ok_or_else(|| anyhow!("character draft is missing"))?;
        let boundaries = self
            .boundaries
            .values()
            .filter(|boundary| boundary.owner_member_id == member_id)
            .collect::<Vec<_>>();
        digest(&(draft, boundaries))
    }

    pub fn world_preview_digest(&self) -> Result<String> {
        let preview = self
            .preview
            .as_ref()
            .ok_or_else(|| anyhow!("session zero has no compiled world preview"))?;
        digest(preview)
    }

    pub fn pooled_cell_allowance(&self) -> u8 {
        self.members
            .values()
            .filter(|member| member.active)
            .fold(0_u8, |total, member| {
                total.saturating_add(member.cell_allowance)
            })
            .min(OPERATOR_CELL_CEILING)
    }

    pub fn approved_brief(&self) -> Result<ApprovedCampaignBrief> {
        if self.status != SessionZeroStatus::Review || self.preview.is_none() {
            return Err(anyhow!("session zero has no final review preview"));
        }
        let brief = self.compilation_brief()?;
        let world_preview_digest = self.world_preview_digest()?;
        for member in self.members.values().filter(|member| member.active) {
            let character_digest = self.character_digest(&member.id)?;
            let approval = self
                .approvals
                .get(&member.id)
                .ok_or_else(|| anyhow!("{} has not approved", member.display_name))?;
            if approval.shared_digest != brief.shared_digest
                || approval.character_digest != character_digest
                || approval.world_preview_digest != world_preview_digest
            {
                return Err(anyhow!("{} approval is stale", member.display_name));
            }
        }
        Ok(brief)
    }

    pub fn compilation_brief(&self) -> Result<ApprovedCampaignBrief> {
        if !self.roster_locked {
            return Err(anyhow!("roster must be locked before compilation"));
        }
        if self
            .decisions
            .values()
            .any(|decision| decision.material && !decision.resolved)
        {
            return Err(anyhow!("material decisions remain unresolved"));
        }
        let gaps = self.compilation_gaps();
        if !gaps.is_empty() {
            return Err(anyhow!(
                "Session Zero draft is incomplete: {}",
                gaps.join("; ")
            ));
        }
        let shared_digest = self.shared_digest()?;
        let active = self
            .members
            .values()
            .filter(|member| member.active)
            .collect::<Vec<_>>();
        let mut character_digests = BTreeMap::new();
        let mut characters = Vec::new();
        let mut member_actor_ids = BTreeMap::new();
        for member in active {
            let character_digest = self.character_digest(&member.id)?;
            character_digests.insert(member.id.clone(), character_digest);
            let draft = self.character_drafts[&member.id].clone();
            member_actor_ids.insert(member.id.clone(), draft.actor_id.clone());
            characters.push(draft);
        }
        Ok(ApprovedCampaignBrief {
            schema: "ghostlight.approved_campaign_brief.v1".into(),
            session_zero_id: self.id,
            host_member_id: self.host_member_id.clone(),
            contract: self.contract.clone(),
            aggregate_boundaries: self.aggregate_boundaries.clone(),
            characters,
            member_actor_ids,
            shared_digest,
            character_digests,
        })
    }

    fn compilation_gaps(&self) -> Vec<String> {
        let mut gaps = Vec::new();
        for (label, value) in [
            ("campaign premise", self.contract.premise.as_str()),
            ("canon horizon", self.contract.canon_horizon.as_str()),
            ("starting location", self.contract.starting_where.as_str()),
            ("starting time", self.contract.starting_when.as_str()),
            (
                "starting pressure",
                self.contract.starting_pressure.as_str(),
            ),
            ("desired goal", self.contract.desired_goal.as_str()),
            ("pacing", self.contract.pacing.as_str()),
            (
                "consequence style",
                self.contract.consequence_style.as_str(),
            ),
            ("narrative focus", self.contract.narrative_focus.as_str()),
            ("DM style", self.contract.dm_style.as_str()),
        ] {
            if value.trim().is_empty() {
                gaps.push(label.to_string());
            }
        }
        if self.contract.tone.is_empty() {
            gaps.push("tone".into());
        }
        let active_members = self
            .members
            .values()
            .filter(|member| member.active)
            .collect::<Vec<_>>();
        if active_members.len() > 1 && self.contract.party_bonds.is_empty() {
            gaps.push("party bonds".into());
        }
        for member in active_members {
            let Some(character) = self.character_drafts.get(&member.id) else {
                gaps.push(format!("{} character draft", member.display_name));
                continue;
            };
            if character.name.trim().is_empty() {
                gaps.push(format!("{} character name", member.display_name));
            }
            if character.public_premise.trim().is_empty() {
                gaps.push(format!("{} public character premise", member.display_name));
            }
            if character.capabilities.is_empty() {
                gaps.push(format!("{} capabilities", member.display_name));
            }
            if character.goals.is_empty() {
                gaps.push(format!("{} goals", member.display_name));
            }
            if character.obligations.is_empty() && character.vulnerabilities.is_empty() {
                gaps.push(format!(
                    "{} obligation or vulnerability",
                    member.display_name
                ));
            }
        }
        gaps
    }
}

#[derive(Clone, Debug)]
pub enum SessionZeroCommand {
    CreateInvites {
        actor_account_hash: String,
        count: u8,
    },
    Join {
        token: String,
        account_hash: String,
        display_name: String,
        cell_allowance: u8,
    },
    Leave {
        actor_account_hash: String,
        expected_revision: u64,
    },
    RemoveMember {
        actor_account_hash: String,
        expected_revision: u64,
        member_id: String,
    },
    PostPlayerMessage {
        actor_account_hash: String,
        expected_revision: u64,
        channel_id: String,
        text: String,
    },
    ApplyDmTurn {
        expected_component_epoch: u64,
        expected_channel_revision: u64,
        channel_id: String,
        member_id: Option<String>,
        supersedes_countered_decision_id: Option<String>,
        delta: SessionZeroDelta,
        model_receipts: Vec<ModelStageReceipt>,
    },
    SetBoundary {
        actor_account_hash: String,
        expected_revision: u64,
        boundary_id: Option<String>,
        topic: String,
        normalized_topic: String,
        level: BoundaryLevel,
    },
    RemoveBoundary {
        actor_account_hash: String,
        expected_revision: u64,
        boundary_id: String,
    },
    ResolveDecision {
        actor_account_hash: String,
        expected_revision: u64,
        decision_id: String,
        accept: bool,
        counter: Option<String>,
    },
    LockRoster {
        actor_account_hash: String,
        expected_revision: u64,
    },
    BeginCompilation {
        actor_account_hash: String,
        expected_revision: u64,
    },
    ReviewCompilationGaps {
        actor_account_hash: String,
        expected_revision: u64,
    },
    DiscardPreview {
        actor_account_hash: String,
        expected_revision: u64,
    },
    InstallPreview {
        expected_revision: u64,
        preview: WorldCompilePreview,
        model_receipts: Vec<ModelStageReceipt>,
    },
    CompilationFailed {
        expected_revision: u64,
        message: String,
    },
    Approve {
        actor_account_hash: String,
        expected_revision: u64,
    },
    MarkPublished {
        actor_account_hash: String,
        expected_revision: u64,
        campaign_id: Uuid,
        seed_digest: String,
    },
    Archive {
        actor_account_hash: String,
        expected_revision: u64,
    },
}

#[derive(Clone, Debug)]
pub struct SessionZeroCommandResult {
    pub state: SessionZeroState,
    pub invite_tokens: Vec<String>,
}

#[derive(Clone)]
pub struct SessionZeroKernel {
    tx: mpsc::Sender<KernelRequest>,
}

struct KernelRequest {
    command: SessionZeroCommand,
    reply: oneshot::Sender<Result<SessionZeroCommandResult>>,
}

impl SessionZeroKernel {
    pub fn initialize(store: &CampaignStore, state: &SessionZeroState) -> Result<()> {
        if !store.keys("session_zero.v1")?.is_empty() {
            return Err(anyhow!("session zero store is not empty"));
        }
        store.insert(
            "session_zero.v1",
            "ghostlight.session_zero.v1",
            &state.id.to_string(),
            state,
        )?;
        Ok(())
    }

    pub fn start(store: CampaignStore, state_id: Uuid) -> Self {
        let (tx, mut rx) = mpsc::channel::<KernelRequest>(64);
        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                let result = execute(&store, state_id, request.command);
                let _ = request.reply.send(result);
            }
        });
        Self { tx }
    }

    pub async fn command(&self, command: SessionZeroCommand) -> Result<SessionZeroCommandResult> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(KernelRequest { command, reply })
            .await
            .map_err(|_| anyhow!("session zero kernel stopped"))?;
        receive
            .await
            .map_err(|_| anyhow!("session zero kernel stopped"))?
    }
}

#[derive(Clone)]
pub struct SessionZeroRuntime {
    pub store: CampaignStore,
    pub kernel: SessionZeroKernel,
}

#[derive(Clone)]
pub struct SessionZeroRegistry {
    root: PathBuf,
    runtimes: Arc<RwLock<BTreeMap<Uuid, SessionZeroRuntime>>>,
}

impl SessionZeroRegistry {
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
            let path = entry.path().join("session-zero.cc");
            if !path.is_file() {
                continue;
            }
            let store = CampaignStore::open(path)?;
            let keys = store.keys("session_zero.v1")?;
            if keys != vec![id.to_string()] {
                return Err(anyhow!(
                    "session zero directory must contain its one matching state row"
                ));
            }
            let (row, mut state) = store
                .load::<SessionZeroState>("session_zero.v1", &id.to_string())?
                .ok_or_else(|| anyhow!("session zero state vanished during load"))?;
            let changed = demote_legacy_nonmaterial_decisions(&mut state)
                | migrate_legacy_model_receipts(&mut state);
            if changed {
                store.replace(&row, "ghostlight.session_zero.v1", &state)?;
            }
            found.insert(
                id,
                SessionZeroRuntime {
                    kernel: SessionZeroKernel::start(store.clone(), id),
                    store,
                },
            );
        }
        *self.runtimes.write().await = found;
        Ok(())
    }

    pub async fn create(&self, state: SessionZeroState) -> Result<SessionZeroRuntime> {
        if self.runtimes.read().await.contains_key(&state.id) {
            return Err(anyhow!("session zero already exists"));
        }
        let directory = self.root.join(state.id.to_string());
        if directory.exists() {
            return Err(anyhow!("session zero directory already exists"));
        }
        let staging = self
            .root
            .join(format!(".creating-{}-{}", state.id, Uuid::new_v4()));
        fs::create_dir(&staging)?;
        let prepared = (|| -> Result<()> {
            let store = CampaignStore::open(staging.join("session-zero.cc"))?;
            SessionZeroKernel::initialize(&store, &state)?;
            drop(store);
            fs::rename(&staging, &directory)?;
            Ok(())
        })();
        if let Err(error) = prepared {
            cleanup_staging(&self.root, &staging);
            return Err(error);
        }
        let store = CampaignStore::open(directory.join("session-zero.cc"))?;
        let runtime = SessionZeroRuntime {
            kernel: SessionZeroKernel::start(store.clone(), state.id),
            store,
        };
        self.runtimes
            .write()
            .await
            .insert(state.id, runtime.clone());
        Ok(runtime)
    }

    pub async fn runtime(&self, id: Uuid) -> Result<SessionZeroRuntime> {
        self.runtimes
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("session zero runtime is not loaded"))
    }

    pub async fn list(&self) -> Vec<Uuid> {
        self.runtimes.read().await.keys().copied().collect()
    }

    pub async fn snapshot(&self, id: Uuid) -> Result<SessionZeroState> {
        let runtime = self.runtime(id).await?;
        runtime
            .store
            .load::<SessionZeroState>("session_zero.v1", &id.to_string())?
            .map(|(_, state)| state)
            .ok_or_else(|| anyhow!("session zero state vanished"))
    }

    pub async fn session_for_account(&self, account_hash: &str) -> Result<Option<Uuid>> {
        let runtimes = self.runtimes.read().await.clone();
        let mut matches = Vec::new();
        for (id, runtime) in runtimes {
            let Some((_, state)) = runtime
                .store
                .load::<SessionZeroState>("session_zero.v1", &id.to_string())?
            else {
                continue;
            };
            if !matches!(
                state.status,
                SessionZeroStatus::Published | SessionZeroStatus::Archived
            ) && state.member_for_account(account_hash).is_some()
            {
                matches.push((state.created_at, id));
            }
        }
        matches.sort();
        Ok(matches.last().map(|(_, id)| *id))
    }

    pub async fn session_for_invite(&self, token: &str) -> Result<Option<Uuid>> {
        let wanted = secret_hash(token);
        let runtimes = self.runtimes.read().await.clone();
        for (id, runtime) in runtimes {
            let Some((_, state)) = runtime
                .store
                .load::<SessionZeroState>("session_zero.v1", &id.to_string())?
            else {
                continue;
            };
            if state.invites.values().any(|invite| {
                invite.token_hash == wanted
                    && invite.consumed_by_member_id.is_none()
                    && invite.expires_at > Utc::now()
            }) {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    pub async fn active_contract_review_for_account(
        &self,
        account_hash: &str,
    ) -> Result<Option<Uuid>> {
        let runtimes = self.runtimes.read().await.clone();
        let mut matches = Vec::new();
        for (id, runtime) in runtimes {
            let Some((_, state)) = runtime
                .store
                .load::<SessionZeroState>("session_zero.v1", &id.to_string())?
            else {
                continue;
            };
            if state.review_campaign_id.is_some()
                && !matches!(
                    state.status,
                    SessionZeroStatus::Published | SessionZeroStatus::Archived
                )
                && state.member_for_account(account_hash).is_some()
            {
                matches.push((state.created_at, id));
            }
        }
        matches.sort();
        Ok(matches.last().map(|(_, id)| *id))
    }
}

fn cleanup_staging(root: &Path, staging: &Path) {
    let safe = staging.parent() == Some(root)
        && staging
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".creating-"));
    if safe && staging.exists() {
        let _ = fs::remove_dir_all(staging);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ProjectedLivedStream {
    lived_stream: String,
}

#[derive(Clone)]
pub struct SessionZeroDirector {
    model: Arc<dyn ModelPort>,
    projector_model: String,
    persona_model: String,
    interpreter_model: String,
}

#[derive(Clone, Copy)]
enum FocusedReplacementLane<'a> {
    Permission(&'a ExtraordinaryPermission),
    Contract,
    Character,
}

impl SessionZeroDirector {
    pub fn new(
        model: Arc<dyn ModelPort>,
        projector_model: impl Into<String>,
        persona_model: impl Into<String>,
        interpreter_model: impl Into<String>,
    ) -> Self {
        Self {
            model,
            projector_model: projector_model.into(),
            persona_model: persona_model.into(),
            interpreter_model: interpreter_model.into(),
        }
    }

    pub async fn respond(
        &self,
        state: &SessionZeroState,
        channel_id: &str,
        member_id: Option<&str>,
        supersedes_countered_decision_id: Option<&str>,
    ) -> Result<(SessionZeroDelta, Vec<ModelStageReceipt>)> {
        require_channel_access_by_member(state, channel_id, member_id)?;
        let permitted = permitted_dm_context(
            state,
            channel_id,
            member_id,
            supersedes_countered_decision_id,
        )?;
        let binding = format!(
            "session-zero:{}:revision:{}:channel:{}:focus:{}",
            state.id,
            state.revision,
            channel_id,
            supersedes_countered_decision_id.unwrap_or("conversation")
        );
        let (lived, mut receipts) = if let Some(decision_id) = supersedes_countered_decision_id {
            (
                ProjectedLivedStream {
                    lived_stream: project_focused_counter_lived_stream(
                        state,
                        member_id,
                        decision_id,
                    )?,
                },
                Vec::new(),
            )
        } else {
            let projector_schema = serde_json::to_value(schema_for!(ProjectedLivedStream))?;
            let projector = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "session_zero_projector".into(),
                    model: self.projector_model.clone(),
                    snapshot_binding: binding.clone(),
                    lived_stream: format!(
                        "You project permitted typed Session Zero state into a compact private lived stream for the campaign DM. Preserve uncertainty, unresolved decisions, evidence gaps, accepted boundaries, and authorship. Never invent state. Stable contract:\n- Player speech is discussion, not world truth.\n- Model changes are proposals.\n- Material bargains need explicit acceptance.\n- Private data may not cross channels.\nReturn one complete JSON object matching this schema exactly.\n\nOUTPUT JSON SCHEMA:\n{}\n\nDYNAMIC PERMITTED CONTEXT:\n{}",
                        serde_json::to_string(&projector_schema)?,
                        serde_json::to_string(&permitted)?
                    ),
                    output_schema: Some(projector_schema),
                    source_receipt_ids: vec![],
                    temperature: Some(0.0),
                    max_output_tokens: Some(1800),
                },
            )
            .await?;
            let lived: ProjectedLivedStream = serde_json::from_value(
                projector
                    .structured
                    .clone()
                    .ok_or_else(|| anyhow!("projector omitted structured output"))?,
            )?;
            (lived, vec![projector.receipt])
        };
        let focused_persona_instruction = if supersedes_countered_decision_id.is_some() {
            " The lived stream is Ghostlight's exact factual rendering of one countered typed proposal. Acknowledge the replacement briefly without restating, renaming, or adding terms; invite the player to review the typed decision."
        } else {
            ""
        };
        let persona = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "session_zero_dm_persona".into(),
                model: self.persona_model.clone(),
                snapshot_binding: binding.clone(),
                lived_stream: format!(
                    "You are {}. {} Lead a candid, collaborative Session Zero. Ask only the most useful next questions; synthesize choices; preserve the player's premise while negotiating costs and limits that create stakes. Do not claim changes are accepted. Speak naturally, with no schema or machine-state language.{}\n\n{}",
                    state.dm_persona.name,
                    state.dm_persona.voice,
                    focused_persona_instruction,
                    lived.lived_stream
                ),
                output_schema: None,
                source_receipt_ids: vec![],
                temperature: Some(0.7),
                max_output_tokens: Some(1200),
            },
        )
        .await?;
        validate_bounded("DM speech", &persona.narrative, 1, 6_000)?;
        let focused = supersedes_countered_decision_id
            .and_then(|decision_id| state.decisions.get(decision_id))
            .and_then(|decision| focused_replacement_lane(decision).map(|lane| (decision, lane)));
        let (interpretation, interpreter_receipt) = if let Some((decision, lane)) = focused {
            run_focused_counter_interpreter(
                self.model.as_ref(),
                &self.interpreter_model,
                &binding,
                decision,
                lane,
                &lived.lived_stream,
                &persona.narrative,
            )
            .await?
        } else {
            let interpreter_context = permitted_interpreter_context(
                state,
                channel_id,
                member_id,
                supersedes_countered_decision_id,
            )?;
            let mut interpreter_schema =
                serde_json::to_value(schema_for!(SessionZeroInterpretation))?;
            require_typed_decision_payloads(&mut interpreter_schema)?;
            require_single_lane_ownership(&mut interpreter_schema)?;
            let interpreter = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "session_zero_interpreter".into(),
                    model: self.interpreter_model.clone(),
                    snapshot_binding: binding,
                    lived_stream: format!(
                        "Extract only NEW typed changes proposed by the DM response. You do not own or reproduce the DM's speech. Never copy current contract fields or existing unresolved decisions into the interpretation during ordinary conversation. Do not infer acceptance from mere discussion. Material character bargains must become unresolved decisions, not direct character grants. Every decision must carry at least one non-null typed proposed_extraordinary_permission, proposed_contract_patch, or proposed_character_patch payload; questions without an exact state change stay in DM speech or suggested replies. If turn_focus is present, it is the one exception: use countered_decision as the exact basis, preserve its unchanged typed fields, apply the pending counter, and emit exactly one fresh decision with the required owner and materiality. Do not emit unrelated decisions or a direct patch. Shared channels cannot alter private character state. Private channels cannot alter the shared contract. Use empty arrays, empty objects, or null for sections with no new change. Return one complete JSON object matching this schema exactly.\n\nOUTPUT JSON SCHEMA:\n{}\n\nDYNAMIC TYPED EXTRACTION CONTEXT:\n{}\n\nDYNAMIC DM RESPONSE:\n{}",
                        serde_json::to_string(&interpreter_schema)?,
                        serde_json::to_string(&interpreter_context)?,
                        serde_json::to_string(&persona.narrative)?
                    ),
                    output_schema: Some(interpreter_schema),
                    source_receipt_ids: vec![],
                    temperature: Some(0.0),
                    max_output_tokens: Some(2600),
                },
            )
            .await?;
            let interpretation: SessionZeroInterpretation = serde_json::from_value(
                interpreter
                    .structured
                    .clone()
                    .ok_or_else(|| anyhow!("interpreter omitted structured output"))?,
            )?;
            (interpretation, interpreter.receipt)
        };
        let delta = SessionZeroDelta {
            contract_patch: interpretation.contract_patch,
            character_patch: interpretation.character_patch,
            decisions: interpretation.decisions,
            dm_speech: persona.narrative,
            suggested_replies: interpretation.suggested_replies,
        };
        validate_dm_delta(state, channel_id, member_id, &delta)?;
        receipts.push(persona.receipt);
        receipts.push(interpreter_receipt);
        Ok((delta, receipts))
    }
}

fn focused_replacement_lane(decision: &SessionZeroDecision) -> Option<FocusedReplacementLane<'_>> {
    match (
        decision.proposed_extraordinary_permission.as_ref(),
        decision.proposed_contract_patch.as_ref(),
        decision.proposed_character_patch.as_ref(),
    ) {
        (Some(permission), None, None) => Some(FocusedReplacementLane::Permission(permission)),
        (None, Some(_), None) => Some(FocusedReplacementLane::Contract),
        (None, None, Some(_)) => Some(FocusedReplacementLane::Character),
        _ => None,
    }
}

async fn run_focused_counter_interpreter(
    model: &dyn ModelPort,
    model_id: &str,
    binding: &str,
    decision: &SessionZeroDecision,
    lane: FocusedReplacementLane<'_>,
    exact_lived_stream: &str,
    persona_response: &str,
) -> Result<(SessionZeroInterpretation, ModelStageReceipt)> {
    let (lane_name, schema, max_output_tokens) = match lane {
        FocusedReplacementLane::Permission(_) => (
            "extraordinary-permission terms",
            serde_json::to_value(schema_for!(FocusedPermissionTerms))?,
            1600,
        ),
        FocusedReplacementLane::Contract => (
            "campaign-contract patch",
            serde_json::to_value(schema_for!(CampaignContractPatch))?,
            1200,
        ),
        FocusedReplacementLane::Character => (
            "private-character patch",
            serde_json::to_value(schema_for!(CharacterDraftPatch))?,
            1800,
        ),
    };
    let interpreter = run_validated_stage(
        model,
        &ModelStageRequest {
            stage: "session_zero_interpreter".into(),
            model: model_id.into(),
            snapshot_binding: binding.into(),
            lived_stream: format!(
                "Translate the player's exact counterproposal into one replacement {lane_name}. Preserve every previous typed field the counter does not change; apply every requested removal or replacement exactly; add nothing. Ghostlight owns decision identity, owner, materiality, evidence bindings, and speech, so emit only the replacement payload described by this schema. The Persona response is social context only and cannot override the exact factual basis. Return one complete JSON object matching the schema.\n\nOUTPUT JSON SCHEMA:\n{}\n\nEXACT FACTUAL BASIS:\n{}\n\nPERSONA RESPONSE:\n{}",
                serde_json::to_string(&schema)?,
                exact_lived_stream,
                serde_json::to_string(persona_response)?,
            ),
            output_schema: Some(schema),
            source_receipt_ids: decision.evidence_receipt_ids.clone(),
            temperature: Some(0.0),
            max_output_tokens: Some(max_output_tokens),
        },
    )
    .await?;
    let structured = interpreter
        .structured
        .clone()
        .ok_or_else(|| anyhow!("focused counter interpreter omitted structured output"))?;
    let mut replacement = SessionZeroDecision {
        schema: "ghostlight.session_zero_decision.v1".into(),
        id: format!("decision:{}", Uuid::new_v4().simple()),
        owner_member_id: decision.owner_member_id.clone(),
        prompt: decision.prompt.clone(),
        proposed_resolution: decision
            .pending_counter
            .clone()
            .ok_or_else(|| anyhow!("focused counter lost its persisted replacement"))?,
        proposed_extraordinary_permission: None,
        proposed_contract_patch: None,
        proposed_character_patch: None,
        evidence_receipt_ids: decision.evidence_receipt_ids.clone(),
        pending_counter: None,
        material: decision.material,
        resolved: false,
    };
    match lane {
        FocusedReplacementLane::Permission(previous) => {
            let terms: FocusedPermissionTerms = serde_json::from_value(structured)?;
            let mut permission = previous.clone();
            permission.name = terms.name;
            permission.reliable_scope = terms.reliable_scope;
            permission.prerequisites = terms.prerequisites;
            permission.costs = terms.costs;
            permission.limits = terms.limits;
            permission.exposure = terms.exposure;
            permission.effect_ceiling = terms.effect_ceiling;
            permission.branch_local = terms.branch_local;
            replacement.proposed_extraordinary_permission = Some(permission);
        }
        FocusedReplacementLane::Contract => {
            let patch: CampaignContractPatch = serde_json::from_value(structured)?;
            if patch == CampaignContractPatch::default() {
                return Err(anyhow!(
                    "focused counter interpreter returned an empty contract patch"
                ));
            }
            replacement.proposed_contract_patch = Some(patch);
        }
        FocusedReplacementLane::Character => {
            let patch: CharacterDraftPatch = serde_json::from_value(structured)?;
            if patch == CharacterDraftPatch::default() {
                return Err(anyhow!(
                    "focused counter interpreter returned an empty character patch"
                ));
            }
            replacement.proposed_character_patch = Some(patch);
        }
    }
    Ok((
        SessionZeroInterpretation {
            decisions: vec![replacement],
            ..Default::default()
        },
        interpreter.receipt,
    ))
}

fn require_typed_decision_payloads(schema: &mut serde_json::Value) -> Result<()> {
    let decision_schema = schema
        .get_mut("$defs")
        .and_then(|defs| defs.get_mut("SessionZeroDecision"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("Session Zero Interpreter schema omitted its decision contract"))?;
    decision_schema.insert(
        "anyOf".into(),
        serde_json::json!([
            {
                "required": ["proposed_extraordinary_permission"],
                "properties": {"proposed_extraordinary_permission": {"type": "object"}}
            },
            {
                "required": ["proposed_contract_patch"],
                "properties": {"proposed_contract_patch": {"type": "object"}}
            },
            {
                "required": ["proposed_character_patch"],
                "properties": {"proposed_character_patch": {"type": "object"}}
            }
        ]),
    );
    Ok(())
}

fn require_single_lane_ownership(schema: &mut serde_json::Value) -> Result<()> {
    let root = schema
        .as_object_mut()
        .ok_or_else(|| anyhow!("Session Zero Interpreter schema has no object root"))?;
    root.insert(
        "allOf".into(),
        serde_json::json!([
            {
                "if": {
                    "properties": {
                        "decisions": {
                            "contains": {
                                "required": ["proposed_contract_patch"],
                                "properties": {"proposed_contract_patch": {"type": "object"}}
                            }
                        }
                    }
                },
                "then": {
                    "properties": {
                        "contract_patch": {"const": CampaignContractPatch::default()}
                    }
                }
            },
            {
                "if": {
                    "properties": {
                        "decisions": {
                            "contains": {
                                "required": ["proposed_character_patch"],
                                "properties": {"proposed_character_patch": {"type": "object"}}
                            }
                        }
                    }
                },
                "then": {
                    "properties": {"character_patch": {"type": "null"}}
                }
            }
        ]),
    );
    Ok(())
}

fn require_channel_access_by_member(
    state: &SessionZeroState,
    channel_id: &str,
    member_id: Option<&str>,
) -> Result<()> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| anyhow!("channel does not exist"))?;
    match channel.kind {
        SessionZeroChannelKind::SharedTable => Ok(()),
        SessionZeroChannelKind::PrivateDm if channel.member_id.as_deref() == member_id => Ok(()),
        SessionZeroChannelKind::PrivateDm => Err(anyhow!("private channel access denied")),
    }
}

fn public_character_projection(draft: &CharacterDraft) -> serde_json::Value {
    serde_json::json!({
        "member_id": draft.member_id,
        "actor_id": draft.actor_id,
        "name": draft.name,
        "public_premise": draft.public_premise,
    })
}

fn project_focused_counter_lived_stream(
    state: &SessionZeroState,
    member_id: Option<&str>,
    decision_id: &str,
) -> Result<String> {
    counter_replacement_focus(state, member_id, Some(decision_id))?
        .ok_or_else(|| anyhow!("focused counter projection is missing its turn focus"))?;
    let decision = state
        .decisions
        .get(decision_id)
        .ok_or_else(|| anyhow!("countered decision is missing"))?;
    let mut previous = display_decision_payload(decision);
    if previous.trim().is_empty() {
        previous = if let Some(member_id) = member_id {
            format!(
                "Legacy private character replacement basis:\n{}",
                display_character_ledger(
                    state
                        .character_drafts
                        .get(member_id)
                        .ok_or_else(|| anyhow!("private character draft is missing"))?
                )
            )
        } else {
            format!(
                "Legacy campaign-contract replacement basis:\nPremise: {}\nCanon horizon: {}\nStarting location: {}\nStarting time: {}\nStarting pressure: {}\nDesired goal: {}\nTone: {}\nThemes: {}\nPacing: {}\nConsequence style: {}\nNarrative focus: {}\nParty bonds: {}\nInternal tension: {}\nDM style: {}",
                state.contract.premise,
                state.contract.canon_horizon,
                state.contract.starting_where,
                state.contract.starting_when,
                state.contract.starting_pressure,
                state.contract.desired_goal,
                display_list(&state.contract.tone),
                display_list(&state.contract.themes),
                state.contract.pacing,
                state.contract.consequence_style,
                state.contract.narrative_focus,
                display_list(&state.contract.party_bonds),
                state.contract.internal_tension,
                state.contract.dm_style,
            )
        };
    }
    let counter = decision
        .pending_counter
        .as_deref()
        .ok_or_else(|| anyhow!("countered decision has no pending counterproposal"))?;
    let boundaries = if state.aggregate_boundaries.is_empty() {
        "No campaign content boundaries are currently recorded.".to_owned()
    } else {
        state
            .aggregate_boundaries
            .iter()
            .map(|boundary| format!("{}: {:?}", boundary.display_topic, boundary.level))
            .collect::<Vec<_>>()
            .join("; ")
    };
    Ok(format!(
        "A player has rejected one material typed proposal in their Session Zero channel. Nothing is accepted yet. Resolve only this replacement and invite the player to review the fresh typed decision.\n\nPrevious typed proposal, rendered exactly:\n{previous}\n\nPlayer's exact counterproposal:\n{counter}\n\nCampaign content boundaries:\n{boundaries}"
    ))
}

fn permitted_dm_context(
    state: &SessionZeroState,
    channel_id: &str,
    member_id: Option<&str>,
    supersedes_countered_decision_id: Option<&str>,
) -> Result<serde_json::Value> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| anyhow!("channel does not exist"))?;
    if let Some(decision_id) = supersedes_countered_decision_id {
        let turn_focus = counter_replacement_focus(state, member_id, Some(decision_id))?
            .expect("a supplied countered decision produces a turn focus");
        let countered_decision = state
            .decisions
            .get(decision_id)
            .expect("the counter focus validated the decision");
        let mut value = serde_json::json!({
            "session_id": state.id,
            "revision": state.revision,
            "channel_kind": channel.kind,
            "member_id": member_id,
            "aggregate_boundaries": state.aggregate_boundaries,
            "turn_focus": turn_focus,
            "countered_decision": countered_decision,
        });
        if !decision_has_typed_payload(countered_decision) {
            if channel.kind == SessionZeroChannelKind::PrivateDm {
                let member_id = member_id.ok_or_else(|| anyhow!("private member is missing"))?;
                value["legacy_replacement_basis"] = serde_json::to_value(
                    state
                        .character_drafts
                        .get(member_id)
                        .ok_or_else(|| anyhow!("private character draft is missing"))?,
                )?;
            } else {
                value["legacy_replacement_basis"] = serde_json::to_value(&state.contract)?;
            }
        }
        return Ok(value);
    }
    let recent_messages = channel
        .message_ids
        .iter()
        .rev()
        .take(16)
        .rev()
        .filter_map(|id| state.messages.get(id))
        .map(|message| {
            serde_json::json!({
                "speaker": message.speaker,
                "author_member_id": message.author_member_id,
                "text": message.text,
            })
        })
        .collect::<Vec<_>>();
    let public_party = state
        .members
        .values()
        .filter(|member| member.active)
        .map(|member| public_character_projection(&state.character_drafts[&member.id]))
        .collect::<Vec<_>>();
    let visible_decisions = visible_decisions_for_turn(state, member_id, None)?;
    let mut value = serde_json::json!({
        "session_id": state.id,
        "revision": state.revision,
        "shared_epoch": state.shared_epoch,
        "boundary_epoch": state.boundary_epoch,
        "status": state.status,
        "contract": state.contract,
        "aggregate_boundaries": state.aggregate_boundaries,
        "public_party": public_party,
        "unresolved_decisions": visible_decisions,
        "turn_focus": null,
        "recent_messages": recent_messages,
        "evidence_coverage": state.preview_evidence_coverage,
    });
    if channel.kind == SessionZeroChannelKind::PrivateDm {
        let member_id = member_id.ok_or_else(|| anyhow!("private member is missing"))?;
        let recent_shared_messages = state
            .channels
            .get("shared:table")
            .into_iter()
            .flat_map(|shared| shared.message_ids.iter().rev().take(8).rev())
            .filter_map(|id| state.messages.get(id))
            .map(|message| {
                serde_json::json!({
                    "speaker": message.speaker,
                    "author_member_id": message.author_member_id,
                    "text": message.text,
                })
            })
            .collect::<Vec<_>>();
        value["recent_shared_messages"] = serde_json::to_value(recent_shared_messages)?;
        value["private_character"] = serde_json::to_value(
            state
                .character_drafts
                .get(member_id)
                .ok_or_else(|| anyhow!("private character draft is missing"))?,
        )?;
        value["private_boundaries"] = serde_json::to_value(
            state
                .boundaries
                .values()
                .filter(|boundary| boundary.owner_member_id == member_id)
                .collect::<Vec<_>>(),
        )?;
    }
    Ok(value)
}

fn permitted_interpreter_context(
    state: &SessionZeroState,
    channel_id: &str,
    member_id: Option<&str>,
    supersedes_countered_decision_id: Option<&str>,
) -> Result<serde_json::Value> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| anyhow!("channel does not exist"))?;
    if let Some(decision_id) = supersedes_countered_decision_id {
        let turn_focus = counter_replacement_focus(state, member_id, Some(decision_id))?
            .expect("a supplied countered decision produces a turn focus");
        let countered_decision = state
            .decisions
            .get(decision_id)
            .expect("the counter focus validated the decision");
        let mut value = serde_json::json!({
            "channel_kind": channel.kind,
            "member_id": member_id,
            "turn_focus": turn_focus,
            "countered_decision": countered_decision,
        });
        if !decision_has_typed_payload(countered_decision) {
            if channel.kind == SessionZeroChannelKind::PrivateDm {
                let member_id = member_id.ok_or_else(|| anyhow!("private member is missing"))?;
                value["legacy_replacement_basis"] = serde_json::to_value(
                    state
                        .character_drafts
                        .get(member_id)
                        .ok_or_else(|| anyhow!("private character draft is missing"))?,
                )?;
            } else {
                value["legacy_replacement_basis"] = serde_json::to_value(&state.contract)?;
            }
        }
        return Ok(value);
    }
    let visible_decisions = visible_decisions_for_turn(state, member_id, None)?;
    let mut value = serde_json::json!({
        "channel_kind": channel.kind,
        "member_id": member_id,
        "current_contract": state.contract,
        "existing_visible_decisions": visible_decisions,
        "turn_focus": null,
    });
    if channel.kind == SessionZeroChannelKind::PrivateDm {
        let member_id = member_id.ok_or_else(|| anyhow!("private member is missing"))?;
        value["current_private_character"] = serde_json::to_value(
            state
                .character_drafts
                .get(member_id)
                .ok_or_else(|| anyhow!("private character draft is missing"))?,
        )?;
    }
    Ok(value)
}

fn visible_decisions_for_turn<'a>(
    state: &'a SessionZeroState,
    member_id: Option<&str>,
    supersedes_countered_decision_id: Option<&str>,
) -> Result<Vec<&'a SessionZeroDecision>> {
    if let Some(decision_id) = supersedes_countered_decision_id {
        return Ok(vec![
            state
                .decisions
                .get(decision_id)
                .ok_or_else(|| anyhow!("countered decision is missing"))?,
        ]);
    }
    Ok(state
        .decisions
        .values()
        .filter(|decision| {
            decision.material
                && !decision.resolved
                && (decision.owner_member_id.is_none()
                    || decision.owner_member_id.as_deref() == member_id)
        })
        .collect())
}

fn counter_replacement_focus(
    state: &SessionZeroState,
    member_id: Option<&str>,
    supersedes_countered_decision_id: Option<&str>,
) -> Result<Option<serde_json::Value>> {
    let Some(decision_id) = supersedes_countered_decision_id else {
        return Ok(None);
    };
    let decision = state
        .decisions
        .get(decision_id)
        .ok_or_else(|| anyhow!("countered decision is missing"))?;
    if decision.resolved {
        return Err(anyhow!("countered decision is already resolved"));
    }
    if decision.owner_member_id.as_deref() != member_id {
        return Err(anyhow!(
            "countered decision is outside this channel's authority"
        ));
    }
    let pending_counter = decision
        .pending_counter
        .as_deref()
        .ok_or_else(|| anyhow!("countered decision has no pending counterproposal"))?;
    Ok(Some(serde_json::json!({
        "kind":"counter_replacement",
        "decision_id":decision.id,
        "original_prompt":decision.prompt,
        "pending_counter":pending_counter,
        "required_owner_member_id":decision.owner_member_id,
        "required_material":decision.material,
        "required_output":"exactly one fresh unresolved decision with a non-empty typed payload",
    })))
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "—".into()
    } else {
        values.join("; ")
    }
}

fn display_relationships(values: &BTreeMap<String, String>) -> String {
    if values.is_empty() {
        "—".into()
    } else {
        values
            .iter()
            .map(|(subject, relationship)| format!("{subject}: {relationship}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn display_extraordinary_permission(permission: &ExtraordinaryPermission) -> String {
    format!(
        "{}\n  Reliable scope: {}\n  Prerequisites: {}\n  Costs: {}\n  Limits: {}\n  Exposure: {}\n  Effect ceiling: {}\n  Branch-local: {}",
        permission.name,
        permission.reliable_scope,
        display_list(&permission.prerequisites),
        display_list(&permission.costs),
        display_list(&permission.limits),
        display_list(&permission.exposure),
        permission.effect_ceiling,
        if permission.branch_local { "yes" } else { "no" },
    )
}

fn display_character_ledger(character: &CharacterDraft) -> String {
    let permissions = if character.extraordinary_permissions.is_empty() {
        "—".into()
    } else {
        character
            .extraordinary_permissions
            .iter()
            .map(display_extraordinary_permission)
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "Name: {}\nPublic premise: {}\nPrivate history: {}\nSecrets: {}\nCapabilities: {}\nKnowledge: {}\nEquipment: {}\nRelationships: {}\nObligations: {}\nVulnerabilities: {}\nGoals: {}\nExtraordinary permissions:\n{}",
        character.name,
        character.public_premise,
        display_list(&character.private_history),
        display_list(&character.secrets),
        display_list(&character.capabilities),
        display_list(&character.knowledge),
        display_list(&character.equipment),
        display_relationships(&character.relationships),
        display_list(&character.obligations),
        display_list(&character.vulnerabilities),
        display_list(&character.goals),
        permissions,
    )
}

fn display_patch_list(values: &[String]) -> String {
    if values.is_empty() {
        "(empty list)".into()
    } else {
        values.join("; ")
    }
}

fn display_contract_patch(patch: &CampaignContractPatch) -> String {
    let mut lines = Vec::new();
    macro_rules! push_string {
        ($field:ident, $label:literal) => {
            if let Some(value) = &patch.$field {
                lines.push(format!(concat!($label, ": {}"), value));
            }
        };
    }
    macro_rules! push_list {
        ($field:ident, $label:literal) => {
            if let Some(values) = &patch.$field {
                lines.push(format!(concat!($label, ": {}"), display_patch_list(values)));
            }
        };
    }
    push_string!(premise, "Premise");
    push_string!(canon_horizon, "Canon horizon");
    push_string!(starting_where, "Starting location");
    push_string!(starting_when, "Starting time");
    push_string!(starting_pressure, "Starting pressure");
    push_string!(desired_goal, "Desired goal");
    push_list!(tone, "Tone");
    push_list!(themes, "Themes");
    push_string!(pacing, "Pacing");
    push_string!(consequence_style, "Consequence style");
    push_string!(narrative_focus, "Narrative focus");
    push_list!(party_bonds, "Party bonds");
    push_string!(internal_tension, "Internal tension");
    push_string!(dm_style, "DM style");
    lines.join("\n")
}

fn display_character_patch(patch: &CharacterDraftPatch) -> String {
    let mut lines = Vec::new();
    if let Some(name) = &patch.name {
        lines.push(format!("Name: {name}"));
    }
    if let Some(public_premise) = &patch.public_premise {
        lines.push(format!("Public premise: {public_premise}"));
    }
    for (values, label) in [
        (&patch.private_history_add, "Private history to add"),
        (&patch.private_history_remove, "Private history to remove"),
        (&patch.secrets_add, "Secrets to add"),
        (&patch.secrets_remove, "Secrets to remove"),
        (&patch.capabilities_add, "Capabilities to add"),
        (&patch.capabilities_remove, "Capabilities to remove"),
        (&patch.knowledge_add, "Knowledge to add"),
        (&patch.knowledge_remove, "Knowledge to remove"),
        (&patch.equipment_add, "Equipment to add"),
        (&patch.equipment_remove, "Equipment to remove"),
        (&patch.obligations_add, "Obligations to add"),
        (&patch.obligations_remove, "Obligations to remove"),
        (&patch.vulnerabilities_add, "Vulnerabilities to add"),
        (&patch.vulnerabilities_remove, "Vulnerabilities to remove"),
        (&patch.goals_add, "Goals to add"),
        (&patch.goals_remove, "Goals to remove"),
    ] {
        if !values.is_empty() {
            lines.push(format!("{label}: {}", values.join("; ")));
        }
    }
    if !patch.relationships.is_empty() {
        lines.push(format!(
            "Relationships to set: {}",
            display_relationships(&patch.relationships)
        ));
    }
    if !patch.relationship_removals.is_empty() {
        lines.push(format!(
            "Relationship keys to remove: {}",
            patch.relationship_removals.join("; ")
        ));
    }
    lines.join("\n")
}

fn display_decision_payload(decision: &SessionZeroDecision) -> String {
    let mut sections = Vec::new();
    if let Some(permission) = &decision.proposed_extraordinary_permission {
        sections.push(format!(
            "Extraordinary permission:\n{}",
            display_extraordinary_permission(permission)
        ));
    }
    if let Some(patch) = &decision.proposed_contract_patch
        && patch != &CampaignContractPatch::default()
    {
        sections.push(format!(
            "Campaign contract patch:\n{}",
            display_contract_patch(patch)
        ));
    }
    if let Some(patch) = &decision.proposed_character_patch
        && patch != &CharacterDraftPatch::default()
    {
        sections.push(format!(
            "Private character patch:\n{}",
            display_character_patch(patch)
        ));
    }
    sections.join("\n")
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PrivateRelationshipSubjectProjection {
    target_id: String,
    subject_kind: String,
    name: String,
    location_id: Option<String>,
    location_name: Option<String>,
    relationship: String,
}

fn private_relationship_subjects(
    preview: &WorldCompilePreview,
    actor_id: &str,
) -> Vec<PrivateRelationshipSubjectProjection> {
    let campaign = &preview.campaign;
    let Some(actor) = campaign.actors.get(actor_id) else {
        return Vec::new();
    };

    actor
        .relationships
        .iter()
        .map(|(target_id, description)| {
            let (subject_kind, name, location_id) =
                if let Some(target) = campaign.actors.get(target_id) {
                    (
                        "actor",
                        target.name.as_str(),
                        Some(target.location_id.as_str()),
                    )
                } else if let Some(target) = campaign.institutions.get(target_id) {
                    ("institution", target.name.as_str(), None)
                } else if let Some(target) = campaign.gestalts.get(target_id) {
                    (
                        "gestalt",
                        target.name.as_str(),
                        Some(target.home_location_id.as_str()),
                    )
                } else if let Some(member_id) = target_id.strip_prefix("member:") {
                    if let Some(target) = campaign.gestalt_members.get(member_id) {
                        let location_id = target.last_location_id.as_deref().or_else(|| {
                            campaign
                                .gestalts
                                .get(&target.gestalt_id)
                                .map(|gestalt| gestalt.home_location_id.as_str())
                        });
                        ("gestalt_member", target.name.as_str(), location_id)
                    } else {
                        ("unknown", target_id.as_str(), None)
                    }
                } else {
                    ("unknown", target_id.as_str(), None)
                };
            let location_name = location_id
                .and_then(|id| campaign.locations.get(id))
                .map(|location| location.name.as_str());
            PrivateRelationshipSubjectProjection {
                target_id: target_id.clone(),
                subject_kind: subject_kind.into(),
                name: name.into(),
                location_id: location_id.map(str::to_owned),
                location_name: location_name.map(str::to_owned),
                relationship: description.clone(),
            }
        })
        .collect()
}

fn display_world_preview(
    preview: &WorldCompilePreview,
    player_actor_ids: &BTreeSet<String>,
) -> String {
    let campaign = &preview.campaign;
    let mut sections = vec![format!("Opening: {}", preview.title)];
    let topology = campaign
        .locations
        .values()
        .map(|location| {
            let routes = location
                .routes
                .values()
                .map(|route| {
                    format!(
                        "{} ({}; {} minutes)",
                        route.destination_id, route.distance, route.travel_minutes
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{} [{}] · within {} · routes: {} · persistent features: {}",
                location.name,
                location.id,
                location.container_id.as_deref().unwrap_or("root"),
                if routes.is_empty() { "none" } else { &routes },
                if location.persistent_features.is_empty() {
                    "none".into()
                } else {
                    location.persistent_features.join("; ")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!("Topology\n{topology}"));

    let cast = campaign
        .actors
        .values()
        .filter(|actor| {
            !player_actor_ids.contains(&actor.id) && !actor.id.starts_with("relationship-anchor:")
        })
        .map(|actor| format!("{} [{}] · {}", actor.name, actor.id, actor.location_id))
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!(
        "Starting cast\n{}",
        if cast.is_empty() { "none" } else { &cast }
    ));

    let institutions = campaign
        .institutions
        .values()
        .map(|institution| {
            format!(
                "{} [{}] · posture: {}",
                institution.name, institution.id, institution.posture
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!(
        "Institutions\n{}",
        if institutions.is_empty() {
            "none"
        } else {
            &institutions
        }
    ));

    let populations = campaign
        .gestalts
        .values()
        .map(|gestalt| {
            format!(
                "{} [{}] · {} · pressures: {}",
                gestalt.name,
                gestalt.id,
                gestalt.home_location_id,
                if gestalt.pressures.is_empty() {
                    "none".into()
                } else {
                    gestalt.pressures.join("; ")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!(
        "Populations\n{}",
        if populations.is_empty() {
            "none"
        } else {
            &populations
        }
    ));

    let clocks = campaign
        .clocks
        .values()
        .map(|clock| {
            format!(
                "{} [{}] · {}/{} · consequence: {}",
                clock.label, clock.id, clock.progress, clock.threshold, clock.consequence
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    sections.push(format!(
        "World clocks\n{}",
        if clocks.is_empty() { "none" } else { &clocks }
    ));
    sections.join("\n\n")
}

pub fn session_zero_surface(
    state: &SessionZeroState,
    account_hash: &str,
) -> Result<serde_json::Value> {
    let member = state
        .member_for_account(account_hash)
        .ok_or_else(|| anyhow!("session zero membership required"))?;
    let channels = state
        .channels
        .values()
        .filter(|channel| {
            channel.kind == SessionZeroChannelKind::SharedTable
                || channel.member_id.as_deref() == Some(member.id.as_str())
        })
        .map(|channel| {
            let messages = channel
                .message_ids
                .iter()
                .filter_map(|id| state.messages.get(id))
                .collect::<Vec<_>>();
            serde_json::json!({
                "id": channel.id,
                "kind": channel.kind,
                "revision": channel.revision,
                "messages": messages,
            })
        })
        .collect::<Vec<_>>();
    let public_party = state
        .members
        .values()
        .filter(|candidate| candidate.active)
        .map(|candidate| {
            let draft = &state.character_drafts[&candidate.id];
            serde_json::json!({
                "member_id": candidate.id,
                "display_name": candidate.display_name,
                "is_host": candidate.is_host,
                "name": draft.name,
                "public_premise": draft.public_premise,
                "approved": state.approvals.contains_key(&candidate.id),
            })
        })
        .collect::<Vec<_>>();
    let private_boundaries = state
        .boundaries
        .values()
        .filter(|boundary| boundary.owner_member_id == member.id)
        .collect::<Vec<_>>();
    let visible_decisions = state
        .decisions
        .values()
        .filter(|decision| {
            decision.owner_member_id.is_none()
                || decision.owner_member_id.as_deref() == Some(member.id.as_str())
        })
        .collect::<Vec<_>>();
    let character = &state.character_drafts[&member.id];
    let player_actor_ids = state
        .character_drafts
        .values()
        .map(|draft| draft.actor_id.clone())
        .collect::<BTreeSet<_>>();
    let relationship_subjects = state
        .preview
        .as_ref()
        .map(|preview| private_relationship_subjects(preview, &character.actor_id))
        .unwrap_or_default();
    let private_preview = state.preview.as_ref().map(|preview| {
        serde_json::json!({
            "title": preview.title,
            "gaps": preview.gaps,
            "branch_assumptions": preview.branch_assumptions,
            "topology": preview.campaign.locations,
            "cast": preview.campaign.actors.values().filter(|actor| !player_actor_ids.contains(&actor.id) && !actor.id.starts_with("relationship-anchor:")).map(|actor| serde_json::json!({"id":actor.id,"name":actor.name,"location_id":actor.location_id})).collect::<Vec<_>>(),
            "institutions": preview.campaign.institutions.values().map(|institution| serde_json::json!({"id":institution.id,"name":institution.name,"posture":institution.posture})).collect::<Vec<_>>(),
            "populations": preview.campaign.gestalts.values().map(|gestalt| serde_json::json!({"id":gestalt.id,"name":gestalt.name,"home_location_id":gestalt.home_location_id,"pressures":gestalt.pressures})).collect::<Vec<_>>(),
            "clocks": preview.campaign.clocks,
            "evidence_coverage": preview.evidence_coverage,
            "relationship_subjects": relationship_subjects,
            "world_preview_digest": state.world_preview_digest().ok(),
        })
    });
    let roster_summary = state
        .members
        .values()
        .filter(|candidate| candidate.active)
        .map(|candidate| {
            format!(
                "{} · {}{}",
                candidate.display_name,
                if state.approvals.contains_key(&candidate.id) {
                    "ready"
                } else {
                    "not ready"
                },
                if candidate.is_host { " · host" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let contract_summary = format!(
        "Premise: {}\nWhere: {}\nWhen: {}\nPressure: {}\nGoal: {}\nTone: {}",
        state.contract.premise,
        state.contract.starting_where,
        state.contract.starting_when,
        state.contract.starting_pressure,
        state.contract.desired_goal,
        state.contract.tone.join(", ")
    );
    let character_summary = display_character_ledger(character);
    let shared_channel_id = state
        .channels
        .values()
        .find(|channel| channel.kind == SessionZeroChannelKind::SharedTable)
        .map(|channel| channel.id.clone())
        .ok_or_else(|| anyhow!("shared Session Zero channel is missing"))?;
    let private_channel_id = state
        .channels
        .values()
        .find(|channel| channel.member_id.as_deref() == Some(member.id.as_str()))
        .map(|channel| channel.id.clone());
    let transcript = channels
        .iter()
        .flat_map(|channel| {
            let channel_label = if channel["kind"] == "shared_table" { "Table" } else { "Private DM" };
            channel["messages"].as_array().into_iter().flatten().filter_map(move |message| {
                Some(serde_json::json!({
                    "id":format!("session-zero.message.{}", message["id"].as_str()?),
                    "kind":"text.dialogue",
                    "props":{"value":format!("{channel_label} · {}: {}", message["speaker"].as_str().unwrap_or("system"), message["text"].as_str().unwrap_or(""))},
                    "children":[]
                }))
            })
        })
        .collect::<Vec<_>>();
    let mut children = vec![
        serde_json::json!({"id":"session-zero.status","kind":"card","props":{"title":format!("{} · revision {}", session_status_label(&state.status), state.revision)},"children":[]}),
        serde_json::json!({"id":"session-zero.roster","kind":"card","props":{"title":format!("Roster · {} pooled cells",state.pooled_cell_allowance())},"children":[{"id":"session-zero.roster.text","kind":"text","props":{"value":roster_summary},"children":[]}]}),
        serde_json::json!({"id":"session-zero.contract","kind":"card","props":{"title":"Campaign contract"},"children":[{"id":"session-zero.contract.text","kind":"text","props":{"value":contract_summary},"children":[]}]}),
        serde_json::json!({"id":"session-zero.character","kind":"card","props":{"title":"Your private character"},"children":[{"id":"session-zero.character.text","kind":"text","props":{"value":character_summary},"children":[]}]}),
        serde_json::json!({"id":"session-zero.conversation","kind":"card","props":{"title":"Session Zero conversation"},"children":transcript}),
        serde_json::json!({
            "id":"session-zero.channel","kind":"control.select","props":{"label":"Speak at","value":shared_channel_id},
            "stateBindings":[local_draft_binding("channel_id", "choice")],
            "children":[
                {"id":"channel.shared","kind":"control.option","props":{"value":shared_channel_id,"label":"Shared table"},"children":[]},
                {"id":"channel.private","kind":"control.option","props":{"value":private_channel_id.clone().unwrap_or_default(),"label":"Private DM","disabled":private_channel_id.is_none()},"children":[]}
            ]
        }),
        serde_json::json!({
            "id":"session-zero.message.draft","kind":"control.input.textarea","props":{"label":"Tell the DM or table","rows":4,"placeholder":"Describe what you want, ask a question, or counter a proposal."},
            "stateBindings":[local_draft_binding("text", "string")],"children":[]
        }),
        command_control(
            "session-zero.message.send",
            "Send",
            "session_zero.message.send",
            serde_json::json!({"expected_revision":state.revision}),
            &["channel_id", "text"],
        ),
        serde_json::json!({"id":"session-zero.boundary.topic","kind":"control.input.text","props":{"label":"Private boundary topic","placeholder":"What should the campaign avoid, veil, or ask about first?"},"stateBindings":[local_draft_binding("topic", "string")],"children":[]}),
        serde_json::json!({"id":"session-zero.boundary.level","kind":"control.select","props":{"label":"Boundary level","value":"ask_first"},"stateBindings":[local_draft_binding("level", "choice")],"children":[
            {"id":"boundary.ask-first","kind":"control.option","props":{"value":"ask_first","label":"Ask first"},"children":[]},
            {"id":"boundary.veil","kind":"control.option","props":{"value":"veil","label":"Veil"},"children":[]},
            {"id":"boundary.line","kind":"control.option","props":{"value":"line","label":"Line"},"children":[]}
        ]}),
        command_control(
            "session-zero.boundary.set",
            "Add private boundary",
            "session_zero.boundary.set",
            serde_json::json!({"expected_revision":state.revision}),
            &["topic", "level"],
        ),
        serde_json::json!({"id":"session-zero.decision.counter","kind":"control.input.textarea","props":{"label":"Counterproposal","rows":3,"maxLength":2000,"placeholder":"State the replacement you want the table or DM to consider (up to 2,000 characters)."},"stateBindings":[local_draft_binding("counter", "string")],"children":[]}),
    ];
    for boundary in &private_boundaries {
        children.push(serde_json::json!({
            "id":format!("session-zero.boundary.{}",boundary.id),
            "kind":"card",
            "props":{"title":format!("Your boundary · {:?}",boundary.level)},
            "children":[
                {"id":format!("session-zero.boundary.{}.topic",boundary.id),"kind":"text","props":{"value":boundary.topic},"children":[]},
                command_control(&format!("session-zero.boundary.{}.remove",boundary.id), "Remove boundary", "session_zero.boundary.remove", serde_json::json!({"expected_revision":state.revision,"target":boundary.id}), &[])
            ]
        }));
    }
    for decision in visible_decisions
        .iter()
        .filter(|decision| !decision.resolved)
    {
        let mut decision_children = vec![serde_json::json!({
            "id":format!("session-zero.decision.{}.proposal",decision.id),
            "kind":"text",
            "props":{"value":decision.pending_counter.as_ref().unwrap_or(&decision.proposed_resolution)},
            "children":[]
        })];
        let payload_summary = display_decision_payload(decision);
        if !payload_summary.is_empty() {
            decision_children.push(serde_json::json!({
                "id":format!("session-zero.decision.{}.typed-payload",decision.id),
                "kind":"text",
                "props":{"value":format!("{}:\n{payload_summary}", if decision.pending_counter.is_some() { "Retired typed change" } else { "Exact typed change" })},
                "children":[]
            }));
        }
        if decision.pending_counter.is_some() {
            decision_children.push(serde_json::json!({
                "id":format!("session-zero.decision.{}.pending",decision.id),
                "kind":"text",
                "props":{"value":"Counterproposal recorded. The previous typed proposal has been retired; the DM is preparing a fresh decision."},
                "children":[]
            }));
            decision_children.push(command_control(
                &format!("session-zero.decision.{}.retry-counter", decision.id),
                "Revise / retry counter",
                "session_zero.decision.resolve",
                serde_json::json!({"expected_revision":state.revision,"decision_id":decision.id,"accept":false}),
                &[],
            ));
        } else {
            if decision_has_typed_payload(decision) {
                decision_children.push(command_control(&format!("session-zero.decision.{}.accept",decision.id), "Accept", "session_zero.decision.resolve", serde_json::json!({"expected_revision":state.revision,"decision_id":decision.id,"accept":true,"counter":null}), &[]));
            } else {
                decision_children.push(serde_json::json!({
                    "id":format!("session-zero.decision.{}.missing-payload",decision.id),
                    "kind":"text",
                    "props":{"value":"This discussion has no typed state change attached and cannot be accepted. Counter it or ask the DM for an exact proposal."},
                    "children":[]
                }));
            }
            decision_children.extend([
                command_control(&format!("session-zero.decision.{}.counter",decision.id), "Counter", "session_zero.decision.resolve", serde_json::json!({"expected_revision":state.revision,"decision_id":decision.id,"accept":false}), &["counter"]),
                command_control(&format!("session-zero.decision.{}.discuss",decision.id), "Discuss", "session_zero.message.send", serde_json::json!({"expected_revision":state.revision,"text":format!("I want to discuss this proposal before deciding: {}",decision.prompt)}), &["channel_id"]),
            ]);
        }
        children.push(serde_json::json!({
            "id":format!("session-zero.decision.{}",decision.id),
            "kind":"card",
            "props":{"title":decision.prompt},
            "children":decision_children
        }));
    }
    if member.is_host && !state.roster_locked {
        children.push(serde_json::json!({"id":"session-zero.invite-count","kind":"control.input.number","props":{"label":"Invitations","value":1,"min":1,"max":7},"stateBindings":[local_draft_binding("count", "number")],"children":[]}));
        children.push(command_control(
            "session-zero.invites.create",
            "Create invitations",
            "session_zero.invites.create",
            serde_json::json!({}),
            &["count"],
        ));
        children.push(command_control(
            "session-zero.roster.lock",
            "Lock roster",
            "session_zero.roster.lock",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
        for candidate in state
            .members
            .values()
            .filter(|candidate| candidate.active && candidate.id != member.id)
        {
            children.push(command_control(
                &format!("session-zero.member.{}.remove", candidate.id),
                &format!("Remove {}", candidate.display_name),
                "session_zero.member.remove",
                serde_json::json!({"expected_revision":state.revision,"target":candidate.id}),
                &[],
            ));
        }
    }
    if !member.is_host && !state.roster_locked {
        children.push(command_control(
            "session-zero.leave",
            "Leave Session Zero",
            "session_zero.leave",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
    }
    if let Some(preview) = state.preview.as_ref() {
        children.push(serde_json::json!({
            "id":"session-zero.world-preview",
            "kind":"card",
            "props":{"title":"Compiled world preview"},
            "children":[{
                "id":"session-zero.world-preview.text",
                "kind":"text",
                "props":{"value":display_world_preview(preview, &player_actor_ids)},
                "children":[]
            }]
        }));
        if !preview.evidence_coverage.is_empty() {
            let evidence_summary = preview
                .evidence_coverage
                .iter()
                .map(|coverage| {
                    format!(
                        "{} · {:?}\n{}",
                        coverage.source_id, coverage.lane, coverage.rationale
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            children.push(serde_json::json!({
                "id":"session-zero.evidence-coverage",
                "kind":"card",
                "props":{"title":"Vault evidence use"},
                "children":[{
                    "id":"session-zero.evidence-coverage.text",
                    "kind":"text",
                    "props":{"value":evidence_summary},
                    "children":[]
                }]
            }));
        }
        if !preview.branch_assumptions.is_empty() {
            children.push(serde_json::json!({
                "id":"session-zero.branch-assumptions",
                "kind":"card",
                "props":{"title":"Branch-local assumptions requiring approval"},
                "children":[{
                    "id":"session-zero.branch-assumptions.text",
                    "kind":"text",
                    "props":{"value":preview.branch_assumptions.iter().map(|assumption|format!("• {assumption}")).collect::<Vec<_>>().join("\n")},
                    "children":[]
                }]
            }));
        }
    }
    if let Some(preview) = state.preview.as_ref()
        && !preview.gaps.is_empty()
    {
        children.push(serde_json::json!({
            "id":"session-zero.evidence-gaps",
            "kind":"card",
            "props":{"title":"Evidence gaps requiring table approval"},
            "children":[{
                "id":"session-zero.evidence-gaps.text",
                "kind":"text",
                "props":{"value":preview.gaps.iter().map(|gap|format!("• {gap}")).collect::<Vec<_>>().join("\n")},
                "children":[]
            }]
        }));
    }
    if member.is_host
        && matches!(
            state.status,
            SessionZeroStatus::RosterLocked | SessionZeroStatus::Drafting
        )
        && state.preview.is_none()
        && state.compilation_brief().is_ok()
    {
        children.push(command_control(
            "session-zero.compile",
            "Compile campaign preview",
            "session_zero.compile",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
    }
    if member.is_host
        && state.preview.is_some()
        && matches!(
            state.status,
            SessionZeroStatus::Drafting
                | SessionZeroStatus::RosterLocked
                | SessionZeroStatus::Review
        )
    {
        children.push(command_control(
            "session-zero.discard-preview",
            "Discard this preview and continue drafting",
            "session_zero.preview.discard",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
    }
    if member.is_host
        && matches!(
            state.status,
            SessionZeroStatus::RosterLocked | SessionZeroStatus::Drafting
        )
        && state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.gaps.is_empty())
    {
        children.push(command_control(
            "session-zero.review-compilation-gaps",
            "Review this exact preview with its evidence gaps",
            "session_zero.compilation_gaps.review",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
    }
    if state.status == SessionZeroStatus::Review {
        if !relationship_subjects.is_empty() {
            let relationship_summary = relationship_subjects
                .iter()
                .map(|subject| {
                    let location = subject
                        .location_name
                        .as_deref()
                        .or(subject.location_id.as_deref())
                        .map(|value| format!(" · located at {value}"))
                        .unwrap_or_default();
                    format!(
                        "{} · {}{}\n{}",
                        subject.name, subject.subject_kind, location, subject.relationship
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            children.push(serde_json::json!({
                "id":"session-zero.private-relationship-subjects",
                "kind":"card",
                "props":{"title":"Your private compiled relationships"},
                "children":[{
                    "id":"session-zero.private-relationship-subjects.text",
                    "kind":"text",
                    "props":{"value":relationship_summary},
                    "children":[]
                }]
            }));
        }
        children.push(command_control(
            "session-zero.approve",
            if state.approvals.contains_key(&member.id) {
                "Approved"
            } else {
                "Approve exact world preview, shared contract, and private draft"
            },
            "session_zero.approve",
            serde_json::json!({"expected_revision":state.revision}),
            &[],
        ));
        if member.is_host && state.approved_brief().is_ok() {
            children.push(command_control(
                "session-zero.publish",
                "Publish campaign",
                "session_zero.publish",
                serde_json::json!({"expected_revision":state.revision}),
                &[],
            ));
        }
    }
    children.push(command_control(
        "ghostlight.logout",
        "Sign out",
        "app.auth.logout",
        serde_json::json!({}),
        &[],
    ));
    Ok(serde_json::json!({
        "type": "surface-state",
        "schema": "gamecult.eve.surface.v1",
        "providerId": "gamecult.ghostlight.dungeon",
        "providerKind": "narrative.session-zero",
        "title": state.name,
        "version": state.revision,
        "updatedAtUtc": Utc::now().to_rfc3339(),
        "session_zero": {
            "id": state.id,
            "status": state.status,
            "revision": state.revision,
            "roster_locked": state.roster_locked,
            "viewer_member_id": member.id,
            "viewer_is_host": member.is_host,
            "active_members": public_party.len(),
            "pooled_cell_allowance": state.pooled_cell_allowance(),
            "contract": state.contract,
            "aggregate_boundaries": state.aggregate_boundaries,
            "public_party": public_party,
            "private_character": state.character_drafts[&member.id],
            "private_boundaries": private_boundaries,
            "decisions": visible_decisions,
            "channels": channels,
            "preview": private_preview,
            "shared_digest": state.shared_digest()?,
            "character_digest": state.character_digest(&member.id)?,
            "world_preview_digest": state.world_preview_digest().ok(),
            "approved": state.approvals.contains_key(&member.id),
            "publish_ready": state.approved_brief().is_ok(),
        },
        "surface": {
            "id": "ghostlight.play",
            "root": {
                "id": "session-zero.root",
                "kind": "surface",
                "props": {},
                "children": children
            },
            "styles": {"tokens": {
                "colorBackground": "#0c1110",
                "colorPanel": "#17201d",
                "colorText": "#e8e1cf",
                "colorMuted": "#9aa69f",
                "colorAccent": "#d49b58"
            }}
        },
        "commands": [
            eve_command("session_zero.message.send", "ghostlight.session_zero_message_send.v1", &["channel_id","text"]),
            eve_command("session_zero.boundary.set", "ghostlight.session_zero_boundary_set.v1", &["topic","level"]),
            eve_command("session_zero.boundary.remove", "ghostlight.session_zero_boundary_remove.v1", &[]),
            eve_command("session_zero.decision.resolve", "ghostlight.session_zero_decision_resolve.v1", &[]),
            eve_command("session_zero.invites.create", "ghostlight.session_zero_invites_create.v1", &["count"]),
            eve_command("session_zero.leave", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.member.remove", "ghostlight.session_zero_member_remove.v1", &[]),
            eve_command("session_zero.roster.lock", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.compile", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.compilation_gaps.review", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.preview.discard", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.approve", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("session_zero.publish", "ghostlight.session_zero_revision_command.v1", &[]),
            eve_command("app.auth.logout", "ghostlight.app_logout.v1", &[])
        ]
    }))
}

fn local_draft_binding(name: &str, value_kind: &str) -> serde_json::Value {
    serde_json::json!({
        "targetProp":"value",
        "pointerId":format!("draft:{name}"),
        "sourceId":"renderer",
        "schemaId":"gamecult.eve.local_draft.v1",
        "routeKind":"local",
        "bindingName":name,
        "documentId":"ghostlight.play.drafts",
        "fieldPath":name,
        "valueKind":value_kind,
        "accessMode":"local-draft",
        "authority":"renderer-ephemeral"
    })
}

fn command_control(
    id: &str,
    label: &str,
    command: &str,
    action: serde_json::Value,
    bindings: &[&str],
) -> serde_json::Value {
    let mut action = action.as_object().cloned().unwrap_or_default();
    action.insert("command".into(), serde_json::Value::String(command.into()));
    serde_json::json!({
        "id":id,
        "kind":"control.button",
        "props":{"label":label,"command":command,"action":action,"captureBindings":bindings},
        "children":[]
    })
}

fn eve_command(command: &str, payload_schema: &str, bindings: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "schema":"gamecult.eve.command.v1",
        "command":command,
        "payloadSchema":payload_schema,
        "captureBindings":bindings,
        "transport":"https-json",
        "authority":"SessionZeroKernel"
    })
}

fn session_status_label(status: &SessionZeroStatus) -> &'static str {
    match status {
        SessionZeroStatus::Drafting => "Drafting",
        SessionZeroStatus::RosterLocked => "Roster locked",
        SessionZeroStatus::Compiling => "Compiling",
        SessionZeroStatus::Review => "Review",
        SessionZeroStatus::Published => "Published",
        SessionZeroStatus::Archived => "Archived",
    }
}

fn execute(
    store: &CampaignStore,
    state_id: Uuid,
    command: SessionZeroCommand,
) -> Result<SessionZeroCommandResult> {
    let key = state_id.to_string();
    let (row, mut state) = store
        .load::<SessionZeroState>("session_zero.v1", &key)?
        .ok_or_else(|| anyhow!("session zero state vanished"))?;
    let mut invite_tokens = Vec::new();
    match command {
        SessionZeroCommand::CreateInvites {
            actor_account_hash,
            count,
        } => {
            require_host(&state, &actor_account_hash)?;
            if state.roster_locked {
                return Err(anyhow!("roster is locked"));
            }
            let available = MAX_SESSION_ZERO_MEMBERS.saturating_sub(
                state
                    .members
                    .values()
                    .filter(|member| member.active)
                    .count(),
            );
            let count = usize::from(count);
            if count == 0 || count > available {
                return Err(anyhow!("invite count exceeds available seats"));
            }
            for _ in 0..count {
                let token = Uuid::new_v4().simple().to_string();
                let token_hash = secret_hash(&token);
                let id = format!("invite:{}", Uuid::new_v4().simple());
                state.invites.insert(
                    id.clone(),
                    SessionZeroInvite {
                        schema: "ghostlight.session_zero_invite.v1".into(),
                        id,
                        token_hash,
                        expires_at: Utc::now() + Duration::days(7),
                        consumed_by_member_id: None,
                    },
                );
                invite_tokens.push(token);
            }
        }
        SessionZeroCommand::Join {
            token,
            account_hash,
            display_name,
            cell_allowance,
        } => {
            if state.roster_locked {
                return Err(anyhow!("roster is locked"));
            }
            if state.member_for_account(&account_hash).is_some() {
                return Err(anyhow!("account is already a member"));
            }
            if state
                .members
                .values()
                .filter(|member| member.active)
                .count()
                >= MAX_SESSION_ZERO_MEMBERS
            {
                return Err(anyhow!("session zero is full"));
            }
            validate_bounded("display name", &display_name, 1, 80)?;
            if cell_allowance == 0 || cell_allowance > OPERATOR_CELL_CEILING {
                return Err(anyhow!("Persona-cell entitlement is out of range"));
            }
            let token_hash = secret_hash(&token);
            let invite = state
                .invites
                .values_mut()
                .find(|invite| {
                    invite.token_hash == token_hash
                        && invite.expires_at > Utc::now()
                        && invite.consumed_by_member_id.is_none()
                })
                .ok_or_else(|| anyhow!("invite is invalid, expired, or consumed"))?;
            let member_id = format!("member:{}", Uuid::new_v4().simple());
            invite.consumed_by_member_id = Some(member_id.clone());
            state.members.insert(
                member_id.clone(),
                SessionZeroMember {
                    schema: "ghostlight.session_zero_member.v1".into(),
                    id: member_id.clone(),
                    account_hash,
                    display_name: display_name.clone(),
                    is_host: false,
                    active: true,
                    cell_allowance,
                    joined_at: Utc::now(),
                },
            );
            let channel_id = format!("private:{member_id}");
            state.channels.insert(
                channel_id.clone(),
                SessionZeroChannel {
                    schema: "ghostlight.session_zero_channel.v1".into(),
                    id: channel_id,
                    kind: SessionZeroChannelKind::PrivateDm,
                    member_id: Some(member_id.clone()),
                    revision: 0,
                    message_ids: vec![],
                },
            );
            state.character_drafts.insert(
                member_id.clone(),
                CharacterDraft {
                    schema: "ghostlight.character_draft.v1".into(),
                    member_id: member_id.clone(),
                    actor_id: format!("player:{}", Uuid::new_v4().simple()),
                    name: display_name,
                    ..CharacterDraft::default()
                },
            );
            state.character_epochs.insert(member_id, 0);
            shared_changed(&mut state);
        }
        SessionZeroCommand::Leave {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            if state.roster_locked {
                return Err(anyhow!("roster is locked"));
            }
            let member_id = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?
                .id
                .clone();
            if member_id == state.host_member_id {
                return Err(anyhow!(
                    "the host must archive rather than abandon the draft"
                ));
            }
            state
                .members
                .get_mut(&member_id)
                .expect("member exists")
                .active = false;
            state.approvals.remove(&member_id);
            shared_changed(&mut state);
        }
        SessionZeroCommand::RemoveMember {
            actor_account_hash,
            expected_revision,
            member_id,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            if state.roster_locked {
                return Err(anyhow!("roster is locked"));
            }
            if member_id == state.host_member_id {
                return Err(anyhow!("the host cannot remove themselves"));
            }
            let member = state
                .members
                .get_mut(&member_id)
                .ok_or_else(|| anyhow!("member does not exist"))?;
            if !member.active {
                return Err(anyhow!("member has already left"));
            }
            member.active = false;
            state.approvals.remove(&member_id);
            shared_changed(&mut state);
        }
        SessionZeroCommand::PostPlayerMessage {
            actor_account_hash,
            expected_revision,
            channel_id,
            text,
        } => {
            require_revision(&state, expected_revision)?;
            validate_bounded("session zero message", &text, 1, 4_000)?;
            let member = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?
                .clone();
            require_channel_access(&state, &member.id, &channel_id)?;
            append_message(
                &mut state,
                channel_id,
                Some(member.id),
                SessionZeroSpeakerKind::Player,
                text,
            )?;
        }
        SessionZeroCommand::ApplyDmTurn {
            expected_component_epoch,
            expected_channel_revision,
            channel_id,
            member_id,
            supersedes_countered_decision_id,
            delta,
            model_receipts,
        } => {
            let channel = state
                .channels
                .get(&channel_id)
                .ok_or_else(|| anyhow!("session zero channel is missing"))?;
            if channel.revision != expected_channel_revision {
                return Err(anyhow!("stale Session Zero channel projection"));
            }
            let live_component_epoch = match channel.kind {
                SessionZeroChannelKind::SharedTable => state.shared_epoch,
                SessionZeroChannelKind::PrivateDm => *state
                    .character_epochs
                    .get(
                        member_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("private member is missing"))?,
                    )
                    .ok_or_else(|| anyhow!("private character epoch is missing"))?,
            };
            if live_component_epoch != expected_component_epoch {
                return Err(anyhow!("stale Session Zero component projection"));
            }
            validate_dm_delta(&state, &channel_id, member_id.as_deref(), &delta)?;
            if let Some(decision_id) = supersedes_countered_decision_id.as_deref() {
                let countered = state
                    .decisions
                    .get(decision_id)
                    .ok_or_else(|| anyhow!("countered decision is missing"))?;
                if countered.resolved || countered.pending_counter.is_none() {
                    return Err(anyhow!("decision has no pending counterproposal"));
                }
                if countered.owner_member_id.as_deref() != member_id.as_deref() {
                    return Err(anyhow!("countered decision channel owner mismatch"));
                }
                validate_counter_replacement(countered, member_id.as_deref(), &delta)?;
            }
            let contract_changed = delta.contract_patch != CampaignContractPatch::default();
            let decisions_changed = !delta.decisions.is_empty();
            apply_contract_patch(&mut state.contract, delta.contract_patch);
            if let Some(patch) = delta.character_patch {
                let owner = member_id.as_deref().ok_or_else(|| {
                    anyhow!("shared DM turn cannot mutate private character state")
                })?;
                let draft = state
                    .character_drafts
                    .get_mut(owner)
                    .ok_or_else(|| anyhow!("character draft is missing"))?;
                apply_character_patch(draft, patch);
                character_changed(&mut state, owner);
            }
            for mut decision in delta.decisions {
                if decision.id.trim().is_empty() {
                    decision.id = format!("decision:{}", Uuid::new_v4().simple());
                }
                state.decisions.insert(decision.id.clone(), decision);
            }
            if let Some(decision_id) = supersedes_countered_decision_id {
                state
                    .decisions
                    .get_mut(&decision_id)
                    .expect("countered decision was just validated")
                    .resolved = true;
            }
            if contract_changed || (decisions_changed && member_id.is_none()) {
                shared_changed(&mut state);
            } else if decisions_changed && let Some(owner) = member_id.as_deref() {
                character_changed(&mut state, owner);
            }
            if !delta.dm_speech.trim().is_empty() {
                append_message(
                    &mut state,
                    channel_id,
                    None,
                    SessionZeroSpeakerKind::Dm,
                    delta.dm_speech,
                )?;
            }
            state.model_receipts.extend(model_receipts);
        }
        SessionZeroCommand::SetBoundary {
            actor_account_hash,
            expected_revision,
            boundary_id,
            topic,
            normalized_topic,
            level,
        } => {
            require_revision(&state, expected_revision)?;
            validate_bounded("boundary topic", &topic, 1, 300)?;
            validate_bounded("normalized boundary topic", &normalized_topic, 1, 160)?;
            let member_id = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?
                .id
                .clone();
            let id = boundary_id.unwrap_or_else(|| format!("boundary:{}", Uuid::new_v4().simple()));
            if let Some(existing) = state.boundaries.get(&id)
                && existing.owner_member_id != member_id
            {
                return Err(anyhow!("only the boundary owner may change it"));
            }
            let now = Utc::now();
            let created_at = state
                .boundaries
                .get(&id)
                .map_or(now, |value| value.created_at);
            state.boundaries.insert(
                id.clone(),
                ContentBoundary {
                    schema: "ghostlight.content_boundary.v1".into(),
                    id,
                    owner_member_id: member_id.clone(),
                    topic,
                    normalized_topic,
                    level,
                    created_at,
                    updated_at: now,
                },
            );
            let before = state.aggregate_boundaries.clone();
            state.aggregate_boundaries = aggregate_boundaries_with_inherited(
                &state.boundaries,
                &state.inherited_aggregate_boundaries,
            );
            state.boundary_epoch = state.boundary_epoch.saturating_add(1);
            state.approvals.remove(&member_id);
            if before != state.aggregate_boundaries {
                shared_changed(&mut state);
            } else {
                retire_preview(&mut state);
            }
        }
        SessionZeroCommand::RemoveBoundary {
            actor_account_hash,
            expected_revision,
            boundary_id,
        } => {
            require_revision(&state, expected_revision)?;
            let member_id = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?
                .id
                .clone();
            let existing = state
                .boundaries
                .get(&boundary_id)
                .ok_or_else(|| anyhow!("boundary does not exist"))?;
            if existing.owner_member_id != member_id {
                return Err(anyhow!("only the boundary owner may remove it"));
            }
            let before = state.aggregate_boundaries.clone();
            state.boundaries.remove(&boundary_id);
            state.aggregate_boundaries = aggregate_boundaries_with_inherited(
                &state.boundaries,
                &state.inherited_aggregate_boundaries,
            );
            state.boundary_epoch = state.boundary_epoch.saturating_add(1);
            state.approvals.remove(&member_id);
            if before != state.aggregate_boundaries {
                shared_changed(&mut state);
            } else {
                retire_preview(&mut state);
            }
        }
        SessionZeroCommand::ResolveDecision {
            actor_account_hash,
            expected_revision,
            decision_id,
            accept,
            counter,
        } => {
            require_revision(&state, expected_revision)?;
            let member_id = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?
                .id
                .clone();
            let decision = state
                .decisions
                .get(&decision_id)
                .cloned()
                .ok_or_else(|| anyhow!("decision is missing"))?;
            if decision.resolved {
                return Err(anyhow!("decision is already resolved"));
            }
            if decision
                .owner_member_id
                .as_deref()
                .is_some_and(|owner| owner != member_id)
            {
                return Err(anyhow!("decision belongs to another member"));
            }
            if accept {
                if decision.pending_counter.is_some() {
                    return Err(anyhow!("counterproposal is awaiting a fresh DM decision"));
                }
                if !decision_has_typed_payload(&decision) {
                    return Err(anyhow!(
                        "decision has no typed state change to accept; discuss or counter it"
                    ));
                }
                if let Some(patch) = decision.proposed_contract_patch.clone() {
                    if decision.owner_member_id.is_some() {
                        return Err(anyhow!(
                            "a private decision cannot amend the shared campaign contract"
                        ));
                    }
                    apply_contract_patch(&mut state.contract, patch);
                    shared_changed(&mut state);
                }
                if let Some(permission) = decision.proposed_extraordinary_permission.clone() {
                    if decision.owner_member_id.as_deref() != Some(member_id.as_str()) {
                        return Err(anyhow!(
                            "extraordinary permission must belong to the accepting player"
                        ));
                    }
                    let draft = state
                        .character_drafts
                        .get_mut(&member_id)
                        .ok_or_else(|| anyhow!("character draft is missing"))?;
                    if permission.actor_id != draft.actor_id {
                        return Err(anyhow!("extraordinary permission targets another actor"));
                    }
                    if permission.name.trim().is_empty()
                        || permission.reliable_scope.trim().is_empty()
                        || permission.effect_ceiling.trim().is_empty()
                    {
                        return Err(anyhow!("extraordinary permission is incomplete"));
                    }
                    draft
                        .extraordinary_permissions
                        .retain(|existing| existing.id != permission.id);
                    draft.extraordinary_permissions.push(permission);
                    character_changed(&mut state, &member_id);
                }
                if let Some(patch) = decision.proposed_character_patch.clone() {
                    if decision.owner_member_id.as_deref() != Some(member_id.as_str()) {
                        return Err(anyhow!(
                            "character proposal must belong to the accepting player"
                        ));
                    }
                    let draft = state
                        .character_drafts
                        .get_mut(&member_id)
                        .ok_or_else(|| anyhow!("character draft is missing"))?;
                    apply_character_patch(draft, patch);
                    character_changed(&mut state, &member_id);
                }
                state
                    .decisions
                    .get_mut(&decision_id)
                    .expect("decision was just validated")
                    .resolved = true;
            } else {
                let proposed_resolution = counter
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| anyhow!("counterproposal is required"))?;
                validate_bounded("counterproposal", &proposed_resolution, 1, 2_000)?;
                let channel_id = decision
                    .owner_member_id
                    .as_ref()
                    .map(|owner| format!("private:{owner}"))
                    .unwrap_or_else(|| "shared:table".into());
                let prompt = decision.prompt.clone();
                let private_decision = decision.owner_member_id.is_some();
                let decision = state
                    .decisions
                    .get_mut(&decision_id)
                    .expect("decision was just validated");
                decision.pending_counter = Some(proposed_resolution.clone());
                decision.resolved = false;
                append_message(
                    &mut state,
                    channel_id,
                    Some(member_id.clone()),
                    SessionZeroSpeakerKind::Player,
                    format!("Counterproposal to '{}': {}", prompt, proposed_resolution),
                )?;
                if private_decision {
                    character_changed(&mut state, &member_id);
                } else {
                    shared_changed(&mut state);
                }
            }
            retire_preview(&mut state);
        }
        SessionZeroCommand::LockRoster {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            if state
                .members
                .values()
                .filter(|member| member.active)
                .count()
                == 0
            {
                return Err(anyhow!("roster cannot be empty"));
            }
            state.roster_locked = true;
            state.status = SessionZeroStatus::RosterLocked;
            shared_changed(&mut state);
        }
        SessionZeroCommand::BeginCompilation {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            state.compilation_brief()?;
            retire_preview(&mut state);
            state.status = SessionZeroStatus::Compiling;
        }
        SessionZeroCommand::ReviewCompilationGaps {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            if !matches!(
                state.status,
                SessionZeroStatus::Drafting | SessionZeroStatus::RosterLocked
            ) {
                return Err(anyhow!("gap-bearing preview is not awaiting review"));
            }
            state.compilation_brief()?;
            let preview = state
                .preview
                .as_ref()
                .ok_or_else(|| anyhow!("gap-bearing preview is missing"))?;
            if preview.gaps.is_empty() {
                return Err(anyhow!("compiled preview has no evidence gaps to review"));
            }
            let shared_digest = state.shared_digest()?;
            if state.preview_shared_digest.as_deref() != Some(shared_digest.as_str()) {
                return Err(anyhow!(
                    "compiled preview no longer matches the shared draft"
                ));
            }
            for member in state.members.values().filter(|member| member.active) {
                if state.preview_character_digests.get(&member.id)
                    != Some(&state.character_digest(&member.id)?)
                {
                    return Err(anyhow!(
                        "compiled preview no longer matches every character"
                    ));
                }
            }
            state.status = SessionZeroStatus::Review;
        }
        SessionZeroCommand::DiscardPreview {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            if state.preview.is_none() {
                return Err(anyhow!("compiled preview is missing"));
            }
            if matches!(
                state.status,
                SessionZeroStatus::Compiling
                    | SessionZeroStatus::Published
                    | SessionZeroStatus::Archived
            ) {
                return Err(anyhow!("compiled preview cannot be discarded now"));
            }
            state.approvals.clear();
            retire_preview(&mut state);
        }
        SessionZeroCommand::InstallPreview {
            expected_revision,
            preview,
            model_receipts,
        } => {
            require_revision(&state, expected_revision)?;
            if !state.roster_locked {
                return Err(anyhow!("roster must be locked before compilation"));
            }
            if state
                .decisions
                .values()
                .any(|decision| decision.material && !decision.resolved)
            {
                return Err(anyhow!("material decisions remain unresolved"));
            }
            state.approvals.clear();
            state.preview_shared_digest = Some(state.shared_digest()?);
            state.preview_character_digests.clear();
            for member in state.members.values().filter(|member| member.active) {
                state
                    .preview_character_digests
                    .insert(member.id.clone(), state.character_digest(&member.id)?);
            }
            state.preview_evidence_coverage = preview.evidence_coverage.clone();
            state.preview_model_receipts.clone_from(&model_receipts);
            state.model_receipts.extend(model_receipts);
            let has_gaps = !preview.gaps.is_empty();
            let gap_text = preview.gaps.join("\n- ");
            state.preview = Some(preview);
            if has_gaps {
                append_message(
                    &mut state,
                    "shared:table".into(),
                    None,
                    SessionZeroSpeakerKind::Dm,
                    format!(
                        "The Vault cannot yet ground these material parts of the opening:\n- {gap_text}\n\nThis exact preview is retained. The host may open it for unanimous approval as branch-local campaign material, or the table may revise the draft before compiling again."
                    ),
                )?;
                state.status = SessionZeroStatus::Drafting;
            } else {
                state.status = SessionZeroStatus::Review;
            }
        }
        SessionZeroCommand::CompilationFailed {
            expected_revision,
            message,
        } => {
            require_revision(&state, expected_revision)?;
            validate_bounded("compilation failure", &message, 1, 4_000)?;
            state.status = SessionZeroStatus::Drafting;
            append_message(
                &mut state,
                "shared:table".into(),
                None,
                SessionZeroSpeakerKind::Dm,
                format!(
                    "The world seed did not pass compilation safely: {message}\n\nNo campaign state was published. Let's revise the draft and try again."
                ),
            )?;
        }
        SessionZeroCommand::Approve {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            if state.status != SessionZeroStatus::Review || state.preview.is_none() {
                return Err(anyhow!("there is no final preview to approve"));
            }
            let member = state
                .member_for_account(&actor_account_hash)
                .ok_or_else(|| anyhow!("account is not a member"))?;
            let member_id = member.id.clone();
            let shared_digest = state.shared_digest()?;
            let character_digest = state.character_digest(&member_id)?;
            let world_preview_digest = state.world_preview_digest()?;
            if state.preview_shared_digest.as_deref() != Some(shared_digest.as_str())
                || state.preview_character_digests.get(&member_id) != Some(&character_digest)
            {
                return Err(anyhow!("preview is stale"));
            }
            state.approvals.insert(
                member_id.clone(),
                SessionZeroApproval {
                    schema: "ghostlight.session_zero_approval.v1".into(),
                    member_id,
                    shared_digest,
                    character_digest,
                    world_preview_digest,
                    approved_at: Utc::now(),
                },
            );
        }
        SessionZeroCommand::MarkPublished {
            actor_account_hash,
            expected_revision,
            campaign_id,
            seed_digest,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            let _ = state.approved_brief()?;
            if let (Some(existing_campaign), Some(existing_digest)) = (
                state.published_campaign_id,
                state.published_seed_digest.as_ref(),
            ) {
                if existing_campaign != campaign_id || existing_digest != &seed_digest {
                    return Err(anyhow!("session zero was published with another seed"));
                }
            }
            state.published_campaign_id = Some(campaign_id);
            state.published_seed_digest = Some(seed_digest);
            state.status = SessionZeroStatus::Published;
        }
        SessionZeroCommand::Archive {
            actor_account_hash,
            expected_revision,
        } => {
            require_revision(&state, expected_revision)?;
            require_host(&state, &actor_account_hash)?;
            state.status = SessionZeroStatus::Archived;
        }
    }
    state.revision = state.revision.saturating_add(1);
    state.updated_at = Utc::now();
    store.replace(&row, "ghostlight.session_zero.v1", &state)?;
    Ok(SessionZeroCommandResult {
        state,
        invite_tokens,
    })
}

fn append_message(
    state: &mut SessionZeroState,
    channel_id: String,
    author_member_id: Option<String>,
    speaker: SessionZeroSpeakerKind,
    text: String,
) -> Result<()> {
    let channel = state
        .channels
        .get_mut(&channel_id)
        .ok_or_else(|| anyhow!("session zero channel is missing"))?;
    let id = format!("message:{}", Uuid::new_v4().simple());
    state.messages.insert(
        id.clone(),
        SessionZeroMessage {
            schema: "ghostlight.session_zero_message.v1".into(),
            id: id.clone(),
            channel_id,
            author_member_id,
            speaker,
            text,
            session_revision: state.revision,
            created_at: Utc::now(),
        },
    );
    channel.message_ids.push(id);
    channel.revision = channel.revision.saturating_add(1);
    Ok(())
}

fn validate_dm_delta(
    state: &SessionZeroState,
    channel_id: &str,
    member_id: Option<&str>,
    delta: &SessionZeroDelta,
) -> Result<()> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| anyhow!("session zero channel is missing"))?;
    match channel.kind {
        SessionZeroChannelKind::SharedTable => {
            if member_id.is_some() || delta.character_patch.is_some() {
                return Err(anyhow!(
                    "shared DM turn cannot mutate private character state"
                ));
            }
            if delta.decisions.iter().any(|decision| {
                decision.owner_member_id.is_some()
                    || decision.proposed_character_patch.is_some()
                    || decision.proposed_extraordinary_permission.is_some()
            }) {
                return Err(anyhow!(
                    "shared DM decisions cannot mutate private character state"
                ));
            }
        }
        SessionZeroChannelKind::PrivateDm => {
            if channel.member_id.as_deref() != member_id {
                return Err(anyhow!("private DM turn owner mismatch"));
            }
            if contract_patch_changes_shared(&delta.contract_patch) {
                return Err(anyhow!("private DM turn cannot mutate the shared contract"));
            }
            if delta.decisions.iter().any(|decision| {
                decision.owner_member_id.as_deref() != member_id
                    || decision.proposed_contract_patch.is_some()
            }) {
                return Err(anyhow!(
                    "private DM decisions must belong to that player and cannot amend the shared contract"
                ));
            }
        }
    }
    let proposes_contract_decision = delta
        .decisions
        .iter()
        .any(|decision| decision.proposed_contract_patch.is_some());
    if contract_patch_changes_shared(&delta.contract_patch) && proposes_contract_decision {
        return Err(anyhow!(
            "a DM turn cannot directly edit the contract and offer a contract decision"
        ));
    }
    let proposes_character_decision = delta
        .decisions
        .iter()
        .any(|decision| decision.proposed_character_patch.is_some());
    if delta.character_patch.is_some() && proposes_character_decision {
        return Err(anyhow!(
            "a DM turn cannot directly edit a character and offer a character decision"
        ));
    }
    validate_bounded("DM speech", &delta.dm_speech, 0, 6_000)?;
    for reply in &delta.suggested_replies {
        validate_bounded("suggested reply", reply, 1, 500)?;
    }
    let mut ids = BTreeSet::new();
    for decision in &delta.decisions {
        if decision.pending_counter.is_some() {
            return Err(anyhow!(
                "DM proposals cannot author player counterproposal state"
            ));
        }
        validate_bounded("decision ID", &decision.id, 1, 240)?;
        validate_bounded("decision prompt", &decision.prompt, 1, 1_000)?;
        validate_bounded(
            "decision proposed resolution",
            &decision.proposed_resolution,
            1,
            2_000,
        )?;
        if !decision_has_typed_payload(decision) {
            return Err(anyhow!(
                "DM decision must carry a non-empty typed state change"
            ));
        }
        if !ids.insert(decision.id.clone()) || state.decisions.contains_key(&decision.id) {
            return Err(anyhow!("DM turn contains a duplicate decision ID"));
        }
    }
    Ok(())
}

fn decision_has_typed_payload(decision: &SessionZeroDecision) -> bool {
    decision.proposed_extraordinary_permission.is_some()
        || decision
            .proposed_contract_patch
            .as_ref()
            .is_some_and(|patch| patch != &CampaignContractPatch::default())
        || decision
            .proposed_character_patch
            .as_ref()
            .is_some_and(|patch| patch != &CharacterDraftPatch::default())
}

fn demote_legacy_nonmaterial_decisions(state: &mut SessionZeroState) -> bool {
    let mut changed = false;
    for decision in state.decisions.values_mut() {
        let generated_opening = decision.id.starts_with("opening:")
            && decision.owner_member_id.is_none()
            && decision.proposed_contract_patch.is_some()
            && decision.prompt.starts_with("Use '")
            && decision
                .prompt
                .ends_with("' as the starting frame for further Session Zero discussion?");
        let payloadless_question = !decision_has_typed_payload(decision);
        if decision.material && (generated_opening || payloadless_question) {
            decision.material = false;
            changed = true;
        }
    }
    changed
}

fn migrate_legacy_model_receipts(state: &mut SessionZeroState) -> bool {
    if !state.model_receipts.is_empty() || state.preview_model_receipts.is_empty() {
        return false;
    }
    let legacy = std::mem::take(&mut state.preview_model_receipts);
    if state.preview.is_some()
        && state.review_campaign_id.is_none()
        && let Some(current_start) = legacy
            .iter()
            .rposition(|receipt| receipt.stage == "custom_retrieval_plan")
        && let Some(current_end) = legacy
            .iter()
            .enumerate()
            .skip(current_start)
            .filter(|(_, receipt)| receipt.stage == "agency_compile")
            .map(|(index, _)| index)
            .next_back()
    {
        state.preview_model_receipts = legacy[current_start..=current_end].to_vec();
    }
    state.model_receipts = legacy;
    true
}

fn validate_counter_replacement(
    countered: &SessionZeroDecision,
    member_id: Option<&str>,
    delta: &SessionZeroDelta,
) -> Result<()> {
    if delta.contract_patch != CampaignContractPatch::default()
        || delta.character_patch.is_some()
        || delta.decisions.len() != 1
    {
        return Err(anyhow!(
            "counter response must contain exactly one fresh decision and no direct state patch"
        ));
    }
    let replacement = &delta.decisions[0];
    if replacement.resolved
        || replacement.owner_member_id.as_deref() != member_id
        || replacement.material != countered.material
    {
        return Err(anyhow!(
            "counter response must preserve the decision owner and materiality"
        ));
    }
    if decision_has_typed_payload(countered) {
        if decision_payload_lanes(replacement) != decision_payload_lanes(countered) {
            return Err(anyhow!(
                "counter response must replace the same typed state lane"
            ));
        }
        if let (Some(previous), Some(next)) = (
            countered.proposed_extraordinary_permission.as_ref(),
            replacement.proposed_extraordinary_permission.as_ref(),
        ) && (previous.id != next.id || previous.actor_id != next.actor_id)
        {
            return Err(anyhow!(
                "counter response cannot change extraordinary permission identity or actor"
            ));
        }
    }
    Ok(())
}

fn decision_payload_lanes(decision: &SessionZeroDecision) -> [bool; 3] {
    [
        decision.proposed_extraordinary_permission.is_some(),
        decision.proposed_contract_patch.is_some(),
        decision.proposed_character_patch.is_some(),
    ]
}

fn contract_patch_changes_shared(patch: &CampaignContractPatch) -> bool {
    patch != &CampaignContractPatch::default()
}

fn apply_contract_patch(contract: &mut CampaignContract, patch: CampaignContractPatch) {
    macro_rules! apply {
        ($field:ident) => {
            if let Some(value) = patch.$field {
                contract.$field = value;
            }
        };
    }
    apply!(premise);
    apply!(canon_horizon);
    apply!(starting_where);
    apply!(starting_when);
    apply!(starting_pressure);
    apply!(desired_goal);
    apply!(tone);
    apply!(themes);
    apply!(pacing);
    apply!(consequence_style);
    apply!(narrative_focus);
    apply!(party_bonds);
    apply!(internal_tension);
    apply!(dm_style);
}

fn apply_character_patch(draft: &mut CharacterDraft, patch: CharacterDraftPatch) {
    if let Some(value) = patch.name {
        draft.name = value;
    }
    if let Some(value) = patch.public_premise {
        draft.public_premise = value;
    }
    remove_exact(&mut draft.private_history, patch.private_history_remove);
    extend_unique(&mut draft.private_history, patch.private_history_add);
    remove_exact(&mut draft.secrets, patch.secrets_remove);
    extend_unique(&mut draft.secrets, patch.secrets_add);
    remove_exact(&mut draft.capabilities, patch.capabilities_remove);
    extend_unique(&mut draft.capabilities, patch.capabilities_add);
    remove_exact(&mut draft.knowledge, patch.knowledge_remove);
    extend_unique(&mut draft.knowledge, patch.knowledge_add);
    remove_exact(&mut draft.equipment, patch.equipment_remove);
    extend_unique(&mut draft.equipment, patch.equipment_add);
    for key in patch.relationship_removals {
        draft.relationships.remove(&key);
    }
    draft.relationships.extend(patch.relationships);
    remove_exact(&mut draft.obligations, patch.obligations_remove);
    extend_unique(&mut draft.obligations, patch.obligations_add);
    remove_exact(&mut draft.vulnerabilities, patch.vulnerabilities_remove);
    extend_unique(&mut draft.vulnerabilities, patch.vulnerabilities_add);
    remove_exact(&mut draft.goals, patch.goals_remove);
    extend_unique(&mut draft.goals, patch.goals_add);
}

fn remove_exact(target: &mut Vec<String>, values: Vec<String>) {
    if values.is_empty() {
        return;
    }
    let removals = values.into_iter().collect::<BTreeSet<_>>();
    target.retain(|value| !removals.contains(value));
}

fn extend_unique(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !value.trim().is_empty() && !target.contains(&value) {
            target.push(value);
        }
    }
}

#[cfg(test)]
fn aggregate_boundaries(boundaries: &BTreeMap<String, ContentBoundary>) -> Vec<AggregatedBoundary> {
    aggregate_boundaries_with_inherited(boundaries, &[])
}

fn aggregate_boundaries_with_inherited(
    boundaries: &BTreeMap<String, ContentBoundary>,
    inherited: &[AggregatedBoundary],
) -> Vec<AggregatedBoundary> {
    let mut by_topic = BTreeMap::<String, AggregatedBoundary>::new();
    for boundary in inherited {
        by_topic.insert(boundary.normalized_topic.clone(), boundary.clone());
    }
    for boundary in boundaries.values() {
        let entry = by_topic
            .entry(boundary.normalized_topic.clone())
            .or_insert_with(|| AggregatedBoundary {
                normalized_topic: boundary.normalized_topic.clone(),
                display_topic: boundary.topic.clone(),
                level: boundary.level.clone(),
            });
        if boundary.level.severity() > entry.level.severity() {
            entry.level = boundary.level.clone();
            entry.display_topic = boundary.topic.clone();
        }
    }
    by_topic.into_values().collect()
}

fn require_channel_access(
    state: &SessionZeroState,
    member_id: &str,
    channel_id: &str,
) -> Result<()> {
    let channel = state
        .channels
        .get(channel_id)
        .ok_or_else(|| anyhow!("session zero channel is missing"))?;
    if channel.kind == SessionZeroChannelKind::PrivateDm
        && channel.member_id.as_deref() != Some(member_id)
    {
        return Err(anyhow!("private channel belongs to another member"));
    }
    Ok(())
}

fn require_host(state: &SessionZeroState, account_hash: &str) -> Result<()> {
    let member = state
        .member_for_account(account_hash)
        .ok_or_else(|| anyhow!("account is not a member"))?;
    if member.id != state.host_member_id || !member.is_host {
        return Err(anyhow!("only the host may perform this command"));
    }
    Ok(())
}

fn require_revision(state: &SessionZeroState, expected: u64) -> Result<()> {
    if state.revision != expected {
        return Err(anyhow!("stale session zero revision"));
    }
    Ok(())
}

fn shared_changed(state: &mut SessionZeroState) {
    state.shared_epoch = state.shared_epoch.saturating_add(1);
    state.approvals.clear();
    retire_preview(state);
}

fn character_changed(state: &mut SessionZeroState, member_id: &str) {
    let epoch = state
        .character_epochs
        .entry(member_id.to_string())
        .or_default();
    *epoch = epoch.saturating_add(1);
    state.approvals.remove(member_id);
    retire_preview(state);
}

fn retire_preview(state: &mut SessionZeroState) {
    state.preview = None;
    state.preview_model_receipts.clear();
    state.preview_shared_digest = None;
    state.preview_character_digests.clear();
    if state.roster_locked {
        state.status = SessionZeroStatus::RosterLocked;
    } else {
        state.status = SessionZeroStatus::Drafting;
    }
}

pub fn membership_from_session(
    state: &SessionZeroState,
    campaign_id: Uuid,
) -> Result<CampaignMembership> {
    let brief = state.approved_brief()?;
    let mut members = BTreeMap::new();
    for member in state.members.values().filter(|member| member.active) {
        let actor_id = brief
            .member_actor_ids
            .get(&member.id)
            .ok_or_else(|| anyhow!("approved character has no actor binding"))?
            .clone();
        members.insert(
            member.id.clone(),
            CampaignMember {
                member_id: member.id.clone(),
                account_hash: member.account_hash.clone(),
                display_name: member.display_name.clone(),
                actor_id,
                is_host: member.is_host,
                active: true,
                cell_allowance: member.cell_allowance,
            },
        );
    }
    let extraordinary_permissions = brief
        .characters
        .iter()
        .map(|character| {
            (
                character.actor_id.clone(),
                character.extraordinary_permissions.clone(),
            )
        })
        .collect();
    Ok(CampaignMembership {
        schema: "ghostlight.campaign_membership.v1".into(),
        campaign_id,
        governance_epoch: 0,
        host_member_id: state.host_member_id.clone(),
        members,
        extraordinary_permissions,
    })
}

pub fn publication_from_session(
    state: &SessionZeroState,
    campaign_id: Uuid,
) -> Result<PublishedSessionZeroSeed> {
    let approved_brief = state.approved_brief()?;
    let membership = membership_from_session(state, campaign_id)?;
    let preview = state
        .preview
        .as_ref()
        .ok_or_else(|| anyhow!("approved world preview is missing"))?;
    let approved_world_preview_digest = state.world_preview_digest()?;
    let approved_evidence_gaps = preview.gaps.clone();
    let approved_branch_assumptions = preview.branch_assumptions.clone();
    let approved_seed_digest = digest(&(
        campaign_id,
        &approved_brief,
        &membership,
        &state.dm_persona,
        &approved_world_preview_digest,
        &approved_evidence_gaps,
        &approved_branch_assumptions,
    ))?;
    Ok(PublishedSessionZeroSeed {
        schema: "ghostlight.published_session_zero_seed.v1".into(),
        session_zero_id: state.id,
        approved_seed_digest,
        contract: state.contract.clone(),
        membership,
        governance: CampaignGovernance {
            schema: "ghostlight.campaign_governance.v1".into(),
            campaign_id,
            governance_epoch: 0,
            time_advance_policy: "unanimous".into(),
            pooled_cell_ceiling: state.pooled_cell_allowance(),
            cooperative_shared_scene_only: true,
            pvp_enabled: false,
        },
        dm_persona: state.dm_persona.clone(),
        approvals: state.approvals.values().cloned().collect(),
        approved_brief,
        approved_world_preview_digest,
        approved_evidence_gaps,
        approved_branch_assumptions,
        boundaries: state.boundaries.values().cloned().collect(),
    })
}

pub fn actor_from_character(draft: &CharacterDraft, location_id: String) -> ActorState {
    ActorState {
        id: draft.actor_id.clone(),
        name: draft.name.clone(),
        location_id,
        capabilities: draft.capabilities.iter().cloned().collect(),
        knowledge: draft.knowledge.iter().cloned().collect(),
        equipment: draft.equipment.iter().cloned().collect(),
        conditions: draft.vulnerabilities.iter().cloned().collect(),
        obligations: draft.obligations.iter().cloned().collect(),
        relationships: draft.relationships.clone(),
        goals: draft.goals.clone(),
        memories: draft
            .private_history
            .iter()
            .chain(&draft.secrets)
            .cloned()
            .collect(),
    }
}

pub fn seed_digest<T: Serialize>(value: &T) -> Result<String> {
    digest(value)
}

fn digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = rmp_serde::to_vec_named(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn secret_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn validate_bounded(label: &str, value: &str, min: usize, max: usize) -> Result<()> {
    let count = value.chars().count();
    if count < min
        || count > max
        || value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(anyhow!(
            "{label} must contain {min} to {max} safe characters"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::tempdir;

    struct SchemaAwareDirectorModel;
    struct FocusedCounterDirectorModel;
    struct FocusedPermissionModel;

    const PERSONA_SPEECH: &str = "**Mars holds.** Your sung name remains ‘The last lamp carried between storms, learning each stranger by the weight they refuse to abandon.’ Does the revised contamination bargain fit Sable’s ability?";

    #[async_trait]
    impl ModelPort for SchemaAwareDirectorModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "session_zero_projector" => {
                    let schema = request
                        .lived_stream
                        .find("OUTPUT JSON SCHEMA:")
                        .expect("projector must receive its exact output contract");
                    let dynamic = request
                        .lived_stream
                        .find("DYNAMIC PERMITTED CONTEXT:")
                        .expect("projector must receive permitted state");
                    assert!(schema < dynamic);
                    Ok(r#"{"lived_stream":"The player wants a serious political campaign on Mars, but tone, character, and stakes remain open."}"#.into())
                }
                "session_zero_dm_persona" => Ok(PERSONA_SPEECH.into()),
                "session_zero_interpreter" => {
                    let schema = request
                        .lived_stream
                        .find("OUTPUT JSON SCHEMA:")
                        .expect("interpreter must receive its exact output contract");
                    let dynamic = request
                        .lived_stream
                        .find("DYNAMIC TYPED EXTRACTION CONTEXT:")
                        .expect("interpreter must receive bounded typed state");
                    assert!(schema < dynamic);
                    assert!(!request.lived_stream.contains("\"dm_speech\""));
                    assert!(!request.lived_stream.contains("\"recent_messages\""));
                    Ok(r#"{"contract_patch":{"starting_where":"Mars in Zhestokost space","tone":["serious","political"]},"character_patch":null,"decisions":[],"suggested_replies":[]}"#.into())
                }
                stage => panic!("unexpected Session Zero model stage {stage}"),
            }
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for FocusedCounterDirectorModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "session_zero_projector" => {
                    panic!("focused counters use the exact deterministic projector")
                }
                "session_zero_dm_persona" => {
                    assert!(request.lived_stream.contains("Previous typed proposal"));
                    assert!(
                        request
                            .lived_stream
                            .contains("Player's exact counterproposal")
                    );
                    assert!(!request.lived_stream.contains("\"schema\":"));
                    Ok("The exact replacement is ready for your review.".into())
                }
                "session_zero_interpreter" => {
                    assert!(request.lived_stream.contains("private-character patch"));
                    assert!(!request.lived_stream.contains("SessionZeroDecision"));
                    assert!(!request.lived_stream.contains("owner_member_id"));
                    assert_eq!(request.max_output_tokens, Some(1800));
                    Ok(serde_json::to_string(&CharacterDraftPatch {
                        name: Some("Sable, whose sung name remains whole".into()),
                        ..Default::default()
                    })?)
                }
                stage => panic!("unexpected focused Session Zero model stage {stage}"),
            }
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for FocusedPermissionModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "session_zero_interpreter");
            assert!(
                request
                    .lived_stream
                    .contains("extraordinary-permission terms")
            );
            assert!(!request.lived_stream.contains("permission:stable"));
            assert!(!request.lived_stream.contains("actor:stable"));
            Ok(serde_json::to_string(&FocusedPermissionTerms {
                name: "Fork-memory synchronization".into(),
                reliable_scope: "One willing nearby mind".into(),
                prerequisites: vec!["Fresh consent".into()],
                costs: vec!["Ordinary contamination fades with rest".into()],
                limits: vec!["No unwilling reading".into()],
                exposure: vec!["Traceable signature".into()],
                effect_ceiling: "Bounded memory sharing".into(),
                branch_local: false,
            })?)
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    fn state() -> SessionZeroState {
        SessionZeroState::new(
            "The Long Way Home".into(),
            "aetheria".into(),
            "account:host".into(),
            "Host".into(),
        )
        .unwrap()
    }

    fn compilable_state() -> SessionZeroState {
        let mut draft = state();
        draft.contract.premise = "Keep a refugee clinic supplied during a political crisis.".into();
        draft.contract.canon_horizon = "After Burden of Proof".into();
        draft.contract.starting_where = "Hellas, Mars".into();
        draft.contract.starting_when = "During a Zhestokost ration strike".into();
        draft.contract.starting_pressure = "A ration strike and refugee convoy".into();
        draft.contract.desired_goal = "Protect the clinic without becoming an informant.".into();
        draft.contract.tone = vec!["serious political drama".into()];
        draft.contract.pacing = "deliberate pressure with sharp turns".into();
        draft.contract.consequence_style = "durable, low arbitrary lethality".into();
        draft.contract.narrative_focus = "institutional leverage and human solidarity".into();
        draft.contract.dm_style = "candid, challenging, and humane".into();
        let character = draft
            .character_drafts
            .get_mut(&draft.host_member_id)
            .unwrap();
        character.public_premise =
            "A Corvid logistics mediator caught between institutions.".into();
        character.capabilities = vec!["Route planning".into()];
        character.goals = vec!["Get the convoy through".into()];
        character.obligations = vec!["A life-debt to the quartermaster".into()];
        draft
    }

    fn model_receipt(stage: &str, receipt_hash: &str, latency_ms: u64) -> ModelStageReceipt {
        ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: receipt_hash.into(),
            provider: "fixture".into(),
            model: "fixture-model".into(),
            stage: stage.into(),
            snapshot_binding: "fixture-snapshot".into(),
            request_hash: format!("request:{stage}"),
            output_hash: format!("output:{stage}"),
            source_receipt_ids: vec![],
            latency_ms,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 100,
            output_chars: 20,
            provider_attempts: vec![],
        }
    }

    #[tokio::test]
    async fn session_zero_keeps_inference_history_separate_from_active_preview_proof() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = compilable_state();
        initial.roster_locked = true;
        initial.status = SessionZeroStatus::Compiling;
        let repeated_a = model_receipt("session_zero_dm_persona", "receipt:repeated", 10);
        let repeated_b = model_receipt("session_zero_dm_persona", "receipt:repeated", 20);
        initial.model_receipts = vec![repeated_a.clone(), repeated_b.clone()];
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, initial.id);
        let current = model_receipt("custom_retrieval_plan", "receipt:current", 30);
        let installed = kernel
            .command(SessionZeroCommand::InstallPreview {
                expected_revision: initial.revision,
                preview: WorldCompilePreview {
                    schema: "ghostlight.world_compile_preview.v1".into(),
                    title: "Current preview".into(),
                    campaign: crate::resolution::tests::campaign(2, 1),
                    evidence_receipts: vec![],
                    evidence_coverage: vec![],
                    gaps: vec![],
                    branch_assumptions: vec![],
                    requires_approval: true,
                },
                model_receipts: vec![current.clone()],
            })
            .await
            .unwrap();

        assert_eq!(
            installed.state.model_receipts,
            [repeated_a, repeated_b, current.clone()]
        );
        assert_eq!(installed.state.preview_model_receipts, [current]);

        let compiling = kernel
            .command(SessionZeroCommand::BeginCompilation {
                actor_account_hash: "account:host".into(),
                expected_revision: installed.state.revision,
            })
            .await
            .unwrap();
        assert!(compiling.state.preview_model_receipts.is_empty());
        assert_eq!(compiling.state.model_receipts.len(), 3);
    }

    #[test]
    fn legacy_receipt_layout_migrates_without_changing_session_revision() {
        let mut draft = compilable_state();
        draft.status = SessionZeroStatus::Review;
        draft.preview = Some(WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Current preview".into(),
            campaign: crate::resolution::tests::campaign(2, 1),
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec![],
            branch_assumptions: vec![],
            requires_approval: true,
        });
        let old = model_receipt("custom_retrieval_plan", "receipt:old-plan", 10);
        let old_world = model_receipt("world_compile", "receipt:old-world", 20);
        let current = model_receipt("custom_retrieval_plan", "receipt:current-plan", 30);
        let current_world = model_receipt("world_compile", "receipt:current-world", 40);
        let current_agency = model_receipt("agency_compile", "receipt:current-agency", 50);
        let trailing_dm = model_receipt("session_zero_dm_persona", "receipt:later-dm", 60);
        let legacy = vec![
            old,
            old_world,
            current.clone(),
            current_world.clone(),
            current_agency.clone(),
            trailing_dm,
        ];
        draft.preview_model_receipts.clone_from(&legacy);
        let revision = draft.revision;
        let status = draft.status.clone();

        assert!(migrate_legacy_model_receipts(&mut draft));
        assert_eq!(draft.model_receipts, legacy);
        assert_eq!(
            draft.preview_model_receipts,
            [current, current_world, current_agency]
        );
        assert_eq!(draft.revision, revision);
        assert_eq!(draft.status, status);
        assert!(!migrate_legacy_model_receipts(&mut draft));
    }

    #[tokio::test]
    async fn director_receives_stable_output_contracts_before_dynamic_context() {
        let director = SessionZeroDirector::new(
            Arc::new(SchemaAwareDirectorModel),
            "projector",
            "persona",
            "interpreter",
        );
        let (delta, receipts) = director
            .respond(&state(), "shared:table", None, None)
            .await
            .unwrap();
        assert_eq!(
            delta.contract_patch.starting_where.as_deref(),
            Some("Mars in Zhestokost space")
        );
        assert_eq!(delta.contract_patch.tone.unwrap(), ["serious", "political"]);
        assert_eq!(delta.dm_speech, PERSONA_SPEECH);
        assert_eq!(receipts.len(), 3);
    }

    #[tokio::test]
    async fn focused_counter_uses_exact_local_projection_before_persona_and_interpreter() {
        let mut draft = state();
        let host_id = draft.host_member_id.clone();
        let target_id = "decision:focused-character".to_owned();
        draft.decisions.insert(
            target_id.clone(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: target_id.clone(),
                owner_member_id: Some(host_id.clone()),
                prompt: "Accept Sable's draft?".into(),
                proposed_resolution: "Use the proposed character patch.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: None,
                proposed_character_patch: Some(CharacterDraftPatch {
                    name: Some("Sable".into()),
                    ..Default::default()
                }),
                evidence_receipt_ids: vec![],
                pending_counter: Some("Keep Sable's full sung name.".into()),
                material: true,
                resolved: false,
            },
        );
        let director = SessionZeroDirector::new(
            Arc::new(FocusedCounterDirectorModel),
            "projector",
            "persona",
            "interpreter",
        );
        let private_channel = format!("private:{host_id}");
        let (delta, receipts) = director
            .respond(&draft, &private_channel, Some(&host_id), Some(&target_id))
            .await
            .unwrap();
        assert_eq!(receipts.len(), 2);
        assert_eq!(
            delta.decisions[0].owner_member_id.as_deref(),
            Some(host_id.as_str())
        );
        assert!(delta.decisions[0].material);
        assert_eq!(
            delta.decisions[0]
                .proposed_character_patch
                .as_ref()
                .and_then(|patch| patch.name.as_deref()),
            Some("Sable, whose sung name remains whole")
        );
    }

    #[tokio::test]
    async fn focused_permission_interpreter_cannot_rewrite_identity_owner_or_evidence() {
        let permission = ExtraordinaryPermission {
            schema: "ghostlight.extraordinary_permission.v1".into(),
            id: "permission:stable".into(),
            actor_id: "actor:stable".into(),
            name: "Fork-memory synchronization".into(),
            reliable_scope: "One willing nearby mind".into(),
            prerequisites: vec!["Fresh consent".into()],
            costs: vec!["Borrowed memories do not fully leave".into()],
            limits: vec!["No unwilling reading".into()],
            exposure: vec!["Traceable signature".into()],
            effect_ceiling: "Bounded memory sharing".into(),
            evidence_receipt_ids: vec!["receipt:stable".into()],
            branch_local: false,
        };
        let decision = SessionZeroDecision {
            schema: "ghostlight.session_zero_decision.v1".into(),
            id: "decision:old".into(),
            owner_member_id: Some("member:stable".into()),
            prompt: "Accept the permission?".into(),
            proposed_resolution: "Old terms".into(),
            proposed_extraordinary_permission: Some(permission.clone()),
            proposed_contract_patch: None,
            proposed_character_patch: None,
            evidence_receipt_ids: vec!["decision-receipt:stable".into()],
            pending_counter: Some("Ordinary contamination fades with rest.".into()),
            material: true,
            resolved: false,
        };
        let (interpretation, _) = run_focused_counter_interpreter(
            &FocusedPermissionModel,
            "fixture",
            "binding",
            &decision,
            FocusedReplacementLane::Permission(&permission),
            "Exact factual basis without persistence identifiers.",
            "Please review the replacement.",
        )
        .await
        .unwrap();
        let replacement = &interpretation.decisions[0];
        let replaced_permission = replacement
            .proposed_extraordinary_permission
            .as_ref()
            .unwrap();
        assert_eq!(replacement.owner_member_id, decision.owner_member_id);
        assert_eq!(replacement.material, decision.material);
        assert_eq!(
            replacement.evidence_receipt_ids,
            decision.evidence_receipt_ids
        );
        assert_eq!(replaced_permission.id, permission.id);
        assert_eq!(replaced_permission.actor_id, permission.actor_id);
        assert_eq!(
            replaced_permission.evidence_receipt_ids,
            permission.evidence_receipt_ids
        );
        assert_eq!(
            replaced_permission.costs,
            ["Ordinary contamination fades with rest"]
        );
    }

    #[test]
    fn focused_counter_schema_excludes_the_full_decision_union() {
        let focused = serde_json::to_string(&schema_for!(CharacterDraftPatch)).unwrap();
        let mut full = serde_json::to_value(schema_for!(SessionZeroInterpretation)).unwrap();
        require_typed_decision_payloads(&mut full).unwrap();
        let full = serde_json::to_string(&full).unwrap();
        assert!(!focused.contains("SessionZeroDecision"));
        assert!(!focused.contains("ExtraordinaryPermission"));
        assert!(!focused.contains("CampaignContractPatch"));
        assert!(
            focused.len() * 2 < full.len(),
            "focused schema={} bytes full union={} bytes",
            focused.len(),
            full.len()
        );
    }

    #[test]
    fn interpreter_schema_rejects_decisions_without_typed_payloads() {
        let mut schema = serde_json::to_value(schema_for!(SessionZeroInterpretation)).unwrap();
        require_typed_decision_payloads(&mut schema).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let decision = SessionZeroDecision {
            schema: "ghostlight.session_zero_decision.v1".into(),
            id: "decision:empty-promise".into(),
            owner_member_id: None,
            prompt: "Materialize the character?".into(),
            proposed_resolution: "Materialize the character exactly as discussed.".into(),
            proposed_extraordinary_permission: None,
            proposed_contract_patch: None,
            proposed_character_patch: None,
            evidence_receipt_ids: vec![],
            pending_counter: None,
            material: true,
            resolved: false,
        };
        let invalid = SessionZeroInterpretation {
            decisions: vec![decision.clone()],
            ..Default::default()
        };
        assert!(!validator.is_valid(&serde_json::to_value(invalid).unwrap()));

        let mut actionable = decision;
        actionable.proposed_character_patch = Some(CharacterDraftPatch {
            name: Some("Sable".into()),
            ..Default::default()
        });
        let valid = SessionZeroInterpretation {
            decisions: vec![actionable],
            ..Default::default()
        };
        assert!(validator.is_valid(&serde_json::to_value(valid).unwrap()));
    }

    #[test]
    fn ui_suggestions_never_enter_dm_or_interpreter_cognition() {
        let mut draft = state();
        draft.decisions.insert(
            "opening:unselected".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "opening:unselected".into(),
                owner_member_id: None,
                prompt: "Use an unselected opening?".into(),
                proposed_resolution: "Age-of-Automation suggestion must remain UI-only.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    starting_when: Some("Age of Automation (2160-2163)".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: false,
                resolved: false,
            },
        );
        draft.decisions.insert(
            "decision:material".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:material".into(),
                owner_member_id: None,
                prompt: "Adopt the negotiated post-Burden horizon?".into(),
                proposed_resolution: "Begin after Burden of Proof.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    canon_horizon: Some("After Burden of Proof".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );

        let dm = permitted_dm_context(&draft, "shared:table", None, None).unwrap();
        let interpreter =
            permitted_interpreter_context(&draft, "shared:table", None, None).unwrap();
        for context in [dm, interpreter] {
            let rendered = serde_json::to_string(&context).unwrap();
            assert!(!rendered.contains("Age of Automation"));
            assert!(!rendered.contains("opening:unselected"));
            assert!(rendered.contains("decision:material"));
            assert!(rendered.contains("After Burden of Proof"));
        }
    }

    #[test]
    fn dm_turn_cannot_split_one_lane_between_direct_and_decision_ownership() {
        let draft = state();
        let duplicate_contract = SessionZeroDelta {
            contract_patch: CampaignContractPatch {
                canon_horizon: Some("After Burden of Proof".into()),
                ..Default::default()
            },
            decisions: vec![SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:duplicate-contract".into(),
                owner_member_id: None,
                prompt: "Adopt the horizon?".into(),
                proposed_resolution: "Begin after Burden of Proof.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    canon_horizon: Some("After Burden of Proof".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            }],
            ..Default::default()
        };

        let mut schema = serde_json::to_value(schema_for!(SessionZeroInterpretation)).unwrap();
        require_typed_decision_payloads(&mut schema).unwrap();
        require_single_lane_ownership(&mut schema).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let split_interpretation = SessionZeroInterpretation {
            contract_patch: duplicate_contract.contract_patch.clone(),
            decisions: duplicate_contract.decisions.clone(),
            ..Default::default()
        };
        assert!(!validator.is_valid(&serde_json::to_value(&split_interpretation).unwrap()));
        assert!(
            validator.is_valid(
                &serde_json::to_value(SessionZeroInterpretation {
                    contract_patch: duplicate_contract.contract_patch.clone(),
                    ..Default::default()
                })
                .unwrap()
            )
        );
        assert!(
            validator.is_valid(
                &serde_json::to_value(SessionZeroInterpretation {
                    decisions: duplicate_contract.decisions.clone(),
                    ..Default::default()
                })
                .unwrap()
            )
        );

        let error = validate_dm_delta(&draft, "shared:table", None, &duplicate_contract)
            .unwrap_err()
            .to_string();
        assert!(error.contains("directly edit the contract"));

        let host_id = draft.host_member_id.clone();
        let actor_id = draft.character_drafts[&host_id].actor_id.clone();
        let character_and_permission = SessionZeroDelta {
            character_patch: Some(CharacterDraftPatch {
                public_premise: Some("A refugee logistics organizer.".into()),
                ..Default::default()
            }),
            decisions: vec![SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:permission".into(),
                owner_member_id: Some(host_id.clone()),
                prompt: "Accept bounded pressure-stress sensing?".into(),
                proposed_resolution: "Touch reveals only recent local stress.".into(),
                proposed_extraordinary_permission: Some(ExtraordinaryPermission {
                    schema: "ghostlight.extraordinary_permission.v1".into(),
                    id: "permission:stress-sense".into(),
                    actor_id,
                    name: "Pressure-stress sensing".into(),
                    reliable_scope: "One touched pressure-bearing machine".into(),
                    prerequisites: vec!["Direct touch".into()],
                    costs: vec!["Vertigo".into()],
                    limits: vec!["No causes or remote history".into()],
                    exposure: vec!["Traceable breathing rhythm".into()],
                    effect_ceiling: "Recent local thermal and pressure stress map".into(),
                    evidence_receipt_ids: vec![],
                    branch_local: true,
                }),
                proposed_contract_patch: None,
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            }],
            ..Default::default()
        };
        assert!(
            validator.is_valid(
                &serde_json::to_value(SessionZeroInterpretation {
                    character_patch: character_and_permission.character_patch.clone(),
                    decisions: character_and_permission.decisions.clone(),
                    ..Default::default()
                })
                .unwrap()
            )
        );
        validate_dm_delta(
            &draft,
            &format!("private:{host_id}"),
            Some(&host_id),
            &character_and_permission,
        )
        .unwrap();
    }

    #[test]
    fn character_patch_can_correct_accepted_identity_without_duplicate_subjects() {
        let mut character = CharacterDraft {
            relationships: BTreeMap::from([(
                "Reed".into(),
                "Ash promised not to let Reed disappear into a manifest.".into(),
            )]),
            capabilities: vec![
                "manifest reconciliation".into(),
                "mistaken capability".into(),
            ],
            ..Default::default()
        };
        let full_name =
            "Reed — full sung name: Rain gathering every lost footstep beneath one patient wing";
        let patch = CharacterDraftPatch {
            relationship_removals: vec!["Reed".into()],
            relationships: BTreeMap::from([(
                full_name.into(),
                "Ash promised not to let Reed disappear into a destination manifest.".into(),
            )]),
            capabilities_remove: vec!["mistaken capability".into()],
            ..Default::default()
        };
        let visible = display_character_patch(&patch);
        assert!(visible.contains("Relationship keys to remove: Reed"));
        assert!(visible.contains("Capabilities to remove: mistaken capability"));

        apply_character_patch(&mut character, patch);

        assert_eq!(character.relationships.len(), 1);
        assert!(!character.relationships.contains_key("Reed"));
        assert!(character.relationships.contains_key(full_name));
        assert_eq!(character.capabilities, ["manifest reconciliation"]);
    }

    #[test]
    fn private_dm_context_carries_bounded_public_session_continuity() {
        let mut draft = state();
        let host_id = draft.host_member_id.clone();
        append_message(
            &mut draft,
            "shared:table".into(),
            Some(host_id.clone()),
            SessionZeroSpeakerKind::Player,
            "The campaign begins in Hellas inside Zhestokost space.".into(),
        )
        .unwrap();
        let private_channel = format!("private:{host_id}");
        append_message(
            &mut draft,
            private_channel.clone(),
            Some(host_id.clone()),
            SessionZeroSpeakerKind::Player,
            "I carry a forged transit credential.".into(),
        )
        .unwrap();

        let context = permitted_dm_context(&draft, &private_channel, Some(&host_id), None).unwrap();
        let shared = serde_json::to_string(&context["recent_shared_messages"]).unwrap();
        assert!(shared.contains("Hellas inside Zhestokost space"));
        assert!(!shared.contains("forged transit credential"));
        assert!(
            serde_json::to_string(&context["recent_messages"])
                .unwrap()
                .contains("forged transit credential")
        );
    }

    #[test]
    fn counter_replacement_focus_projects_only_the_exact_pending_decision() {
        let mut draft = state();
        let host_id = draft.host_member_id.clone();
        let target_id = "decision:target".to_string();
        let decision =
            |id: String, prompt: &str, pending_counter: Option<String>| SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id,
                owner_member_id: Some(host_id.clone()),
                prompt: prompt.into(),
                proposed_resolution: "A retired proposal".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: None,
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter,
                material: true,
                resolved: false,
            };
        let mut target = decision(
            target_id.clone(),
            "Replace the memory cost?",
            Some("Ordinary contamination fades with rest.".into()),
        );
        target.proposed_extraordinary_permission = Some(ExtraordinaryPermission {
            schema: "ghostlight.extraordinary_permission.v1".into(),
            id: "permission:fork-memory".into(),
            actor_id: draft.character_drafts[&host_id].actor_id.clone(),
            name: "Fork-memory synchronization".into(),
            reliable_scope: "One willing nearby mind".into(),
            prerequisites: vec!["Fresh consent".into()],
            costs: vec!["Borrowed memories do not fully leave".into()],
            limits: vec!["No unwilling reading".into()],
            exposure: vec!["Traceable synchronization signature".into()],
            effect_ceiling: "Bounded memory and procedural-skill sharing".into(),
            evidence_receipt_ids: vec![],
            branch_local: false,
        });
        draft.decisions.insert(target_id.clone(), target);
        draft.decisions.insert(
            "decision:unrelated".into(),
            decision(
                "decision:unrelated".into(),
                "UNRELATED-DECISION-MARKER",
                None,
            ),
        );
        let private_channel = format!("private:{host_id}");

        let projected =
            permitted_dm_context(&draft, &private_channel, Some(&host_id), Some(&target_id))
                .unwrap();
        let extraction = permitted_interpreter_context(
            &draft,
            &private_channel,
            Some(&host_id),
            Some(&target_id),
        )
        .unwrap();
        for context in [&projected, &extraction] {
            let encoded = serde_json::to_string(context).unwrap();
            assert!(encoded.contains("Ordinary contamination fades with rest"));
            assert!(encoded.contains("counter_replacement"));
            assert!(encoded.contains("exactly one fresh unresolved decision"));
            assert!(!encoded.contains("UNRELATED-DECISION-MARKER"));
            assert!(
                encoded.len() < 6_000,
                "focused context was {} bytes",
                encoded.len()
            );
            assert!(context.get("recent_messages").is_none());
            assert!(context.get("recent_shared_messages").is_none());
            assert!(context.get("private_character").is_none());
            assert!(context.get("current_private_character").is_none());
            assert_eq!(context["countered_decision"]["id"], target_id);
        }
        let lived =
            project_focused_counter_lived_stream(&draft, Some(&host_id), &target_id).unwrap();
        assert!(lived.contains("Previous typed proposal, rendered exactly"));
        assert!(lived.contains("Borrowed memories do not fully leave"));
        assert!(lived.contains("Player's exact counterproposal"));
        assert!(lived.contains("Ordinary contamination fades with rest"));
        assert!(!lived.contains("UNRELATED-DECISION-MARKER"));
        assert!(!lived.contains("ghostlight.session_zero_decision"));
        assert!(!lived.contains(&target_id));

        let mut legacy = draft.clone();
        let accepted_permission = legacy.decisions[&target_id]
            .proposed_extraordinary_permission
            .clone()
            .unwrap();
        legacy
            .character_drafts
            .get_mut(&host_id)
            .unwrap()
            .extraordinary_permissions
            .push(accepted_permission);
        legacy
            .decisions
            .get_mut(&target_id)
            .unwrap()
            .proposed_extraordinary_permission = None;
        let legacy_context =
            permitted_dm_context(&legacy, &private_channel, Some(&host_id), Some(&target_id))
                .unwrap();
        let legacy_encoded = serde_json::to_string(&legacy_context).unwrap();
        assert!(legacy_encoded.contains("legacy_replacement_basis"));
        assert!(legacy_encoded.contains("permission:fork-memory"));
        assert!(!legacy_encoded.contains("UNRELATED-DECISION-MARKER"));
        assert!(
            legacy_encoded.len() < 8_000,
            "legacy focused context was {} bytes",
            legacy_encoded.len()
        );
        let legacy_lived =
            project_focused_counter_lived_stream(&legacy, Some(&host_id), &target_id).unwrap();
        assert!(legacy_lived.contains("Legacy private character replacement basis"));
        assert!(legacy_lived.contains("Fork-memory synchronization"));
        assert!(!legacy_lived.contains("UNRELATED-DECISION-MARKER"));
    }

    #[tokio::test]
    async fn private_channels_and_boundaries_preserve_ownership() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let initial = state();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store.clone(), initial.id);
        let invite = kernel
            .command(SessionZeroCommand::CreateInvites {
                actor_account_hash: "account:host".into(),
                count: 1,
            })
            .await
            .unwrap();
        let joined = kernel
            .command(SessionZeroCommand::Join {
                token: invite.invite_tokens[0].clone(),
                account_hash: "account:guest".into(),
                display_name: "Guest".into(),
                cell_allowance: FIXTURE_CELL_ALLOWANCE,
            })
            .await
            .unwrap();
        let guest = joined.state.member_for_account("account:guest").unwrap();
        let private = format!("private:{}", guest.id);
        let error = kernel
            .command(SessionZeroCommand::PostPlayerMessage {
                actor_account_hash: "account:host".into(),
                expected_revision: joined.state.revision,
                channel_id: private,
                text: "I should not see this room.".into(),
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("belongs to another member"));

        let bounded = kernel
            .command(SessionZeroCommand::SetBoundary {
                actor_account_hash: "account:guest".into(),
                expected_revision: joined.state.revision,
                boundary_id: None,
                topic: "Graphic eye injury".into(),
                normalized_topic: "eye injury".into(),
                level: BoundaryLevel::Line,
            })
            .await
            .unwrap();
        assert_eq!(bounded.state.aggregate_boundaries.len(), 1);
        let encoded = serde_json::to_string(&bounded.state.aggregate_boundaries).unwrap();
        assert!(!encoded.contains(&guest.id));
        assert!(!encoded.contains("account:guest"));
    }

    #[test]
    fn strictest_boundary_wins_without_attribution() {
        let now = Utc::now();
        let boundaries = BTreeMap::from([
            (
                "a".into(),
                ContentBoundary {
                    schema: "ghostlight.content_boundary.v1".into(),
                    id: "a".into(),
                    owner_member_id: "one".into(),
                    topic: "Spiders".into(),
                    normalized_topic: "spiders".into(),
                    level: BoundaryLevel::AskFirst,
                    created_at: now,
                    updated_at: now,
                },
            ),
            (
                "b".into(),
                ContentBoundary {
                    schema: "ghostlight.content_boundary.v1".into(),
                    id: "b".into(),
                    owner_member_id: "two".into(),
                    topic: "Spiders".into(),
                    normalized_topic: "spiders".into(),
                    level: BoundaryLevel::Veil,
                    created_at: now,
                    updated_at: now,
                },
            ),
        ]);
        let aggregate = aggregate_boundaries(&boundaries);
        assert_eq!(aggregate[0].level, BoundaryLevel::Veil);
        assert!(!serde_json::to_string(&aggregate).unwrap().contains("two"));
    }

    #[test]
    fn pooled_allowance_is_bounded_by_operator_ceiling() {
        let mut state = state();
        for index in 0..7 {
            let id = format!("member:{index}");
            state.members.insert(
                id.clone(),
                SessionZeroMember {
                    schema: "ghostlight.session_zero_member.v1".into(),
                    id,
                    account_hash: format!("account:{index}"),
                    display_name: format!("P{index}"),
                    is_host: false,
                    active: true,
                    cell_allowance: 32,
                    joined_at: Utc::now(),
                },
            );
        }
        assert_eq!(state.pooled_cell_allowance(), OPERATOR_CELL_CEILING);
    }

    #[test]
    fn blank_conversation_only_draft_cannot_enter_world_compilation() {
        let mut draft = state();
        draft.roster_locked = true;
        let error = draft.compilation_brief().unwrap_err().to_string();
        assert!(error.contains("Session Zero draft is incomplete"));
        assert!(error.contains("starting location"));
        assert!(error.contains("public character premise"));
        assert!(error.contains("capabilities"));
    }

    #[test]
    fn optional_opening_or_role_suggestions_do_not_block_custom_compilation() {
        let mut draft = state();
        draft.roster_locked = true;
        draft.contract.premise = "Keep a refugee clinic supplied during a political crisis.".into();
        draft.contract.canon_horizon = "After Burden of Proof".into();
        draft.contract.starting_where = "Hellas, Mars".into();
        draft.contract.starting_when = "Zhestokost administration".into();
        draft.contract.starting_pressure = "A ration strike and refugee convoy".into();
        draft.contract.desired_goal = "Protect the clinic without becoming an informant.".into();
        draft.contract.tone = vec!["serious political drama".into()];
        draft.contract.pacing = "deliberate pressure with sharp turns".into();
        draft.contract.consequence_style = "durable, low arbitrary lethality".into();
        draft.contract.narrative_focus = "institutional leverage and human solidarity".into();
        draft.contract.dm_style = "candid, challenging, and humane".into();
        let character = draft
            .character_drafts
            .get_mut(&draft.host_member_id)
            .unwrap();
        character.public_premise =
            "A Corvid logistics mediator caught between institutions.".into();
        character.capabilities = vec!["Route planning".into()];
        character.goals = vec!["Get the convoy through".into()];
        character.obligations = vec!["A life-debt to the quartermaster".into()];
        draft.decisions.insert(
            "opening:optional".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "opening:optional".into(),
                owner_member_id: None,
                prompt: "Use this generated opening?".into(),
                proposed_resolution: "A generated frame the player may ignore.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    starting_where: Some("Somewhere else".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: false,
                resolved: false,
            },
        );
        assert!(draft.compilation_brief().is_ok());
        draft
            .decisions
            .get_mut("opening:optional")
            .unwrap()
            .material = true;
        assert!(
            draft
                .compilation_brief()
                .unwrap_err()
                .to_string()
                .contains("material decisions")
        );
    }

    #[test]
    fn legacy_generated_openings_are_demoted_without_advancing_session_state() {
        let mut draft = state();
        let revision = draft.revision;
        let shared_epoch = draft.shared_epoch;
        draft.decisions.insert(
            "opening:legacy".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "opening:legacy".into(),
                owner_member_id: None,
                prompt: "Use 'A Legacy Opening' as the starting frame for further Session Zero discussion?".into(),
                proposed_resolution: "An optional generated opening.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    starting_where: Some("Somewhere the player did not choose".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );

        assert!(demote_legacy_nonmaterial_decisions(&mut draft));
        assert!(!draft.decisions["opening:legacy"].material);
        assert_eq!(draft.revision, revision);
        assert_eq!(draft.shared_epoch, shared_epoch);
        assert!(!demote_legacy_nonmaterial_decisions(&mut draft));
    }

    #[tokio::test]
    async fn registry_load_persists_the_payloadless_question_demotion() {
        let root = tempdir().unwrap();
        let mut draft = state();
        let id = draft.id;
        let revision = draft.revision;
        draft.decisions.insert(
            "decision:persisted-legacy-question".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:persisted-legacy-question".into(),
                owner_member_id: None,
                prompt: "Which campaign aspiration should lead?".into(),
                proposed_resolution: "Choose one of two discussion prompts.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: None,
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );
        let directory = root.path().join(id.to_string());
        fs::create_dir(&directory).unwrap();
        let store = CampaignStore::open(directory.join("session-zero.cc")).unwrap();
        SessionZeroKernel::initialize(&store, &draft).unwrap();
        drop(store);

        let registry = SessionZeroRegistry::new(root.path()).unwrap();
        registry.load_existing().await.unwrap();
        let loaded = registry.snapshot(id).await.unwrap();
        assert!(!loaded.decisions["decision:persisted-legacy-question"].material);
        assert_eq!(loaded.revision, revision);
    }

    #[tokio::test]
    async fn published_session_zero_cannot_own_campaign_entry_navigation() {
        let root = tempdir().unwrap();
        let mut published = state();
        published.status = SessionZeroStatus::Published;
        let published_id = published.id;
        let published_directory = root.path().join(published_id.to_string());
        fs::create_dir(&published_directory).unwrap();
        let published_store =
            CampaignStore::open(published_directory.join("session-zero.cc")).unwrap();
        SessionZeroKernel::initialize(&published_store, &published).unwrap();
        drop(published_store);

        let registry = SessionZeroRegistry::new(root.path()).unwrap();
        registry.load_existing().await.unwrap();

        assert_eq!(
            registry.session_for_account("account:host").await.unwrap(),
            None,
            "published negotiation history must not replace the campaign entry surface"
        );
    }

    #[tokio::test]
    async fn registry_load_persists_legacy_receipt_ownership_migration() {
        let root = tempdir().unwrap();
        let mut draft = compilable_state();
        let id = draft.id;
        let revision = draft.revision;
        draft.status = SessionZeroStatus::Review;
        draft.preview = Some(WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Current preview".into(),
            campaign: crate::resolution::tests::campaign(2, 1),
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec![],
            branch_assumptions: vec![],
            requires_approval: true,
        });
        let old = model_receipt("world_compile", "receipt:old-world", 10);
        let current_plan = model_receipt("custom_retrieval_plan", "receipt:current-plan", 20);
        let current_world = model_receipt("world_compile", "receipt:current-world", 30);
        let current_agency = model_receipt("agency_compile", "receipt:current-agency", 40);
        draft.preview_model_receipts = vec![
            old.clone(),
            current_plan.clone(),
            current_world.clone(),
            current_agency.clone(),
        ];
        let directory = root.path().join(id.to_string());
        fs::create_dir(&directory).unwrap();
        let store = CampaignStore::open(directory.join("session-zero.cc")).unwrap();
        SessionZeroKernel::initialize(&store, &draft).unwrap();
        drop(store);

        let registry = SessionZeroRegistry::new(root.path()).unwrap();
        registry.load_existing().await.unwrap();
        let loaded = registry.snapshot(id).await.unwrap();
        assert_eq!(loaded.revision, revision);
        assert_eq!(
            loaded.model_receipts,
            [
                old,
                current_plan.clone(),
                current_world.clone(),
                current_agency.clone()
            ]
        );
        assert_eq!(
            loaded.preview_model_receipts,
            [current_plan, current_world, current_agency]
        );

        let runtime = registry.runtime(id).await.unwrap();
        let (_, persisted) = runtime
            .store
            .load::<SessionZeroState>("session_zero.v1", &id.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(persisted.model_receipts, loaded.model_receipts);
        assert_eq!(
            persisted.preview_model_receipts,
            loaded.preview_model_receipts
        );
    }

    #[tokio::test]
    async fn actor_filtered_surface_never_projects_other_private_state_or_account_hashes() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = state();
        let host_id = initial.host_member_id.clone();
        initial.character_drafts.get_mut(&host_id).unwrap().secrets =
            vec!["HOST-ONLY-CIPHER-ORCHID".into()];
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, initial.id);
        let invite = kernel
            .command(SessionZeroCommand::CreateInvites {
                actor_account_hash: "account:host".into(),
                count: 1,
            })
            .await
            .unwrap();
        let joined = kernel
            .command(SessionZeroCommand::Join {
                token: invite.invite_tokens[0].clone(),
                account_hash: "account:guest".into(),
                display_name: "Guest".into(),
                cell_allowance: FIXTURE_CELL_ALLOWANCE,
            })
            .await
            .unwrap();
        let encoded =
            serde_json::to_string(&session_zero_surface(&joined.state, "account:guest").unwrap())
                .unwrap();
        assert!(!encoded.contains("HOST-ONLY-CIPHER-ORCHID"));
        assert!(!encoded.contains("account:host"));
        assert!(!encoded.contains("account:guest"));
        assert!(!encoded.contains("token_hash"));
        let member_schema = serde_json::to_string(&schema_for!(SessionZeroMember)).unwrap();
        assert!(!member_schema.contains("account_hash"));
    }

    #[test]
    fn private_preview_projects_exact_relationship_subjects_for_player_approval() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        let mut quartermaster = campaign.actors["player"].clone();
        quartermaster.id = "relationship-anchor:quartermaster".into();
        quartermaster.name = "convoy quartermaster".into();
        campaign
            .actors
            .insert(quartermaster.id.clone(), quartermaster);
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .relationships
            .insert(
                "relationship-anchor:quartermaster".into(),
                "owes them a life-debt".into(),
            );
        campaign
            .institutions
            .get_mut("faction-0000")
            .unwrap()
            .resources = vec!["OPERATOR-ONLY-RESERVE".into()];
        campaign.institutions.get_mut("faction-0000").unwrap().goals =
            vec!["PRIVATE-INSTITUTION-GOAL".into()];
        let preview = WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Private relationship witness".into(),
            campaign,
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec![],
            branch_assumptions: vec![],
            requires_approval: true,
        };

        let subjects = private_relationship_subjects(&preview, "player");
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].subject_kind, "actor");
        assert_eq!(subjects[0].name, "convoy quartermaster");
        assert_eq!(subjects[0].location_id.as_deref(), Some("center"));
        assert_eq!(subjects[0].location_name.as_deref(), Some("Center"));
        assert_eq!(subjects[0].relationship, "owes them a life-debt");
        assert!(private_relationship_subjects(&preview, "not-the-player").is_empty());

        let mut draft = state();
        let actor_id = draft.character_drafts[&draft.host_member_id]
            .actor_id
            .clone();
        let mut projected_preview = preview;
        let mut player = projected_preview.campaign.actors.remove("player").unwrap();
        player.id = actor_id.clone();
        projected_preview.campaign.player_actor_id = actor_id.clone();
        projected_preview.campaign.actors.insert(actor_id, player);
        draft.preview = Some(projected_preview);
        draft.status = SessionZeroStatus::Review;
        let surface = serde_json::to_string(
            &session_zero_surface(&draft, "account:host").expect("owner surface"),
        )
        .unwrap();
        assert!(surface.contains("Your private compiled relationships"));
        assert!(surface.contains("Compiled world preview"));
        assert!(surface.contains("Topology"));
        assert!(surface.contains("Institutions"));
        assert!(surface.contains("convoy quartermaster"));
        assert!(surface.contains("owes them a life-debt"));
        assert!(!surface.contains("OPERATOR-ONLY-RESERVE"));
        assert!(!surface.contains("PRIVATE-INSTITUTION-GOAL"));
    }

    #[test]
    fn session_zero_surface_exposes_accept_counter_discuss_and_owned_boundary_controls_as_bindings()
    {
        let mut draft = state();
        let host = draft.host_member_id.clone();
        draft.boundaries.insert(
            "boundary:host".into(),
            ContentBoundary {
                schema: "ghostlight.content_boundary.v1".into(),
                id: "boundary:host".into(),
                owner_member_id: host,
                topic: "medical horror".into(),
                normalized_topic: "medical horror".into(),
                level: BoundaryLevel::Veil,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
        draft.decisions.insert(
            "decision:tone".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:tone".into(),
                owner_member_id: None,
                prompt: "How severe should consequences be?".into(),
                proposed_resolution: "Consequences are durable but rarely lethal.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    consequence_style: Some("Consequences are durable but rarely lethal.".into()),
                    ..Default::default()
                }),
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );

        let encoded =
            serde_json::to_string(&session_zero_surface(&draft, "account:host").unwrap()).unwrap();
        assert!(encoded.contains("Accept"));
        assert!(encoded.contains("Counter"));
        assert!(encoded.contains("Discuss"));
        assert!(encoded.contains("session_zero.boundary.remove"));
        assert!(encoded.contains("\"bindingName\":\"counter\""));
        assert!(encoded.contains("\"maxLength\":2000"));
        assert!(!encoded.contains("payload.fields"));
        assert!(!encoded.contains("\"kind\":\"form\""));
    }

    #[test]
    fn compilable_locked_draft_can_retry_after_compiler_failure() {
        let mut draft = compilable_state();
        draft.roster_locked = true;
        draft.status = SessionZeroStatus::Drafting;
        let encoded =
            serde_json::to_string(&session_zero_surface(&draft, "account:host").unwrap()).unwrap();
        assert!(encoded.contains("Compile campaign preview"));

        draft.status = SessionZeroStatus::Compiling;
        let encoded =
            serde_json::to_string(&session_zero_surface(&draft, "account:host").unwrap()).unwrap();
        assert!(!encoded.contains("Compile campaign preview"));
    }

    #[tokio::test]
    async fn gap_preview_is_retained_and_every_approval_binds_its_exact_world_digest() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = compilable_state();
        initial.roster_locked = true;
        initial.status = SessionZeroStatus::Compiling;
        initial.model_receipts = vec![
            model_receipt("global_agency_doctrine_synthesis", "receipt:repeated", 10),
            model_receipt("global_agency_doctrine_synthesis", "receipt:repeated", 20),
        ];
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, initial.id);
        let gap = "The requested clinic district is branch-local.".to_owned();
        let current_receipt = model_receipt("custom_retrieval_plan", "receipt:current-preview", 30);
        let preview = WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Hellas clinic pressure".into(),
            campaign: crate::resolution::tests::campaign(2, 1),
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec![gap.clone()],
            branch_assumptions: vec!["The convoy arrived this morning.".into()],
            requires_approval: true,
        };
        let installed = kernel
            .command(SessionZeroCommand::InstallPreview {
                expected_revision: initial.revision,
                preview,
                model_receipts: vec![current_receipt.clone()],
            })
            .await
            .unwrap();
        assert_eq!(installed.state.status, SessionZeroStatus::Drafting);
        assert_eq!(
            installed.state.preview.as_ref().unwrap().gaps,
            [gap.clone()]
        );
        let surface =
            serde_json::to_string(&session_zero_surface(&installed.state, "account:host").unwrap())
                .unwrap();
        assert!(surface.contains("Evidence gaps requiring table approval"));
        assert!(surface.contains("Review this exact preview with its evidence gaps"));
        assert!(surface.contains("Discard this preview and continue drafting"));
        assert!(!surface.contains("Compile campaign preview"));

        let reviewing = kernel
            .command(SessionZeroCommand::ReviewCompilationGaps {
                actor_account_hash: "account:host".into(),
                expected_revision: installed.state.revision,
            })
            .await
            .unwrap();
        assert_eq!(reviewing.state.status, SessionZeroStatus::Review);
        let approved = kernel
            .command(SessionZeroCommand::Approve {
                actor_account_hash: "account:host".into(),
                expected_revision: reviewing.state.revision,
            })
            .await
            .unwrap();
        let exact_digest = approved.state.world_preview_digest().unwrap();
        assert_eq!(
            approved
                .state
                .approvals
                .values()
                .next()
                .unwrap()
                .world_preview_digest,
            exact_digest
        );
        let publication = publication_from_session(
            &approved.state,
            approved.state.preview.as_ref().unwrap().campaign.id,
        )
        .unwrap();
        assert_eq!(publication.approved_world_preview_digest, exact_digest);
        assert_eq!(publication.approved_evidence_gaps, [gap]);
        assert_eq!(approved.state.model_receipts.len(), 3);
        assert_eq!(
            approved.state.preview_model_receipts,
            [current_receipt.clone()]
        );
        let published_store =
            CampaignStore::open(dir.path().join("published-campaign.cc")).unwrap();
        let published_campaign = approved.state.preview.as_ref().unwrap();
        published_store
            .create_session_zero_campaign(
                &published_campaign.campaign,
                &published_campaign.evidence_receipts,
                &approved.state.preview_model_receipts,
                &publication,
            )
            .unwrap();
        assert_eq!(
            published_store.keys("persona_stage_receipt.v1").unwrap(),
            [current_receipt.storage_key()]
        );

        let mut substituted = approved.state;
        substituted.preview.as_mut().unwrap().title = "A different world".into();
        assert!(substituted.approved_brief().is_err());
        assert!(
            publication_from_session(
                &substituted,
                substituted.preview.as_ref().unwrap().campaign.id
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn host_can_discard_a_noncanonical_preview_without_changing_the_draft() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = compilable_state();
        initial.roster_locked = true;
        initial.status = SessionZeroStatus::Drafting;
        initial.preview = Some(WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Retired candidate".into(),
            campaign: crate::resolution::tests::campaign(2, 1),
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec!["A material gap".into()],
            branch_assumptions: vec![],
            requires_approval: true,
        });
        let original_contract = initial.contract.clone();
        let original_character = initial.character_drafts[&initial.host_member_id].clone();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, initial.id);

        let discarded = kernel
            .command(SessionZeroCommand::DiscardPreview {
                actor_account_hash: "account:host".into(),
                expected_revision: initial.revision,
            })
            .await
            .unwrap();

        assert!(discarded.state.preview.is_none());
        assert_eq!(discarded.state.status, SessionZeroStatus::RosterLocked);
        assert_eq!(discarded.state.contract, original_contract);
        assert_eq!(
            discarded.state.character_drafts[&discarded.state.host_member_id],
            original_character
        );
        let surface =
            serde_json::to_string(&session_zero_surface(&discarded.state, "account:host").unwrap())
                .unwrap();
        assert!(surface.contains("Compile campaign preview"));
        assert!(!surface.contains("Discard this preview"));
    }

    #[test]
    fn player_surface_projects_the_complete_private_ledger_and_exact_typed_change() {
        let mut draft = state();
        let host = draft.host_member_id.clone();
        let actor_id = draft.character_drafts[&host].actor_id.clone();
        let character = draft.character_drafts.get_mut(&host).unwrap();
        character.name = "Sable".into();
        character.public_premise = "Corvid logistics mediator".into();
        character.private_history = vec!["FULL-SUNG-NAME-ANCHOR".into()];
        character.secrets = vec!["FORGED-TRANSIT-CREDENTIAL".into()];
        character.capabilities = vec!["RATION-LEDGER-RECONCILIATION".into()];
        character.knowledge = vec!["HELLAS-SUPPLY-ROUTES".into()];
        character.equipment = vec!["MINOR-RATION-AUDIT-SEAL".into()];
        character
            .relationships
            .insert("convoy-quartermaster".into(), "LIFE-DEBT".into());
        character.obligations = vec!["CLINIC-EXPECTS-EXCEPTIONS".into()];
        character.vulnerabilities = vec!["TRACEABLE-SYNC-SIGNATURE".into()];
        character.goals = vec!["KEEP-CONVOY-SUPPLIED".into()];
        character.extraordinary_permissions = vec![ExtraordinaryPermission {
            schema: "ghostlight.extraordinary_permission.v1".into(),
            id: "permission:fork-memory".into(),
            actor_id,
            name: "FORK-MEMORY-SYNCHRONIZATION".into(),
            reliable_scope: "ONE-WILLING-MIND-AT-CLOSE-RANGE".into(),
            prerequisites: vec!["FRESH-CONSENT".into()],
            costs: vec!["MIGRAINE".into()],
            limits: vec!["NO-COMPULSION".into()],
            exposure: vec!["TECHNICAL-SIGNATURE".into()],
            effect_ceiling: "BOUNDED-MEMORY-OR-SKILL".into(),
            evidence_receipt_ids: vec![],
            branch_local: true,
        }];
        draft.decisions.insert(
            "decision:sable".into(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: "decision:sable".into(),
                owner_member_id: Some(host),
                prompt: "Materialize Sable?".into(),
                proposed_resolution: "Record the negotiated character.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(CampaignContractPatch {
                    starting_where: Some("MARS-HELLAS-ZHESTOKOST".into()),
                    tone: Some(vec!["COLD-INSTITUTIONAL-PRESSURE".into()]),
                    ..Default::default()
                }),
                proposed_character_patch: Some(CharacterDraftPatch {
                    goals_add: vec!["BUILD-A-DURABLE-REFUGEE-ROUTE".into()],
                    equipment_add: vec!["COUNTERSIGNED-LEDGER".into()],
                    ..Default::default()
                }),
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );

        let encoded =
            serde_json::to_string(&session_zero_surface(&draft, "account:host").unwrap()).unwrap();
        for expected in [
            "FULL-SUNG-NAME-ANCHOR",
            "FORGED-TRANSIT-CREDENTIAL",
            "RATION-LEDGER-RECONCILIATION",
            "HELLAS-SUPPLY-ROUTES",
            "MINOR-RATION-AUDIT-SEAL",
            "LIFE-DEBT",
            "CLINIC-EXPECTS-EXCEPTIONS",
            "TRACEABLE-SYNC-SIGNATURE",
            "KEEP-CONVOY-SUPPLIED",
            "FORK-MEMORY-SYNCHRONIZATION",
            "ONE-WILLING-MIND-AT-CLOSE-RANGE",
            "MARS-HELLAS-ZHESTOKOST",
            "COLD-INSTITUTIONAL-PRESSURE",
            "BUILD-A-DURABLE-REFUGEE-ROUTE",
            "COUNTERSIGNED-LEDGER",
            "Exact typed change",
        ] {
            assert!(
                encoded.contains(expected),
                "missing player projection: {expected}"
            );
        }
    }

    #[tokio::test]
    async fn independent_private_dm_outputs_commit_across_unrelated_global_revisions() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let initial = state();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, initial.id);
        let invite = kernel
            .command(SessionZeroCommand::CreateInvites {
                actor_account_hash: "account:host".into(),
                count: 1,
            })
            .await
            .unwrap();
        let joined = kernel
            .command(SessionZeroCommand::Join {
                token: invite.invite_tokens[0].clone(),
                account_hash: "account:guest".into(),
                display_name: "Guest".into(),
                cell_allowance: FIXTURE_CELL_ALLOWANCE,
            })
            .await
            .unwrap();
        let host_id = joined.state.host_member_id.clone();
        let guest_id = joined
            .state
            .member_for_account("account:guest")
            .unwrap()
            .id
            .clone();
        let host_channel = format!("private:{host_id}");
        let guest_channel = format!("private:{guest_id}");
        let host_message = kernel
            .command(SessionZeroCommand::PostPlayerMessage {
                actor_account_hash: "account:host".into(),
                expected_revision: joined.state.revision,
                channel_id: host_channel.clone(),
                text: "I want a debt to the harbor guild.".into(),
            })
            .await
            .unwrap();
        let both_messages = kernel
            .command(SessionZeroCommand::PostPlayerMessage {
                actor_account_hash: "account:guest".into(),
                expected_revision: host_message.state.revision,
                channel_id: guest_channel.clone(),
                text: "I want to find my missing sibling.".into(),
            })
            .await
            .unwrap();
        let private_delta = |goal: &str| SessionZeroDelta {
            character_patch: Some(CharacterDraftPatch {
                goals_add: vec![goal.into()],
                ..Default::default()
            }),
            dm_speech: format!("Let's make {goal} concrete."),
            ..Default::default()
        };
        let host_applied = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: 0,
                expected_channel_revision: 1,
                channel_id: host_channel,
                member_id: Some(host_id.clone()),
                supersedes_countered_decision_id: None,
                delta: private_delta("repay the harbor guild"),
                model_receipts: vec![],
            })
            .await
            .unwrap();
        assert!(host_applied.state.revision > both_messages.state.revision);
        let guest_applied = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: 0,
                expected_channel_revision: 1,
                channel_id: guest_channel,
                member_id: Some(guest_id.clone()),
                supersedes_countered_decision_id: None,
                delta: private_delta("find my missing sibling"),
                model_receipts: vec![],
            })
            .await
            .unwrap();
        assert!(
            guest_applied.state.character_drafts[&host_id]
                .goals
                .contains(&"repay the harbor guild".into())
        );
        assert!(
            guest_applied.state.character_drafts[&guest_id]
                .goals
                .contains(&"find my missing sibling".into())
        );
    }

    #[tokio::test]
    async fn local_dm_failure_notice_changes_only_the_conversation() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let initial = state();
        let state_id = initial.id;
        let channel_id = "shared:table".to_string();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, state_id);
        let posted = kernel
            .command(SessionZeroCommand::PostPlayerMessage {
                actor_account_hash: "account:host".into(),
                expected_revision: initial.revision,
                channel_id: channel_id.clone(),
                text: "Please try this turn.".into(),
            })
            .await
            .unwrap();
        let before = posted.state.clone();
        let receipt = model_receipt("session_zero_dm_persona", "receipt:dm-notice", 10);

        let noticed = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: before.shared_epoch,
                expected_channel_revision: before.channels[&channel_id].revision,
                channel_id: channel_id.clone(),
                member_id: None,
                supersedes_countered_decision_id: None,
                delta: SessionZeroDelta {
                    dm_speech: "I couldn't finish that response. No draft state changed.".into(),
                    ..Default::default()
                },
                model_receipts: vec![receipt.clone()],
            })
            .await
            .unwrap();

        assert_eq!(noticed.state.contract, before.contract);
        assert_eq!(noticed.state.character_drafts, before.character_drafts);
        assert_eq!(noticed.state.decisions, before.decisions);
        assert_eq!(noticed.state.boundaries, before.boundaries);
        assert_eq!(noticed.state.shared_epoch, before.shared_epoch);
        assert_eq!(noticed.state.character_epochs, before.character_epochs);
        assert_eq!(noticed.state.model_receipts, [receipt]);
        assert_eq!(
            noticed.state.preview_model_receipts,
            before.preview_model_receipts
        );
        assert_eq!(noticed.state.revision, before.revision + 1);
        assert_eq!(
            noticed.state.channels[&channel_id].revision,
            before.channels[&channel_id].revision + 1
        );
        let last_message_id = noticed.state.channels[&channel_id]
            .message_ids
            .last()
            .unwrap();
        assert_eq!(
            noticed.state.messages[last_message_id].speaker,
            SessionZeroSpeakerKind::Dm
        );
    }

    #[tokio::test]
    async fn payloadless_legacy_decision_cannot_be_accepted_or_claim_a_commit() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = state();
        let decision_id = "decision:empty-promise".to_string();
        initial.decisions.insert(
            decision_id.clone(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: decision_id.clone(),
                owner_member_id: None,
                prompt: "Materialize the character?".into(),
                proposed_resolution: "Materialize the character exactly as discussed.".into(),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: None,
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );
        let before = initial.clone();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store.clone(), initial.id);

        let surface =
            serde_json::to_string(&session_zero_surface(&initial, "account:host").unwrap())
                .unwrap();
        assert!(!surface.contains(&format!("session-zero.decision.{decision_id}.accept")));
        assert!(surface.contains("has no typed state change attached"));

        let error = kernel
            .command(SessionZeroCommand::ResolveDecision {
                actor_account_hash: "account:host".into(),
                expected_revision: initial.revision,
                decision_id,
                accept: true,
                counter: None,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no typed state change"));
        assert_eq!(
            store
                .load::<SessionZeroState>("session_zero.v1", &initial.id.to_string())
                .unwrap()
                .unwrap()
                .1,
            before
        );
    }

    #[tokio::test]
    async fn material_character_bargain_is_inert_until_its_owner_accepts() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let initial = state();
        let member_id = initial.host_member_id.clone();
        let actor_id = initial.character_drafts[&member_id].actor_id.clone();
        let state_id = initial.id;
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store, state_id);
        let decision_id = "decision:dangerous-gift".to_string();
        let proposed = ExtraordinaryPermission {
            schema: "ghostlight.extraordinary_permission.v1".into(),
            id: "permission:storm-step".into(),
            actor_id,
            name: "Storm step".into(),
            reliable_scope: "Cross one visible gap in a flash of lightning".into(),
            prerequisites: vec!["A charged storm is overhead".into()],
            costs: vec!["Become visibly marked by the storm".into()],
            limits: vec!["Cannot carry another person".into()],
            exposure: vec!["Grounding wards can detect the transit".into()],
            effect_ceiling: "Movement only; never bypasses a sealed ward".into(),
            evidence_receipt_ids: vec![],
            branch_local: true,
        };
        let proposed_turn = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: 0,
                expected_channel_revision: 0,
                channel_id: format!("private:{member_id}"),
                member_id: Some(member_id.clone()),
                supersedes_countered_decision_id: None,
                delta: SessionZeroDelta {
                    decisions: vec![SessionZeroDecision {
                        schema: "ghostlight.session_zero_decision.v1".into(),
                        id: decision_id.clone(),
                        owner_member_id: Some(member_id.clone()),
                        prompt: "Accept Storm step with these limits?".into(),
                        proposed_resolution: "Grant the bounded permission.".into(),
                        proposed_extraordinary_permission: Some(proposed.clone()),
                        proposed_contract_patch: None,
                        proposed_character_patch: None,
                        evidence_receipt_ids: vec![],
                        pending_counter: None,
                        material: true,
                        resolved: false,
                    }],
                    dm_speech: "This power needs a cost and a ceiling.".into(),
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await
            .unwrap();
        assert!(
            proposed_turn.state.character_drafts[&member_id]
                .extraordinary_permissions
                .is_empty()
        );

        let accepted = kernel
            .command(SessionZeroCommand::ResolveDecision {
                actor_account_hash: "account:host".into(),
                expected_revision: proposed_turn.state.revision,
                decision_id,
                accept: true,
                counter: None,
            })
            .await
            .unwrap();
        assert_eq!(
            accepted.state.character_drafts[&member_id].extraordinary_permissions,
            vec![proposed]
        );
    }

    #[tokio::test]
    async fn counterproposal_preserves_inert_payload_until_fresh_typed_decision_arrives() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let mut initial = state();
        let member_id = initial.host_member_id.clone();
        let actor_id = initial.character_drafts[&member_id].actor_id.clone();
        let original_decision_id = "decision:fork-memory".to_string();
        let permission = |id: &str, costs: Vec<String>| ExtraordinaryPermission {
            schema: "ghostlight.extraordinary_permission.v1".into(),
            id: id.into(),
            actor_id: actor_id.clone(),
            name: "Fork-memory synchronization".into(),
            reliable_scope: "Synchronize briefly with one willing nearby mind".into(),
            prerequisites: vec!["The other mind gives informed consent".into()],
            costs,
            limits: vec!["Cannot read an unwilling mind".into()],
            exposure: vec!["Leaves a traceable synchronization signature".into()],
            effect_ceiling: "Shared impressions, never identity overwrite".into(),
            evidence_receipt_ids: vec![],
            branch_local: true,
        };
        initial.decisions.insert(
            original_decision_id.clone(),
            SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: original_decision_id.clone(),
                owner_member_id: Some(member_id.clone()),
                prompt: "Accept the proposed fork-memory limits?".into(),
                proposed_resolution: "Memory contamination always fades after rest.".into(),
                proposed_extraordinary_permission: Some(permission(
                    "permission:fork-memory:original",
                    vec!["Temporary memory contamination".into()],
                )),
                proposed_contract_patch: None,
                proposed_character_patch: None,
                evidence_receipt_ids: vec![],
                pending_counter: None,
                material: true,
                resolved: false,
            },
        );
        let state_id = initial.id;
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store.clone(), state_id);

        let counter_text = "Contamination usually fades, but intense synchronization can leave permanent associative scars.";
        let countered = kernel
            .command(SessionZeroCommand::ResolveDecision {
                actor_account_hash: "account:host".into(),
                expected_revision: 0,
                decision_id: original_decision_id.clone(),
                accept: false,
                counter: Some(counter_text.into()),
            })
            .await
            .unwrap();
        let pending = &countered.state.decisions[&original_decision_id];
        assert_eq!(pending.pending_counter.as_deref(), Some(counter_text));
        assert_eq!(
            pending
                .proposed_extraordinary_permission
                .as_ref()
                .unwrap()
                .id,
            "permission:fork-memory:original"
        );
        assert!(pending.proposed_contract_patch.is_none());
        assert!(pending.proposed_character_patch.is_none());
        assert!(!pending.resolved);
        let private_channel = format!("private:{member_id}");
        let last_message_id = countered.state.channels[&private_channel]
            .message_ids
            .last()
            .unwrap();
        assert!(
            countered.state.messages[last_message_id]
                .text
                .contains(counter_text)
        );
        let pending_surface =
            serde_json::to_string(&session_zero_surface(&countered.state, "account:host").unwrap())
                .unwrap();
        assert!(pending_surface.contains("Counterproposal recorded"));
        assert!(pending_surface.contains("Retired typed change"));
        assert!(!pending_surface.contains(&format!(
            "session-zero.decision.{original_decision_id}.accept"
        )));

        let before_rejected_accept = countered.state.clone();
        let rejected_accept = kernel
            .command(SessionZeroCommand::ResolveDecision {
                actor_account_hash: "account:host".into(),
                expected_revision: countered.state.revision,
                decision_id: original_decision_id.clone(),
                accept: true,
                counter: None,
            })
            .await
            .unwrap_err();
        assert!(
            rejected_accept
                .to_string()
                .contains("awaiting a fresh DM decision")
        );
        assert_eq!(
            store
                .load::<SessionZeroState>("session_zero.v1", &state_id.to_string())
                .unwrap()
                .unwrap()
                .1,
            before_rejected_accept
        );

        let rejected_replacement = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: countered.state.character_epochs[&member_id],
                expected_channel_revision: countered.state.channels[&private_channel].revision,
                channel_id: private_channel.clone(),
                member_id: Some(member_id.clone()),
                supersedes_countered_decision_id: Some(original_decision_id.clone()),
                delta: SessionZeroDelta {
                    dm_speech: "I need to think about that.".into(),
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await
            .unwrap_err();
        assert!(
            rejected_replacement
                .to_string()
                .contains("exactly one fresh decision")
        );
        assert_eq!(
            store
                .load::<SessionZeroState>("session_zero.v1", &state_id.to_string())
                .unwrap()
                .unwrap()
                .1,
            before_rejected_accept
        );

        let wrong_lane = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: countered.state.character_epochs[&member_id],
                expected_channel_revision: countered.state.channels[&private_channel].revision,
                channel_id: private_channel.clone(),
                member_id: Some(member_id.clone()),
                supersedes_countered_decision_id: Some(original_decision_id.clone()),
                delta: SessionZeroDelta {
                    decisions: vec![SessionZeroDecision {
                        schema: "ghostlight.session_zero_decision.v1".into(),
                        id: "decision:wrong-lane".into(),
                        owner_member_id: Some(member_id.clone()),
                        prompt: "Replace the ability with a goal?".into(),
                        proposed_resolution: "This is not the countered state lane.".into(),
                        proposed_extraordinary_permission: None,
                        proposed_contract_patch: None,
                        proposed_character_patch: Some(CharacterDraftPatch {
                            goals_add: vec!["Wrong replacement lane".into()],
                            ..Default::default()
                        }),
                        evidence_receipt_ids: vec![],
                        pending_counter: None,
                        material: true,
                        resolved: false,
                    }],
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await
            .unwrap_err();
        assert!(wrong_lane.to_string().contains("same typed state lane"));

        let wrong_identity = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: countered.state.character_epochs[&member_id],
                expected_channel_revision: countered.state.channels[&private_channel].revision,
                channel_id: private_channel.clone(),
                member_id: Some(member_id.clone()),
                supersedes_countered_decision_id: Some(original_decision_id.clone()),
                delta: SessionZeroDelta {
                    decisions: vec![SessionZeroDecision {
                        schema: "ghostlight.session_zero_decision.v1".into(),
                        id: "decision:wrong-permission-identity".into(),
                        owner_member_id: Some(member_id.clone()),
                        prompt: "Accept a different permission?".into(),
                        proposed_resolution: counter_text.into(),
                        proposed_extraordinary_permission: Some(permission(
                            "permission:fork-memory:impostor",
                            vec!["Intense synchronization can leave scars".into()],
                        )),
                        proposed_contract_patch: None,
                        proposed_character_patch: None,
                        evidence_receipt_ids: vec![],
                        pending_counter: None,
                        material: true,
                        resolved: false,
                    }],
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await
            .unwrap_err();
        assert!(wrong_identity.to_string().contains("permission identity"));
        assert_eq!(
            store
                .load::<SessionZeroState>("session_zero.v1", &state_id.to_string())
                .unwrap()
                .unwrap()
                .1,
            before_rejected_accept
        );

        let replacement_decision_id = "decision:fork-memory:revised".to_string();
        let revised_permission = permission(
            "permission:fork-memory:original",
            vec![
                "Migraines and memory contamination".into(),
                "Intense synchronization can leave permanent associative scars".into(),
            ],
        );
        let replaced = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: countered.state.character_epochs[&member_id],
                expected_channel_revision: countered.state.channels[&private_channel].revision,
                channel_id: private_channel,
                member_id: Some(member_id.clone()),
                supersedes_countered_decision_id: Some(original_decision_id.clone()),
                delta: SessionZeroDelta {
                    decisions: vec![SessionZeroDecision {
                        schema: "ghostlight.session_zero_decision.v1".into(),
                        id: replacement_decision_id.clone(),
                        owner_member_id: Some(member_id.clone()),
                        prompt: "Accept the revised fork-memory bargain?".into(),
                        proposed_resolution: counter_text.into(),
                        proposed_extraordinary_permission: Some(revised_permission.clone()),
                        proposed_contract_patch: None,
                        proposed_character_patch: None,
                        evidence_receipt_ids: vec![],
                        pending_counter: None,
                        material: true,
                        resolved: false,
                    }],
                    dm_speech: "That cost preserves the ability and its stakes.".into(),
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await
            .unwrap();
        assert!(replaced.state.decisions[&original_decision_id].resolved);
        assert!(!replaced.state.decisions[&replacement_decision_id].resolved);
        assert!(
            replaced.state.character_drafts[&member_id]
                .extraordinary_permissions
                .is_empty()
        );

        let accepted = kernel
            .command(SessionZeroCommand::ResolveDecision {
                actor_account_hash: "account:host".into(),
                expected_revision: replaced.state.revision,
                decision_id: replacement_decision_id,
                accept: true,
                counter: None,
            })
            .await
            .unwrap();
        assert_eq!(
            accepted.state.character_drafts[&member_id].extraordinary_permissions,
            vec![revised_permission]
        );
    }

    #[tokio::test]
    async fn malformed_shared_dm_delta_cannot_cross_private_authority() {
        let dir = tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("session-zero.cc")).unwrap();
        let initial = state();
        let state_id = initial.id;
        let before = initial.clone();
        SessionZeroKernel::initialize(&store, &initial).unwrap();
        let kernel = SessionZeroKernel::start(store.clone(), state_id);
        let result = kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: 0,
                expected_channel_revision: 0,
                channel_id: "shared:table".into(),
                member_id: None,
                supersedes_countered_decision_id: None,
                delta: SessionZeroDelta {
                    character_patch: Some(CharacterDraftPatch {
                        secrets_add: vec!["leaked private history".into()],
                        ..Default::default()
                    }),
                    dm_speech: "This must not commit.".into(),
                    ..Default::default()
                },
                model_receipts: vec![],
            })
            .await;
        assert!(result.is_err());
        let persisted = store
            .load::<SessionZeroState>("session_zero.v1", &state_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted, before);
    }
}
