use crate::domain::{Campaign, NarrationProjection};
use serde_json::{Value, json};

pub fn player_surface(campaign: &Campaign, narrations: &[NarrationProjection]) -> Value {
    let mut story = narrations.iter().map(|n| json!({"id":format!("narration-{}",n.source_revision),"kind":"text","props":{"value":n.text},"children":[]})).collect::<Vec<_>>();
    story.extend(campaign.transcript.iter().map(|t| json!({"id":format!("turn-{}-{}",t.revision,t.speaker),"kind":"text","props":{"value":format!("{}: {}",t.speaker,t.text)},"children":[]})));
    let player = &campaign.actors[&campaign.player_actor_id];
    let location = &campaign.locations[&player.location_id];
    let ledger = format!(
        "Capabilities: {}\nEquipment: {}\nConditions: {}\nObligations: {}\nKnown facts: {}",
        join(&player.capabilities),
        join(&player.equipment),
        join(&player.conditions),
        join(&player.obligations),
        join(&player.knowledge)
    );
    let pressures = campaign
        .clocks
        .values()
        .map(|clock| {
            format!(
                "{} {}/{} — {}",
                clock.label, clock.progress, clock.threshold, clock.consequence
            )
        })
        .chain(
            campaign
                .institutions
                .values()
                .map(|x| format!("{} — {}", x.name, x.posture)),
        )
        .collect::<Vec<_>>()
        .join("\n");
    let news=campaign.news.iter().map(|item|json!({"id":item.id,"kind":"text","props":{"value":format!("[{}] {}",item.channel,item.headline)},"children":[]})).collect::<Vec<_>>();
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation", "title":campaign.name, "version":campaign.revision,
      "surface":{"id":format!("ghostlight.campaign.{}",campaign.id),"root":{"id":"dungeon.root","kind":"surface","props":{},"children":[
        {"id":"dungeon.status","kind":"card","props":{"title":format!("{} · revision {} · {}",campaign.name,campaign.revision,campaign.world_time)},"children":[]},
        {"id":"dungeon.location","kind":"card","props":{"title":format!("{} · {}",location.name,player.name)},"children":[{"id":"dungeon.pressures","kind":"text","props":{"value":pressures},"children":[]}]},
        {"id":"dungeon.ledger","kind":"card","props":{"title":"Character ledger"},"children":[{"id":"dungeon.ledger.text","kind":"text","props":{"value":ledger},"children":[]}]},
        {"id":"dungeon.news","kind":"card","props":{"title":"Accessible news and rumors"},"children":news},
        {"id":"dungeon.transcript","kind":"card","props":{"title":"Story"},"children":story},
        {"id":"dungeon.composer","kind":"text-input","props":{"label":"What do you attempt?","commandId":"attempt.assess"},"children":[]}
      ]},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[{"id":"attempt.assess","schema":"gamecult.eve.command.v1","receiptSchema":"ghostlight.player_action_assessment.v1"}]
    })
}

fn join(values: &std::collections::BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}
