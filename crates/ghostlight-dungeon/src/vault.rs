use crate::domain::{SourceWitness, VaultEvidenceReceipt};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
}
impl VoidBotMcpVault {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
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
        let text = self.client.post(&self.endpoint)
            .header("accept", "application/json, text/event-stream")
            .json(&serde_json::json!({"jsonrpc":"2.0","id":"ghostlight-vault","method":"tools/call","params":{"name":name,"arguments":arguments}}))
            .send().await?.error_for_status()?.text().await?;
        let payload = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .ok_or_else(|| anyhow!("VoidBot MCP returned no event payload"))?;
        Ok(serde_json::from_str(payload)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, response::IntoResponse, routing::post};

    #[tokio::test]
    async fn voidbot_search_results_become_exact_evidence_witnesses() {
        let app=Router::new().route("/mcp",post(||async { ([("content-type","text/event-stream")], "event: message\ndata: {\"result\":{\"structuredContent\":{\"results\":[{\"sourceId\":\"AetheriaLore:place.md\",\"path\":\"place.md\",\"lineStart\":7,\"lineEnd\":9,\"text\":\"The route takes six hours.\"}]}}}\n\n").into_response() }));
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
        assert_eq!(result.witnesses[0].exact_locator, "place.md:7-9");
        assert!(result.witnesses[0].content_hash.starts_with("sha256:"));
        assert_eq!(result.witnesses[0].authority_lane, "AetheriaLore");
    }

    #[tokio::test]
    async fn voidbot_exact_document_is_hashed_as_the_complete_archive_witness() {
        let app=Router::new().route("/mcp",post(||async { ([ ("content-type","text/event-stream") ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"found\":true,\"sourceId\":\"AetheriaLore:forge.md\",\"repoName\":\"AetheriaLore\",\"path\":\"forge.md\",\"content\":\"John keeps the forge.\\nThe road takes six hours.\",\"lastModifiedAt\":\"2026-01-01T00:00:00Z\"}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let witness = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .exact_document("AetheriaLore:forge.md")
            .await
            .unwrap();
        assert_eq!(
            witness.excerpt,
            "John keeps the forge.\nThe road takes six hours."
        );
        assert_eq!(witness.exact_locator, "forge.md");
        assert_eq!(witness.authority_lane, "AetheriaLore");
        assert_eq!(
            witness.content_hash,
            format!("sha256:{:x}", Sha256::digest(witness.excerpt.as_bytes()))
        );
    }

    #[tokio::test]
    async fn voidbot_context_combines_the_typed_chunk_window() {
        let app=Router::new().route("/mcp",post(||async { ([ ("content-type","text/event-stream") ], "event: message\ndata: {\"result\":{\"structuredContent\":{\"found\":true,\"sourceId\":\"AetheriaLore:forge.md\",\"repoName\":\"AetheriaLore\",\"path\":\"forge.md\",\"chunks\":[{\"lineStart\":1,\"lineEnd\":2,\"text\":\"first\"},{\"lineStart\":3,\"lineEnd\":4,\"text\":\"second\"}]}}}\n\n").into_response() }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let witness = VoidBotMcpVault::new(format!("http://{address}/mcp"))
            .surrounding_context("AetheriaLore:forge.md", 0)
            .await
            .unwrap();
        assert_eq!(witness.excerpt, "first\nsecond");
        assert_eq!(witness.exact_locator, "forge.md:1-4");
    }
}
#[async_trait]
impl VaultProvider for VoidBotMcpVault {
    async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt> {
        let response = self
            .call_tool(
                "search_sources",
                serde_json::json!({
                    "query":query.query,
                    "limit":query.limit,
                    "repoName": if query.authority_lanes.iter().any(|lane| lane == "AetheriaLore") {
                        Some("AetheriaLore")
                    } else {
                        None
                    }
                }),
            )
            .await?;
        let results = response
            .pointer("/result/structuredContent/results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("VoidBot MCP returned no typed search results"))?;
        let witnesses = results
            .iter()
            .map(|item| witness_from_result(item, query))
            .collect::<Result<Vec<_>>>()?;
        Ok(receipt(self.provider_id(), query, witnesses))
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
            authority_lane: value
                .get("repoName")
                .and_then(|v| v.as_str())
                .unwrap_or("source_context")
                .into(),
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
        Ok(SourceWitness {
            source_id: source_id.into(),
            exact_locator: path.into(),
            content_hash: format!("sha256:{:x}", Sha256::digest(content.as_bytes())),
            excerpt: content.into(),
            authority_lane: value
                .get("repoName")
                .and_then(|v| v.as_str())
                .unwrap_or("source_archive")
                .into(),
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

fn witness_from_result(item: &serde_json::Value, query: &VaultQuery) -> Result<SourceWitness> {
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
    Ok(SourceWitness {
        source_id: source_id.into(),
        exact_locator: locator,
        content_hash: format!("sha256:{:x}", Sha256::digest(excerpt.as_bytes())),
        excerpt: excerpt.into(),
        authority_lane: query.authority_lanes.join(","),
        temporal_scope: query.temporal_scope.clone(),
    })
}

fn receipt(
    provider: &str,
    query: &VaultQuery,
    witnesses: Vec<SourceWitness>,
) -> VaultEvidenceReceipt {
    let query_bytes = rmp_serde::to_vec_named(query).expect("query serializes");
    let query_hash = format!("sha256:{:x}", Sha256::digest(query_bytes));
    VaultEvidenceReceipt {
        schema: "ghostlight.vault_evidence_receipt.v1".into(),
        id: format!("vault:{query_hash}"),
        provider: provider.into(),
        query_hash,
        witnesses,
        retrieved_at: Utc::now(),
    }
}
