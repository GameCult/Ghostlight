//! Aetheria's catalog file, read once into the two things Ghostlight takes from
//! it: the verse grammar, as a `GrammarBinding`, and the factions, as records
//! keyed for import.
//!
//! The catalog is Aetheria's: this reader opens it read-only and never writes
//! it, and it ignores every schema it does not mirror. The mirrors below decode
//! `aetheria.verse_grammar` v1, `aetheria.verse_verb` v1 and `aetheria.faction`
//! v1 by integer key: a MessagePack-keyed object is an array indexed by key,
//! nil in the gaps, so a field Aetheria adds later at a higher key is ignored
//! and a field it drops is an error naming the record and the key. A record
//! that does not decode, an enum value outside the mirror and a verb that
//! breaks the grammar's name or role rules are errors, never skipped rows.

use crate::ImportKey;
use crate::grammar::{
    GrammarBinding, GrammarError, GrammarReferentKind, GrammarRole, GrammarVerb, RenderPath,
};
use cultcache_rs::SingleFileMessagePackBackingStore;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

const GRAMMAR_SCHEMA: &str = "aetheria.verse_grammar";
const VERB_SCHEMA: &str = "aetheria.verse_verb";
const FACTION_SCHEMA: &str = "aetheria.faction";

const VERB_NAME_MAX: usize = 48;
const ROLE_NAME_MAX: usize = 32;
/// The invoking subject, never a role.
const ACTOR_ROLE: &str = "actor";

/// What Ghostlight takes from Aetheria's catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct AetheriaCatalog {
    pub grammar: GrammarBinding,
    /// In record-key order.
    pub factions: Vec<AetheriaFaction>,
}

/// One `aetheria.faction` record. `allegiance` is carried and read by no
/// consumer yet.
#[derive(Clone, Debug, PartialEq)]
pub struct AetheriaFaction {
    pub key: ImportKey,
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub allegiance: BTreeMap<ImportKey, f32>,
}

/// Which of the grammar's name or role rules a verb broke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum VerbRule {
    #[error("verb name is not canonical")]
    Name,
    #[error("role name is not canonical")]
    RoleName,
    #[error("role name is reserved for the invoking subject")]
    ReservedRole,
    #[error("role name is declared twice")]
    DuplicateRole,
}

/// Why a catalog could not be read. Records are named by record key and fields
/// by integer key; no error carries a value the catalog held.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CatalogReadError {
    #[error("the catalog file cannot be read")]
    Unreadable,
    #[error("the catalog holds no aetheria.verse_grammar global")]
    NoGrammar,
    #[error("the catalog holds more than one aetheria.verse_grammar record")]
    MultipleGrammars,
    #[error("record {record:?} field {field} is missing or not the mirrored type")]
    MalformedRecord { record: String, field: u8 },
    #[error("record {record:?} field {field} is not a known render path")]
    UnknownRenderPath { record: String, field: u8 },
    #[error("record {record:?} field {field} is not a known referent kind")]
    UnknownReferentKind { record: String, field: u8 },
    #[error("verb record {record:?} breaks a grammar rule: {rule}")]
    InvalidVerb { record: String, rule: VerbRule },
    #[error("the verbs do not bind a grammar: {0}")]
    Grammar(#[from] GrammarError),
}

/// Reads the catalog at `path` without writing it. An absent file is
/// `Unreadable`, not an empty catalog.
pub fn read_aetheria_catalog(path: impl AsRef<Path>) -> Result<AetheriaCatalog, CatalogReadError> {
    let path = path.as_ref();
    std::fs::metadata(path).map_err(|_| CatalogReadError::Unreadable)?;
    let mut rows = SingleFileMessagePackBackingStore::new(path)
        .pull_all_read_only_snapshot()
        .map_err(|_| CatalogReadError::Unreadable)?;
    rows.sort_by(|a, b| a.key.cmp(&b.key));

    let mut revision = None;
    let mut verbs = Vec::new();
    let mut factions = Vec::new();
    for row in &rows {
        match row.r#type.as_str() {
            GRAMMAR_SCHEMA => {
                if revision.is_some() {
                    return Err(CatalogReadError::MultipleGrammars);
                }
                let cells = Cells::of(&row.key, &row.payload)?;
                revision = Some(u32::try_from(cells.int(1)?).map_err(|_| cells.malformed(1))?);
            }
            VERB_SCHEMA => verbs.push(read_verb(&Cells::of(&row.key, &row.payload)?)?),
            FACTION_SCHEMA => {
                factions.push(read_faction(&row.key, &Cells::of(&row.key, &row.payload)?)?)
            }
            _ => {}
        }
    }
    let revision = revision.ok_or(CatalogReadError::NoGrammar)?;
    Ok(AetheriaCatalog {
        grammar: GrammarBinding::new(GRAMMAR_SCHEMA, revision, verbs)?,
        factions,
    })
}

/// A record's payload: the positional array of its keyed fields.
struct Cells<'a> {
    record: &'a str,
    cells: Vec<Value>,
}

impl<'a> Cells<'a> {
    fn of(record: &'a str, payload: &[u8]) -> Result<Self, CatalogReadError> {
        match rmp_serde::from_slice::<Value>(payload) {
            Ok(Value::Array(cells)) => Ok(Self { record, cells }),
            _ => Err(CatalogReadError::MalformedRecord {
                record: record.to_owned(),
                field: 0,
            }),
        }
    }

    fn malformed(&self, field: u8) -> CatalogReadError {
        CatalogReadError::MalformedRecord {
            record: self.record.to_owned(),
            field,
        }
    }

    fn cell(&self, field: u8) -> Result<&Value, CatalogReadError> {
        self.cells
            .get(usize::from(field))
            .ok_or_else(|| self.malformed(field))
    }

    fn int(&self, field: u8) -> Result<i64, CatalogReadError> {
        self.cell(field)?
            .as_i64()
            .ok_or_else(|| self.malformed(field))
    }

    fn text(&self, field: u8) -> Result<String, CatalogReadError> {
        self.cell(field)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| self.malformed(field))
    }

    /// A string Aetheria may leave null; null reads as empty.
    fn text_or_empty(&self, field: u8) -> Result<String, CatalogReadError> {
        match self.cell(field)? {
            Value::Null => Ok(String::new()),
            Value::String(text) => Ok(text.clone()),
            _ => Err(self.malformed(field)),
        }
    }

    fn array(&self, field: u8) -> Result<&Vec<Value>, CatalogReadError> {
        self.cell(field)?
            .as_array()
            .ok_or_else(|| self.malformed(field))
    }
}

fn read_verb(cells: &Cells) -> Result<(String, GrammarVerb), CatalogReadError> {
    let invalid = |rule| CatalogReadError::InvalidVerb {
        record: cells.record.to_owned(),
        rule,
    };
    let name = cells.text(1)?;
    if !canonical_name(&name, VERB_NAME_MAX) {
        return Err(invalid(VerbRule::Name));
    }
    let render_path = match cells.int(2)? {
        1 => RenderPath::ShipAction,
        2 => RenderPath::Conversation,
        _ => {
            return Err(CatalogReadError::UnknownRenderPath {
                record: cells.record.to_owned(),
                field: 2,
            });
        }
    };
    let mut roles: Vec<GrammarRole> = Vec::new();
    for role in cells.array(3)? {
        let role = Cells {
            record: cells.record,
            cells: role.as_array().cloned().ok_or_else(|| cells.malformed(3))?,
        };
        let role_name = role.text(1).map_err(|_| cells.malformed(3))?;
        if !canonical_name(&role_name, ROLE_NAME_MAX) {
            return Err(invalid(VerbRule::RoleName));
        }
        if role_name == ACTOR_ROLE {
            return Err(invalid(VerbRule::ReservedRole));
        }
        if roles.iter().any(|seen| seen.role == role_name) {
            return Err(invalid(VerbRule::DuplicateRole));
        }
        let referent = match role.int(2).map_err(|_| cells.malformed(3))? {
            1 => GrammarReferentKind::Person,
            2 => GrammarReferentKind::Faction,
            3 => GrammarReferentKind::Population,
            4 => GrammarReferentKind::Place,
            5 => GrammarReferentKind::Cargo,
            _ => {
                return Err(CatalogReadError::UnknownReferentKind {
                    record: cells.record.to_owned(),
                    field: 3,
                });
            }
        };
        roles.push(GrammarRole {
            role: role_name,
            referent,
        });
    }
    Ok((name, GrammarVerb { render_path, roles }))
}

/// A lower-case letter, then lower-case letters, digits and underscores.
fn canonical_name(name: &str, max_len: usize) -> bool {
    let mut chars = name.chars();
    name.len() <= max_len
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn read_faction(key: &str, cells: &Cells) -> Result<AetheriaFaction, CatalogReadError> {
    let Some(allegiance) = cells.cell(12)?.as_object() else {
        return Err(cells.malformed(12));
    };
    let mut standing = BTreeMap::new();
    for (other, weight) in allegiance {
        let weight = weight.as_f64().ok_or_else(|| cells.malformed(12))?;
        standing.insert(
            ImportKey::new(FACTION_SCHEMA, other.as_str()),
            weight as f32,
        );
    }
    Ok(AetheriaFaction {
        key: ImportKey::new(FACTION_SCHEMA, key),
        name: cells.text(1)?,
        short_name: cells.text_or_empty(2)?,
        description: cells.text_or_empty(3)?,
        allegiance: standing,
    })
}

#[cfg(test)]
mod tests;
