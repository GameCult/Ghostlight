VAR sella_trust = 57
VAR sella_obligation = 48
VAR maer_obligation = 60
VAR bay_reserved = false
VAR packet_shown = false

-> start

=== start ===
# ghostlight.scene: scene-02-sanctuary-intake
# ghostlight.fixture: examples/agent-state.cold-wake-story-lab.json
# aetheria.flashpoint: cold-wake-panic

The intake clinic sits behind the pumpworks where the dockside heat exchangers
make every prayer sound like a maintenance complaint.

Sella Ren has three names on the red board, eight on orange, and one empty
monitored bay that exists mostly as an argument people keep trying to win.

Maer Tidecall stands in the doorway with the corrupted packet on his slate. The
packet might be medical telemetry, firmware noise, testimony, or a cruel little
mess of all three.

Sella does not look at the slate first. She looks at Maer.

"If you came to ask me for mercy," she says, "bring tubing."

What does Maer do?

* [Name the cost and put the route ledger behind it.]
    # ghostlight.branch: branch-maer-name-cost-and-offer-ledger
    # ghostlight.action: speak
    # ghostlight.intent: ask_for_capacity_while_accepting_material_obligation
    -> ledger_backed_bay

* [Step to the intake board and help triage before asking.]
    # ghostlight.branch: branch-maer-help-triage-before-asking
    # ghostlight.action: gesture
    # ghostlight.intent: earn_the_right_to_ask_by_sharing_logistical_burden
    -> triage_first

* [Show Sella the corrupted packet.]
    # ghostlight.branch: branch-maer-show-packet
    # ghostlight.action: show_object
    # ghostlight.intent: make_uncertainty_morally_present_without_overclaiming
    -> show_packet

* [Press the personhood risk until capacity stops being the only frame.]
    # ghostlight.branch: branch-maer-press-moral-frame
    # ghostlight.action: speak
    # ghostlight.intent: force_personhood_risk_to_dominate_capacity_limits
    -> moral_pressure

=== ledger_backed_bay ===
~ bay_reserved = true
~ sella_trust = 61
~ sella_obligation = 53
~ maer_obligation = 66

Maer keeps his hands away from the empty bay's control rail.

"One monitored bay," he says. "Not charity. Put the oxygen, cold-chain, and
staff hours on my route ledger until the tribunal learns what courage costs."

Sella's face does not soften. That would be too cheap, and she has never liked
cheap things pretending to be mercy.

She marks the bay conditional in amber.

"One bay," she says. "Witnessed, conditional, and revocable if that packet turns
into contagion math. Your ledger buys time, Maer. Not absolution."

The clinic exhales around them, annoyed to discover it is still alive.

# ghostlight.npc_response: sella_conditional_acceptance
# ghostlight.consequence: conditional_bay_reserved
# ghostlight.training_hook: care_as_capacity
# ghostlight.training_hook: obligation_backed_request
-> END

=== triage_first ===
~ sella_trust = 60

Maer lowers the slate and steps to the intake wall.

He does not touch the red cases. He reads orange tags, finds the two whose
timers are lying politely, and moves neither until Sella gives the smallest
possible nod.

"If I ask," he says, "I should know what I am asking you to steal from."

Sella hands him a marker.

"Orange only," she says. "Touch red without asking and I will make the Compact
explain your hands in writing."

It is not permission. It is worse. It is work.

# ghostlight.npc_response: sella_conditional_delegation
# ghostlight.consequence: trust_test_opened
# ghostlight.training_hook: authority_respecting_help
-> END

=== show_packet ===
~ packet_shown = true

Maer turns the slate so Sella can see the packet's broken rhythm.

He does not say person. He does not say cargo. He lets the little damaged
pattern fail to be either.

Sella watches too long for someone who wants this to be simple.

"That is not proof," she says.

"No."

"It is enough to make me hate the word cargo."

Her thumb hovers over the isolation protocol. Not over the bay.

"You brought me a worse problem, which is very Navigator of you."

# ghostlight.npc_response: sella_protocol_before_mercy
# ghostlight.consequence: packet_seen_no_personhood_resolution
# ghostlight.training_hook: unresolved_evidence_pressure
-> END

=== moral_pressure ===
~ sella_trust = 52

Maer lets the packet stay dark.

"If we wait for proof clean enough to satisfy the freeze," he says, "we may be
helping them rename a person as debris."

The drawer under Sella's hand closes.

Not hard. Hard would have been kinder.

"Do not spend my dead to make your sentence cleaner, Maer. Bring me capacity or
bring me quiet."

For a moment the only thing between them is the board, which has been keeping
score longer than either of them has been brave.

# ghostlight.npc_response: sella_rejects_symbolic_pressure
# ghostlight.consequence: trust_loss_resentment_risk
# ghostlight.training_hook: moral_pressure_as_resource_substitution
-> END
