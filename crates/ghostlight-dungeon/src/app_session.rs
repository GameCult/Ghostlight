//! Local custody for authenticated Heimdall sessions.
//!
//! This store owns only browser-session continuity. It does not own world
//! selection, commands, idempotency, exports, or fictional state.

use crate::heimdall::{VerifiedSessionAdmission, VerifiedSessionRefresh};
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

const STORE_TYPE: &str = "ghostlight.app_session_store.v2";
const STORE_SCHEMA: &str = "ghostlight.app_session_store.v2";
const STORE_KEY: &str = "primary";
const MAX_SESSION_RECORDS: usize = 1_024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AppSessionState {
    schema: String,
    sessions: BTreeMap<String, AppSession>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AppSession {
    schema: String,
    cookie_hash: String,
    account_subject_hash: String,
    heimdall_session_id: String,
    access_revision: u64,
    verified_capabilities: Vec<String>,
    access_expires_at: DateTime<Utc>,
    refresh_expires_at: DateTime<Utc>,
    encrypted_refresh_claim: String,
    created_at: DateTime<Utc>,
    last_refresh_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

pub(crate) struct RefreshCandidate {
    pub(crate) cookie_hash: String,
    pub(crate) account_subject_hash: String,
    pub(crate) heimdall_session_id: String,
    pub(crate) access_revision: u64,
    pub(crate) refresh_claim: String,
}

/// Opaque proof that the session owner verified a live Heimdall principal.
///
/// The account hash is intentionally readable by world ingress, but this type
/// can only be constructed while `AppSessionOwner` holds valid custody.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedPrincipalEvidence {
    account_subject_hash: String,
}

impl VerifiedPrincipalEvidence {
    pub(crate) fn account_subject_hash(&self) -> &str {
        &self.account_subject_hash
    }
}

pub(crate) struct AppSessionOwner {
    store: OwnedRedbMessagePackBackingStore,
    row: CultCacheEnvelope,
    state: AppSessionState,
    wrapping_key: [u8; 32],
    healthy: bool,
}

impl AppSessionOwner {
    pub(crate) fn open(
        store_path: impl AsRef<Path>,
        wrapping_key_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let wrapping_key = read_wrapping_key(wrapping_key_path.as_ref())?;
        let store = OwnedRedbMessagePackBackingStore::new(store_path.as_ref())?;
        store
            .validate_path_identity()
            .context("app-session store path identity is invalid")?;
        let rows = store.pull_all()?;
        let (row, state) = match rows.as_slice() {
            [] => {
                let state = AppSessionState {
                    schema: STORE_SCHEMA.into(),
                    sessions: BTreeMap::new(),
                };
                let row = envelope(&state)?;
                if !store.compare_and_swap_batch(&[], vec![row.clone()])? {
                    bail!("app-session store changed during initialization");
                }
                (row, state)
            }
            [row]
                if row.r#type == STORE_TYPE
                    && row.key == STORE_KEY
                    && row.schema_id.as_deref() == Some(STORE_SCHEMA) =>
            {
                let state: AppSessionState =
                    rmp_serde::from_slice(&row.payload).context("app-session state is corrupt")?;
                if rmp_serde::to_vec_named(&state)? != row.payload {
                    bail!("app-session state is not canonical");
                }
                if state.schema != STORE_SCHEMA {
                    bail!("app-session state schema is invalid");
                }
                validate_state(&state, &wrapping_key)?;
                (row.clone(), state)
            }
            _ => bail!(
                "app-session v2 store contains foreign or legacy records; start with a fresh store"
            ),
        };
        Ok(Self {
            store,
            row,
            state,
            wrapping_key,
            healthy: true,
        })
    }

    pub(crate) fn create_session(
        &mut self,
        input: VerifiedSessionAdmission,
    ) -> anyhow::Result<String> {
        if !input
            .capabilities()
            .iter()
            .any(|value| value == "app_access")
        {
            bail!("Heimdall session lacks app_access");
        }
        if input.account_id().trim().is_empty()
            || input.heimdall_session_id().trim().is_empty()
            || input.refresh_claim().is_empty()
            || input.refresh_expires_at() <= input.access_expires_at()
        {
            bail!("Heimdall session material is invalid");
        }
        let mut cookie = [0_u8; 32];
        rand::rng().fill_bytes(&mut cookie);
        let raw_cookie = URL_SAFE_NO_PAD.encode(cookie);
        let cookie_hash = secret_hash(&raw_cookie);
        let account_subject_hash = secret_hash(&format!("heimdall-account:{}", input.account_id()));
        let now = Utc::now();
        let session = AppSession {
            schema: "ghostlight.app_session.v2".into(),
            cookie_hash: cookie_hash.clone(),
            account_subject_hash: account_subject_hash.clone(),
            heimdall_session_id: input.heimdall_session_id().to_owned(),
            access_revision: input.access_revision(),
            verified_capabilities: input.capabilities().to_vec(),
            access_expires_at: input.access_expires_at(),
            refresh_expires_at: input.refresh_expires_at(),
            encrypted_refresh_claim: wrap_refresh(
                &self.wrapping_key,
                &account_subject_hash,
                input.heimdall_session_id(),
                input.refresh_claim(),
            )?,
            created_at: now,
            last_refresh_at: now,
            revoked_at: None,
        };
        let mut next = self.state.clone();
        for previous in next.sessions.values_mut().filter(|previous| {
            previous.revoked_at.is_none()
                && previous.heimdall_session_id == input.heimdall_session_id()
        }) {
            previous.revoked_at = Some(now);
        }
        next.sessions.insert(cookie_hash, session);
        self.commit(next)?;
        Ok(raw_cookie)
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.healthy
    }

    pub(crate) fn validate_custody(&mut self) -> anyhow::Result<()> {
        self.ensure_owned()
    }

    pub(crate) fn account_for_cookie(
        &mut self,
        raw_cookie: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<VerifiedPrincipalEvidence>> {
        self.ensure_owned()?;
        let Some(session) = self.state.sessions.get(&secret_hash(raw_cookie)) else {
            return Ok(None);
        };
        Ok((session.revoked_at.is_none()
            && session.access_expires_at > now
            && session.refresh_expires_at > now
            && session
                .verified_capabilities
                .iter()
                .any(|value| value == "app_access"))
        .then(|| VerifiedPrincipalEvidence {
            account_subject_hash: session.account_subject_hash.clone(),
        }))
    }

    pub(crate) fn revoke_cookie(&mut self, raw_cookie: &str) -> anyhow::Result<bool> {
        self.ensure_owned()?;
        let cookie_hash = secret_hash(raw_cookie);
        let mut next = self.state.clone();
        let Some(session) = next.sessions.get_mut(&cookie_hash) else {
            return Ok(false);
        };
        if session.revoked_at.is_some() {
            return Ok(false);
        }
        session.revoked_at = Some(Utc::now());
        self.commit(next)?;
        Ok(true)
    }

    pub(crate) fn revoke_cookie_hash(&mut self, cookie_hash: &str) -> anyhow::Result<bool> {
        self.ensure_owned()?;
        let mut next = self.state.clone();
        let Some(session) = next.sessions.get_mut(cookie_hash) else {
            return Ok(false);
        };
        if session.revoked_at.is_some() {
            return Ok(false);
        }
        session.revoked_at = Some(Utc::now());
        self.commit(next)?;
        Ok(true)
    }

    pub(crate) fn session_for_logout(
        &mut self,
        raw_cookie: &str,
    ) -> anyhow::Result<Option<RefreshCandidate>> {
        self.ensure_owned()?;
        let Some(session) = self.state.sessions.get(&secret_hash(raw_cookie)) else {
            return Ok(None);
        };
        match self.refresh_candidate(session) {
            Ok(candidate) => Ok(Some(candidate)),
            Err(error) => self.poison(error),
        }
    }

    pub(crate) fn sessions_due_for_refresh(
        &mut self,
        now: DateTime<Utc>,
        refresh_before: chrono::Duration,
    ) -> anyhow::Result<Vec<RefreshCandidate>> {
        self.ensure_owned()?;
        let candidates = self
            .state
            .sessions
            .values()
            .filter(|session| {
                session.revoked_at.is_none()
                    && session.refresh_expires_at > now
                    && session.access_expires_at <= now + refresh_before
            })
            .map(|session| self.refresh_candidate(session))
            .collect::<anyhow::Result<Vec<_>>>();
        match candidates {
            Ok(candidates) => Ok(candidates),
            Err(error) => self.poison(error),
        }
    }

    pub(crate) fn apply_refresh(
        &mut self,
        cookie_hash: &str,
        expected_access_revision: u64,
        input: VerifiedSessionRefresh,
    ) -> anyhow::Result<()> {
        if !input
            .capabilities()
            .iter()
            .any(|value| value == "app_access")
        {
            bail!("refreshed Heimdall session lacks app_access");
        }
        if input.refresh_claim().is_empty()
            || input.refresh_expires_at() <= input.access_expires_at()
        {
            bail!("refreshed Heimdall session material is invalid");
        }
        let mut next = self.state.clone();
        let session = next
            .sessions
            .get_mut(cookie_hash)
            .context("local session vanished during refresh")?;
        if session.revoked_at.is_some()
            || session.access_revision != expected_access_revision
            || input.access_revision() <= expected_access_revision
            || session.heimdall_session_id != input.heimdall_session_id()
            || session.account_subject_hash
                != secret_hash(&format!("heimdall-account:{}", input.account_id()))
        {
            bail!("local session changed during refresh");
        }
        session.access_revision = input.access_revision();
        session.verified_capabilities = input.capabilities().to_vec();
        session.access_expires_at = input.access_expires_at();
        session.refresh_expires_at = input.refresh_expires_at();
        session.encrypted_refresh_claim = wrap_refresh(
            &self.wrapping_key,
            &session.account_subject_hash,
            &session.heimdall_session_id,
            input.refresh_claim(),
        )?;
        session.last_refresh_at = Utc::now();
        self.commit(next)
    }

    fn refresh_candidate(&self, session: &AppSession) -> anyhow::Result<RefreshCandidate> {
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
    }

    fn commit(&mut self, mut next: AppSessionState) -> anyhow::Result<()> {
        self.ensure_owned()?;
        prune_sessions(&mut next, Utc::now());
        let next_row = envelope(&next)?;
        let swapped = self
            .store
            .compare_and_swap_batch(std::slice::from_ref(&self.row), vec![next_row.clone()]);
        if !matches!(swapped, Ok(true)) {
            return self.poison(anyhow::anyhow!(
                "app-session commit outcome is uncertain; sole ownership was lost"
            ));
        }
        if self.store.validate_path_identity().is_err() {
            return self.poison(anyhow::anyhow!(
                "app-session store path changed after durable commit"
            ));
        }
        self.row = next_row;
        self.state = next;
        Ok(())
    }

    fn ensure_owned(&mut self) -> anyhow::Result<()> {
        if !self.healthy {
            bail!("app-session custody is poisoned; reopen the owner");
        }
        if self.store.validate_path_identity().is_err() {
            self.healthy = false;
            bail!("app-session store ownership was lost");
        }
        Ok(())
    }

    fn poison<T>(&mut self, error: anyhow::Error) -> anyhow::Result<T> {
        self.healthy = false;
        Err(error)
    }
}

pub(crate) fn secret_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn envelope(state: &AppSessionState) -> anyhow::Result<CultCacheEnvelope> {
    Ok(CultCacheEnvelope {
        key: STORE_KEY.into(),
        r#type: STORE_TYPE.into(),
        payload: rmp_serde::to_vec_named(state)?,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(STORE_SCHEMA.into()),
    })
}

fn validate_state(state: &AppSessionState, wrapping_key: &[u8; 32]) -> anyhow::Result<()> {
    if state.schema != STORE_SCHEMA {
        bail!("app-session state schema is invalid");
    }
    for (key, session) in &state.sessions {
        if key != &session.cookie_hash
            || !is_secret_hash(key)
            || !is_secret_hash(&session.account_subject_hash)
            || session.schema != "ghostlight.app_session.v2"
            || session.heimdall_session_id.trim().is_empty()
            || session.heimdall_session_id.trim() != session.heimdall_session_id
            || !session
                .verified_capabilities
                .iter()
                .any(|value| value == "app_access")
            || session.refresh_expires_at <= session.access_expires_at
            || session.last_refresh_at < session.created_at
            || session
                .revoked_at
                .is_some_and(|revoked_at| revoked_at < session.created_at)
        {
            bail!("app-session state contains a noncanonical session");
        }
        unwrap_refresh(
            wrapping_key,
            &session.account_subject_hash,
            &session.heimdall_session_id,
            &session.encrypted_refresh_claim,
        )
        .context("app-session state contains an unreadable refresh claim")?;
    }
    Ok(())
}

fn is_secret_hash(value: &str) -> bool {
    value.len() == 71
        && value.strip_prefix("sha256:").is_some_and(|hex| {
            hex.bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
}

fn prune_sessions(state: &mut AppSessionState, now: DateTime<Utc>) {
    let retention_floor = now - chrono::Duration::days(7);
    state.sessions.retain(|_, session| {
        session.refresh_expires_at > retention_floor
            && session
                .revoked_at
                .is_none_or(|revoked_at| revoked_at > retention_floor)
    });
    if state.sessions.len() <= MAX_SESSION_RECORDS {
        return;
    }
    let mut oldest = state
        .sessions
        .iter()
        .map(|(key, session)| {
            let live = session.revoked_at.is_none() && session.refresh_expires_at > now;
            (live, session.last_refresh_at, key.clone())
        })
        .collect::<Vec<_>>();
    oldest.sort();
    for (_, _, key) in oldest
        .into_iter()
        .take(state.sessions.len() - MAX_SESSION_RECORDS)
    {
        state.sessions.remove(&key);
    }
}

fn read_wrapping_key(path: &Path) -> anyhow::Result<[u8; 32]> {
    let source = std::fs::read(path)
        .with_context(|| format!("failed to read session wrapping key {}", path.display()))?;
    let decoded = if source.len() == 32 {
        source
    } else {
        URL_SAFE_NO_PAD
            .decode(std::str::from_utf8(&source)?.trim())
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
    let aad = format!("ghostlight.app_session.refresh.v2\n{account_hash}\n{session_id}");
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
    let iv: [u8; 12] = URL_SAFE_NO_PAD
        .decode(iv)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("wrapped refresh IV is invalid"))?;
    let ciphertext = URL_SAFE_NO_PAD.decode(ciphertext)?;
    let aad = format!("ghostlight.app_session.refresh.v2\n{account_hash}\n{session_id}");
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

    fn session(claim: &str) -> VerifiedSessionAdmission {
        VerifiedSessionAdmission::fixture(
            "account-1",
            "heimdall-session-1",
            3,
            Utc::now() + chrono::Duration::minutes(5),
            Utc::now() + chrono::Duration::days(7),
            claim,
        )
    }

    #[test]
    fn store_rejects_an_alternate_messagepack_shape() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [5_u8; 32]).unwrap();
        let store_path = directory.path().join("app-sessions-v2.cc");
        let store = OwnedRedbMessagePackBackingStore::new(&store_path).unwrap();
        let state = AppSessionState {
            schema: STORE_SCHEMA.into(),
            sessions: BTreeMap::new(),
        };
        let row = CultCacheEnvelope {
            key: STORE_KEY.into(),
            r#type: STORE_TYPE.into(),
            payload: rmp_serde::to_vec(&state).unwrap(),
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(STORE_SCHEMA.into()),
        };
        assert!(store.compare_and_swap_batch(&[], vec![row]).unwrap());
        drop(store);
        assert!(AppSessionOwner::open(&store_path, &key).is_err());
    }

    #[test]
    fn stores_only_session_custody_and_reopens() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [7_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions-v2.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let cookie = owner
            .create_session(session("private-refresh-claim"))
            .unwrap();
        let account = owner
            .account_for_cookie(&cookie, Utc::now())
            .unwrap()
            .unwrap();
        let account_hash = account.account_subject_hash().to_owned();
        drop(owner);

        let mut reopened = AppSessionOwner::open(&store, &key).unwrap();
        assert_eq!(
            reopened
                .account_for_cookie(&cookie, Utc::now())
                .unwrap()
                .unwrap()
                .account_subject_hash(),
            account_hash
        );
        assert!(reopened.revoke_cookie(&cookie).unwrap());
        assert_eq!(
            reopened.account_for_cookie(&cookie, Utc::now()).unwrap(),
            None
        );
    }

    #[test]
    fn exact_heimdall_adoption_retry_rotates_the_only_live_cookie() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [9_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions-v2.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let first = owner
            .create_session(session("same-refresh-receipt"))
            .unwrap();
        let second = owner
            .create_session(session("same-refresh-receipt"))
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(owner.account_for_cookie(&first, Utc::now()).unwrap(), None);
        assert!(
            owner
                .account_for_cookie(&second, Utc::now())
                .unwrap()
                .is_some()
        );
        assert_eq!(
            owner
                .state
                .sessions
                .values()
                .filter(|session| session.revoked_at.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn rejects_legacy_or_foreign_store_instead_of_migrating_it() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [3_u8; 32]).unwrap();
        let store_path = directory.path().join("app-sessions-v2.cc");
        let store = OwnedRedbMessagePackBackingStore::new(&store_path).unwrap();
        let foreign = CultCacheEnvelope {
            key: "primary".into(),
            r#type: "app_session_store.v1".into(),
            payload: vec![],
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some("ghostlight.app_session_store.v1".into()),
        };
        assert!(store.compare_and_swap_batch(&[], vec![foreign]).unwrap());
        assert!(AppSessionOwner::open(&store_path, &key).is_err());
    }

    #[test]
    fn refresh_claim_is_encrypted_and_exactly_bound() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [11_u8; 32]).unwrap();
        let store = directory.path().join("app-sessions-v2.cc");
        let mut owner = AppSessionOwner::open(&store, &key).unwrap();
        let cookie = owner
            .create_session(session("needle-private-refresh"))
            .unwrap();
        let candidate = owner.session_for_logout(&cookie).unwrap().unwrap();
        assert_eq!(candidate.refresh_claim, "needle-private-refresh");
        drop(owner);
        let bytes = std::fs::read(store).unwrap();
        assert!(
            !bytes
                .windows("needle-private-refresh".len())
                .any(|window| window == b"needle-private-refresh")
        );
    }

    #[test]
    fn replacing_the_store_path_revokes_session_authentication_until_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [19_u8; 32]).unwrap();
        let path = directory.path().join("app-sessions-v2.cc");
        let displaced = directory.path().join("displaced.cc");
        let mut owner = AppSessionOwner::open(&path, &key).unwrap();
        let cookie = owner
            .create_session(session("private-refresh-claim"))
            .unwrap();

        std::fs::rename(&path, &displaced).unwrap();
        std::fs::File::create(&path).unwrap();
        assert!(owner.account_for_cookie(&cookie, Utc::now()).is_err());
        assert!(!owner.is_healthy());

        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&displaced, &path).unwrap();
        assert!(owner.account_for_cookie(&cookie, Utc::now()).is_err());
        drop(owner);

        let mut reopened = AppSessionOwner::open(&path, &key).unwrap();
        assert!(
            reopened
                .account_for_cookie(&cookie, Utc::now())
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn reopen_rejects_corrupt_inner_session_custody() {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [23_u8; 32]).unwrap();
        let path = directory.path().join("app-sessions-v2.cc");
        let mut owner = AppSessionOwner::open(&path, &key).unwrap();
        owner
            .create_session(session("private-refresh-claim"))
            .unwrap();
        drop(owner);

        let store = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
        let rows = store.pull_all().unwrap();
        let mut state: AppSessionState = rmp_serde::from_slice(&rows[0].payload).unwrap();
        state
            .sessions
            .values_mut()
            .next()
            .unwrap()
            .encrypted_refresh_claim = "not-an-encrypted-claim".into();
        let corrupt = envelope(&state).unwrap();
        assert!(store.compare_and_swap_batch(&rows, vec![corrupt]).unwrap());
        drop(store);

        assert!(AppSessionOwner::open(&path, &key).is_err());
    }
}
