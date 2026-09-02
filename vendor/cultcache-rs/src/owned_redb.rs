use crate::{CacheBackingStore, CultCacheEnvelope, PushAllOptions};
use anyhow::{Context, Result, anyhow, bail};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const REDB_ENVELOPES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("cultcache_envelopes");

struct OwnedRedbInner {
    database: Database,
    _owner_lock: File,
    file_identity: String,
}

/// A pinned redb store for one long-lived service owner. This handle holds the
/// CultCache external lock, an open file handle, and the redb database for its
/// entire lifetime. Clones share that exact ownership authority.
#[derive(Clone)]
pub struct OwnedRedbMessagePackBackingStore {
    path: PathBuf,
    inner: Arc<OwnedRedbInner>,
}

impl std::fmt::Debug for OwnedRedbMessagePackBackingStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OwnedRedbMessagePackBackingStore")
            .field("path", &self.path)
            .field("file_identity", &self.inner.file_identity)
            .finish_non_exhaustive()
    }
}

impl OwnedRedbMessagePackBackingStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let lock_path = redb_lock_path(&path);
        let owner_lock = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open {}", lock_path.display()))?;
        fs2::FileExt::try_lock_exclusive(&owner_lock).with_context(|| {
            format!(
                "redb CultCache {} already has an active owner",
                path.display()
            )
        })?;

        // Give redb ownership of the already pinned file, structurally closing
        // path substitution during database creation. The post-open identity
        // comparison separately verifies that the pathname still names it.
        let database_file = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("failed to pin redb CultCache {}", path.display()))?;
        let held_identity = file_identity_from_file(&database_file)?;
        let database = Database::builder()
            .create_file(database_file)
            .with_context(|| format!("failed to open redb CultCache {}", path.display()))?;
        let path_identity_file = File::open(&path)?;
        let path_identity = file_identity_from_file(&path_identity_file)?;
        if held_identity != path_identity {
            bail!(
                "redb CultCache path identity changed while opening: held {held_identity}, path {path_identity}"
            );
        }
        let write = database.begin_write()?;
        {
            write.open_table(REDB_ENVELOPES)?;
        }
        write.commit()?;
        Ok(Self {
            path,
            inner: Arc::new(OwnedRedbInner {
                database,
                _owner_lock: owner_lock,
                file_identity: held_identity,
            }),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_identity(&self) -> &str {
        &self.inner.file_identity
    }

    pub fn require_file_identity(&self, expected: &str) -> Result<()> {
        if expected != self.file_identity() {
            bail!(
                "redb CultCache file identity mismatch: expected {expected}, owned {}",
                self.file_identity()
            );
        }
        Ok(())
    }

    pub fn validate_path_identity(&self) -> Result<()> {
        let current_file = File::open(&self.path)
            .with_context(|| format!("owned redb path {} is missing", self.path.display()))?;
        let current = file_identity_from_file(&current_file)?;
        self.require_file_identity(&current).with_context(|| {
            format!(
                "owned redb path {} no longer names the pinned file",
                self.path.display()
            )
        })
    }

    pub fn compare_and_swap_batch(
        &self,
        expected: &[CultCacheEnvelope],
        replacements: Vec<CultCacheEnvelope>,
    ) -> Result<bool> {
        if replacements.is_empty() {
            return Err(anyhow!(
                "conditional batch requires a non-empty replacement set"
            ));
        }
        let expected_ids = unique_batch_ids(expected, "expected")?;
        let replacement_ids = unique_batch_ids(&replacements, "replacement")?;
        if !expected_ids.is_subset(&replacement_ids) {
            return Err(anyhow!(
                "conditional batch must replace every expected identity"
            ));
        }
        let write = self.inner.database.begin_write()?;
        let matched = {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            let mut valid = true;
            for row in expected {
                if read_redb_entry(&table, row)? != Some(row.clone()) {
                    valid = false;
                    break;
                }
            }
            if valid {
                for row in &replacements {
                    if !expected_ids.contains(&entry_id(row))
                        && read_redb_entry(&table, row)?.is_some()
                    {
                        valid = false;
                        break;
                    }
                }
            }
            if valid {
                for row in &replacements {
                    insert_redb_entry(&mut table, row)?;
                }
            }
            valid
        };
        if matched {
            write.commit()?;
        } else {
            write.abort()?;
        }
        Ok(matched)
    }

    pub fn append_if_snapshot_unchanged(
        &self,
        expected_snapshot: &[CultCacheEnvelope],
        additions: Vec<CultCacheEnvelope>,
    ) -> Result<bool> {
        if additions.is_empty() {
            return Err(anyhow!("conditional snapshot append requires additions"));
        }
        unique_batch_ids(expected_snapshot, "expected snapshot")?;
        let addition_ids = unique_batch_ids(&additions, "snapshot additions")?;
        let write = self.inner.database.begin_write()?;
        let matched = {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            let mut current = read_all_redb(&table)?;
            let mut expected = expected_snapshot.to_vec();
            current.sort_by_key(entry_id);
            expected.sort_by_key(entry_id);
            let valid = current == expected
                && !current
                    .iter()
                    .any(|row| addition_ids.contains(&entry_id(row)));
            if valid {
                for row in &additions {
                    insert_redb_entry(&mut table, row)?;
                }
            }
            valid
        };
        if matched {
            write.commit()?;
        } else {
            write.abort()?;
        }
        Ok(matched)
    }

    pub fn replace_and_append_if_snapshot_unchanged(
        &self,
        expected_snapshot: &[CultCacheEnvelope],
        replacements: Vec<CultCacheEnvelope>,
    ) -> Result<bool> {
        if replacements.is_empty() {
            return Err(anyhow!(
                "conditional snapshot replacement requires replacements"
            ));
        }
        unique_batch_ids(expected_snapshot, "expected snapshot")?;
        unique_batch_ids(&replacements, "snapshot replacements")?;
        let write = self.inner.database.begin_write()?;
        let matched = {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            let mut current = read_all_redb(&table)?;
            let mut expected = expected_snapshot.to_vec();
            current.sort_by_key(entry_id);
            expected.sort_by_key(entry_id);
            let valid = current == expected;
            if valid {
                for row in &replacements {
                    insert_redb_entry(&mut table, row)?;
                }
            }
            valid
        };
        if matched {
            write.commit()?;
        } else {
            write.abort()?;
        }
        Ok(matched)
    }
}

impl CacheBackingStore for OwnedRedbMessagePackBackingStore {
    fn pull_all(&self) -> Result<Vec<CultCacheEnvelope>> {
        let read = self.inner.database.begin_read()?;
        let table = read.open_table(REDB_ENVELOPES)?;
        read_all_redb(&table)
    }

    fn push(&mut self, entry: &CultCacheEnvelope) -> Result<()> {
        let write = self.inner.database.begin_write()?;
        {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            insert_redb_entry(&mut table, entry)?;
        }
        write.commit()?;
        Ok(())
    }

    fn delete(&mut self, entry: &CultCacheEnvelope) -> Result<()> {
        let write = self.inner.database.begin_write()?;
        {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            let key = redb_identity(entry)?;
            table.remove(key.as_slice())?;
        }
        write.commit()?;
        Ok(())
    }

    fn push_all(&mut self, entries: &[CultCacheEnvelope], _options: PushAllOptions) -> Result<()> {
        unique_batch_ids(entries, "push all")?;
        let write = self.inner.database.begin_write()?;
        {
            let mut table = write.open_table(REDB_ENVELOPES)?;
            let keys = table
                .iter()?
                .map(|row| Ok(row?.0.value().to_vec()))
                .collect::<Result<Vec<_>>>()?;
            for key in keys {
                table.remove(key.as_slice())?;
            }
            for entry in entries {
                insert_redb_entry(&mut table, entry)?;
            }
        }
        write.commit()?;
        Ok(())
    }
}

fn unique_batch_ids(
    entries: &[CultCacheEnvelope],
    label: &str,
) -> Result<BTreeSet<(String, String)>> {
    let ids = entries.iter().map(entry_id).collect::<BTreeSet<_>>();
    if ids.len() != entries.len() {
        return Err(anyhow!(
            "conditional batch {label} set contains duplicate identities"
        ));
    }
    Ok(ids)
}

fn entry_id(entry: &CultCacheEnvelope) -> (String, String) {
    (entry.r#type.clone(), entry.key.clone())
}

fn redb_lock_path(path: &Path) -> PathBuf {
    let mut lock_name = path
        .file_name()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| "cultcache.cc".into());
    lock_name.push(".lock");
    path.with_file_name(lock_name)
}

#[cfg(unix)]
fn file_identity_from_file(file: &File) -> Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(format!(
        "unix:{:016x}:{:016x}",
        metadata.dev(),
        metadata.ino()
    ))
}

#[cfg(windows)]
fn file_identity_from_file(file: &File) -> Result<String> {
    use std::mem::zeroed;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    let success =
        unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut information) };
    if success == 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to read Windows file identity");
    }
    let index = ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64;
    Ok(format!(
        "windows:{:08x}:{index:016x}",
        information.dwVolumeSerialNumber
    ))
}

fn redb_identity(entry: &CultCacheEnvelope) -> Result<Vec<u8>> {
    rmp_serde::to_vec(&entry_id(entry)).context("failed to encode CultCache identity")
}

fn read_redb_entry(
    table: &impl ReadableTable<&'static [u8], &'static [u8]>,
    identity: &CultCacheEnvelope,
) -> Result<Option<CultCacheEnvelope>> {
    let key = redb_identity(identity)?;
    table
        .get(key.as_slice())?
        .map(|value| {
            rmp_serde::from_slice(value.value()).context("failed to decode redb CultCache envelope")
        })
        .transpose()
}

fn insert_redb_entry(
    table: &mut redb::Table<&[u8], &[u8]>,
    entry: &CultCacheEnvelope,
) -> Result<()> {
    let key = redb_identity(entry)?;
    let value = rmp_serde::to_vec(entry).context("failed to encode redb CultCache envelope")?;
    table.insert(key.as_slice(), value.as_slice())?;
    Ok(())
}

fn read_all_redb(
    table: &impl ReadableTable<&'static [u8], &'static [u8]>,
) -> Result<Vec<CultCacheEnvelope>> {
    let mut entries = table
        .iter()?
        .map(|row| {
            let (_, value) = row?;
            rmp_serde::from_slice(value.value()).context("failed to decode redb CultCache envelope")
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by_key(entry_id);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_envelope(r#type: &str, key: &str, payload: &[u8]) -> CultCacheEnvelope {
        CultCacheEnvelope {
            key: key.to_string(),
            r#type: r#type.to_string(),
            payload: payload.to_vec(),
            stored_at: "2026-07-13T00:00:00Z".to_string(),
            schema_id: Some(r#type.to_string()),
        }
    }

    #[test]
    fn legacy_redb_identity_remains_tuple_encoded() -> Result<()> {
        let entry = test_envelope("model", "current", b"one");
        assert_eq!(
            redb_identity(&entry)?,
            vec![
                0x92, 0xa5, b'm', b'o', b'd', b'e', b'l', 0xa7, b'c', b'u', b'r', b'r', b'e', b'n',
                b't',
            ]
        );
        Ok(())
    }

    #[test]
    fn owned_redb_clones_share_authority_and_fresh_owner_is_refused() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("owned.cc");
        let owner = OwnedRedbMessagePackBackingStore::new(&path)?;
        let clone = owner.clone();
        assert_eq!(owner.file_identity(), clone.file_identity());
        assert!(OwnedRedbMessagePackBackingStore::new(&path).is_err());
        drop(owner);
        assert!(OwnedRedbMessagePackBackingStore::new(&path).is_err());
        drop(clone);
        assert!(OwnedRedbMessagePackBackingStore::new(&path).is_ok());
        Ok(())
    }

    #[test]
    fn owned_redb_file_identity_is_stable_across_writes_and_reopen() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("identity.cc");
        let identity = {
            let mut owner = OwnedRedbMessagePackBackingStore::new(&path)?;
            let identity = owner.file_identity().to_string();
            owner.push(&test_envelope("model", "current", b"one"))?;
            owner.validate_path_identity()?;
            owner.require_file_identity(&identity)?;
            identity
        };
        let reopened = OwnedRedbMessagePackBackingStore::new(&path)?;
        assert_eq!(reopened.file_identity(), identity);
        assert_eq!(reopened.pull_all()?.len(), 1);
        Ok(())
    }

    #[test]
    fn owned_redb_identity_helper_refuses_mismatch() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let owner = OwnedRedbMessagePackBackingStore::new(temp.path().join("mismatch.cc"))?;
        assert!(owner.require_file_identity("not-the-owned-file").is_err());
        owner.require_file_identity(owner.file_identity())?;
        Ok(())
    }

    #[test]
    fn owned_redb_batch_refuses_stale_member_and_commits_success_atomically() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut owner = OwnedRedbMessagePackBackingStore::new(temp.path().join("batch-owned.cc"))?;
        let first = test_envelope("model", "first", b"one");
        let second = test_envelope("model", "second", b"one");
        owner.push(&first)?;
        owner.push(&second)?;
        let first_two = test_envelope("model", "first", b"two");
        let second_two = test_envelope("model", "second", b"two");
        let companion = test_envelope("receipt", "batch", b"committed");
        let stale_second = test_envelope("model", "second", b"stale");
        assert!(!owner.compare_and_swap_batch(
            &[first.clone(), stale_second],
            vec![first_two.clone(), second_two.clone(), companion.clone()],
        )?);
        assert_eq!(owner.pull_all()?, vec![first.clone(), second.clone()]);
        assert!(owner.compare_and_swap_batch(
            &[first, second],
            vec![first_two.clone(), second_two.clone(), companion.clone()],
        )?);
        assert_eq!(owner.pull_all()?, vec![first_two, second_two, companion]);
        Ok(())
    }

    #[test]
    fn owned_redb_snapshot_append_refuses_stale_or_colliding_state() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut owner =
            OwnedRedbMessagePackBackingStore::new(temp.path().join("snapshot-append.cc"))?;
        let model = test_envelope("model", "current", b"revision-1");
        owner.push(&model)?;
        let stale = owner.pull_all()?;
        owner.push(&test_envelope("event", "concurrent", b"live"))?;
        assert!(!owner.append_if_snapshot_unchanged(
            &stale,
            vec![test_envelope("readiness", "proof", b"ready")],
        )?);
        let exact = owner.pull_all()?;
        assert!(owner.append_if_snapshot_unchanged(
            &exact,
            vec![test_envelope("readiness", "proof", b"ready")],
        )?);
        let current = owner.pull_all()?;
        assert!(!owner.append_if_snapshot_unchanged(
            &current,
            vec![test_envelope("model", "current", b"collision")],
        )?);
        Ok(())
    }

    #[test]
    fn owned_redb_snapshot_replacement_and_append_refuses_concurrent_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut owner =
            OwnedRedbMessagePackBackingStore::new(temp.path().join("snapshot-replace.cc"))?;
        let current = test_envelope("session", "current", b"active");
        owner.push(&current)?;
        let stale = owner.pull_all()?;
        let concurrent = test_envelope("event", "concurrent", b"live");
        owner.push(&concurrent)?;
        let completed = test_envelope("session", "current", b"completed");
        let receipt = test_envelope("receipt", "terminal", b"done");
        assert!(!owner.replace_and_append_if_snapshot_unchanged(
            &stale,
            vec![completed.clone(), receipt.clone()],
        )?);
        let exact = owner.pull_all()?;
        assert!(owner.replace_and_append_if_snapshot_unchanged(
            &exact,
            vec![completed.clone(), receipt.clone()],
        )?);
        assert_eq!(owner.pull_all()?, vec![concurrent, receipt, completed]);
        Ok(())
    }

    #[test]
    fn owned_redb_path_replacement_cannot_redirect_pinned_authority() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("pinned.cc");
        let displaced = temp.path().join("displaced.cc");
        let mut owner = OwnedRedbMessagePackBackingStore::new(&path)?;
        let identity = owner.file_identity().to_string();
        owner.push(&test_envelope("model", "before", b"one"))?;
        fs::rename(&path, &displaced)?;
        File::create(&path)?;
        assert!(owner.validate_path_identity().is_err());
        assert_eq!(owner.file_identity(), identity);
        owner.push(&test_envelope("model", "after", b"two"))?;
        assert_eq!(owner.pull_all()?.len(), 2);
        assert_ne!(
            file_identity_from_file(&File::open(&path)?)?,
            owner.file_identity()
        );
        Ok(())
    }
}
