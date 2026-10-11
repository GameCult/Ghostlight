//! The canonical Aetheria verse world's genesis.
//!
//! One intent lowers to one `CreateWorld`: a commons, every grammar verb
//! declared as an affordance, and each Aetheria faction as an institution
//! imported by its record key. The world is bound to the catalog's grammar, so
//! the kernel refuses any affordance the grammar does not carry.
//!
//! Phase A verbs record the act and change no component: an outcome band with
//! no effects, and the act carried as the utterance that makes an entry
//! admissible. Ghostlight owns each verb's lowering to kernel effects (ruling
//! `verb-effects-ghostlight`), and those arrive with phase B.

use crate::aetheria::AetheriaCatalog;
use crate::mailbox::{MailboxError, commons_declaration, commons_ref};
use crate::patch::{
    AffordanceDeclaration, AffordanceKindName, AudienceSpec, Declaration, DraftHandle, EvidenceRef,
    FactDeclaration, FactStandingRef, OutcomeBand, Precondition, Ref, Statement, SubjectDeclaration,
    kernel_speak_entry, kernel_speak_grant,
};
use crate::{
    CommandId, CreateWorld, LensWeights, NewController, PrincipalId, SubjectKind,
    WorldPatch, WorldScaleIntentRef,
};
use std::collections::BTreeMap;

const COMMIT_LEN: usize = 40;

/// What a caller states to create the canonical verse world. Everything else a
/// world needs (the commons, the subjects, the affordances, their grants and
/// the provenance of what is known) is derived from the catalog.
#[derive(Clone, Debug)]
pub struct CreateVerseWorldIntent {
    pub id: CommandId,
    pub title: String,
    pub brief: String,
    pub catalog: AetheriaCatalog,
    /// The Aetheria commit the catalog was read at: 40 lower-case hex digits.
    /// It is the provenance of every fact the world holds about a faction.
    pub aetheria_commit: String,
    pub lens_weights: LensWeights,
    pub targets: BTreeMap<SubjectKind, u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum VerseGenesisError {
    #[error("aetheria_commit is not 40 lower-case hex digits")]
    NotACommit,
    #[error(transparent)]
    Mailbox(#[from] MailboxError),
}

impl CreateVerseWorldIntent {
    /// The one lowering. The kernel supplies the `speak` entry at genesis and
    /// holds it to the grammar, so it is granted here and never declared.
    pub(crate) fn lower(self, owner: PrincipalId) -> Result<CreateWorld, VerseGenesisError> {
        let commit = &self.aetheria_commit;
        if commit.len() != COMMIT_LEN
            || !commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(VerseGenesisError::NotACommit);
        }

        let speak = kernel_speak_entry().kind;
        let mut grants = kernel_speak_grant();
        let mut declarations = vec![commons_declaration()];
        for (kind, verb) in self.catalog.grammar.verbs() {
            if kind == speak.0 {
                continue;
            }
            let handle = DraftHandle::new(format!("verb-{kind}"));
            grants.insert(Ref::Draft(handle.clone()));
            declarations.push(Declaration::Affordance(AffordanceDeclaration {
                handle,
                kind: AffordanceKindName(kind.to_owned()),
                roles: verb.role_specs(),
                preconditions: vec![Precondition::CanBroadcast {
                    via: AudienceSpec::Colocated,
                }],
                effect_slots: Vec::new(),
                outcome_bands: vec![OutcomeBand {
                    weight: 1,
                    effects: Vec::new(),
                }],
                // The kernel refuses an entry that can neither change a
                // component nor speak, and a speaking entry needs an audience,
                // so a phase A verb is its own utterance to the room: the act
                // is recorded as speech and nothing else changes.
                carries_speech: true,
            }));
        }

        let mut evidence = Vec::new();
        for (position, faction) in self.catalog.factions.iter().enumerate() {
            declarations.push(Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(format!("faction-{position}")),
                label: faction.name.trim().to_owned(),
                kind: SubjectKind::Institution,
                controller: NewController::OperationalAgent,
                affordances: grants.clone(),
                position: Some(commons_ref()),
                import_key: Some(faction.key.clone()),
            }));
            // A faction with no description has nothing to state.
            if let Some(statement) = Statement::new(faction.description.trim()) {
                let receipt = EvidenceRef::new(format!(
                    "aetheria:{commit}:{}/{}",
                    faction.key.schema(),
                    faction.key.record()
                ));
                evidence.push(receipt.clone());
                declarations.push(Declaration::Fact(FactDeclaration {
                    handle: DraftHandle::new(format!("fact-{position}")),
                    label: faction.name.trim().to_owned(),
                    statement,
                    standing: FactStandingRef::Canonical { evidence: receipt },
                }));
            }
        }

        Ok(CreateWorld {
            id: self.id,
            owner,
            title: self.title,
            brief: self.brief,
            patch: WorldPatch {
                declarations,
                operations: Vec::new(),
                evidence,
            },
            scale_intent: WorldScaleIntentRef {
                targets: self.targets,
                jurisdictions: BTreeMap::new(),
            },
            lens_weights: self.lens_weights,
            grammar: Some(self.catalog.grammar),
        })
    }
}

#[cfg(test)]
mod tests;
