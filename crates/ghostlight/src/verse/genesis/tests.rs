use super::*;
use crate::aetheria::read_aetheria_catalog;
use crate::patch::{FactStanding, Mismatch};
use crate::tests::{auth_principal, owner, stock_weights};
use crate::{
    CommandBody, ControllerMode, GrammarBinding, GrammarReferentKind, GrammarRole, GrammarVerb,
    KernelError, PrincipalCommandIntent, RenderPath, VerifiedPrincipalEvidence, WorldKernel,
    WorldMailbox, WorldPhase,
};
use chrono::{Duration, Utc};
use std::collections::BTreeSet;

/// Aetheria master when the fixture catalog was written.
const COMMIT: &str = "92b82e9aa5bb422c568c4bfc85a8f641426a41fc";
const OTHER_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/aetheria-verse-catalog.cc"
);

fn catalog() -> AetheriaCatalog {
    read_aetheria_catalog(FIXTURE).unwrap()
}

fn intent_of(catalog: AetheriaCatalog, commit: &str) -> CreateVerseWorldIntent {
    CreateVerseWorldIntent {
        id: CommandId::new(),
        title: "The Verse".into(),
        brief: "Aetheria.".into(),
        catalog,
        aetheria_commit: commit.into(),
        lens_weights: stock_weights(),
        targets: BTreeMap::new(),
    }
}

/// A world created through the kernel from the lowering, for what the
/// snapshot hides: the facts' evidence.
fn kernel_of(intent: CreateVerseWorldIntent) -> (tempfile::TempDir, Result<WorldKernel, KernelError>) {
    let dir = tempfile::tempdir().unwrap();
    let world = intent.lower(owner()).unwrap();
    let result = WorldKernel::create(dir.path().join("world.cc"), world, &auth_principal(owner()))
        .map(|(kernel, _)| kernel);
    (dir, result)
}

struct Verse {
    _dir: tempfile::TempDir,
    _task: tokio::task::JoinHandle<()>,
    mailbox: WorldMailbox,
    principal: VerifiedPrincipalEvidence,
    receipt: crate::CreationReceipt,
}

async fn verse(intent: CreateVerseWorldIntent) -> Verse {
    let dir = tempfile::tempdir().unwrap();
    let (mailbox, task) = WorldMailbox::open(dir.path().join("world.cc")).unwrap();
    let principal =
        VerifiedPrincipalEvidence::fixture("verse-owner", Utc::now() + Duration::hours(1));
    let receipt = mailbox.create_verse(intent, &principal).await.unwrap();
    Verse {
        _dir: dir,
        _task: task,
        mailbox,
        principal,
        receipt,
    }
}

#[tokio::test]
async fn verse_genesis_from_the_aetheria_fixture() {
    let catalog = catalog();
    let verse = verse(intent_of(catalog.clone(), COMMIT)).await;
    let snapshot = verse.mailbox.snapshot().await.unwrap();

    // One institution per faction, keyed by its record, named by its Name.
    let mut factions: Vec<_> = snapshot
        .subjects
        .iter()
        .map(|subject| {
            (
                subject.import_key.clone().unwrap(),
                subject.label.clone(),
                subject.kind,
                subject.controller_mode,
            )
        })
        .collect();
    factions.sort_by(|a, b| a.0.cmp(&b.0));
    let expected: Vec<_> = catalog
        .factions
        .iter()
        .map(|faction| {
            (
                faction.key.clone(),
                faction.name.clone(),
                SubjectKind::Institution,
                Some(ControllerMode::OperationalAgent),
            )
        })
        .collect();
    assert_eq!(factions.len(), 2);
    assert_eq!(factions, expected);

    // One fact per faction, carrying its description.
    let statements: BTreeSet<_> = snapshot
        .facts
        .iter()
        .map(|fact| fact.statement.as_str().to_owned())
        .collect();
    assert_eq!(
        statements,
        catalog
            .factions
            .iter()
            .map(|faction| faction.description.clone())
            .collect::<BTreeSet<_>>()
    );

    // The world is bound to the catalog's grammar.
    assert_eq!(snapshot.grammar, Some(catalog.grammar.identity()));
    assert_eq!(verse.receipt.world_id, snapshot.world_id);

    // Every faction stands in the one commons.
    let commons: Vec<_> = snapshot
        .places
        .iter()
        .filter(|place| place.label == "The Commons")
        .collect();
    assert_eq!(commons.len(), 1);
    assert!(
        snapshot
            .subjects
            .iter()
            .all(|subject| subject.position == Some(commons[0].id))
    );
}

#[test]
fn each_fact_cites_its_faction_record_and_the_commit_it_was_read_at() {
    for commit in [COMMIT, OTHER_COMMIT] {
        let (_dir, kernel) = kernel_of(intent_of(catalog(), commit));
        let kernel = kernel.unwrap();
        let evidence: BTreeSet<String> = kernel
            .state
            .facts
            .values()
            .map(|fact| match &fact.standing {
                FactStanding::Canonical { evidence } => evidence.text().to_owned(),
                other => panic!("a verse fact is canonical, not {other:?}"),
            })
            .collect();
        assert_eq!(
            evidence,
            catalog()
                .factions
                .iter()
                .map(|faction| format!(
                    "aetheria:{commit}:aetheria.faction/{}",
                    faction.key.record()
                ))
                .collect::<BTreeSet<_>>()
        );
    }
}

#[test]
fn a_commit_is_forty_lower_case_hex_digits() {
    for bad in [
        "",
        &COMMIT[..39],
        &format!("{COMMIT}0"),
        &COMMIT.to_uppercase(),
        &format!("{}g", &COMMIT[..39]),
    ] {
        assert!(
            matches!(
                intent_of(catalog(), bad).lower(owner()),
                Err(VerseGenesisError::NotACommit)
            ),
            "{bad:?}"
        );
    }
    assert!(intent_of(catalog(), COMMIT).lower(owner()).is_ok());
    assert!(!VerseGenesisError::NotACommit.to_string().contains(COMMIT));
}

#[test]
fn verse_genesis_refuses_a_duplicate_faction_record() {
    let mut twice = catalog();
    let first = twice.factions[0].clone();
    twice.factions.push(first);
    // The lowering does not decide: it declares all three.
    let world = intent_of(twice.clone(), COMMIT).lower(owner()).unwrap();
    let subjects = world
        .patch
        .declarations
        .iter()
        .filter(|declaration| matches!(declaration, Declaration::Subject(_)))
        .count();
    assert_eq!(subjects, 3);

    let (_dir, kernel) = kernel_of(intent_of(twice, COMMIT));
    let Err(KernelError::PatchRejected(set)) = kernel else {
        panic!("a second faction under one record key was admitted");
    };
    assert!(
        set.iter()
            .any(|mismatch| matches!(mismatch, Mismatch::DuplicateImportKey { .. })),
        "{set:?}"
    );
}

#[tokio::test]
async fn every_granted_affordance_is_a_grammar_verb() {
    let catalog = catalog();
    let verse = verse(intent_of(catalog.clone(), COMMIT)).await;
    let snapshot = verse.mailbox.snapshot().await.unwrap();

    let mut kinds: Vec<_> = snapshot
        .affordances
        .iter()
        .map(|affordance| affordance.entry.kind.0.clone())
        .collect();
    kinds.sort();
    // Each verb once: the kernel's speak is the grammar's speak.
    assert_eq!(
        kinds,
        catalog
            .grammar
            .verbs()
            .map(|(kind, _)| kind.to_owned())
            .collect::<Vec<_>>()
    );
    assert!(kinds.contains(&"speak".to_owned()));

    // Each has its render path from the grammar, and phase A changes nothing.
    for affordance in &snapshot.affordances {
        let verb = catalog.grammar.verb(&affordance.entry.kind.0).unwrap();
        assert_eq!(affordance.render_path, Some(verb.render_path));
        assert!(affordance.entry.effect_slots.is_empty());
        assert_eq!(affordance.entry.outcome_bands.len(), 1);
        assert!(affordance.entry.outcome_bands[0].effects.is_empty());
    }

    // Every faction is granted every verb.
    let all: BTreeSet<_> = snapshot.affordances.iter().map(|a| a.id).collect();
    for subject in &snapshot.subjects {
        assert_eq!(subject.affordances, all);
    }
}

#[tokio::test]
async fn factions_get_opportunities() {
    let verse = verse(intent_of(catalog(), COMMIT)).await;
    for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
        let revision = verse.mailbox.snapshot().await.unwrap().revision;
        verse
            .mailbox
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: verse.receipt.world_id,
                    expected_revision: revision,
                    body,
                },
                &verse.principal,
            )
            .await
            .unwrap();
    }
    let snapshot = verse.mailbox.snapshot().await.unwrap();
    assert_eq!(snapshot.phase, WorldPhase::Active);
    assert_eq!(snapshot.opportunities.len(), catalog().factions.len());
    let scopes: BTreeSet<_> = snapshot.opportunities.iter().map(|o| o.scope).collect();
    assert_eq!(scopes.len(), snapshot.opportunities.len());
    for opportunity in &snapshot.opportunities {
        assert_eq!(opportunity.controller_mode, ControllerMode::OperationalAgent);
        assert_eq!(opportunity.affordance_ids.len(), snapshot.affordances.len());
    }
}

#[test]
fn a_faction_without_a_description_holds_no_fact() {
    let mut bare = catalog();
    bare.factions[0].description = "  ".into();
    bare.factions[1].description = "  Second.  ".into();
    let (_dir, kernel) = kernel_of(intent_of(bare, COMMIT));
    let facts: Vec<_> = kernel
        .unwrap()
        .state
        .facts
        .values()
        .map(|fact| fact.statement.as_str().to_owned())
        .collect();
    assert_eq!(facts, vec!["Second.".to_owned()]);
}

#[test]
fn a_grammar_the_kernel_speak_cannot_live_in_refuses_genesis() {
    let mut mute = catalog();
    mute.grammar = GrammarBinding::new(
        "aetheria.verse_grammar",
        1,
        [(
            "haul".to_owned(),
            GrammarVerb {
                render_path: RenderPath::ShipAction,
                roles: vec![GrammarRole {
                    role: "cargo".into(),
                    referent: GrammarReferentKind::Cargo,
                }],
            },
        )],
    )
    .unwrap();
    let (_dir, kernel) = kernel_of(intent_of(mute, COMMIT));
    let Err(KernelError::PatchRejected(set)) = kernel else {
        panic!("a world whose grammar has no speak was admitted");
    };
    assert!(
        set.iter()
            .any(|mismatch| matches!(mismatch, Mismatch::AffordanceKindNotInGrammar { .. })),
        "{set:?}"
    );
}

#[tokio::test]
async fn the_dungeon_ingress_still_creates_an_unbound_world() {
    let dir = tempfile::tempdir().unwrap();
    let (mailbox, _task) = WorldMailbox::open(dir.path().join("world.cc")).unwrap();
    let principal =
        VerifiedPrincipalEvidence::fixture("dungeon-owner", Utc::now() + Duration::hours(1));
    mailbox
        .create(
            crate::CreateWorldIntent {
                id: CommandId::new(),
                title: "Dungeon".into(),
                brief: String::new(),
                human_subject_label: "Player".into(),
                narrative_persona_label: None,
                operational_agent_label: None,
                targets: BTreeMap::new(),
                jurisdictions: Vec::new(),
                lens_weights: stock_weights(),
            },
            &principal,
        )
        .await
        .unwrap();
    let snapshot = mailbox.snapshot().await.unwrap();
    assert_eq!(snapshot.grammar, None);
    assert!(
        snapshot
            .places
            .iter()
            .any(|place| place.label == "The Commons")
    );
}
