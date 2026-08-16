use crate::domain::WorldActionProposal;

pub fn winner(proposals: &[WorldActionProposal]) -> Option<WorldActionProposal> {
    proposals.iter().cloned().max_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| right.actor_id.cmp(&left.actor_id))
            .then_with(|| right.intent.cmp(&left.intent))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(actor: &str, priority: i16) -> WorldActionProposal {
        WorldActionProposal {
            actor_id: actor.into(),
            intent: "act".into(),
            intended_effect: "change something".into(),
            priority,
            state_references: vec![],
        }
    }

    #[test]
    fn priority_wins_with_stable_actor_tie_break() {
        let proposals = vec![
            proposal("bert", 5),
            proposal("anna", 5),
            proposal("cara", 4),
        ];
        assert_eq!(winner(&proposals).unwrap().actor_id, "anna");
    }
}
