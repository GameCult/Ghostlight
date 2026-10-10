use super::*;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope};
use serde_json::{Value, json};

/// Written by Aetheria's own fixture writer (`VerseGrammarTests.WriteFixture`)
/// at Aetheria master 92b82e9a; never edited. See `tests/fixtures/aetheria-verse-catalog.provenance.txt`.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/aetheria-verse-catalog.cc"
);

fn fixture_rows() -> Vec<CultCacheEnvelope> {
    SingleFileMessagePackBackingStore::new(FIXTURE)
        .pull_all_read_only_snapshot()
        .unwrap()
}

fn row_of<'a>(rows: &'a mut [CultCacheEnvelope], key: &str) -> &'a mut CultCacheEnvelope {
    rows.iter_mut().find(|row| row.key == key).unwrap()
}

/// Re-encodes one record's positional payload after `edit`.
fn edit(rows: &mut [CultCacheEnvelope], key: &str, change: impl FnOnce(&mut Vec<Value>)) {
    let row = row_of(rows, key);
    let Value::Array(mut cells) = rmp_serde::from_slice(&row.payload).unwrap() else {
        panic!("{key} is not a positional record");
    };
    change(&mut cells);
    row.payload = rmp_serde::to_vec(&Value::Array(cells)).unwrap();
}

fn catalog_of(rows: &[CultCacheEnvelope]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.cc");
    let mut store = SingleFileMessagePackBackingStore::new(&path);
    for row in rows {
        store.push(row).unwrap();
    }
    (dir, path)
}

fn read(rows: &[CultCacheEnvelope]) -> Result<AetheriaCatalog, CatalogReadError> {
    let (_dir, path) = catalog_of(rows);
    read_aetheria_catalog(&path)
}

fn verb(path: RenderPath, roles: &[(&str, GrammarReferentKind)]) -> GrammarVerb {
    GrammarVerb {
        render_path: path,
        roles: roles
            .iter()
            .map(|(role, referent)| GrammarRole {
                role: (*role).into(),
                referent: *referent,
            })
            .collect(),
    }
}

/// What Aetheria's fixture writer authored, spelled out by hand.
fn authored_verbs() -> Vec<(String, GrammarVerb)> {
    vec![
        (
            "haul".into(),
            verb(
                RenderPath::ShipAction,
                &[
                    ("cargo", GrammarReferentKind::Cargo),
                    ("destination", GrammarReferentKind::Place),
                ],
            ),
        ),
        (
            "attack".into(),
            verb(
                RenderPath::ShipAction,
                &[("target", GrammarReferentKind::Faction)],
            ),
        ),
        ("speak".into(), verb(RenderPath::Conversation, &[])),
    ]
}

fn faction_key(record: &str) -> ImportKey {
    ImportKey::new("aetheria.faction", record)
}

#[test]
fn reads_the_aetheria_written_fixture() {
    let catalog = read_aetheria_catalog(FIXTURE).unwrap();

    let identity = catalog.grammar.identity();
    assert_eq!(identity.schema_id, "aetheria.verse_grammar");
    assert_eq!(identity.revision, 1);
    for (kind, expected) in authored_verbs() {
        assert_eq!(catalog.grammar.verb(&kind), Some(&expected), "{kind}");
    }
    assert!(catalog.grammar.verb("mine").is_none());

    assert_eq!(
        catalog.factions,
        vec![
            AetheriaFaction {
                key: faction_key("faction-adrasteia"),
                name: "Adrasteia".into(),
                short_name: "ADR".into(),
                description: "First.".into(),
                allegiance: BTreeMap::from([(faction_key("faction-brannoch"), 0.75)]),
            },
            AetheriaFaction {
                key: faction_key("faction-brannoch"),
                name: "Brannoch".into(),
                short_name: "BRN".into(),
                description: "Second.".into(),
                allegiance: BTreeMap::from([(faction_key("faction-adrasteia"), 0.25)]),
            },
        ]
    );
}

#[test]
fn grammar_digest_matches_a_binding_built_by_hand() {
    let by_hand = GrammarBinding::new("aetheria.verse_grammar", 1, authored_verbs()).unwrap();
    let read = read_aetheria_catalog(FIXTURE).unwrap().grammar;
    assert_eq!(read.identity().digest, by_hand.identity().digest);
    assert_eq!(read, by_hand);

    // The digest follows the verbs: a role the file drops is a different grammar.
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-haul", |cells| {
        cells[3].as_array_mut().unwrap().pop();
    });
    assert_ne!(
        read_catalog_digest(&rows),
        by_hand.identity().digest,
        "dropping a role did not move the digest"
    );
}

fn read_catalog_digest(rows: &[CultCacheEnvelope]) -> String {
    read(rows).unwrap().grammar.identity().digest
}

#[test]
fn refuses_a_catalog_without_a_grammar() {
    let mut rows = fixture_rows();
    rows.retain(|row| row.r#type != "aetheria.verse_grammar");
    assert_eq!(read(&rows), Err(CatalogReadError::NoGrammar));
}

#[test]
fn refuses_a_second_grammar_record() {
    let mut rows = fixture_rows();
    let mut second = rows
        .iter()
        .find(|row| row.r#type == "aetheria.verse_grammar")
        .unwrap()
        .clone();
    second.key.push_str("-again");
    rows.push(second);
    assert_eq!(read(&rows), Err(CatalogReadError::MultipleGrammars));
}

#[test]
fn refuses_an_unknown_render_path() {
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-haul", |cells| cells[2] = json!(3));
    let error = read(&rows).unwrap_err();
    assert_eq!(
        error,
        CatalogReadError::UnknownRenderPath {
            record: "verb-haul".into(),
            field: 2
        }
    );
    assert!(!error.to_string().contains('3'), "the value was echoed");
}

#[test]
fn refuses_an_unknown_referent_kind() {
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-attack", |cells| {
        cells[3][0][2] = json!(9);
    });
    assert_eq!(
        read(&rows),
        Err(CatalogReadError::UnknownReferentKind {
            record: "verb-attack".into(),
            field: 3
        })
    );
}

#[test]
fn a_verb_breaking_the_name_or_role_rules_is_refused_by_key() {
    let broken = |mutate: fn(&mut Vec<Value>)| {
        let mut rows = fixture_rows();
        edit(&mut rows, "verb-haul", mutate);
        read(&rows).unwrap_err()
    };
    let invalid = |rule| CatalogReadError::InvalidVerb {
        record: "verb-haul".into(),
        rule,
    };
    assert_eq!(
        broken(|cells| cells[1] = json!("Haul")),
        invalid(VerbRule::Name)
    );
    assert_eq!(
        broken(|cells| cells[1] = json!("1haul")),
        invalid(VerbRule::Name)
    );
    assert_eq!(
        broken(|cells| cells[1] = json!("")),
        invalid(VerbRule::Name)
    );
    assert_eq!(
        broken(|cells| cells[1] = json!("h".repeat(49))),
        invalid(VerbRule::Name)
    );
    assert_eq!(
        broken(|cells| cells[3][0][1] = json!("Cargo")),
        invalid(VerbRule::RoleName)
    );
    assert_eq!(
        broken(|cells| cells[3][0][1] = json!("c".repeat(33))),
        invalid(VerbRule::RoleName)
    );
    assert_eq!(
        broken(|cells| cells[3][0][1] = json!("actor")),
        invalid(VerbRule::ReservedRole)
    );
    assert_eq!(
        broken(|cells| cells[3][1][1] = json!("cargo")),
        invalid(VerbRule::DuplicateRole)
    );
}

#[test]
fn the_longest_canonical_names_are_admitted() {
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-haul", |cells| {
        cells[1] = json!(format!("h{}9", "_9".repeat(23)));
        cells[3][0][1] = json!(format!("c{}", "z".repeat(31)));
    });
    let catalog = read(&rows).unwrap();
    assert_eq!(catalog.grammar.identity().revision, 1);
}

#[test]
fn two_verbs_with_one_name_do_not_bind() {
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-haul", |cells| cells[1] = json!("attack"));
    assert_eq!(
        read(&rows),
        Err(CatalogReadError::Grammar(GrammarError::DuplicateVerb))
    );
}

#[test]
fn a_record_that_is_not_the_mirrored_shape_is_named_by_key_and_field() {
    let malformed = |record: &str, field| CatalogReadError::MalformedRecord {
        record: record.into(),
        field,
    };

    let mut rows = fixture_rows();
    row_of(&mut rows, "faction-brannoch").payload =
        rmp_serde::to_vec(&json!("not an array")).unwrap();
    assert_eq!(read(&rows), Err(malformed("faction-brannoch", 0)));

    let mut rows = fixture_rows();
    edit(&mut rows, "faction-brannoch", |cells| {
        cells[1] = Value::Null
    });
    assert_eq!(read(&rows), Err(malformed("faction-brannoch", 1)));

    let mut rows = fixture_rows();
    edit(&mut rows, "faction-brannoch", |cells| {
        cells[12] = json!({"faction-adrasteia": "heavy"});
    });
    assert_eq!(read(&rows), Err(malformed("faction-brannoch", 12)));

    let mut rows = fixture_rows();
    edit(&mut rows, "faction-brannoch", |cells| cells.truncate(12));
    assert_eq!(read(&rows), Err(malformed("faction-brannoch", 12)));

    let mut rows = fixture_rows();
    let grammar = rows
        .iter()
        .find(|row| row.r#type == "aetheria.verse_grammar")
        .unwrap()
        .key
        .clone();
    edit(&mut rows, &grammar, |cells| cells[1] = json!(-1));
    assert_eq!(read(&rows), Err(malformed(&grammar, 1)));
}

#[test]
fn fields_aetheria_adds_and_schemas_it_does_not_mirror_are_ignored() {
    let mut rows = fixture_rows();
    edit(&mut rows, "verb-speak", |cells| cells.push(json!("later")));
    edit(&mut rows, "faction-adrasteia", |cells| cells.push(json!(7)));
    let mut stranger = rows[0].clone();
    stranger.key = "ship-1".into();
    stranger.r#type = "aetheria.ship".into();
    stranger.schema_id = Some("sha256:not-mirrored".into());
    stranger.payload = vec![0xc1];
    rows.push(stranger);

    assert_eq!(
        read(&rows).unwrap(),
        read_aetheria_catalog(FIXTURE).unwrap()
    );
}

#[test]
fn a_null_short_name_or_description_reads_as_empty() {
    let mut rows = fixture_rows();
    edit(&mut rows, "faction-adrasteia", |cells| {
        cells[2] = Value::Null;
        cells[3] = Value::Null;
    });
    let catalog = read(&rows).unwrap();
    assert_eq!(catalog.factions[0].short_name, "");
    assert_eq!(catalog.factions[0].description, "");
}

#[test]
fn an_absent_file_is_unreadable_not_an_empty_catalog() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        read_aetheria_catalog(dir.path().join("missing.cc")),
        Err(CatalogReadError::Unreadable)
    );
}

#[test]
fn reading_leaves_the_catalog_and_its_directory_untouched() {
    let (dir, path) = catalog_of(&fixture_rows());
    let bytes = std::fs::read(&path).unwrap();
    let listing = |dir: &std::path::Path| {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let before = listing(dir.path());
    read_aetheria_catalog(&path).unwrap();
    read_aetheria_catalog(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(listing(dir.path()), before, "the reader created a file");
}
