use crate::domain::Campaign;
use serde_json::{Value, json};

pub fn player_surface(campaign: &Campaign) -> Value {
    let turns = campaign.transcript.iter().map(|t| json!({"id":format!("turn-{}-{}",t.revision,t.speaker),"kind":"text","props":{"value":format!("{}: {}",t.speaker,t.text)},"children":[]})).collect::<Vec<_>>();
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation", "title":campaign.name, "version":campaign.revision,
      "surface":{"id":format!("ghostlight.campaign.{}",campaign.id),"root":{"id":"dungeon.root","kind":"surface","props":{},"children":[
        {"id":"dungeon.status","kind":"card","props":{"title":format!("{} · revision {} · {}",campaign.name,campaign.revision,campaign.world_time)},"children":[]},
        {"id":"dungeon.transcript","kind":"card","props":{"title":"Story"},"children":turns},
        {"id":"dungeon.composer","kind":"text-input","props":{"label":"What do you attempt?","commandId":"attempt.assess"},"children":[]}
      ]},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[{"id":"attempt.assess","schema":"gamecult.eve.command.v1","receiptSchema":"ghostlight.player_action_assessment.v1"}]
    })
}
