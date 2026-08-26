use crate::domain::{SourceWitness, VaultEvidenceReceipt};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

const VOIDBOT_SEARCH_RESULT_LIMIT: u8 = 12;
pub const DEFAULT_VAULT_ID: &str = "aetheria";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VaultSourceManifest {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub provider_id: String,
    pub repository_name: String,
    pub source_root: String,
    pub git_remote: String,
    pub compiler_authority_lanes: Vec<String>,
    pub player_authority_lanes: Vec<String>,
}

pub fn bundled_vault_manifests() -> Vec<VaultSourceManifest> {
    vec![
        VaultSourceManifest {
            schema: "ghostlight.vault_source_manifest.v1".into(),
            id: "aetheria".into(),
            title: "Aetheria".into(),
            provider_id: "voidbot.aetheria".into(),
            repository_name: "AetheriaLore".into(),
            source_root: "Aetheria/Worldbuilding/".into(),
            git_remote: "https://github.com/GameCult/AetheriaLore.git".into(),
            compiler_authority_lanes: vec!["aetheria.canon_worldbuilding".into()],
            player_authority_lanes: vec!["aetheria.canon_worldbuilding".into()],
        },
        VaultSourceManifest {
            schema: "ghostlight.vault_source_manifest.v1".into(),
            id: "kalsa".into(),
            title: "Kalsa".into(),
            provider_id: "voidbot.kalsa".into(),
            repository_name: "Kalsa".into(),
            source_root: "Kalsa/".into(),
            git_remote: "https://github.com/GameCult/Kalsa.git".into(),
            compiler_authority_lanes: vec!["kalsa.public".into(), "kalsa.gm_canon".into()],
            player_authority_lanes: vec!["kalsa.public".into()],
        },
    ]
}

pub fn canonical_vault_id(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        // `fixture` is the persisted legacy migration label used before Vault
        // selection became typed. Those campaigns were compiled from the
        // bundled Aetheria provider.
        "aetheria" | "aetherialore" | "voidbot.aetheria" | "fixture" => Some("aetheria"),
        "kalsa" | "voidbot.kalsa" => Some("kalsa"),
        _ => None,
    }
}

#[derive(Debug, Error)]
#[error("{provider} Vault retrieval is unavailable: {detail}")]
pub struct VaultUnavailable {
    provider: String,
    detail: String,
}

impl VaultUnavailable {
    fn new(provider: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            detail: detail.into(),
        }
    }
}

pub fn is_vault_unavailable(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<VaultUnavailable>().is_some())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultQuery {
    pub query: String,
    pub authority_lanes: Vec<String>,
    pub temporal_scope: String,
    pub limit: u8,
}

#[async_trait]
pub trait VaultProvider: Send + Sync {
    async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt>;
    async fn surrounding_context(&self, source_id: &str, chunk_index: u32)
    -> Result<SourceWitness>;
    async fn exact_document(&self, source_id: &str) -> Result<SourceWitness>;
    fn provider_id(&self) -> &'static str;
}

#[derive(Clone)]
pub struct FixtureVault {
    pub witnesses: Vec<SourceWitness>,
}
#[async_trait]
impl VaultProvider for FixtureVault {
    async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt> {
        Ok(receipt(
            self.provider_id(),
            query,
            self.witnesses
                .iter()
                .take(query.limit as usize)
                .cloned()
                .collect(),
        ))
    }
    async fn exact_document(&self, source_id: &str) -> Result<SourceWitness> {
        self.witnesses
            .iter()
            .find(|w| w.source_id == source_id)
            .cloned()
            .ok_or_else(|| anyhow!("fixture source not found"))
    }
    async fn surrounding_context(&self, source_id: &str, _: u32) -> Result<SourceWitness> {
        self.exact_document(source_id).await
    }
    fn provider_id(&self) -> &'static str {
        "fixture"
    }
}

#[derive(Clone)]
pub struct VoidBotMcpVault {
    client: Client,
    endpoint: String,
    manifests: BTreeMap<String, VaultSourceManifest>,
}
impl VoidBotMcpVault {
    pub fn new(endpoint: impl Into<String>) -> Self {
        let manifests = bundled_vault_manifests()
            .into_iter()
            .map(|manifest| (manifest.id.clone(), manifest))
            .collect();
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            manifests,
        }
    }
    pub fn starfire_loopback() -> Self {
        Self::new("http://127.0.0.1:17875/mcp")
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let response = self.client.post(&self.endpoint)
            .header("accept", "application/json, text/event-stream")
            .json(&serde_json::json!({"jsonrpc":"2.0","id":"ghostlight-vault","method":"tools/call","params":{"name":name,"arguments":arguments}}))
            .send()
            .await
            .map_err(|error| VaultUnavailable::new(self.provider_id(), error.to_string()))?
            .error_for_status()
            .map_err(|error| VaultUnavailable::new(self.provider_id(), error.to_string()))?;
        let text = response
            .text()
            .await
            .map_err(|error| VaultUnavailable::new(self.provider_id(), error.to_string()))?;
        let payload = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .ok_or_else(|| {
                VaultUnavailable::new(self.provider_id(), "MCP returned no event payload")
            })?;
        let value: serde_json::Value = serde_json::from_str(payload).map_err(|error| {
            VaultUnavailable::new(
                self.provider_id(),
                format!("MCP returned an invalid event payload: {error}"),
            )
        })?;
        if let Some(error) = value.pointer("/error/message").and_then(|v| v.as_str()) {
            return Err(VaultUnavailable::new(
                self.provider_id(),
                format!("MCP JSON-RPC error: {error}"),
            )
            .into());
        }
        if value.pointer("/result/isError").and_then(|v| v.as_bool()) == Some(true) {
            let detail = value
                .pointer("/result/content/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("tool returned an unspecified error");
            return Err(VaultUnavailable::new(
                self.provider_id(),
                format!("MCP tool error: {detail}"),
            )
            .into());
        }
        Ok(value)
    }

    fn manifest_for_query(&self, query: &VaultQuery) -> Result<&VaultSourceManifest> {
        let selected = query
            .authority_lanes
            .iter()
            .find_map(|value| canonical_vault_id(value))
            .ok_or_else(|| anyhow!("Vault query has no configured Vault identity"))?;
        self.manifests
            .get(selected)
            .ok_or_else(|| anyhow!("unknown Vault {selected}"))
    }

    fn manifest_for_source(&self, source_id: &str) -> Result<&VaultSourceManifest> {
        let repository = source_id
            .split_once(':')
            .map(|(repository, _)| repository)
            .ok_or_else(|| anyhow!("Vault source ID has no repository namespace"))?;
        self.manifests
            .values()
            .find(|manifest| manifest.repository_name.eq_ignore_ascii_case(repository))
            .ok_or_else(|| anyhow!("Vault source {source_id} is outside every configured Vault"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, response::IntoResponse, routing::post};

    #[test]
    fn vault_unavailability_survives_anyhow_context() {
        let unavailable = anyhow::Error::new(VaultUnavailable::new(
            "voidbot.aetheria",
            "embedding service is offline",
        ))
        .context("world compilation retrieval failed");
        assert!(is_vault_unavailable(&unavailable));
        assert!(!is_vault_unavailable(&anyhow!(
            "world candidate violated topology"
        )));
    }

    #[tokio::test]
    async fn voidbot_search_results_become_exact_evidence_witnesses() {
        let app=Router::new().route("/mcp",post(||async { ([("content-type","text/event-stream")], "event: message\ndata: {\"result\":{\"structuredContent\":{\"results\":[{\"sourceId\":\"AetheriaLore:Aetheria/Worldbuilding/place.md\",\"path\":\"Aetheria/Worldbuilding/place.md\",\"lineStart\":7,\"lineEnd\":9,\"text\":\"The route takes six hours.\"}]}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let vault = VoidBotMcpVault::new(format!("http://{address}/mcp"));
        let result = vault
            .search(&VaultQuery {
                query: "route".into(),
                authority_lanes: vec!["AetheriaLore".into()],
                temporal_scope: "era".into(),
                limit: 3,
            })
            .await
            .unwrap();
        assert_eq!(
            result.witnesses[0].exact_locator,
            "Aetheria/Worldbuilding/place.md:7-9"
        );
        assert!(result.witnesses[0].content_hash.starts_with("sha256:"));
        assert_eq!(
            result.witnesses[0].authority_lane,
            "aetheria.canon_worldbuilding"
        );
    }

    #[tokio::test]
    async fn voidbot_search_caps_requests_to_the_provider_contract() {
        let app = Router::new().route(
            "/mcp",
            post(|axum::Json(body): axum::Json<serde_json::Value>| async move {
                assert_eq!(
                    body.pointer("/params/arguments/limit")
                        .and_then(|value| value.as_u64()),
                    Some(VOIDBOT_SEARCH_RESULT_LIMIT as u64)
                );
                assert_eq!(
                    body.pointer("/params/arguments/pathPrefix")
                        .and_then(|value| value.as_str()),
                    Some("Aetheria/Worldbuilding/")
                );
                ([
                    ("content-type", "text/event-stream"),
                ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"results\":[{\"sourceId\":\"AetheriaLore:Aetheria/Worldbuilding/place.md\",\"path\":\"Aetheria/Worldbuilding/place.md\",\"lineStart\":7,\"lineEnd\":9,\"text\":\"The route takes six hours.\"}]}}}\n\n").into_response()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let requested = VaultQuery {
            query: "route".into(),
            authority_lanes: vec!["AetheriaLore".into()],
            temporal_scope: "era".into(),
            limit: 18,
        };
        let result = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .search(&requested)
            .await
            .unwrap();
        let mut effective = requested;
        effective.limit = VOIDBOT_SEARCH_RESULT_LIMIT;
        assert_eq!(
            result.query_hash,
            receipt("voidbot.aetheria", &effective, vec![]).query_hash
        );
    }

    #[tokio::test]
    async fn voidbot_exact_document_is_hashed_as_the_complete_archive_witness() {
        let app=Router::new().route("/mcp",post(||async { ([ ("content-type","text/event-stream") ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"found\":true,\"sourceId\":\"AetheriaLore:Aetheria/Worldbuilding/forge.md\",\"repoName\":\"AetheriaLore\",\"path\":\"Aetheria/Worldbuilding/forge.md\",\"content\":\"John keeps the forge.\\nThe road takes six hours.\",\"lastModifiedAt\":\"2026-01-01T00:00:00Z\"}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let witness = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .exact_document("AetheriaLore:Aetheria/Worldbuilding/forge.md")
            .await
            .unwrap();
        assert_eq!(
            witness.excerpt,
            "John keeps the forge.\nThe road takes six hours."
        );
        assert_eq!(witness.exact_locator, "Aetheria/Worldbuilding/forge.md");
        assert_eq!(witness.authority_lane, "aetheria.canon_worldbuilding");
        assert_eq!(
            witness.content_hash,
            format!("sha256:{:x}", Sha256::digest(witness.excerpt.as_bytes()))
        );
    }

    #[tokio::test]
    async fn voidbot_context_combines_the_typed_chunk_window() {
        let app=Router::new().route("/mcp",post(||async { ([ ("content-type","text/event-stream") ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"found\":true,\"sourceId\":\"AetheriaLore:Aetheria/Worldbuilding/forge.md\",\"repoName\":\"AetheriaLore\",\"path\":\"Aetheria/Worldbuilding/forge.md\",\"chunks\":[{\"lineStart\":1,\"lineEnd\":2,\"text\":\"first\"},{\"lineStart\":3,\"lineEnd\":4,\"text\":\"second\"}]}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let witness = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .surrounding_context("AetheriaLore:Aetheria/Worldbuilding/forge.md", 0)
            .await
            .unwrap();
        assert_eq!(witness.excerpt, "first\nsecond");
        assert_eq!(witness.exact_locator, "Aetheria/Worldbuilding/forge.md:1-4");
    }

    #[test]
    fn aetheria_paths_preserve_document_authority() {
        assert_eq!(
            authority_lane_for_path("Aetheria/Worldbuilding/Factions/Corvid.md"),
            "aetheria.canon_worldbuilding"
        );
        assert_eq!(
            authority_lane_for_path("Aetheria/Fiction/The Burden of Proof.md"),
            "aetheria.canonical_fiction"
        );
        assert_eq!(
            authority_lane_for_path("Aetheria/Stories/Corvid Collective First Exodus.md"),
            "aetheria.legacy_story"
        );
        assert_eq!(
            authority_lane_for_path("Aetheria/static/interactive/corvid.branch.json"),
            "aetheria.fixture_artifact"
        );
        assert_eq!(
            authority_lane_for_path("Aetheria/Brainstorming/Stories/draft.md"),
            "aetheria.draft_working"
        );
    }

    #[test]
    fn kalsa_manifest_keeps_public_and_gm_lanes_distinct() {
        let manifest = bundled_vault_manifests()
            .into_iter()
            .find(|manifest| manifest.id == "kalsa")
            .unwrap();
        assert_eq!(
            authority_lane_for_source(&manifest, "Kalsa/Public/World/Magic and Miracles.md")
                .unwrap(),
            "kalsa.public"
        );
        assert_eq!(
            authority_lane_for_source(
                &manifest,
                "Kalsa/Spoilers/Foundations/Divine Interception.md"
            )
            .unwrap(),
            "kalsa.gm_canon"
        );
        assert!(
            authority_lane_for_source(&manifest, "workshop/Direction and Constraints.md").is_err()
        );
        let public_query = VaultQuery {
            query: "starting places".into(),
            authority_lanes: vec!["kalsa".into(), "visibility.player".into()],
            temporal_scope: "all".into(),
            limit: 3,
        };
        assert_eq!(query_source_root(&manifest, &public_query), "Kalsa/Public/");
    }

    #[tokio::test]
    async fn kalsa_search_is_repository_and_root_scoped() {
        let app = Router::new().route(
            "/mcp",
            post(|axum::Json(body): axum::Json<serde_json::Value>| async move {
                assert_eq!(
                    body.pointer("/params/arguments/repoName")
                        .and_then(|value| value.as_str()),
                    Some("Kalsa")
                );
                assert_eq!(
                    body.pointer("/params/arguments/pathPrefix")
                        .and_then(|value| value.as_str()),
                    Some("Kalsa/")
                );
                ([
                    ("content-type", "text/event-stream"),
                ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"results\":[{\"sourceId\":\"Kalsa:Kalsa/Spoilers/Places/Low Sere.md\",\"path\":\"Kalsa/Spoilers/Places/Low Sere.md\",\"lineStart\":7,\"lineEnd\":9,\"text\":\"Low Sere keeps three ledgers.\"}]}}}\n\n").into_response()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let result = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .search(&VaultQuery {
                query: "Low Sere".into(),
                authority_lanes: vec!["kalsa".into()],
                temporal_scope: "campaign start".into(),
                limit: 3,
            })
            .await
            .unwrap();
        assert_eq!(result.provider, "voidbot.kalsa");
        assert_eq!(result.witnesses[0].authority_lane, "kalsa.gm_canon");
    }

    #[tokio::test]
    async fn a_vault_cannot_return_another_repository() {
        let app=Router::new().route("/mcp",post(||async { ([("content-type","text/event-stream")], "event: message\ndata: {\"result\":{\"structuredContent\":{\"results\":[{\"sourceId\":\"Kalsa:Kalsa/Public/index.md\",\"path\":\"Kalsa/Public/index.md\",\"lineStart\":1,\"lineEnd\":2,\"text\":\"Wrong world.\"}]}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let error = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .search(&VaultQuery {
                query: "route".into(),
                authority_lanes: vec!["aetheria".into()],
                temporal_scope: "era".into(),
                limit: 3,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("crossed repository boundary"));
    }
}
#[async_trait]
impl VaultProvider for VoidBotMcpVault {
    async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt> {
        let mut effective_query = query.clone();
        effective_query.limit = effective_query.limit.min(VOIDBOT_SEARCH_RESULT_LIMIT);
        let manifest = self.manifest_for_query(&effective_query)?;
        let source_root = query_source_root(manifest, &effective_query);
        let response = self
            .call_tool(
                "search_sources",
                serde_json::json!({
                    "query":effective_query.query,
                    "limit":effective_query.limit,
                    "repoName":manifest.repository_name,
                    "pathPrefix":source_root,
                }),
            )
            .await?;
        let results = response
            .pointer("/result/structuredContent/results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("VoidBot MCP returned no typed search results"))?;
        let witnesses = results
            .iter()
            .map(|item| witness_from_result(item, &effective_query, manifest, source_root))
            .collect::<Result<Vec<_>>>()?;
        Ok(receipt(&manifest.provider_id, &effective_query, witnesses))
    }
    async fn surrounding_context(
        &self,
        source_id: &str,
        chunk_index: u32,
    ) -> Result<SourceWitness> {
        let response = self
            .call_tool(
                "get_source_context",
                serde_json::json!({"sourceId":source_id,"chunkIndex":chunk_index}),
            )
            .await?;
        let value = response
            .pointer("/result/structuredContent")
            .ok_or_else(|| anyhow!("VoidBot MCP returned no source context"))?;
        let manifest = self.manifest_for_source(source_id)?;
        let chunks = value
            .get("chunks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("VoidBot source context contained no chunks"))?;
        let excerpt = chunks
            .iter()
            .filter_map(|chunk| chunk.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        if excerpt.is_empty() {
            return Err(anyhow!("VoidBot source context contained no text"));
        }
        let path = value
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(source_id);
        validate_manifest_source(manifest, source_id, path)?;
        let first_line = chunks
            .first()
            .and_then(|chunk| chunk.get("lineStart"))
            .and_then(|v| v.as_u64());
        let last_line = chunks
            .last()
            .and_then(|chunk| chunk.get("lineEnd"))
            .and_then(|v| v.as_u64());
        let exact_locator = match (first_line, last_line) {
            (Some(first), Some(last)) => format!("{path}:{first}-{last}"),
            _ => format!("{source_id}#chunk={chunk_index}"),
        };
        Ok(SourceWitness {
            source_id: source_id.into(),
            exact_locator,
            content_hash: format!("sha256:{:x}", Sha256::digest(excerpt.as_bytes())),
            excerpt,
            authority_lane: authority_lane_for_source(manifest, path)?.into(),
            temporal_scope: "unspecified".into(),
        })
    }
    async fn exact_document(&self, source_id: &str) -> Result<SourceWitness> {
        let response = self
            .call_tool(
                "get_exact_source_document",
                serde_json::json!({"sourceId":source_id}),
            )
            .await?;
        let value = response
            .pointer("/result/structuredContent")
            .ok_or_else(|| anyhow!("VoidBot MCP returned no exact source document"))?;
        let manifest = self.manifest_for_source(source_id)?;
        if value.get("found").and_then(|v| v.as_bool()) != Some(true) {
            return Err(anyhow!("VoidBot exact source document was not found"));
        }
        let content = value
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("VoidBot exact source document contained no content"))?;
        let path = value
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(source_id);
        validate_manifest_source(manifest, source_id, path)?;
        Ok(SourceWitness {
            source_id: source_id.into(),
            exact_locator: path.into(),
            content_hash: format!("sha256:{:x}", Sha256::digest(content.as_bytes())),
            excerpt: content.into(),
            authority_lane: authority_lane_for_source(manifest, path)?.into(),
            temporal_scope: value
                .get("lastModifiedAt")
                .and_then(|v| v.as_str())
                .unwrap_or("unspecified")
                .into(),
        })
    }
    fn provider_id(&self) -> &'static str {
        "voidbot.aetheria"
    }
}

fn witness_from_result(
    item: &serde_json::Value,
    query: &VaultQuery,
    manifest: &VaultSourceManifest,
    required_root: &str,
) -> Result<SourceWitness> {
    let source_id = item
        .get("sourceId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("search result missing sourceId"))?;
    let excerpt = item
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("search result missing text"))?;
    let locator = match (
        item.get("path").and_then(|v| v.as_str()),
        item.get("lineStart").and_then(|v| v.as_u64()),
        item.get("lineEnd").and_then(|v| v.as_u64()),
    ) {
        (Some(path), Some(start), Some(end)) => format!("{path}:{start}-{end}"),
        _ => source_id.into(),
    };
    let path = item
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or(source_id);
    validate_manifest_source(manifest, source_id, path)?;
    validate_source_root(required_root, path)?;
    Ok(SourceWitness {
        source_id: source_id.into(),
        exact_locator: locator,
        content_hash: format!("sha256:{:x}", Sha256::digest(excerpt.as_bytes())),
        excerpt: excerpt.into(),
        authority_lane: authority_lane_for_source(manifest, path)?.into(),
        temporal_scope: query.temporal_scope.clone(),
    })
}

fn query_source_root<'a>(manifest: &'a VaultSourceManifest, query: &VaultQuery) -> &'a str {
    if manifest.id == "kalsa"
        && query
            .authority_lanes
            .iter()
            .any(|lane| lane == "visibility.player")
    {
        "Kalsa/Public/"
    } else {
        &manifest.source_root
    }
}

fn validate_source_root(required_root: &str, path: &str) -> Result<()> {
    if !path
        .replace('\\', "/")
        .to_ascii_lowercase()
        .starts_with(&required_root.to_ascii_lowercase())
    {
        return Err(anyhow!(
            "Vault provider crossed requested visibility boundary: expected {required_root}, received {path}"
        ));
    }
    Ok(())
}

fn validate_manifest_source(
    manifest: &VaultSourceManifest,
    source_id: &str,
    path: &str,
) -> Result<()> {
    let repository = source_id
        .split_once(':')
        .map(|(repository, _)| repository)
        .ok_or_else(|| anyhow!("Vault source ID has no repository namespace"))?;
    if !repository.eq_ignore_ascii_case(&manifest.repository_name) {
        return Err(anyhow!(
            "Vault provider crossed repository boundary: expected {}, received {source_id}",
            manifest.repository_name
        ));
    }
    let normalized_path = path.replace('\\', "/");
    if !normalized_path
        .to_ascii_lowercase()
        .starts_with(&manifest.source_root.to_ascii_lowercase())
    {
        return Err(anyhow!(
            "Vault provider crossed source-root boundary: expected {}, received {path}",
            manifest.source_root
        ));
    }
    Ok(())
}

fn authority_lane_for_source(manifest: &VaultSourceManifest, path: &str) -> Result<&'static str> {
    match manifest.id.as_str() {
        "aetheria" => Ok(authority_lane_for_path(path)),
        "kalsa" => {
            let normalized = path.replace('\\', "/").to_ascii_lowercase();
            if normalized.starts_with("kalsa/public/") {
                Ok("kalsa.public")
            } else if normalized.starts_with("kalsa/spoilers/") {
                Ok("kalsa.gm_canon")
            } else if normalized == "kalsa/index.md" {
                Ok("kalsa.navigation")
            } else {
                Err(anyhow!(
                    "Kalsa source path has no admitted authority lane: {path}"
                ))
            }
        }
        other => Err(anyhow!("Vault {other} has no authority classifier")),
    }
}

/// Classify source authority at the Vault boundary. Downstream models receive a
/// typed document role instead of having to infer authority from prose.
fn authority_lane_for_path(path: &str) -> &'static str {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/static/interactive/") {
        "aetheria.fixture_artifact"
    } else if normalized.contains("/brainstorming/") {
        "aetheria.draft_working"
    } else if normalized.contains("/game design/") {
        "aetheria.design_reference"
    } else if normalized.contains("/fiction/") {
        "aetheria.canonical_fiction"
    } else if normalized.contains("/stories/") {
        "aetheria.legacy_story"
    } else if normalized.contains("/worldbuilding/") {
        "aetheria.canon_worldbuilding"
    } else {
        "aetheria.vault_document"
    }
}

fn receipt(
    provider: &str,
    query: &VaultQuery,
    witnesses: Vec<SourceWitness>,
) -> VaultEvidenceReceipt {
    let query_bytes = rmp_serde::to_vec_named(query).expect("query serializes");
    let query_hash = format!("sha256:{:x}", Sha256::digest(query_bytes));
    let retrieved_at = Utc::now();
    let receipt_bytes =
        rmp_serde::to_vec_named(&(provider, &query_hash, &witnesses, &retrieved_at))
            .expect("vault receipt identity serializes");
    VaultEvidenceReceipt {
        schema: "ghostlight.vault_evidence_receipt.v1".into(),
        id: format!("vault:sha256:{:x}", Sha256::digest(receipt_bytes)),
        provider: provider.into(),
        query_hash,
        witnesses,
        retrieved_at,
    }
}
