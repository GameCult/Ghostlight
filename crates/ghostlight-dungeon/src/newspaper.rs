use crate::domain::{Campaign, Event};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperIssue {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub at: DateTime<Utc>,
    pub source_world_revision: u64,
    pub articles: Vec<WorldNewspaperArticle>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperArticle {
    pub id: String,
    pub section: String,
    pub headline: String,
    pub dateline: Option<String>,
    pub body: String,
    pub channel: String,
    pub reliability: String,
    pub event_ids: Vec<String>,
}

pub fn compose_world_newspaper(
    campaign: &Campaign,
    title: impl Into<String>,
    max_articles: usize,
) -> Result<WorldNewspaperIssue> {
    if max_articles == 0 {
        return Err(anyhow!("newspaper article budget must be positive"));
    }
    let title = title.into();
    if title.trim().is_empty() {
        return Err(anyhow!("newspaper title is empty"));
    }
    let events = campaign
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect::<BTreeMap<_, _>>();
    let mut news = campaign.news.iter().collect::<Vec<_>>();
    news.sort_by(|left, right| right.at.cmp(&left.at).then_with(|| left.id.cmp(&right.id)));

    let mut articles = Vec::new();
    for issue in news.into_iter().take(max_articles) {
        if issue.event_ids.is_empty() {
            return Err(anyhow!("news item {} has no committed event", issue.id));
        }
        let source_events = issue
            .event_ids
            .iter()
            .map(|event_id| {
                let event = events.get(event_id.as_str()).copied().ok_or_else(|| {
                    anyhow!("news item {} cites unknown event {event_id}", issue.id)
                })?;
                if !event.public_channels.contains(&issue.channel) {
                    return Err(anyhow!(
                        "news item {} uses channel absent from event {event_id}",
                        issue.id
                    ));
                }
                Ok(event)
            })
            .collect::<Result<Vec<_>>>()?;
        let primary = source_events[0];
        let dateline = event_dateline(campaign, primary);
        let body = source_events
            .iter()
            .map(|event| event.summary.trim())
            .collect::<Vec<_>>()
            .join(" ");
        if body.is_empty() {
            return Err(anyhow!("news item {} cites an empty event", issue.id));
        }
        articles.push(WorldNewspaperArticle {
            id: format!("article:{}", issue.id),
            section: section_for(primary).into(),
            headline: issue.headline.clone(),
            dateline,
            body,
            channel: issue.channel.clone(),
            reliability: issue.reliability.clone(),
            event_ids: issue.event_ids.clone(),
        });
    }
    let at = articles
        .first()
        .and_then(|article| {
            article
                .event_ids
                .first()
                .and_then(|event_id| events.get(event_id.as_str()))
                .map(|event| event.at)
        })
        .unwrap_or(campaign.world_time);
    let identity = rmp_serde::to_vec_named(&(
        campaign.id,
        campaign.revision,
        &title,
        articles
            .iter()
            .map(|article| &article.id)
            .collect::<Vec<_>>(),
    ))?;
    Ok(WorldNewspaperIssue {
        schema: "ghostlight.world_newspaper_issue.v1".into(),
        id: format!("newspaper:sha256:{:x}", Sha256::digest(identity)),
        title,
        at,
        source_world_revision: campaign.revision,
        articles,
    })
}

pub fn render_world_newspaper_markdown(issue: &WorldNewspaperIssue) -> String {
    let mut rendered = format!(
        "# {}\n\n*World revision {} · {}*\n",
        issue.title,
        issue.source_world_revision,
        issue.at.format("%Y-%m-%d %H:%M UTC")
    );
    for article in &issue.articles {
        rendered.push_str(&format!(
            "\n## {} — {}\n\n",
            article.section, article.headline
        ));
        if let Some(dateline) = &article.dateline {
            rendered.push_str(&format!("**{dateline}** — "));
        }
        rendered.push_str(&article.body);
        rendered.push_str(&format!(
            "\n\n*Via {} · {} · events: {}*\n",
            article.channel,
            article.reliability,
            article.event_ids.join(", ")
        ));
    }
    rendered
}

fn event_dateline(campaign: &Campaign, event: &Event) -> Option<String> {
    event.location_ids.first().map(|location_id| {
        campaign
            .locations
            .get(location_id)
            .map(|location| location.name.clone())
            .unwrap_or_else(|| location_id.clone())
    })
}

fn section_for(event: &Event) -> &'static str {
    match event.kind.as_str() {
        "institution_action" => "Courts & Councils",
        "gestalt_individuation" => "Names to Know",
        "strategic_activity_outcome" => "Dispatches",
        "gestalt_action" | "gestalt_activity" | "gestalt_migration" => "Realms",
        _ => "World",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Event, NewsIssue};

    #[test]
    fn newspaper_is_a_projection_of_public_committed_events() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.revision = 3;
        campaign.events.push(Event {
            id: "event:seal-scandal".into(),
            at: campaign.world_time,
            kind: "institution_action".into(),
            summary:
                "The Thorn Court admits its royal seal was pawned to pay a dragon's gambling debt."
                    .into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec!["room".into()],
            public_channels: vec!["court broadsheet".into()],
        });
        campaign.news.push(NewsIssue {
            id: "news:seal-scandal".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: "CROWN SEAL PAWNED; DRAGON REFUSES COMMENT".into(),
            event_ids: vec!["event:seal-scandal".into()],
            reliability: "committed public channel".into(),
        });

        let issue = compose_world_newspaper(&campaign, "The Underdeep Clarion", 8).unwrap();
        let markdown = render_world_newspaper_markdown(&issue);
        assert_eq!(issue.articles[0].event_ids, ["event:seal-scandal"]);
        assert!(markdown.contains("CROWN SEAL PAWNED"));
        assert!(markdown.contains("dragon's gambling debt"));
        assert!(markdown.contains("event:seal-scandal"));
    }

    #[test]
    fn newspaper_rejects_a_headline_without_a_committed_event() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.news.push(NewsIssue {
            id: "news:invented".into(),
            at: campaign.world_time,
            channel: "whisper wire".into(),
            headline: "UNFOUNDED NONSENSE".into(),
            event_ids: vec!["event:missing".into()],
            reliability: "rumor".into(),
        });
        let error = compose_world_newspaper(&campaign, "The Clarion", 8).unwrap_err();
        assert!(error.to_string().contains("unknown event"));
    }
}
