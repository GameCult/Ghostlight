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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest<'a> {
    app_slug: &'a str,
    mode: &'a str,
    return_to: &'a str,
    handoff: Handoff<'a>,
    requested_scopes: [&'a str; 1],
}

#[derive(Serialize)]
struct Handoff<'a> {
    kind: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartResponse {
    pub authorization_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RedeemRequest<'a> {
    completion_code: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RedeemResponse {
    access_token: String,
    shared_capabilities: Vec<String>,
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
    pub fn public_demo() -> anyhow::Result<Self> {
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
        })
    }

    #[cfg(test)]
    pub fn fixture() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://heimdall.invalid".into(),
            public_app_url: "https://ghostlight.invalid/".into(),
        }
    }

    pub async fn start(&self) -> anyhow::Result<StartResponse> {
        self.client
            .post(format!("{}/v1/oauth/discord/start", self.base_url))
            .json(&StartRequest {
                app_slug: APP_SLUG,
                mode: "sign_in",
                return_to: &self.public_app_url,
                handoff: Handoff {
                    kind: "browser_completion",
                },
                requested_scopes: ["identify"],
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Heimdall start response was malformed")
    }

    pub async fn redeem(&self, completion_code: &str) -> anyhow::Result<AccessClaims> {
        let redeemed: RedeemResponse = self
            .client
            .post(format!(
                "{}/v1/apps/{APP_SLUG}/auth-completions/redeem",
                self.base_url
            ))
            .json(&RedeemRequest { completion_code })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Heimdall redeem response was malformed")?;
        if !redeemed
            .shared_capabilities
            .iter()
            .any(|value| value == "app_access")
        {
            bail!("Heimdall did not grant Ghostlight app_access");
        }
        self.verify(&redeemed.access_token).await
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
