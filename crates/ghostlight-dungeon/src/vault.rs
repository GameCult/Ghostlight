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
    pub fn starfire_loopback() -> Self {
        Self {
            client: Client::new(),
            endpoint: "http://127.0.0.1:17875/mcp".into(),
        }
    }
}
#[async_trait]
impl VaultProvider for VoidBotMcpVault {
    async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt> {
        let response: serde_json::Value = self.client.post(&self.endpoint).json(&serde_json::json!({"jsonrpc":"2.0","id":"ghostlight-vault","method":"tools/call","params":{"name":"search_sources","arguments":{"query":query.query,"limit":query.limit}}})).send().await?.error_for_status()?.json().await?;
        let witnesses = response
            .pointer("/result/structuredContent/witnesses")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| anyhow!("VoidBot MCP returned no typed witnesses"))?;
        Ok(receipt(self.provider_id(), query, witnesses))
    }
    async fn exact_document(&self, _source_id: &str) -> Result<SourceWitness> {
        Err(anyhow!(
            "exact-document VoidBot adapter awaits restored trusted crossing contract verification"
        ))
    }
    fn provider_id(&self) -> &'static str {
        "voidbot.aetheria"
    }
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
