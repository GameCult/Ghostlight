use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use ghostlight_dungeon::persistence::CampaignStore;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSessionState {
    pub schema: String,
    pub sessions: BTreeMap<String, AppSession>,
    pub account_preferences: BTreeMap<String, AccountPreferences>,
    #[serde(default)]
    pub command_receipts: BTreeMap<String, EveCommandCacheEntry>,
    #[serde(default)]
    pub campaign_export_grants: BTreeMap<String, CampaignExportGrant>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CampaignExportGrant {
    pub schema: String,
    pub token_hash: String,
    pub account_subject_hash: String,
    pub campaign_id: uuid::Uuid,
    pub export_path: std::path::PathBuf,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

pub struct CampaignExportResource {
    pub campaign_id: uuid::Uuid,
    pub export_path: std::path::PathBuf,
    pub filename: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EveCommandCacheEntry {
    pub schema: String,
    pub account_subject_hash: String,
    pub idempotency_key: String,
    pub operation_id: String,
    #[serde(default)]
    pub command_result_messagepack: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

pub enum CommandReservation {
    Reserved,
    Pending,
    Cached(serde_json::Value),
}

pub struct RefreshCandidate {
    pub cookie_hash: String,
    pub account_subject_hash: String,
    pub heimdall_session_id: String,
    pub access_revision: u64,
    pub refresh_claim: String,
}

pub struct RefreshedSession<'a> {
    pub expected_access_revision: u64,
    pub access_revision: u64,
    pub capabilities: Vec<String>,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub refresh_claim: &'a str,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSession {
    pub schema: String,
    pub cookie_hash: String,
    pub account_subject_hash: String,
    pub heimdall_session_id: String,
    pub access_revision: u64,
    pub verified_capabilities: Vec<String>,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub encrypted_refresh_claim: String,
    pub created_at: DateTime<Utc>,
    pub last_refresh_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AccountPreferences {
    pub schema: String,
    pub account_subject_hash: String,
    pub selected_campaign_id: Option<uuid::Uuid>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewSession<'a> {
    pub account_id: &'a str,
    pub heimdall_session_id: &'a str,
    pub access_revision: u64,
    pub capabilities: Vec<String>,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
    pub refresh_claim: &'a str,
}

pub struct AppSessionOwner {
    store: CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    state: AppSessionState,
    wrapping_key: [u8; 32],
}

impl AppSessionOwner {
    pub fn open(
        store_path: impl AsRef<Path>,
        wrapping_key_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let store = CampaignStore::open(store_path)?;
        let (row, state) = match store.load::<AppSessionState>("app_session_store.v1", "primary")? {
            Some(value) => value,
            None => {
                let state = AppSessionState {
                    schema: "ghostlight.app_session_store.v1".into(),
                    sessions: BTreeMap::new(),
                    account_preferences: BTreeMap::new(),
                    command_receipts: BTreeMap::new(),
                    campaign_export_grants: BTreeMap::new(),
                };
                let row = store.insert(
                    "app_session_store.v1",
                    "ghostlight.app_session_store.v1",
                    "primary",
                    &state,
                )?;
                (row, state)
            }
        };
        Ok(Self {
            store,
            row,
            state,
            wrapping_key: read_wrapping_key(wrapping_key_path.as_ref())?,
        })
    }

    pub fn create_session(&mut self, input: NewSession<'_>) -> anyhow::Result<String> {
        if !input.capabilities.iter().any(|value| value == "app_access") {
            bail!("Heimdall session lacks app_access");
        }
        let mut cookie = [0_u8; 32];
        rand::rng().fill_bytes(&mut cookie);
        let raw_cookie = URL_SAFE_NO_PAD.encode(cookie);
        let cookie_hash = secret_hash(&raw_cookie);
        let account_subject_hash = secret_hash(&format!("heimdall-account:{}", input.account_id));
        let now = Utc::now();
        let session = AppSession {
            schema: "ghostlight.app_session.v1".into(),
            cookie_hash: cookie_hash.clone(),
            account_subject_hash: account_subject_hash.clone(),
            heimdall_session_id: input.heimdall_session_id.into(),
            access_revision: input.access_revision,
            verified_capabilities: input.capabilities,
            access_expires_at: input.access_expires_at,
            refresh_expires_at: input.refresh_expires_at,
            encrypted_refresh_claim: wrap_refresh(
                &self.wrapping_key,
                &account_subject_hash,
                input.heimdall_session_id,
                input.refresh_claim,
            )?,
            created_at: now,
            last_refresh_at: now,
            revoked_at: None,
        };
        let mut next = self.state.clone();
        next.sessions.insert(cookie_hash, session);
        next.account_preferences
            .entry(account_subject_hash.clone())
            .or_insert(AccountPreferences {
                schema: "ghostlight.account_preferences.v1".into(),
                account_subject_hash,
                selected_campaign_id: None,
                updated_at: now,
            });
        self.commit(next)?;
        Ok(raw_cookie)
    }

    pub fn account_for_cookie(&self, raw_cookie: &str, now: DateTime<Utc>) -> Option<String> {
        let session = self.state.sessions.get(&secret_hash(raw_cookie))?;
        (session.revoked_at.is_none()
            && session.access_expires_at > now
            && session
                .verified_capabilities
                .iter()
                .any(|value| value == "app_access"))
        .then(|| session.account_subject_hash.clone())
    }

    pub fn revoke_cookie(&mut self, raw_cookie: &str) -> anyhow::Result<bool> {
        let hash = secret_hash(raw_cookie);
        let mut next = self.state.clone();
        let Some(session) = next.sessions.get_mut(&hash) else {
            return Ok(false);
        };
        session.revoked_at = Some(Utc::now());
        self.commit(next)?;
        Ok(true)
    }

    pub fn revoke_cookie_hash(&mut self, cookie_hash: &str) -> anyhow::Result<bool> {
        let mut next = self.state.clone();
        let Some(session) = next.sessions.get_mut(cookie_hash) else {
            return Ok(false);
        };
        session.revoked_at = Some(Utc::now());
        self.commit(next)?;
        Ok(true)
    }

    pub fn sessions_due_for_refresh(
        &self,
        now: DateTime<Utc>,
        refresh_before: chrono::Duration,
    ) -> anyhow::Result<Vec<RefreshCandidate>> {
        self.state
            .sessions
            .values()
            .filter(|session| {
                session.revoked_at.is_none()
                    && session.refresh_expires_at > now
                    && session.access_expires_at <= now + refresh_before
            })
            .map(|session| {
                Ok(RefreshCandidate {
                    cookie_hash: session.cookie_hash.clone(),
                    account_subject_hash: session.account_subject_hash.clone(),
                    heimdall_session_id: session.heimdall_session_id.clone(),
                    access_revision: session.access_revision,
                    refresh_claim: unwrap_refresh(
                        &self.wrapping_key,
                        &session.account_subject_hash,
                        &session.heimdall_session_id,
                        &session.encrypted_refresh_claim,
                    )?,
                })
            })
            .collect()
    }

    pub fn session_for_logout(&self, raw_cookie: &str) -> anyhow::Result<Option<RefreshCandidate>> {
        let Some(session) = self.state.sessions.get(&secret_hash(raw_cookie)) else {
            return Ok(None);
        };
        if session.revoked_at.is_some() {
            return Ok(None);
        }
        Ok(Some(RefreshCandidate {
            cookie_hash: session.cookie_hash.clone(),
            account_subject_hash: session.account_subject_hash.clone(),
            heimdall_session_id: session.heimdall_session_id.clone(),
            access_revision: session.access_revision,
            refresh_claim: unwrap_refresh(
                &self.wrapping_key,
                &session.account_subject_hash,
                &session.heimdall_session_id,
                &session.encrypted_refresh_claim,
            )?,
        }))
    }

    pub fn apply_refresh(
        &mut self,
        cookie_hash: &str,
        input: RefreshedSession<'_>,
    ) -> anyhow::Result<()> {
        if !input.capabilities.iter().any(|value| value == "app_access") {
            bail!("refreshed Heimdall session lacks app_access");
        }
        let mut next = self.state.clone();
        let session = next
            .sessions
            .get_mut(cookie_hash)
            .context("local session vanished during refresh")?;
        if session.revoked_at.is_some() || session.access_revision != input.expected_access_revision
        {
            bail!("local session changed during refresh");
        }
        session.access_revision = input.access_revision;
        session.verified_capabilities = input.capabilities;
        session.access_expires_at = input.access_expires_at;
        session.refresh_expires_at = input.refresh_expires_at;
        session.encrypted_refresh_claim = wrap_refresh(
            &self.wrapping_key,
            &session.account_subject_hash,
            &session.heimdall_session_id,
            input.refresh_claim,
        )?;
        session.last_refresh_at = Utc::now();
        self.commit(next)
    }

    pub fn selected_campaign(&self, account_subject_hash: &str) -> Option<uuid::Uuid> {
        self.state
            .account_preferences
            .get(account_subject_hash)?
            .selected_campaign_id
    }

    pub fn select_campaign(
        &mut self,
        account_subject_hash: &str,
        campaign_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let mut next = self.state.clone();
        let preferences = next
            .account_preferences
            .entry(account_subject_hash.into())
            .or_insert(AccountPreferences {
                schema: "ghostlight.account_preferences.v1".into(),
                account_subject_hash: account_subject_hash.into(),
                selected_campaign_id: None,
                updated_at: Utc::now(),
            });
        preferences.selected_campaign_id = Some(campaign_id);
        preferences.updated_at = Utc::now();
        self.commit(next)
    }

    pub fn clear_selected_campaign(&mut self, account_subject_hash: &str) -> anyhow::Result<()> {
        let mut next = self.state.clone();
        let preferences = next
            .account_preferences
            .entry(account_subject_hash.into())
            .or_insert(AccountPreferences {
                schema: "ghostlight.account_preferences.v1".into(),
                account_subject_hash: account_subject_hash.into(),
                selected_campaign_id: None,
                updated_at: Utc::now(),
            });
        preferences.selected_campaign_id = None;
        preferences.updated_at = Utc::now();
        self.commit(next)
    }

    pub fn migrate_preference(
        &mut self,
        account_subject_hash: &str,
        campaign_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        if self.selected_campaign(account_subject_hash).is_none() {
            self.select_campaign(account_subject_hash, campaign_id)?;
        }
        Ok(())
    }

    pub fn reserve_command(
        &mut self,
        account_subject_hash: &str,
        idempotency_key: &str,
        operation_id: &str,
    ) -> anyhow::Result<CommandReservation> {
        let key = command_cache_key(account_subject_hash, idempotency_key);
        if let Some(entry) = self.state.command_receipts.get(&key) {
            if entry.operation_id != operation_id
                || entry.account_subject_hash != account_subject_hash
            {
                bail!("idempotency key is already bound to another operation");
            }
            return match &entry.command_result_messagepack {
                Some(bytes) => Ok(CommandReservation::Cached(rmp_serde::from_slice(bytes)?)),
                None => Ok(CommandReservation::Pending),
            };
        }
        let mut next = self.state.clone();
        next.command_receipts.insert(
            key,
            EveCommandCacheEntry {
                schema: "ghostlight.eve_command_cache_entry.v1".into(),
                account_subject_hash: account_subject_hash.into(),
                idempotency_key: idempotency_key.into(),
                operation_id: operation_id.into(),
                command_result_messagepack: None,
                created_at: Utc::now(),
            },
        );
        self.commit(next)?;
        Ok(CommandReservation::Reserved)
    }

    pub fn record_command_result(
        &mut self,
        account_subject_hash: &str,
        idempotency_key: &str,
        operation_id: &str,
        result: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let key = command_cache_key(account_subject_hash, idempotency_key);
        let mut next = self.state.clone();
        let entry = next
            .command_receipts
            .get_mut(&key)
            .ok_or_else(|| anyhow::anyhow!("command result has no reservation"))?;
        if entry.operation_id != operation_id || entry.account_subject_hash != account_subject_hash
        {
            bail!("command result disagrees with its reservation");
        }
        entry.command_result_messagepack = Some(rmp_serde::to_vec_named(result)?);
        self.commit(next)
    }

    pub fn issue_campaign_export_grant(
        &mut self,
        account_subject_hash: &str,
        campaign_id: uuid::Uuid,
        export_path: std::path::PathBuf,
        filename: String,
        now: DateTime<Utc>,
        lifetime: chrono::Duration,
    ) -> anyhow::Result<String> {
        if lifetime <= chrono::Duration::zero() {
            bail!("campaign export grant lifetime must be positive");
        }
        if filename.is_empty()
            || filename.contains('/')
            || filename.contains('\\')
            || filename.chars().any(char::is_control)
        {
            bail!("campaign export filename is invalid");
        }
        let mut token_bytes = [0_u8; 32];
        rand::rng().fill_bytes(&mut token_bytes);
        let raw_token = URL_SAFE_NO_PAD.encode(token_bytes);
        let token_hash = secret_hash(&format!("campaign-export:{raw_token}"));
        let mut next = self.state.clone();
        next.campaign_export_grants.retain(|_, grant| {
            grant.expires_at > now
                || grant
                    .consumed_at
                    .is_some_and(|at| at > now - chrono::Duration::days(1))
        });
        next.campaign_export_grants.insert(
            token_hash.clone(),
            CampaignExportGrant {
                schema: "ghostlight.campaign_export_grant.v1".into(),
                token_hash,
                account_subject_hash: account_subject_hash.into(),
                campaign_id,
                export_path,
                filename,
                created_at: now,
                expires_at: now + lifetime,
                consumed_at: None,
            },
        );
        self.commit(next)?;
        Ok(raw_token)
    }

    pub fn consume_campaign_export_grant(
        &mut self,
        raw_token: &str,
        account_subject_hash: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<CampaignExportResource>> {
        let token_hash = secret_hash(&format!("campaign-export:{raw_token}"));
        let Some(grant) = self.state.campaign_export_grants.get(&token_hash) else {
            return Ok(None);
        };
        if grant.account_subject_hash != account_subject_hash
            || grant.expires_at <= now
            || grant.consumed_at.is_some()
        {
            return Ok(None);
        }
        let resource = CampaignExportResource {
            campaign_id: grant.campaign_id,
            export_path: grant.export_path.clone(),
            filename: grant.filename.clone(),
        };
        let mut next = self.state.clone();
        next.campaign_export_grants
            .get_mut(&token_hash)
            .context("campaign export grant vanished during consumption")?
            .consumed_at = Some(now);
        self.commit(next)?;
        Ok(Some(resource))
    }

    fn commit(&mut self, next: AppSessionState) -> anyhow::Result<()> {
        let next_row = self
            .store
            .replace(&self.row, "ghostlight.app_session_store.v1", &next)?;
        self.row = next_row;
        self.state = next;
        Ok(())
    }
}

fn command_cache_key(account_subject_hash: &str, idempotency_key: &str) -> String {
    secret_hash(&format!(
        "eve-command:{account_subject_hash}:{idempotency_key}"
    ))
}

pub fn secret_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn read_wrapping_key(path: &Path) -> anyhow::Result<[u8; 32]> {
    let source =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let decoded = if source.len() == 32 {
        source
    } else {
        let text = std::str::from_utf8(&source)?.trim();
        URL_SAFE_NO_PAD
            .decode(text)
            .context("session wrapping key must be 32 raw bytes or base64url")?
    };
    decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("session wrapping key must be exactly 32 bytes"))
}

fn wrap_refresh(
    key: &[u8; 32],
    account_hash: &str,
    session_id: &str,
    claim: &str,
) -> anyhow::Result<String> {
    let mut iv = [0_u8; 12];
    rand::rng().fill_bytes(&mut iv);
    let aad = format!("ghostlight.app_session.refresh.v1\n{account_hash}\n{session_id}");
    let ciphertext = Aes256Gcm::new_from_slice(key)?
        .encrypt(
            (&iv).into(),
            aes_gcm::aead::Payload {
                msg: claim.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to wrap Heimdall refresh claim"))?;
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(iv),
        URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

fn unwrap_refresh(
    key: &[u8; 32],
    account_hash: &str,
    session_id: &str,
    wrapped: &str,
) -> anyhow::Result<String> {
    let (iv, ciphertext) = wrapped
        .split_once('.')
        .context("wrapped refresh claim is malformed")?;
    let iv = URL_SAFE_NO_PAD.decode(iv)?;
    let iv: [u8; 12] = iv
        .try_into()
        .map_err(|_| anyhow::anyhow!("wrapped refresh IV is invalid"))?;
    let ciphertext = URL_SAFE_NO_PAD.decode(ciphertext)?;
    let aad = format!("ghostlight.app_session.refresh.v1\n{account_hash}\n{session_id}");
    let plaintext = Aes256Gcm::new_from_slice(key)?
        .decrypt(
            (&iv).into(),
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to unwrap Heimdall refresh claim"))?;
    String::from_utf8(plaintext).context("Heimdall refresh claim is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn app_session_persists_cookie_hash_and_selected_campaign_without_campaign_lists() {
        let directory = tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [7_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let cookie = owner
            .create_session(NewSession {
                account_id: "account-1",
                heimdall_session_id: "heimdall-session-1",
                access_revision: 3,
                capabilities: vec!["app_access".into()],
                access_expires_at: Utc::now() + chrono::Duration::minutes(5),
                refresh_expires_at: Utc::now() + chrono::Duration::days(7),
                refresh_claim: "private-refresh-claim",
            })
            .unwrap();
        let account = owner.account_for_cookie(&cookie, Utc::now()).unwrap();
        assert!(!store.to_string_lossy().contains(&cookie));
        let campaign = uuid::Uuid::new_v4();
        owner.select_campaign(&account, campaign).unwrap();
        drop(owner);
        let reopened = AppSessionOwner::open(&store, &key).unwrap();
        assert_eq!(
            reopened.account_for_cookie(&cookie, Utc::now()),
            Some(account.clone())
        );
        assert_eq!(reopened.selected_campaign(&account), Some(campaign));
        drop(reopened);
        let bytes = std::fs::read(&store).unwrap();
        assert!(
            !bytes
                .windows(cookie.len())
                .any(|window| window == cookie.as_bytes())
        );
        assert!(
            !bytes
                .windows("private-refresh-claim".len())
                .any(|window| window == b"private-refresh-claim")
        );
    }

    #[test]
    fn campaign_entry_clears_only_the_selected_preference() {
        let directory = tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [7_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let account = "sha256:player";
        owner
            .select_campaign(account, uuid::Uuid::new_v4())
            .unwrap();

        owner.clear_selected_campaign(account).unwrap();

        assert_eq!(owner.selected_campaign(account), None);
        assert!(owner.state.account_preferences.contains_key(account));
    }

    #[test]
    fn campaign_export_grant_is_hashed_account_bound_expiring_and_single_use() {
        let directory = tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [7_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let campaign_id = uuid::Uuid::new_v4();
        let export_path = directory.path().join("campaign.cc");
        let now = Utc::now();
        let token = owner
            .issue_campaign_export_grant(
                "sha256:owner",
                campaign_id,
                export_path.clone(),
                "campaign.cc".into(),
                now,
                chrono::Duration::minutes(15),
            )
            .unwrap();

        assert!(
            owner
                .consume_campaign_export_grant(&token, "sha256:intruder", now)
                .unwrap()
                .is_none()
        );
        let resource = owner
            .consume_campaign_export_grant(&token, "sha256:owner", now)
            .unwrap()
            .unwrap();
        assert_eq!(resource.campaign_id, campaign_id);
        assert_eq!(resource.export_path, export_path);
        assert!(
            owner
                .consume_campaign_export_grant(&token, "sha256:owner", now)
                .unwrap()
                .is_none()
        );
        drop(owner);
        let bytes = std::fs::read(store).unwrap();
        assert!(
            !bytes
                .windows(token.len())
                .any(|window| window == token.as_bytes())
        );

        let mut reopened =
            AppSessionOwner::open(directory.path().join("app-sessions-expired.cc"), &key).unwrap();
        let expired = reopened
            .issue_campaign_export_grant(
                "sha256:owner",
                campaign_id,
                directory.path().join("expired.cc"),
                "expired.cc".into(),
                now,
                chrono::Duration::seconds(1),
            )
            .unwrap();
        assert!(
            reopened
                .consume_campaign_export_grant(
                    &expired,
                    "sha256:owner",
                    now + chrono::Duration::seconds(2),
                )
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn refresh_is_exact_session_cas_and_rotates_private_claim() {
        let directory = tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [11_u8; 32]).unwrap();
        let mut owner =
            AppSessionOwner::open(directory.path().join("app-sessions.cc"), &key).unwrap();
        let now = Utc::now();
        let cookie = owner
            .create_session(NewSession {
                account_id: "account-1",
                heimdall_session_id: "session-1",
                access_revision: 4,
                capabilities: vec!["app_access".into()],
                access_expires_at: now + chrono::Duration::minutes(2),
                refresh_expires_at: now + chrono::Duration::days(7),
                refresh_claim: "refresh-before",
            })
            .unwrap();

        let due = owner
            .sessions_due_for_refresh(now, chrono::Duration::minutes(5))
            .unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].refresh_claim, "refresh-before");
        owner
            .apply_refresh(
                &due[0].cookie_hash,
                RefreshedSession {
                    expected_access_revision: 4,
                    access_revision: 5,
                    capabilities: vec!["app_access".into()],
                    access_expires_at: now + chrono::Duration::hours(1),
                    refresh_expires_at: now + chrono::Duration::days(7),
                    refresh_claim: "refresh-after",
                },
            )
            .unwrap();
        assert!(
            owner
                .apply_refresh(
                    &due[0].cookie_hash,
                    RefreshedSession {
                        expected_access_revision: 4,
                        access_revision: 6,
                        capabilities: vec!["app_access".into()],
                        access_expires_at: now + chrono::Duration::hours(1),
                        refresh_expires_at: now + chrono::Duration::days(7),
                        refresh_claim: "stale-refresh",
                    }
                )
                .is_err()
        );
        assert_eq!(
            owner
                .session_for_logout(&cookie)
                .unwrap()
                .unwrap()
                .refresh_claim,
            "refresh-after"
        );
    }
}
