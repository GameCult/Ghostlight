use anyhow::{Context, bail};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const APP_SLUG: &str = "ghostlight";

#[derive(Clone)]
pub struct HeimdallClient {
    client: Client,
    base_url: String,
    public_app_url: String,
    callback_url: String,
    discord_guild_id: String,
    discord_role_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest<'a> {
    app_slug: &'a str,
    mode: &'a str,
    return_to: &'a str,
    handoff: Handoff<'a>,
    requested_scopes: [&'a str; 2],
    entitlement_policy: DiscordRolePolicy<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Handoff<'a> {
    kind: &'a str,
    attempt_id: &'a str,
    callback_url: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordRolePolicy<'a> {
    kind: &'a str,
    guild_id: &'a str,
    allowed_role_ids: [&'a str; 1],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartResponse {
    pub authorization_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendCallback {
    pub source: String,
    pub kind: String,
    pub handoff_kind: String,
    pub attempt_id: String,
    pub status: String,
    pub provider: String,
    pub app_slug: String,
    pub mode: String,
    pub return_to: String,
    pub access_token: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccessClaims {
    pub iss: String,
    pub aud: String,
    pub typ: String,
    pub account_id: String,
    pub capabilities: Vec<String>,
    pub app: AppClaim,
}

#[derive(Debug, Deserialize)]
pub struct AppClaim {
    pub slug: String,
}

impl HeimdallClient {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            base_url: std::env::var("GHOSTLIGHT_HEIMDALL_BASE_URL")
                .unwrap_or_else(|_| "https://heimdall.gamecult.org".into())
                .trim_end_matches('/')
                .into(),
            public_app_url: std::env::var("GHOSTLIGHT_PUBLIC_APP_URL")
                .unwrap_or_else(|_| "https://yggdrasil.gamecult.org/ghostlight/".into()),
            callback_url: std::env::var("GHOSTLIGHT_HEIMDALL_CALLBACK_URL").unwrap_or_else(|_| {
                "https://yggdrasil.gamecult.org/ghostlight/api/auth/heimdall/callback".into()
            }),
            discord_guild_id: required_env("GHOSTLIGHT_DISCORD_GUILD_ID")?,
            discord_role_id: required_env("GHOSTLIGHT_DISCORD_ROLE_ID")?,
        })
    }

    #[cfg(test)]
    pub fn fixture() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://heimdall.invalid".into(),
            public_app_url: "https://ghostlight.invalid/".into(),
            callback_url: "https://ghostlight.invalid/api/auth/heimdall/callback".into(),
            discord_guild_id: "guild-kltst".into(),
            discord_role_id: "role-kltst".into(),
        }
    }

    pub async fn start(&self, attempt_id: &str) -> anyhow::Result<StartResponse> {
        self.client
            .post(format!("{}/v1/oauth/discord/start", self.base_url))
            .json(&StartRequest {
                app_slug: APP_SLUG,
                mode: "sign_in",
                return_to: &self.public_app_url,
                handoff: Handoff {
                    kind: "backend_callback",
                    attempt_id,
                    callback_url: &self.callback_url,
                },
                requested_scopes: ["identify", "guilds.members.read"],
                entitlement_policy: DiscordRolePolicy {
                    kind: "discord_role_access",
                    guild_id: &self.discord_guild_id,
                    allowed_role_ids: [&self.discord_role_id],
                },
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Heimdall start response was malformed")
    }

    pub async fn verify_callback(
        &self,
        callback: &BackendCallback,
    ) -> anyhow::Result<AccessClaims> {
        self.validate_callback_envelope(callback)?;
        if callback.status != "success" {
            bail!("Heimdall callback did not grant Ghostlight access");
        }
        self.verify(
            callback
                .access_token
                .as_deref()
                .context("Heimdall success callback omitted its access token")?,
        )
        .await
    }

    pub fn validate_callback_envelope(&self, callback: &BackendCallback) -> anyhow::Result<()> {
        if callback.source != "heimdall"
            || callback.kind != "oauth_result"
            || callback.handoff_kind != "backend_callback"
            || callback.provider != "discord"
            || callback.app_slug != APP_SLUG
            || callback.mode != "sign_in"
            || callback.return_to != self.public_app_url
        {
            bail!("Heimdall callback did not match the Ghostlight login contract");
        }
        Ok(())
    }

    async fn verify(&self, token: &str) -> anyhow::Result<AccessClaims> {
        let header = decode_header(token)?;
        if header.alg != Algorithm::EdDSA {
            bail!("Heimdall used an unsupported signing algorithm");
        }
        let kid = header.kid.context("Heimdall token omitted kid")?;
        let jwks: JwkSet = self
            .client
            .get(format!("{}/.well-known/jwks.json", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let jwk = jwks
            .find(&kid)
            .context("Heimdall signing key was not published")?;
        let key = DecodingKey::from_jwk(jwk)?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&[APP_SLUG]);
        validation.set_issuer(&[self.base_url.as_str()]);
        let claims = decode::<AccessClaims>(token, &key, &validation)?.claims;
        if claims.typ != "heimdall_access"
            || claims.aud != APP_SLUG
            || claims.iss != self.base_url
            || claims.app.slug != APP_SLUG
            || !claims.capabilities.iter().any(|v| v == "app_access")
        {
            bail!("Heimdall claim did not grant Ghostlight app_access");
        }
        Ok(claims)
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    let value = value.trim();
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value.to_owned())
}
