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

    let mut articles: Vec<WorldNewspaperArticle> = Vec::new();
    let mut last_article_at = None;
    for issue in news {
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
        let article = WorldNewspaperArticle {
            id: format!("article:{}", issue.id),
            section: section_for(primary).into(),
            headline: issue.headline.clone(),
            dateline,
            body,
            channel: issue.channel.clone(),
            reliability: issue.reliability.clone(),
            event_ids: issue.event_ids.clone(),
        };
        if last_article_at == Some(primary.at)
            && articles.last().is_some_and(|previous| {
                previous.section == article.section
                    && previous.headline == article.headline
                    && previous.dateline == article.dateline
                    && previous.body == article.body
                    && previous.channel == article.channel
                    && previous.reliability == article.reliability
            })
        {
            let previous = articles.last_mut().expect("matching article exists");
            for event_id in article.event_ids {
                if !previous.event_ids.contains(&event_id) {
                    previous.event_ids.push(event_id);
                }
            }
            continue;
        }
        if articles.len() == max_articles {
            break;
        }
        last_article_at = Some(primary.at);
        articles.push(article);
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
        "strategic_activity_outcome" => "Consequences",
        "gestalt_action" | "gestalt_activity" | "gestalt_migration" => "Realms",
        "actor_activity" | "actor_move" | "member_activity" | "member_migration" => {
            "People & Plots"
        }
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
        let summary = "The Thorn Court admits its royal seal was pawned to pay a dragon's gambling debt, then dismisses the treasurer who carried the confession into open court."
            .to_string();
        campaign.events.push(Event {
            id: "event:seal-scandal".into(),
            at: campaign.world_time,
            kind: "institution_action".into(),
            summary: summary.clone(),
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
            headline: crate::domain::committed_news_headline(&summary),
            event_ids: vec!["event:seal-scandal".into()],
            reliability: "committed public channel".into(),
        });

        let issue = compose_world_newspaper(&campaign, "The Underdeep Clarion", 8).unwrap();
        let markdown = render_world_newspaper_markdown(&issue);
        assert_eq!(issue.articles[0].event_ids, ["event:seal-scandal"]);
        assert_eq!(
            issue.articles[0].headline,
            crate::domain::committed_news_headline(&summary)
        );
        assert_ne!(issue.articles[0].headline, summary);
        assert_eq!(issue.articles[0].body, summary);
        assert!(markdown.contains("dismisses the treasurer"));
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

    #[test]
    fn newspaper_collapses_one_public_attempt_with_multiple_typed_effect_events() {
        let mut campaign = crate::kernel::tests::campaign();
        let summary = "Veska Rill sends the kiln ledgers to every guild and seals the originals.";
        for (suffix, kind) in [
            ("communicate", "actor_activity"),
            ("seal", "actor_activity"),
        ] {
            let event_id = format!("event:{suffix}");
            campaign.events.push(Event {
                id: event_id.clone(),
                at: campaign.world_time,
                kind: kind.into(),
                summary: summary.into(),
                actor_ids: vec!["player".into()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["room".into()],
                public_channels: vec!["court broadsheet".into()],
            });
            campaign.news.push(NewsIssue {
                id: format!("news:{suffix}"),
                at: campaign.world_time,
                channel: "court broadsheet".into(),
                headline: crate::domain::committed_news_headline(summary),
                event_ids: vec![event_id],
                reliability: "committed public channel".into(),
            });
        }

        let issue = compose_world_newspaper(&campaign, "The Underdeep Clarion", 8).unwrap();
        assert_eq!(issue.articles.len(), 1);
        assert_eq!(issue.articles[0].event_ids.len(), 2);
        assert_eq!(issue.articles[0].section, "People & Plots");
        assert_eq!(issue.articles[0].body, summary);
    }
}
