use crate::{
    domain::{
        ActorState, BranchOrigin, Campaign, FactScope, GestaltMemberDelta, GestaltPersonaState,
        InstitutionState, Location, VaultEvidenceReceipt, WorldClock, WorldCompilePreview,
        WorldFact,
    },
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
    vault::{VaultProvider, VaultQuery},
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OpeningRequest {
    pub setting: String,
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OpeningSuggestion {
    pub id: String,
    pub title: String,
    pub era: String,
    pub place: String,
    pub pressure: String,
    pub player_hook: String,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RoleSuggestion {
    pub id: String,
    pub name: String,
    pub premise: String,
    pub capabilities: Vec<String>,
    pub obligations: Vec<String>,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CustomStart {
    pub campaign_name: String,
    pub who: String,
    pub where_: String,
    pub when: String,
    pub goal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SelectedStart {
    pub campaign_name: String,
    pub opening: OpeningSuggestion,
    pub role: RoleSuggestion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuggestedOpenings {
    pub openings: Vec<OpeningSuggestion>,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub model_receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuggestedRoles {
    pub roles: Vec<RoleSuggestion>,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub model_receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct OpeningSet {
    openings: Vec<OpeningSuggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct RoleSet {
    roles: Vec<RoleSuggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledSeed {
    title: String,
    canon_cutoff: String,
    world_time: DateTime<Utc>,
    tick_hours: u32,
    player: ActorState,
    locations: Vec<Location>,
    actors: Vec<ActorState>,
    #[serde(default)]
    gestalts: Vec<GestaltPersonaState>,
    #[serde(default)]
    gestalt_members: Vec<GestaltMemberDelta>,
    institutions: Vec<InstitutionState>,
    clocks: Vec<WorldClock>,
    facts: Vec<WorldFact>,
    gaps: Vec<String>,
    branch_assumptions: Vec<String>,
    opening_narration: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledExpansionSeed {
    locations: Vec<Location>,
    facts: Vec<WorldFact>,
    gaps: Vec<String>,
}

pub struct WorldCompiler {
    vault: Arc<dyn VaultProvider>,
    model: Arc<dyn ModelPort>,
    compiler_model: String,
}

impl WorldCompiler {
    pub fn new(
        vault: Arc<dyn VaultProvider>,
        model: Arc<dyn ModelPort>,
        compiler_model: impl Into<String>,
    ) -> Self {
        Self {
            vault,
            model,
            compiler_model: compiler_model.into(),
        }
    }

    pub async fn suggest_openings(&self, request: OpeningRequest) -> Result<SuggestedOpenings> {
        let queries = [
            format!("{} history eras locations institutions", request.setting),
            format!("{} conflicts pressures ordinary life", request.setting),
            format!("{} travel geography routes", request.setting),
        ];
        let receipts = self.retrieve_all(&queries, "all", 8).await?;
        let evidence = evidence_text(&receipts);
        let output = self.structured("world_openings", "opening-suggestions", &format!("Generate exactly three source-grounded openings. They must use distinct eras, places, and pressures. Do not fill material evidence gaps with invention. REQUEST:\n{}\nEVIDENCE:\n{}", serde_json::to_string(&request)?, evidence), serde_json::to_value(schema_for!(OpeningSet))?, receipt_ids(&receipts)).await?;
        let parsed: OpeningSet = serde_json::from_value(output.0)?;
        if parsed.openings.len() != 3 {
            return Err(anyhow!("world compiler must return exactly three openings"));
        }
        ensure_distinct_openings(&parsed.openings)?;
        Ok(SuggestedOpenings {
            openings: parsed.openings,
            evidence_receipts: receipts,
            model_receipt: output.1,
        })
    }

    pub async fn suggest_roles(&self, opening: &OpeningSuggestion) -> Result<SuggestedRoles> {
        let queries = [
            format!(
                "{} {} people occupations institutions",
                opening.era, opening.place
            ),
            format!(
                "{} capabilities obligations {}",
                opening.place, opening.pressure
            ),
        ];
        let receipts = self.retrieve_all(&queries, &opening.era, 8).await?;
        let output = self.structured("world_roles", &format!("roles:{}", opening.id), &format!("Generate exactly three materially distinct player roles grounded in this opening and evidence. OPENING:\n{}\nEVIDENCE:\n{}", serde_json::to_string(opening)?, evidence_text(&receipts)), serde_json::to_value(schema_for!(RoleSet))?, receipt_ids(&receipts)).await?;
        let parsed: RoleSet = serde_json::from_value(output.0)?;
        if parsed.roles.len() != 3 {
            return Err(anyhow!("world compiler must return exactly three roles"));
        }
        Ok(SuggestedRoles {
            roles: parsed.roles,
            evidence_receipts: receipts,
            model_receipt: output.1,
        })
    }

    pub async fn compile_custom(
        &self,
        start: CustomStart,
    ) -> Result<(WorldCompilePreview, ModelStageReceipt)> {
        let queries = [
            format!(
                "{} {} geography routes containment",
                start.where_, start.when
            ),
            format!("{} {} people institutions powers", start.where_, start.when),
            format!("{} capabilities mechanics {}", start.who, start.goal),
        ];
        let receipts = self.retrieve_all(&queries, &start.when, 10).await?;
        let output = self.structured("world_compile", "custom-start", &format!("Compile a bounded playable region and institutional pressure graph. Emit only supported canon facts; mark reversible texture provisional_local and list material gaps. The player location and every actor location must exist. Every route destination must exist, travel time must be positive, clocks need positive thresholds, and the player id must be unique. Represent populations that can act collectively (villages, crews, crowds, departments, corporations) as gestalt Personas. Seed a small roster of plausible durable member identities for people the player may encounter; member deltas contain only departures from their gestalt baseline and begin dematerialized. Do not duplicate a gestalt member in actors. Keep named plot-critical people as ordinary actors. Every gestalt home location and member gestalt reference must exist. START:\n{}\nEVIDENCE:\n{}", serde_json::to_string(&start)?, evidence_text(&receipts)), serde_json::to_value(schema_for!(CompiledSeed))?, receipt_ids(&receipts)).await?;
        let seed: CompiledSeed = serde_json::from_value(output.0)?;
        let campaign = seed_to_campaign(seed.clone(), &receipts)?;
        validate_campaign_seed(&campaign)?;
        Ok((
            WorldCompilePreview {
                schema: "ghostlight.world_compile_preview.v1".into(),
                title: seed.title,
                campaign,
                evidence_receipts: receipts,
                gaps: seed.gaps,
                branch_assumptions: seed.branch_assumptions,
                requires_approval: true,
            },
            output.1,
        ))
    }

    pub async fn compile_selected(
        &self,
        start: SelectedStart,
    ) -> Result<(WorldCompilePreview, ModelStageReceipt)> {
        self.compile_custom(CustomStart {
            campaign_name: start.campaign_name,
            who: format!("{} — {}", start.role.name, start.role.premise),
            where_: start.opening.place,
            when: start.opening.era,
            goal: format!("{}; {}", start.opening.player_hook, start.opening.pressure),
        })
        .await
    }

    pub async fn compile_destination(
        &self,
        campaign: &Campaign,
        origin_location_id: &str,
        destination_request: &str,
    ) -> Result<(crate::domain::RegionExpansionPreview, ModelStageReceipt)> {
        let origin = campaign
            .locations
            .get(origin_location_id)
            .ok_or_else(|| anyhow!("origin location is unknown"))?;
        let queries = [
            format!(
                "{} {} geography containment routes",
                origin.name, destination_request
            ),
            format!("{} travel time institutions access", destination_request),
        ];
        let receipts = self
            .retrieve_all(&queries, &campaign.branch_origin.canon_cutoff, 10)
            .await?;
        let output=self.structured("destination_compile",&format!("campaign:{}:revision:{}",campaign.id,campaign.revision),&format!("Compile only the requested bounded destination region. Every new location id must be new. At least one new location must route back to origin id {} with a positive travel time. Do not rewrite existing geography. CAMPAIGN LOCATIONS:\n{}\nREQUEST:\n{}\nEVIDENCE:\n{}",origin_location_id,serde_json::to_string(&campaign.locations)?,destination_request,evidence_text(&receipts)),serde_json::to_value(schema_for!(CompiledExpansionSeed))?,receipt_ids(&receipts)).await?;
        let seed: CompiledExpansionSeed = serde_json::from_value(output.0)?;
        let evidence_ids = receipt_ids(&receipts);
        let affected_sources: Vec<String> = receipts
            .iter()
            .flat_map(|r| r.witnesses.iter().map(|w| w.source_id.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let candidates = seed
            .gaps
            .iter()
            .enumerate()
            .map(|(index, gap)| crate::domain::CanonCandidate {
                schema: "ghostlight.canon_candidate.v1".into(),
                id: format!(
                    "canon-candidate:{}:r{}:{}",
                    campaign.id,
                    campaign.revision,
                    index + 1
                ),
                originating_campaign_id: campaign.id,
                gap: gap.clone(),
                evidence_receipt_ids: evidence_ids.clone(),
                conflicts: vec![],
                proposed_wording: format!("Clarify the documented answer to: {gap}"),
                affected_vault_sources: affected_sources.clone(),
                status: "review".into(),
            })
            .collect();
        let expansion = crate::domain::RegionExpansion {
            origin_location_id: origin_location_id.into(),
            locations: seed.locations,
            facts: seed.facts,
        };
        validate_region_expansion(campaign, &expansion)?;
        Ok((
            crate::domain::RegionExpansionPreview {
                schema: "ghostlight.region_expansion_preview.v1".into(),
                campaign_id: campaign.id,
                expected_revision: campaign.revision,
                expansion,
                evidence_receipts: receipts,
                gaps: seed.gaps,
                canon_candidates: candidates,
                requires_approval: true,
            },
            output.1,
        ))
    }

    async fn retrieve_all(
        &self,
        queries: &[String],
        temporal_scope: &str,
        limit: u8,
    ) -> Result<Vec<VaultEvidenceReceipt>> {
        let mut receipts = Vec::new();
        for query in queries {
            receipts.push(
                self.vault
                    .search(&VaultQuery {
                        query: query.clone(),
                        authority_lanes: vec!["Aetheria".into(), "AetheriaLore".into()],
                        temporal_scope: temporal_scope.into(),
                        limit,
                    })
                    .await?,
            );
        }
        if receipts.iter().all(|r| r.witnesses.is_empty()) {
            return Err(anyhow!("Vault returned no evidence; compilation refused"));
        }
        Ok(receipts)
    }

    async fn structured(
        &self,
        stage: &str,
        binding: &str,
        prompt: &str,
        schema: serde_json::Value,
        sources: Vec<String>,
    ) -> Result<(serde_json::Value, ModelStageReceipt)> {
        let prompt = format!(
            "{prompt}\n\nOUTPUT JSON SCHEMA (follow exactly):\n{}",
            serde_json::to_string_pretty(&schema)?
        );
        let out = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: stage.into(),
                model: self.compiler_model.clone(),
                snapshot_binding: binding.into(),
                lived_stream: prompt,
                output_schema: Some(schema),
                source_receipt_ids: sources,
            },
        )
        .await?;
        Ok((
            out.structured
                .ok_or_else(|| anyhow!("compiler returned no structured output"))?,
            out.receipt,
        ))
    }
}

pub fn validate_region_expansion(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
) -> Result<()> {
    if !campaign
        .locations
        .contains_key(&expansion.origin_location_id)
    {
        return Err(anyhow!("destination expansion origin is unknown"));
    }
    let new_ids: BTreeSet<_> = expansion.locations.iter().map(|x| x.id.as_str()).collect();
    if expansion.locations.is_empty() || new_ids.len() != expansion.locations.len() {
        return Err(anyhow!("destination expansion has no unique locations"));
    }
    if new_ids
        .iter()
        .any(|id| campaign.locations.contains_key(*id))
    {
        return Err(anyhow!(
            "destination expansion collides with stable topology"
        ));
    }
    let known = |id: &str| campaign.locations.contains_key(id) || new_ids.contains(id);
    let mut attached = false;
    for location in &expansion.locations {
        for route in location.routes.values() {
            if route.travel_minutes == 0 || !known(&route.destination_id) {
                return Err(anyhow!("destination expansion has a dangling route"));
            }
            if route.destination_id == expansion.origin_location_id {
                attached = true;
            }
        }
    }
    if !attached {
        return Err(anyhow!("destination expansion is not attached to origin"));
    }
    Ok(())
}

fn evidence_text(receipts: &[VaultEvidenceReceipt]) -> String {
    receipts
        .iter()
        .flat_map(|r| r.witnesses.iter())
        .map(|w| {
            format!(
                "[{} | {} | {}] {}",
                w.source_id, w.exact_locator, w.content_hash, w.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn receipt_ids(receipts: &[VaultEvidenceReceipt]) -> Vec<String> {
    receipts.iter().map(|r| r.id.clone()).collect()
}
fn ensure_distinct_openings(items: &[OpeningSuggestion]) -> Result<()> {
    let eras: BTreeSet<_> = items.iter().map(|x| x.era.trim().to_lowercase()).collect();
    let places: BTreeSet<_> = items
        .iter()
        .map(|x| x.place.trim().to_lowercase())
        .collect();
    let pressures: BTreeSet<_> = items
        .iter()
        .map(|x| x.pressure.trim().to_lowercase())
        .collect();
    if eras.len() != 3 || places.len() != 3 || pressures.len() != 3 {
        Err(anyhow!(
            "openings are not distinct across era, place, and pressure"
        ))
    } else {
        Ok(())
    }
}

fn seed_to_campaign(seed: CompiledSeed, receipts: &[VaultEvidenceReceipt]) -> Result<Campaign> {
    let id = Uuid::new_v4();
    let player_id = seed.player.id.clone();
    let now = Utc::now();
    let mut actors: BTreeMap<_, _> = seed.actors.into_iter().map(|x| (x.id.clone(), x)).collect();
    if actors.insert(player_id.clone(), seed.player).is_some() {
        return Err(anyhow!("player id duplicates an NPC"));
    }
    let evidence_receipt_ids = receipt_ids(receipts);
    let affected_sources: Vec<String> = receipts
        .iter()
        .flat_map(|r| r.witnesses.iter().map(|w| w.source_id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let canon_candidates = seed
        .gaps
        .iter()
        .enumerate()
        .map(|(index, gap)| {
            let candidate = crate::domain::CanonCandidate {
                schema: "ghostlight.canon_candidate.v1".into(),
                id: format!("canon-candidate:{}:{}", id, index + 1),
                originating_campaign_id: id,
                gap: gap.clone(),
                evidence_receipt_ids: evidence_receipt_ids.clone(),
                conflicts: vec![],
                proposed_wording: format!("Clarify the documented answer to: {gap}"),
                affected_vault_sources: affected_sources.clone(),
                status: "review".into(),
            };
            (candidate.id.clone(), candidate)
        })
        .collect();
    Ok(Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id,
        name: seed.title,
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: seed.canon_cutoff,
            evidence_receipt_ids,
        },
        world_time: seed.world_time,
        tick_hours: seed.tick_hours,
        player_actor_id: player_id,
        locations: seed
            .locations
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        actors,
        institutions: seed
            .institutions
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        clocks: seed.clocks.into_iter().map(|x| (x.id.clone(), x)).collect(),
        facts: seed
            .facts
            .into_iter()
            .map(|mut x| {
                if x.scope == FactScope::CanonBaseline && x.evidence_receipt_ids.is_empty() {
                    x.scope = FactScope::ProvisionalLocal
                };
                (x.id.clone(), x)
            })
            .collect(),
        transcript: vec![crate::domain::NarrativeTurn {
            revision: 0,
            at: now,
            speaker: "world".into(),
            text: seed.opening_narration,
        }],
        last_player_activity: now,
        pending_ticks: 0,
        away_ticks_processed: 0,
        events: vec![],
        news: vec![],
        canon_candidates,
        gestalts: seed
            .gestalts
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        gestalt_members: seed
            .gestalt_members
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        pending_world_proposals: vec![],
    })
}

pub fn validate_campaign_seed(c: &Campaign) -> Result<()> {
    if c.tick_hours == 0 {
        return Err(anyhow!("strategic tick duration must be positive"));
    }
    if !c.actors.contains_key(&c.player_actor_id) {
        return Err(anyhow!("player actor is missing"));
    }
    for actor in c.actors.values() {
        if !c.locations.contains_key(&actor.location_id) {
            return Err(anyhow!(
                "actor {} occupies unknown location {}",
                actor.id,
                actor.location_id
            ));
        }
    }
    for gestalt in c.gestalts.values() {
        if !c.locations.contains_key(&gestalt.home_location_id) {
            return Err(anyhow!(
                "gestalt {} occupies unknown home location {}",
                gestalt.id,
                gestalt.home_location_id
            ));
        }
    }
    for member in c.gestalt_members.values() {
        if !c.gestalts.contains_key(&member.gestalt_id) {
            return Err(anyhow!(
                "gestalt member {} references unknown gestalt {}",
                member.id,
                member.gestalt_id
            ));
        }
        if member.materialized_actor_id.is_some() {
            return Err(anyhow!(
                "compiled gestalt member {} must begin dematerialized",
                member.id
            ));
        }
    }
    for location in c.locations.values() {
        if let Some(parent) = &location.container_id {
            if parent == &location.id || !c.locations.contains_key(parent) {
                return Err(anyhow!("location {} has invalid container", location.id));
            }
        }
        for route in location.routes.values() {
            if route.travel_minutes == 0 || !c.locations.contains_key(&route.destination_id) {
                return Err(anyhow!("location {} has invalid route", location.id));
            }
        }
    }
    for clock in c.clocks.values() {
        if clock.threshold == 0 || clock.progress > clock.threshold {
            return Err(anyhow!("clock {} is invalid", clock.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::SourceWitness, model::ModelPort, vault::FixtureVault};
    use async_trait::async_trait;

    struct CompilerModel {
        invalid_route: bool,
    }
    #[async_trait]
    impl ModelPort for CompilerModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            Ok(match request.stage.as_str() {
                "world_openings" => serde_json::json!({"openings":[
                    {"id":"a","title":"Ash","era":"early","place":"ring","pressure":"strike","player_hook":"work","evidence_receipt_ids":[]},
                    {"id":"b","title":"Glass","era":"middle","place":"moon","pressure":"siege","player_hook":"survive","evidence_receipt_ids":[]},
                    {"id":"c","title":"Rain","era":"late","place":"station","pressure":"election","player_hook":"choose","evidence_receipt_ids":[]}
                ]}).to_string(),
                "world_compile" => {
                    let destination = if self.invalid_route { "missing" } else { "yard" };
                    serde_json::json!({
                        "title":"Grounded test", "canon_cutoff":"fixture", "world_time":"2026-01-01T00:00:00Z", "tick_hours":6,
                        "player":{"id":"player","name":"Tester","location_id":"yard","capabilities":[],"knowledge":[],"equipment":[],"conditions":[],"obligations":[],"relationships":{},"goals":["learn"]},
                        "locations":[{"id":"yard","name":"Yard","container_id":null,"routes":{"out":{"destination_id":destination,"distance":"near","travel_minutes":5}},"persistent_features":["same yard"]}],
                        "actors":[],
                        "gestalts":[{"schema":"ghostlight.gestalt_persona_state.v1","id":"yard-workers","name":"Yard workers","version":0,"home_location_id":"yard","shared_capabilities":["maintain machinery"],"shared_knowledge":["yard routines"],"resources":["tool shed"],"goals":["finish the shift"],"pressures":["the gate is failing"]}],
                        "gestalt_members":[{"schema":"ghostlight.gestalt_member_delta.v1","id":"john","gestalt_id":"yard-workers","version":0,"name":"John the smith","capability_additions":["forge hinges"],"capability_removals":[],"knowledge_additions":[],"knowledge_removals":[],"equipment":["hammer"],"conditions":[],"obligations":[],"relationships":{},"goals":[],"memories":[],"last_location_id":"yard","materialized_actor_id":null}],
                        "institutions":[],"clocks":[{"id":"shift","label":"Shift ends","progress":0,"threshold":4,"consequence":"night"}],
                        "facts":[{"id":"f","statement":"A witnessed fact","scope":"canon_baseline","evidence_receipt_ids":["fixture"]}],
                        "gaps":["Who owns the outer gate?"],"branch_assumptions":[],"opening_narration":"The yard persists."
                    }).to_string()
                }
                _ => return Err(anyhow!("unexpected stage")),
            })
        }
        fn provider(&self) -> &'static str {
            "compiler-fixture"
        }
    }

    fn vault() -> Arc<FixtureVault> {
        Arc::new(FixtureVault {
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:test.md".into(),
                exact_locator: "test.md:1-2".into(),
                content_hash: "sha256:test".into(),
                excerpt: "A stable witnessed place.".into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
        })
    }

    #[tokio::test]
    async fn opening_stage_requires_three_distinct_axes() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "pro",
        );
        let output = compiler
            .suggest_openings(OpeningRequest {
                setting: "Aetheria".into(),
                constraints: vec![],
            })
            .await
            .unwrap();
        assert_eq!(output.openings.len(), 3);
        assert_eq!(output.evidence_receipts.len(), 3);
    }

    #[tokio::test]
    async fn compile_returns_approval_preview_without_committing() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "pro",
        );
        let (preview, _) = compiler
            .compile_custom(CustomStart {
                campaign_name: "Test".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await
            .unwrap();
        assert!(preview.requires_approval);
        assert_eq!(preview.campaign.revision, 0);
        assert_eq!(preview.campaign.locations.len(), 1);
        assert_eq!(preview.campaign.canon_candidates.len(), 1);
        assert_eq!(preview.campaign.gestalts.len(), 1);
        assert_eq!(
            preview.campaign.gestalt_members["john"].name,
            "John the smith"
        );
        assert!(
            preview.campaign.gestalt_members["john"]
                .materialized_actor_id
                .is_none()
        );
    }

    #[tokio::test]
    async fn compiler_refuses_dream_route_to_unknown_location() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: true,
            }),
            "pro",
        );
        let result = compiler
            .compile_custom(CustomStart {
                campaign_name: "Test".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await;
        assert!(result.unwrap_err().to_string().contains("invalid route"));
    }
}
