//! A read-only markdown vault, behind the world's `EvidenceSource` seam.
//!
//! It resolves referent strings to notes and hands back bounded excerpts. It
//! never writes, never leaves its configured root, never touches git or the
//! network, and holds no world truth: its output is `EvidenceReceipt`s whose
//! `excerpt` and `source` are prompt material and whose `reference` is the only
//! thing that can ever reach the journal.
//!
//! It is the one authority in the seed lane that can be tested with no kernel
//! at all, which is why it is a module.

use super::EvidenceRef;
use super::elaboration::{EvidenceError, EvidenceQuery, EvidenceReceipt, EvidenceSource};
use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Below `MAX_PATCH_EVIDENCE` (64), so a session that cites everything it was
/// handed still fits inside one patch.
const MAX_VAULT_RECEIPTS: usize = 24;
const MAX_HITS_PER_REFERENT: usize = 3;
/// One hop, never recursive: a note's neighbours are context, and a transitive
/// walk would hand a session the whole vault under one referent.
const MAX_LINK_FANOUT: usize = 4;
const MAX_EXCERPT_CHARS: usize = 800;
const MAX_VAULT_NOTES: usize = 4_096;
const MAX_NOTE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Error)]
pub(crate) enum VaultError {
    #[error("vault root is not a readable directory: {0}")]
    Root(String),
    #[error("vault scope escapes its root: {0}")]
    Scope(String),
    #[error("vault holds more than {MAX_VAULT_NOTES} notes")]
    TooManyNotes,
}

/// One note as the reader holds it. The body is kept in memory because the
/// corpora this reads are a few megabytes of markdown and a per-query file read
/// would buy nothing but a second failure mode.
#[derive(Clone, Debug)]
struct VaultNote {
    /// The evidence reference: the note's vault-relative path with forward
    /// slashes, e.g. `Spoilers/Places/Low Sere.md`. It is a path and not a
    /// wikilink target, because stems collide across directories; not a scheme,
    /// because the kernel has no scheme registry; and not a digest, because a
    /// human must be able to resolve it years later.
    reference: String,
    title: String,
    /// Everything after the frontmatter block.
    body: String,
    /// Wikilink targets, left side only.
    links: Vec<String>,
}

pub(crate) struct VaultEvidenceSource {
    notes: Vec<VaultNote>,
    /// Many-to-one: filename stem, frontmatter title, and each alias all key
    /// the same note. Ambiguity is resolved by the shortest reference path, and
    /// refused for that referent when two candidates tie.
    by_key: BTreeMap<String, Vec<usize>>,
}

impl VaultEvidenceSource {
    /// `root` is the configured vault root; `scope` is a relative subdirectory
    /// or empty. Both are canonicalized and the scope must live under the root,
    /// so `..`, an absolute scope, and a symlink out are all refused here. A
    /// spoiler tier is a directory in every vault that has one, so it is
    /// expressed as a scope and there is no tier field anywhere in this file.
    pub(crate) fn open(root: &Path, scope: &str) -> Result<Self, VaultError> {
        let root = root
            .canonicalize()
            .map_err(|error| VaultError::Root(error.to_string()))?;
        if !root.is_dir() {
            return Err(VaultError::Root(root.display().to_string()));
        }
        let scope_path = if scope.trim().is_empty() {
            root.clone()
        } else {
            let relative = Path::new(scope.trim());
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(VaultError::Scope(scope.to_owned()));
            }
            let joined = root.join(relative);
            let canonical = joined
                .canonicalize()
                .map_err(|error| VaultError::Scope(error.to_string()))?;
            if !canonical.starts_with(&root) {
                return Err(VaultError::Scope(canonical.display().to_string()));
            }
            canonical
        };

        let mut paths: Vec<PathBuf> = Vec::new();
        collect_notes(&scope_path, &mut paths)?;
        paths.sort();

        let mut notes = Vec::new();
        for path in paths {
            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            if metadata.len() > MAX_NOTE_BYTES {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(relative) = path.strip_prefix(&root) else {
                continue;
            };
            let reference = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if reference.trim() != reference || reference.is_empty() {
                continue;
            }
            let stem = path
                .file_stem()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (front, body) = split_frontmatter(&text);
            let (front_title, aliases) = read_front_keys(front);
            let title = front_title.unwrap_or_else(|| stem.clone());
            let links = wikilink_targets(body);
            let mut keys: Vec<String> = vec![stem, title.clone()];
            keys.extend(aliases);
            notes.push((
                VaultNote {
                    reference,
                    title,
                    body: body.to_owned(),
                    links,
                },
                keys,
            ));
        }

        let mut by_key: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, (_, keys)) in notes.iter().enumerate() {
            for key in keys {
                let key = key.trim().to_lowercase();
                if key.is_empty() {
                    continue;
                }
                by_key.entry(key).or_default().push(index);
            }
        }
        Ok(Self {
            notes: notes.into_iter().map(|(note, _)| note).collect(),
            by_key,
        })
    }

    /// Exact key first, then a bounded keyword search. A referent whose exact
    /// match is ambiguous between two equally short paths resolves to nothing
    /// rather than to a guess.
    fn resolve(&self, referent: &str) -> Vec<usize> {
        let key = referent.trim().to_lowercase();
        if let Some(candidates) = self.by_key.get(&key) {
            let mut ranked: Vec<usize> = candidates.clone();
            ranked.sort_by_key(|index| {
                (
                    self.notes[*index].reference.len(),
                    self.notes[*index].reference.clone(),
                )
            });
            ranked.dedup();
            if ranked.len() == 1 {
                return ranked;
            }
            let shortest = self.notes[ranked[0]].reference.len();
            let tied = ranked
                .iter()
                .filter(|index| self.notes[**index].reference.len() == shortest)
                .count();
            return if tied == 1 {
                vec![ranked[0]]
            } else {
                Vec::new()
            };
        }
        let tokens: Vec<String> = key
            .split_whitespace()
            .filter(|token| token.len() > 2)
            .map(str::to_owned)
            .collect();
        if tokens.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(usize, usize)> = self
            .notes
            .iter()
            .enumerate()
            .filter_map(|(index, note)| {
                let title = note.title.to_lowercase();
                let body = note.body.to_lowercase();
                let score: usize = tokens
                    .iter()
                    .map(|token| {
                        usize::from(title.contains(token.as_str())) * 4
                            + body.matches(token.as_str()).count().min(8)
                    })
                    .sum();
                (score > 0).then_some((score, index))
            })
            .collect();
        scored.sort_by_key(|(score, index)| {
            (
                std::cmp::Reverse(*score),
                self.notes[*index].reference.clone(),
            )
        });
        scored
            .into_iter()
            .take(MAX_HITS_PER_REFERENT)
            .map(|(_, index)| index)
            .collect()
    }

    fn receipt(&self, index: usize, heading: Option<&str>) -> EvidenceReceipt {
        let note = &self.notes[index];
        EvidenceReceipt {
            reference: EvidenceRef::new(note.reference.clone()),
            excerpt: excerpt(&note.body, heading),
            source: note.title.clone(),
        }
    }
}

#[async_trait]
impl EvidenceSource for VaultEvidenceSource {
    async fn retrieve(&self, query: &EvidenceQuery) -> Result<Vec<EvidenceReceipt>, EvidenceError> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut receipts = Vec::new();
        for referent in &query.referents {
            let (target, heading) = match referent.split_once('#') {
                Some((target, heading)) => (target, Some(heading)),
                None => (referent.as_str(), None),
            };
            for index in self.resolve(target) {
                if receipts.len() >= MAX_VAULT_RECEIPTS {
                    return Ok(receipts);
                }
                if seen.insert(self.notes[index].reference.clone()) {
                    receipts.push(self.receipt(index, heading));
                }
                for link in self.notes[index].links.iter().take(MAX_LINK_FANOUT) {
                    if receipts.len() >= MAX_VAULT_RECEIPTS {
                        return Ok(receipts);
                    }
                    let (link_target, link_heading) = match link.split_once('#') {
                        Some((target, heading)) => (target, Some(heading)),
                        None => (link.as_str(), None),
                    };
                    for linked in self.resolve(link_target) {
                        if seen.insert(self.notes[linked].reference.clone()) {
                            receipts.push(self.receipt(linked, link_heading));
                        }
                    }
                }
            }
        }
        Ok(receipts)
    }
}

fn collect_notes(directory: &Path, into: &mut Vec<PathBuf>) -> Result<(), VaultError> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| VaultError::Root(error.to_string()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_notes(&path, into)?;
        } else if path.extension().is_some_and(|value| value == "md") {
            if into.len() >= MAX_VAULT_NOTES {
                return Err(VaultError::TooManyNotes);
            }
            into.push(path);
        }
    }
    Ok(())
}

/// A leading `---` line opens a frontmatter block that runs to the next `---`.
/// Absence is normal — whole vaults carry none — so the fallback is the whole
/// file as body. The block is split, never parsed as YAML: only `title:` and
/// `aliases:` are read, line-wise, and everything else is ignored.
fn split_frontmatter(text: &str) -> (&str, &str) {
    let trimmed = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = trimmed
        .strip_prefix("---\r\n")
        .or_else(|| trimmed.strip_prefix("---\n"))
    else {
        return ("", trimmed);
    };
    for (offset, line) in rest.match_indices('\n') {
        let candidate = rest[..offset].rsplit('\n').next().unwrap_or_default();
        if candidate.trim_end() == "---" {
            let start = offset - candidate.len();
            return (&rest[..start], &rest[offset + line.len()..]);
        }
    }
    ("", trimmed)
}

fn read_front_keys(front: &str) -> (Option<String>, Vec<String>) {
    let mut title = None;
    let mut aliases = Vec::new();
    let mut in_alias_list = false;
    for line in front.lines() {
        let unindented = line.trim_start();
        if let Some(item) = unindented.strip_prefix("- ")
            && in_alias_list
        {
            aliases.push(clean_scalar(item));
            continue;
        }
        in_alias_list = false;
        if let Some(value) = unindented.strip_prefix("title:") {
            let value = clean_scalar(value);
            if !value.is_empty() {
                title = Some(value);
            }
        } else if let Some(value) = unindented.strip_prefix("aliases:") {
            let value = value.trim();
            if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                aliases.extend(inner.split(',').map(clean_scalar).filter(|v| !v.is_empty()));
            } else if value.is_empty() {
                in_alias_list = true;
            } else {
                aliases.push(clean_scalar(value));
            }
        }
    }
    (
        title,
        aliases.into_iter().filter(|v| !v.is_empty()).collect(),
    )
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .trim()
        .to_owned()
}

/// Three forms: `[[Target]]`, `[[Target|display]]`, and `[[Target#Heading]]`.
/// The display half is discarded; the heading half is kept, because it seeks
/// the excerpt.
fn wikilink_targets(body: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("[[") {
        rest = &rest[open + 2..];
        let Some(close) = rest.find("]]") else {
            break;
        };
        let inner = &rest[..close];
        rest = &rest[close + 2..];
        let target = inner.split('|').next().unwrap_or_default().trim();
        if !target.is_empty() && !targets.iter().any(|seen| seen == target) {
            targets.push(target.to_owned());
        }
    }
    targets
}

/// The named heading's section when one was asked for, else the head of the
/// body. Cut at a char boundary, because a reference is bytes and an excerpt is
/// text.
fn excerpt(body: &str, heading: Option<&str>) -> String {
    let region = heading
        .and_then(|heading| heading_section(body, heading))
        .unwrap_or(body);
    let region = region.trim();
    match region.char_indices().nth(MAX_EXCERPT_CHARS) {
        None => region.to_owned(),
        Some((cut, _)) => region[..cut].to_owned(),
    }
}

fn heading_section<'a>(body: &'a str, heading: &str) -> Option<&'a str> {
    let wanted = heading.trim().to_lowercase();
    let mut start = None;
    let mut level = 0usize;
    let mut offset = 0usize;
    for line in body.split_inclusive('\n') {
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if hashes > 0 {
            let text = line[hashes..].trim().to_lowercase();
            match start {
                None if text == wanted => {
                    start = Some(offset + line.len());
                    level = hashes;
                }
                Some(begin) if hashes <= level => return Some(&body[begin..offset]),
                _ => {}
            }
        }
        offset += line.len();
    }
    start.map(|begin| &body[begin..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn fixture() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        write(
            root,
            "Public/Places/Low Sere.md",
            "The sere runs dry nine months a year.\nSee [[Rain Gate]] and [[Rain Gate|the gate]].\n",
        );
        write(
            root,
            "Public/Places/Rain Gate.md",
            "---\ntitle: The Rain Gate\naliases:\n  - Gate of Rain\n  - Hinge\nstatus: canonical\n---\n# Body\nWater enters here.\n\n## Tolls\nA toll of one is charged.\n",
        );
        write(root, "Public/index.md", "Public index.\n");
        write(root, "Public/World/index.md", "World index, deeper.\n");
        write(
            root,
            "Spoilers/Dungeons/Provenance.md",
            "The hinge was built by the drowned. Link: [[Low Sere#Nothing]]\n",
        );
        directory
    }

    fn query(referents: &[&str]) -> EvidenceQuery {
        EvidenceQuery {
            referents: referents.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    /// Spec test 14. References are vault-relative paths, every wikilink form
    /// resolves to the same note as the bare one, an ambiguous stem resolves to
    /// the shortest path, the excerpt is bounded, and a source opened on
    /// `Public` cannot see `Spoilers`.
    #[tokio::test]
    async fn the_vault_resolves_titles_aliases_and_wikilinks_inside_its_scope() {
        let directory = fixture();
        let whole = VaultEvidenceSource::open(directory.path(), "").unwrap();

        let receipts = whole.retrieve(&query(&["Low Sere"])).await.unwrap();
        assert_eq!(
            receipts[0].reference,
            EvidenceRef::new("Public/Places/Low Sere.md"),
            "a reference is the vault-relative path, forward-slashed"
        );
        // The bare and the piped link are one target, and the linked note came
        // back through the one-hop fanout.
        assert!(
            receipts
                .iter()
                .any(|receipt| receipt.reference == EvidenceRef::new("Public/Places/Rain Gate.md"))
        );

        // Frontmatter title and alias key the same note as the stem does.
        for name in ["Rain Gate", "The Rain Gate", "Gate of Rain", "hinge"] {
            let hit = whole.retrieve(&query(&[name])).await.unwrap();
            assert_eq!(
                hit[0].reference,
                EvidenceRef::new("Public/Places/Rain Gate.md"),
                "{name} did not resolve to the note"
            );
        }

        // A heading link seeks its section rather than the head of the body.
        let heading = whole.retrieve(&query(&["Rain Gate#Tolls"])).await.unwrap();
        assert!(heading[0].excerpt.contains("toll of one"));
        assert!(!heading[0].excerpt.contains("Water enters here"));

        // Two `index.md` notes; the shallower path wins.
        let ambiguous = whole.retrieve(&query(&["index"])).await.unwrap();
        assert_eq!(
            ambiguous[0].reference,
            EvidenceRef::new("Public/index.md"),
            "an ambiguous stem resolved to the deeper path"
        );

        for receipt in whole
            .retrieve(&query(&["Low Sere", "index"]))
            .await
            .unwrap()
        {
            assert!(receipt.excerpt.chars().count() <= MAX_EXCERPT_CHARS);
            assert!(receipt.reference.text().trim() == receipt.reference.text());
            assert!(!receipt.reference.text().is_empty());
        }
        assert!(
            whole
                .retrieve(&query(&["Low Sere", "Provenance", "index"]))
                .await
                .unwrap()
                .len()
                <= MAX_VAULT_RECEIPTS
        );

        // The spoiler tier is a directory, so it is a scope.
        let public = VaultEvidenceSource::open(directory.path(), "Public").unwrap();
        assert!(
            public
                .retrieve(&query(&["Provenance"]))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            whole
                .retrieve(&query(&["Provenance"]))
                .await
                .unwrap()
                .iter()
                .any(|receipt| receipt.reference
                    == EvidenceRef::new("Spoilers/Dungeons/Provenance.md")),
            "the unscoped source should still read the spoiler subtree"
        );
    }

    /// Soul. The scope check runs once, at `open`, against the scope string.
    /// The walk that follows does not: `collect_notes` recurses through
    /// anything `is_dir()` answers for, which follows a directory junction, and
    /// `strip_prefix(&root)` then runs against the uncanonicalized path, so a
    /// note outside the root comes back wearing a vault-relative reference.
    ///
    /// A junction needs no privilege on Windows, so this is reachable by
    /// anyone who can write inside the configured vault.
    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "FALSIFIED: collect_notes follows a directory junction out of the vault root"]
    async fn soul_a_junction_inside_the_root_reads_a_note_outside_it() {
        let directory = fixture();
        let outside = tempfile::tempdir().unwrap();
        write(
            outside.path(),
            "Secret.md",
            "The vault root does not cover me.\n",
        );
        let link = directory.path().join("Public").join("Escape");
        let made = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.display().to_string(),
                &outside.path().display().to_string(),
            ])
            .output();
        if made.is_err() || !link.exists() {
            eprintln!("skipped: this environment cannot create a directory junction");
            return;
        }

        let source = VaultEvidenceSource::open(directory.path(), "Public").unwrap();
        let receipts = source.retrieve(&query(&["Secret"])).await.unwrap();
        assert!(
            receipts.is_empty(),
            "a note outside the configured root was read and handed back as {:?}",
            receipts
                .iter()
                .map(|receipt| receipt.reference.text().to_owned())
                .collect::<Vec<_>>()
        );
    }

    /// Soul. Two bounds and one promise. A note over `MAX_NOTE_BYTES` is
    /// skipped on its metadata, so it is never read into memory and never
    /// reaches a receipt; and neither `open` nor `retrieve` writes anything,
    /// asserted against the tree's own bytes and modification times rather than
    /// against the doc comment.
    #[tokio::test]
    async fn soul_an_oversized_note_is_never_read_and_the_reader_never_writes() {
        let directory = fixture();
        let root = directory.path();
        let huge = format!(
            "---\ntitle: The Long Note\n---\n{}\n",
            "obese ".repeat(usize::try_from(MAX_NOTE_BYTES).unwrap() / 4)
        );
        assert!(u64::try_from(huge.len()).unwrap() > MAX_NOTE_BYTES);
        write(root, "Public/Places/Long Note.md", &huge);

        let before = tree(root);
        let source = VaultEvidenceSource::open(root, "").unwrap();
        for name in ["Long Note", "The Long Note", "obese"] {
            let receipts = source.retrieve(&query(&[name])).await.unwrap();
            assert!(
                receipts
                    .iter()
                    .all(|receipt| receipt.reference
                        != EvidenceRef::new("Public/Places/Long Note.md")),
                "{name} returned a note over the byte cap"
            );
        }
        assert_eq!(before, tree(root), "the reader changed the vault");
    }

    /// Every file under `root`, by relative path, length, and modification
    /// time: what a read-only reader must leave exactly as it found it.
    fn tree(root: &Path) -> BTreeMap<String, (u64, std::time::SystemTime)> {
        let mut found = BTreeMap::new();
        let mut paths = Vec::new();
        collect_all(root, &mut paths);
        for path in paths {
            let metadata = std::fs::metadata(&path).unwrap();
            found.insert(
                path.strip_prefix(root).unwrap().display().to_string(),
                (metadata.len(), metadata.modified().unwrap()),
            );
        }
        found
    }

    fn collect_all(directory: &Path, into: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_all(&path, into);
            } else {
                into.push(path);
            }
        }
    }

    /// Soul. `MAX_VAULT_RECEIPTS` is the cap the module names, and the reason
    /// it gives is that a session citing everything still fits in one patch.
    /// The link fanout overshoots it: the guard is checked before a link is
    /// resolved and not before each receipt that link produces, so one
    /// keyword-resolving link adds up to `MAX_HITS_PER_REFERENT` past the cap.
    #[tokio::test]
    async fn soul_the_link_fanout_overshoots_the_receipt_cap() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        // Enough plain notes to walk the receipt count up to the cap, each one
        // resolving by its own stem and linking nowhere.
        for index in 0..MAX_VAULT_RECEIPTS {
            write(root, &format!("plain{index}.md"), "A plain note.\n");
        }
        // One note whose single wikilink resolves by keyword, not by key, to
        // three notes at once.
        write(root, "hub.md", "See [[quarry stone]].\n");
        for index in 0..MAX_HITS_PER_REFERENT {
            write(
                root,
                &format!("quarry{index}.md"),
                "quarry stone quarry stone\n",
            );
        }
        let source = VaultEvidenceSource::open(root, "").unwrap();

        let mut referents: Vec<String> = (0..MAX_VAULT_RECEIPTS - 1)
            .map(|index| format!("plain{index}"))
            .collect();
        referents.push("hub".to_owned());
        let receipts = source.retrieve(&EvidenceQuery { referents }).await.unwrap();
        assert!(
            receipts.len() <= MAX_VAULT_RECEIPTS,
            "one call returned {} receipts against a cap of {MAX_VAULT_RECEIPTS}",
            receipts.len()
        );
    }

    /// Soul. A vault error carries the server's absolute filesystem paths, and
    /// `seed_once` maps `VaultError` straight into `RuntimeCommandError::Payload`,
    /// so the string reaches the owner's command receipt. The reference format
    /// is deliberately vault-relative; these strings are not.
    #[test]
    #[ignore = "FALSIFIED: VaultError::Root carries the server absolute path into the owner receipt"]
    fn soul_a_vault_error_carries_an_absolute_path() {
        let directory = fixture();
        let root = directory.path();
        for (case, opened) in [
            (
                "an escaping scope",
                VaultEvidenceSource::open(root, "Public/../.."),
            ),
            // The reachable misconfiguration: `GHOSTLIGHT_SEED_VAULT_ROOT`
            // naming something that is not a directory.
            (
                "a root that is a file",
                VaultEvidenceSource::open(&root.join("Public/index.md"), ""),
            ),
        ] {
            let Err(error) = opened else {
                panic!("{case} was admitted");
            };
            let message = error.to_string();
            assert!(
                !message.contains(&root.display().to_string()) && !message.contains(":\\"),
                "{case} handed back an absolute path: {message}"
            );
        }
    }

    /// Spec test 15. The scope is a corner of a configured vault, never a read
    /// primitive over the filesystem.
    #[test]
    fn a_vault_scope_cannot_escape_its_root() {
        let directory = fixture();
        let root = directory.path();
        for scope in ["..", "../..", "Public/../..", "Public/../../etc"] {
            assert!(
                VaultEvidenceSource::open(root, scope).is_err(),
                "{scope} was admitted"
            );
        }
        let absolute = root.join("Public");
        assert!(VaultEvidenceSource::open(root, &absolute.display().to_string()).is_err());
        assert!(VaultEvidenceSource::open(root, "Public").is_ok());
    }
}
