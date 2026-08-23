use crate::{
    domain::{Campaign, NarrationProjection},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Narrator {
    pub model: Arc<dyn ModelPort>,
    pub model_name: String,
    pub verifier_model_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NarrationVerification {
    faithful_to_draft: bool,
    prose: String,
    unsupported_claims: Vec<String>,
}

impl Narrator {
    pub async fn project(
        &self,
        store: &CampaignStore,
        campaign: &Campaign,
    ) -> Result<(NarrationProjection, Vec<ModelStageReceipt>)> {
        let player = &campaign.actors[&campaign.player_actor_id];
        let location = &campaign.locations[&player.location_id];
        let recent_events: Vec<_> = campaign.events.iter().rev().take(4).cloned().collect();
        let latest_turn = campaign.transcript.last().cloned();
        let publication = store
            .load::<crate::session_zero::PublishedSessionZeroSeed>(
                "session_zero_publication.v1",
                &campaign.id.to_string(),
            )?
            .map(|(_, value)| value);
        let active_boundaries = store
            .load::<crate::session_zero::ActiveContractBoundaryPolicy>(
                "active_contract_boundary_policy.v1",
                &campaign.id.to_string(),
            )?
            .map(|(_, value)| value);
        let aggregate_boundaries = active_boundaries
            .filter(|active| {
                publication.as_ref().is_none_or(|published| {
                    active.review_session_zero_id != published.session_zero_id
                })
            })
            .map(|active| active.aggregate_boundaries)
            .or_else(|| {
                publication
                    .as_ref()
                    .map(|value| value.approved_brief.aggregate_boundaries.clone())
            })
            .unwrap_or_default();
        let visible_actors: Vec<_> = campaign
            .actors
            .values()
            .filter(|actor| actor.location_id == player.location_id)
            .map(|actor| {
                serde_json::json!({
                    "id": actor.id,
                    "name": actor.name,
                    "conditions": actor.conditions,
                })
            })
            .collect();
        let public_slice = serde_json::json!({
            "viewer_actor": {
                "id": player.id,
                "name": player.name,
            },
            "world_time": campaign.world_time,
            "location": {
                "id": location.id,
                "name": location.name,
                "persistent_features": location.persistent_features,
            },
            "visible_actors": visible_actors,
            "latest_events": recent_events,
            "latest_committed_turn": latest_turn,
            "campaign_contract": publication.as_ref().map(|value| &value.contract),
            "aggregate_content_boundaries": aggregate_boundaries,
        });
        let public_slice_json = serde_json::to_string(&public_slice)?;
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "narrator".into(),
                model: self.model_name.clone(),
                snapshot_binding: format!(
                    "campaign:{}:revision:{}",
                    campaign.id, campaign.revision
                ),
                lived_stream: format!(
                    "Narrate only latest_committed_turn and any latest_events it directly caused, in concrete, concise interactive-fiction prose. viewer_actor is the reader's character and the only actor that may be addressed in second person. If latest_committed_turn belongs to another visible actor, narrate that actor in third person and preserve their exact attribution; never transfer their speech, knowledge, uncertainty, choice, or action to the viewer. The campaign contract governs tone, pacing, focus, consequences, and DM style. Obey every aggregate content boundary: line excludes the topic, veil keeps it off-screen, ask_first permits no new depiction without a current explicit player acceptance. Never expose boundary attribution. Location, time, and visible actors are grounding constraints, not a request to restate every field. Do not repeat older setup, list routes, or recap unrelated world state. Do not mention JSON, state, commits, revisions, or the source representation. Every environmental noun, sensory adjective, object state, action, and consequence must be traceable to the supplied JSON. Do not invent lighting, temperature, sound, motion, posture, dialogue, private thoughts, expertise, geography, findings, or outcomes. It is better to be spare than to fabricate texture. Emit prose only.\n\n{}",
                    public_slice_json
                ),
                output_schema: None,
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
                temperature: Some(0.2),
                max_output_tokens: Some(256),
            },
        )
        .await?;
        let text = output.narrative.trim();
        if text.is_empty() || text.starts_with('{') || text.contains("```json") {
            return Err(anyhow!("narrator violated prose projection membrane"));
        }
        let verification = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "narration_verifier".into(),
                model: self.verifier_model_name.clone(),
                snapshot_binding: format!(
                    "campaign:{}:revision:{}:narration-verifier",
                    campaign.id, campaign.revision
                ),
                lived_stream: format!(
                    "You are Ghostlight's independent narration verifier. The typed source slice and draft are projections of one already-committed world revision. Return the draft unchanged only when every concrete claim, outcome, actor attribution, object state, sensory detail, and causal implication is entailed by the source. Otherwise return minimal corrected interactive-fiction prose containing only entailed claims. Preserve an explicit committed success, mixed result, or failure exactly; later reaction speech or uncertainty cannot reverse it. Do not invent a more specific finding than the committed event or actor knowledge. Do not add atmosphere merely to improve style. faithful_to_draft is true only when prose is byte-for-byte the supplied draft. unsupported_claims contains short descriptions of removed or corrected claims and is empty when faithful. Return one JSON object only.\n\nTYPED SOURCE SLICE:\n{}\n\nDRAFT NARRATION:\n{}",
                    public_slice_json,
                    serde_json::to_string(text)?
                ),
                output_schema: Some(serde_json::to_value(schema_for!(NarrationVerification))?),
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
                temperature: Some(0.0),
                max_output_tokens: Some(384),
            },
        )
        .await?;
        let verification_value: NarrationVerification = serde_json::from_value(
            verification
                .structured
                .clone()
                .ok_or_else(|| anyhow!("narration verifier produced no typed result"))?,
        )?;
        let verified_text = verification_value.prose.trim();
        if verified_text.is_empty()
            || verified_text.len() > 2_000
            || verified_text.starts_with('{')
            || verified_text.contains("```json")
            || verification_value.unsupported_claims.len() > 8
            || verification_value
                .unsupported_claims
                .iter()
                .any(|claim| claim.trim().is_empty() || claim.len() > 240)
            || (verification_value.faithful_to_draft && verified_text != text)
        {
            return Err(anyhow!("narration verifier returned an invalid projection"));
        }
        let current = store
            .load::<Campaign>("campaign.v1", &campaign.id.to_string())?
            .map(|(_, campaign)| campaign)
            .ok_or_else(|| anyhow!("campaign vanished during narration"))?;
        if current.revision != campaign.revision {
            return Err(anyhow!("narration snapshot became stale"));
        }
        let projection = NarrationProjection {
            schema: "ghostlight.narration_projection.v1".into(),
            id: format!("{}:{}", campaign.id, campaign.revision),
            campaign_id: campaign.id,
            source_revision: campaign.revision,
            text: verified_text.into(),
            event_ids: recent_events.into_iter().map(|event| event.id).collect(),
            model_receipt_hash: verification.receipt.storage_key().to_owned(),
            published_at: Utc::now(),
        };
        Ok((projection, vec![output.receipt, verification.receipt]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ActorState, BranchOrigin, Campaign, Location, NarrativeTurn},
        model::ModelStageRequest,
    };
    use async_trait::async_trait;
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Mutex,
    };

    struct CaptureNarratorModel {
        prompt: Mutex<String>,
    }

    struct CorrectingNarratorModel;

    #[async_trait]
    impl ModelPort for CorrectingNarratorModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            if request.stage == "narration_verifier" {
                return Ok(serde_json::json!({
                    "faithful_to_draft":false,
                    "prose":"The clinic director answers.",
                    "unsupported_claims":["The draft invented a damaged seal."]
                })
                .to_string());
            }
            Ok("The clinic director answers beside an invented damaged seal.".into())
        }

        fn provider(&self) -> &'static str {
            "narrator-correcting"
        }
    }

    #[async_trait]
    impl ModelPort for CaptureNarratorModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            if request.stage == "narration_verifier" {
                return Ok(serde_json::json!({
                    "faithful_to_draft":true,
                    "prose":"The clinic director answers without moving the ledger.",
                    "unsupported_claims":[]
                })
                .to_string());
            }
            *self.prompt.lock().unwrap() = request.lived_stream.clone();
            Ok("The clinic director answers without moving the ledger.".into())
        }

        fn provider(&self) -> &'static str {
            "narrator-capture"
        }
    }

    fn campaign() -> Campaign {
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
            id: uuid::Uuid::new_v4(),
            name: "Narrator test".into(),
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
    async fn narration_is_revision_bound_and_does_not_mutate_campaign() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let seed = campaign();
        store.create_campaign(&seed, &[], &[]).unwrap();
        let narrator = Narrator {
            model: Arc::new(CaptureNarratorModel {
                prompt: Mutex::new(String::new()),
            }),
            model_name: "fixture".into(),
            verifier_model_name: "fixture".into(),
        };
        let (projection, _) = narrator.project(&store, &seed).await.unwrap();
        assert_eq!(projection.source_revision, 0);
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted, seed);
        assert!(store.keys("narration_projection.v1").unwrap().is_empty());
    }

    #[tokio::test]
    async fn narrator_receives_exact_viewer_ownership_for_an_npc_turn() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut seed = campaign();
        let mut director = seed.actors["player"].clone();
        director.id = "clinic-director".into();
        director.name = "Clinic Director".into();
        seed.actors.insert(director.id.clone(), director);
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: Utc::now(),
            speaker: "clinic-director".into(),
            text: "I will keep the triage ledger.".into(),
        });
        store.create_campaign(&seed, &[], &[]).unwrap();
        let model = Arc::new(CaptureNarratorModel {
            prompt: Mutex::new(String::new()),
        });
        let narrator = Narrator {
            model: model.clone(),
            model_name: "fixture".into(),
            verifier_model_name: "fixture".into(),
        };

        narrator.project(&store, &seed).await.unwrap();

        let prompt = model.prompt.lock().unwrap();
        assert!(prompt.contains("\"viewer_actor\":{\"id\":\"player\",\"name\":\"Player\"}"));
        assert!(prompt.contains("\"speaker\":\"clinic-director\""));
        assert!(prompt.contains("the only actor that may be addressed in second person"));
        assert!(
            prompt
                .contains("never transfer their speech, knowledge, uncertainty, choice, or action")
        );
    }

    #[tokio::test]
    async fn narration_publishes_the_verified_correction_and_binds_both_stage_receipts() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let seed = campaign();
        store.create_campaign(&seed, &[], &[]).unwrap();
        let narrator = Narrator {
            model: Arc::new(CorrectingNarratorModel),
            model_name: "fixture-pro".into(),
            verifier_model_name: "fixture-flash".into(),
        };

        let (projection, receipts) = narrator.project(&store, &seed).await.unwrap();

        assert_eq!(projection.text, "The clinic director answers.");
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].stage, "narrator");
        assert_eq!(receipts[1].stage, "narration_verifier");
        assert_eq!(projection.model_receipt_hash, receipts[1].storage_key());
    }
}
