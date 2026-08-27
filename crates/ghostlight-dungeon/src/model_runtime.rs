use crate::{
    model::{DeepSeekPort, ModelPort, ModelRuntimeStatus, OpenRouterPort},
    model_connector::CodexConnectorModelPort,
};
use anyhow::{Context, Result, bail};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRuntimeSelection {
    pub provider: String,
    pub fast_model: String,
    pub balanced_model: String,
    pub capable_model: String,
    pub credential_path: PathBuf,
    pub connector_endpoint: SocketAddr,
    pub runtime_id: String,
    pub connector_max_concurrent_requests: usize,
}

impl ModelRuntimeSelection {
    pub fn from_environment(runtime_root: impl AsRef<Path>) -> Result<Self> {
        let provider = std::env::var("GHOSTLIGHT_MODEL_PROVIDER")
            .unwrap_or_else(|_| "deepseek".into())
            .to_ascii_lowercase();
        let (
            default_fast_model,
            default_balanced_model,
            default_capable_model,
            default_secret_name,
        ) = match provider.as_str() {
            "deepseek" => (
                "deepseek-v4-flash",
                "deepseek-v4-pro",
                "deepseek-v4-pro",
                "deepseek.dpapi",
            ),
            "openrouter" => (
                "stealth/ox-alpha",
                "stealth/ox-alpha",
                "stealth/ox-alpha",
                "openrouter.key",
            ),
            "codex-connector" => (
                "gpt-5.6-luna",
                "gpt-5.6-terra",
                "gpt-5.6-luna",
                "codex-connector.key",
            ),
            unsupported => bail!("unsupported model provider {unsupported}"),
        };
        Ok(Self {
            provider,
            fast_model: std::env::var("GHOSTLIGHT_MODEL_FAST")
                .unwrap_or_else(|_| default_fast_model.into()),
            balanced_model: std::env::var("GHOSTLIGHT_MODEL_BALANCED")
                .unwrap_or_else(|_| default_balanced_model.into()),
            capable_model: std::env::var("GHOSTLIGHT_MODEL_CAPABLE")
                .unwrap_or_else(|_| default_capable_model.into()),
            credential_path: std::env::var_os("GHOSTLIGHT_MODEL_CREDENTIAL")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    runtime_root
                        .as_ref()
                        .join("secrets")
                        .join(default_secret_name)
                }),
            connector_endpoint: std::env::var("GHOSTLIGHT_MODEL_CONNECTOR")
                .unwrap_or_else(|_| "127.0.0.1:4103".to_string())
                .parse()
                .context("GHOSTLIGHT_MODEL_CONNECTOR must be a socket address")?,
            runtime_id: std::env::var("GHOSTLIGHT_RUNTIME_ID")
                .unwrap_or_else(|_| "ghostlight-dungeon-yggdrasil".to_string()),
            connector_max_concurrent_requests: std::env::var(
                "GHOSTLIGHT_CODEX_CONNECTOR_MAX_CONCURRENT_REQUESTS",
            )
            .unwrap_or_else(|_| "8".to_string())
            .parse::<usize>()
            .context("GHOSTLIGHT_CODEX_CONNECTOR_MAX_CONCURRENT_REQUESTS must be an integer")?,
        })
    }

    pub fn status(&self, readiness: impl Into<String>) -> ModelRuntimeStatus {
        ModelRuntimeStatus {
            provider: self.provider.clone(),
            fast_model: self.fast_model.clone(),
            balanced_model: self.balanced_model.clone(),
            capable_model: self.capable_model.clone(),
            readiness: readiness.into(),
        }
    }

    pub fn open(&self) -> Result<Option<Arc<dyn ModelPort>>> {
        if !self.credential_path.is_file() {
            return Ok(None);
        }
        let provider: Arc<dyn ModelPort> = match self.provider.as_str() {
            "deepseek" => Arc::new(DeepSeekPort::from_runtime_secret_with_models(
                &self.credential_path,
                self.fast_model.clone(),
                self.capable_model.clone(),
            )?),
            "openrouter" => Arc::new(OpenRouterPort::from_runtime_secret(
                &self.credential_path,
                self.fast_model.clone(),
                self.capable_model.clone(),
            )?),
            "codex-connector" => Arc::new(CodexConnectorModelPort::from_runtime_secret(
                self.connector_endpoint,
                &self.credential_path,
                self.runtime_id.clone(),
                self.fast_model.clone(),
                self.balanced_model.clone(),
                self.capable_model.clone(),
                self.connector_max_concurrent_requests,
            )?),
            unsupported => bail!("unsupported model provider {unsupported}"),
        };
        Ok(Some(provider))
    }
}
