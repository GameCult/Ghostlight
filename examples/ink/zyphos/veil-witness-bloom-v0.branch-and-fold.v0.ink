// ghostlight.artifact_id: veil_witness_bloom_v0_branch_fold
// ghostlight.fixture_id: veil-witness-bloom-v0
// ghostlight.scene_id: veil-witness-bloom-v0.sluice-nine-return-processing
// ghostlight.final_ink_path: examples/ink/zyphos/veil-witness-bloom-v0.branch-and-fold.v0.ink

VAR witness_clarity = 1
VAR flower_integrity = 2
VAR host_safety = 2
VAR worker_cover = 1
VAR ecology_trust = 1
VAR record_contradiction = 0
VAR checkpoint_suspicion = 1
VAR route_exposure = 0
VAR seed_custody = 0
VAR mat_testimony = 0

-> start

=== start ===
Sluice Nine begins its shift after eclipse egress, when returning bodies are warm enough to sweat and the forms are still pretending chemistry has office hours.

The checkpoint is a narrow grown-root hall at the inner edge of the Airawa Empire. The arrival arch opens to the old forest road. The doctrine arch opens toward imperial streets. Between them, a low rinse dais crosses a slotted floor. A controlled candle fungal road waits beneath the arrival-side grate; a strip of prismwake mat borders the drain; the incinerator throat sits beside the doctrine arch. Above it all, a translucent supervisor membrane watches the hall with a pale, lidless shimmer.

Sivren braces one clawed upper hand against a root rib and uses both smaller lower hands to line up mineral cups, grooming combs, record nodules, and disposal cloths. Their digitigrade feet grip the damp floor seams. Four limbs for work, two for staying attached to the state while it improves you.

-> routine_roles

=== routine_roles ===
Sivren is a return-processing groomer. The work is simple when described by someone who does not do it: remove unmanaged symbionts, compare body chemistry with the official nodule, and send the citizen through the doctrine arch clean.

Supervisor Nahl stands behind the membrane on the doctrine side, grown plate sections polished, lower hands folded over the day's treatment ledger. Nahl calls burden-flower displays parasite sickness. This is official language, which means it has survived more bodies than argument.

No returner waits at the arrival arch yet. Sivren has time for one ordinary kindness or one ordinary precaution.

-> routine_hub

=== routine_hub ===
// ghostlight.choice_layer: routine_preparation
+ [Give the waiting mineral cups a clean salt charge for any flower that arrives hungry.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_flower_feed
    ~ witness_clarity = witness_clarity + 2
    ~ flower_integrity = flower_integrity + 1
    ~ checkpoint_suspicion = checkpoint_suspicion + 1
    Sivren tips measured salt into the mineral cups and stirs with one blunt lower digit.

    The supervisor membrane brightens. Generosity toward unmanaged parasites is not forbidden. It is merely recorded with unusually good penmanship.
    -> routine_fold
+ [Clean the prismwake test strip until its old visitor-colors separate again.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: prime_mat_witness
    ~ mat_testimony = mat_testimony + 1
    ~ ecology_trust = ecology_trust + 1
    Sivren kneels their long digitigrade legs and works mineral water through the living strip with both lower hands.

    The mat answers with a restrained silver-blue pulse. It has not forgiven yesterday's boot traffic. It has agreed to distinguish it from today's.
    -> routine_fold
+ [Feed clean peelings through the arrival grate before the fungal road has to ask.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_road_credit
    ~ ecology_trust = ecology_trust + 2
    ~ route_exposure = route_exposure + 1
    Sivren pushes the shift's clean peelings through the grate. Amber fungal beads open below, one by one, tasting the gift and the hand that offered it.

    Nahl marks the exchange. Imperial roads are not supposed to bargain. Old roads remain poor readers of policy.
    -> routine_fold
+ [Pre-file the clean-shift line before the returners arrive.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_worker_cover
    ~ worker_cover = worker_cover + 2
    ~ checkpoint_suspicion = checkpoint_suspicion - 1
    Sivren presses the blank nodule to their wrist seam and records the expected result: no contamination, no contradiction, no delay.

    The line accepts it. Prediction is the empire's favorite kind of evidence.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_disclosure
The arrival arch opens.

Oru steps in from the old road, a returned line-grafter with travel mud between hooked foot-talons and an imperial treatment nodule warm against the throat plates. Their two upper arms carry a sealed tool sling. Their smaller lower hands are empty and held where Nahl can see them.

Sivren knows Oru from three seasons of checkpoint jokes and one winter when the drain froze shut. Oru used to complain that Sivren's grooming combs were cold. Today they look at Sivren with the courteous blankness reserved for a worker encountered for the first time.

The nodule reports successful correction after foreign-root exposure. No forbidden contact retained. No fear response. No recognition hazard.

{worker_cover >= 3: Sivren's pre-filed clean line waits in the ledger, a small bureaucratic roof with room for exactly one lie.}
{ecology_trust >= 3: Beneath the grate, the old fungal road holds its amber beads open toward Sivren.}
{mat_testimony >= 1: The cleaned prismwake strip lies silver-blue beside the drain, ready to price the next claim.}

-> mandatory_rinse

=== mandatory_rinse ===
Sivren opens Oru's travel sling and performs the mandatory mineral rinse.

A hand-sized burden flower uncurls from beneath Oru's shoulder plate. Its clasping roots have been flattened into the seam. Yellow runs across its leaves for fear. Violet follows for recognition. Then every color snaps pale at once, the bodily shape of something ordered quiet.

Oru watches the flower as if it belongs to someone else.

Nahl's membrane turns from pearl to inspection white.

-> bloom_choice

=== bloom_choice ===
// ghostlight.choice_layer: witness_bloom_response
+ [Offer the flower the charged mineral cup and let the old chemistry finish speaking.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: clarify_witness_bloom
    ~ witness_clarity = witness_clarity + 1
    ~ flower_integrity = flower_integrity + 1
    ~ host_safety = host_safety - 1
    ~ checkpoint_suspicion = checkpoint_suspicion + 1
    Sivren sets the cup under the sensory filaments. The flower drinks.

    Yellow. Violet. Pale silence. Again, more cleanly.

    Oru's throat seams remain dark. Their body has obeyed the edit. The flower's body has not.
    -> supervisor_arrival
+ [Cup the flower beneath a lower hand and let one seed bead catch in the grooming cloth.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: shelter_flower_trace
    ~ seed_custody = seed_custody + 1
    ~ flower_integrity = flower_integrity + 1
    ~ worker_cover = worker_cover - 1
    Sivren folds a disposal cloth around the flower with two soft lower hands. A black seed bead sticks in the mineral nap.

    From above, the gesture can still be called containment. From inside the cloth, the roots tighten around a witness nobody has yet agreed to hear.
    -> supervisor_arrival
+ [Press the rinse runoff onto the prismwake strip and ask the mat to keep its own account.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: create_mat_testimony
    ~ mat_testimony = mat_testimony + 2
    ~ record_contradiction = record_contradiction + 1
    ~ checkpoint_suspicion = checkpoint_suspicion + 1
    Sivren drags two lower fingertips through the runoff and touches the living strip.

    The mat flashes Oru's present calm in blue-white, the flower's stored fear in yellow, and a hard violet edge where the states fail to agree. It does not accuse. It invoices the contradiction to everyone watching.
    -> supervisor_arrival
+ [Ask Oru whether they remember being afraid on the old road.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: ask_edited_host
    ~ record_contradiction = record_contradiction + 1
    ~ witness_clarity = witness_clarity + 1
    ~ host_safety = host_safety - 1
    "Do you remember fear?" Sivren asks.

    Oru glances at the flower, the nodule, then Sivren. "I remember being treated for an irrational response."

    Their upper claws tighten against the tool sling. The body supplies a footnote the sentence has been trained not to need.
    -> supervisor_arrival

=== supervisor_arrival ===
// ghostlight.fold: supervisor_pressure_after_bloom
The doctrine arch opens behind the incinerator. Nahl enters at floor level, upper claws resting lightly on the root ribs, lower hands already separating a disposal seal from its backing.

"Residual parasite display," Nahl says. "Remove it. Admit the corrected body."

The order is neat because it divides Oru from the evidence growing out of Oru's plate seam.

{witness_clarity >= 4: The flower repeats fear, recognition, and imposed quiet in a sequence precise enough to survive honest comparison.}
{witness_clarity <= 2: The display stutters. Hunger, old chemistry, and treatment damage remain tangled.}
{mat_testimony >= 2: The prismwake strip holds a violet-edged mismatch beside Oru's official calm.}
{seed_custody >= 1: A black seed bead waits inside Sivren's damp grooming cloth.}
{checkpoint_suspicion >= 3: The supervisor membrane seals the arrival arch. The checkpoint has begun preserving its version of events.}

-> pressure_choice

=== pressure_choice ===
// ghostlight.choice_layer: disclosure_under_supervision
+ [Repeat "parasite sickness" while palming the seed bead under a wrist plate.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: mask_and_keep_seed
    ~ worker_cover = worker_cover + 1
    ~ seed_custody = seed_custody + 1
    ~ flower_integrity = flower_integrity - 1
    "Parasite sickness," Sivren says.

    Their lower hand performs compliance in public and custody in miniature. The seed bead slips under a wrist plate. The flower loses a leaf to the disposal seal.

    Nahl hears agreement. Oru sees only the hand.
    -> disclosure_threshold
+ {mat_testimony >= 2} [Invoke Sluice Nine's two-body hold and make Nahl answer the mat.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: force_official_comparison
    ~ record_contradiction = record_contradiction + 2
    ~ host_safety = host_safety + 1
    ~ checkpoint_suspicion = checkpoint_suspicion + 1
    "The flower and the mat disagree with the nodule," Sivren says. "Two living reports. Sluice Nine holds the record."

    Nahl turns one polished plate toward the prismwake strip. The pause is brief. It is still the first thing in the room the official story did not schedule.
    -> disclosure_threshold
+ {ecology_trust >= 3} [Drop a shed leaf through the grate and let the old fungal road carry the mismatch.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: seed_chemical_rumor
    ~ route_exposure = route_exposure + 2
    ~ seed_custody = seed_custody + 1
    ~ flower_integrity = flower_integrity - 1
    Sivren loosens one shed leaf and lets it fall through the arrival grate.

    The fungal beads close over it. Amber light runs three body-lengths toward the old road, then stops. A message has not been delivered. A hungry route has accepted material worth remembering.

    Nahl looks down. Old infrastructure has poor discretion and excellent jurisdictional timing.
    -> disclosure_threshold
+ [Seal the whole flower and feed it to the incinerator throat.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: obey_flower_destruction
    ~ flower_integrity = 0
    ~ seed_custody = 0
    ~ record_contradiction = 0
    ~ worker_cover = worker_cover + 2
    ~ host_safety = host_safety + 1
    Sivren closes the disposal cloth, presses it into the incinerator throat, and holds the seal until the roots stop moving.

    The flower burns sweet, then bitter. Oru is now easier to admit. Sivren is now easier to keep.
    -> disclosure_threshold

=== disclosure_threshold ===
// ghostlight.fold: final_memetic_sovereignty_choice
Nahl places the treatment ledger on the rinse dais.

"Choose the record," the supervisor says.

This is how the empire describes choices made after it has closed one arch and warmed the incinerator.

{flower_integrity <= 0: The disposal cloth is ash. No living display remains in the hall.}
{flower_integrity >= 3: The burden flower still grips Oru's plate seam, damaged but able to bloom again.}
{record_contradiction >= 3: The nodule, host response, and mat account no longer fit inside one clean line.}
{route_exposure >= 2: Amber beads beneath the grate carry the taste of the incident toward the old road. Any disclosure there will also disclose the route that received it.}
{worker_cover >= 3: Sivren's clean-shift line and obedient language could carry one hidden object through the doctrine arch.}
{checkpoint_suspicion >= 4: Nahl stands close enough to see each lower hand and every plate seam.}

The flower never held the missing thought. It held the bruise around it. Sivren can decide which body, record, or route must carry that bruise next.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: disclosure_path
+ [Quarantine Oru and the living flower together outside the arrival arch.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prioritize_living_witness
    {witness_clarity >= 3 && flower_integrity >= 2 && host_safety >= 2:
        Sivren writes "unresolved body mismatch" and presses the quarantine marker into the arrivalward rim of the rinse dais.
        -> ending_living_witness_success
    - else:
        Sivren reaches for the quarantine control with evidence too damaged, too hungry, or too publicly handled to protect its host.
        -> ending_living_witness_cost
    }
+ [Carry the seed beneath your own plate, file parasite sickness, and admit Oru.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: prioritize_body_courier
    {seed_custody >= 2 && worker_cover >= 3:
        Sivren seals the clean line and steps through the doctrine arch with one wrist held carefully still.
        -> ending_body_courier_success
    - else:
        Sivren tries to make one body carry two incompatible records under Nahl's open inspection.
        -> ending_body_courier_cost
    }
+ [Bind the prismwake mismatch to the official treatment record.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_official_contradiction
    {mat_testimony >= 2 && record_contradiction >= 3:
        Sivren presses the treatment nodule into the violet-edged mat trace before Nahl can separate them.
        -> ending_record_success
    - else:
        Sivren offers the ledger a contradiction with too few bodies behind it.
        -> ending_record_cost
    }
+ [Give Nahl the clean line and send Oru home without the flower.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: prioritize_host_over_disclosure
    {flower_integrity <= 0 && host_safety >= 3 && worker_cover >= 3:
        Sivren signs the admission.
        -> ending_silence_success
    - else:
        Sivren tries to close the incident while some visible part of it remains alive.
        -> ending_silence_cost
    }

=== ending_living_witness_success ===
// ghostlight.ending_label: living_witness_preserved
// ghostlight.training_hook: disclosure_preserves_uncertainty_and_body_cost
The arrival arch opens onto the old road. Oru steps back through it carrying the flower that remembers their fear better than they do.

They are not freed by this. They are delayed, marked, and barred from the imperial streets until several independent ecologies agree on what the bloom means.

{route_exposure >= 2: The fungal road already knows the taste. Its amber beads open farther than discretion would prefer.}
{route_exposure < 2: The road receives Oru as a new claim, not a familiar one, and charges the delay in darkness.}

Sivren has protected a witness by giving it time and other witnesses. The cost walks beside it on six limbs.
-> END

=== ending_living_witness_cost ===
// ghostlight.ending_label: living_witness_compromised
// ghostlight.training_hook: weak_bloom_becomes_exile_risk
Nahl accepts quarantine too quickly.

The flower's broken display becomes proof of contamination instead of proof of editing. Oru is escorted back toward the old road with a sealed treatment mark and no authority allowed to call the difference coercion.

{checkpoint_suspicion >= 4: Sivren is kept inside the checkpoint while the supervisor membrane records every plate seam.}

A damaged witness survives. So does the empire's preferred explanation of it.
-> END

=== ending_body_courier_success ===
// ghostlight.ending_label: embodied_rumor_carried
// ghostlight.training_hook: disclosure_through_a_second_body
Oru passes through the doctrine arch, officially calm and privately absent from their own alarm.

Sivren follows at shift end. Under the wrist plate, the seed bead tastes new sweat and keeps the old mismatch badly, incompletely, alive.

{route_exposure >= 2: The old road also carries a shed-leaf trace. Two routes now hold pieces, and either can expose the other.}
{route_exposure < 2: Sivren's body is the only route. A single accident, grooming order, or hungry bloom can end it.}

The secret has not been recovered. Chemical rumor has acquired legs.
-> END

=== ending_body_courier_cost ===
// ghostlight.ending_label: embodied_rumor_seized
// ghostlight.training_hook: concealment_fails_under_body_inspection
Nahl catches Sivren's careful wrist before the doctrine arch.

The supervisor braces one upper claw on the root rib beside Sivren and uses a smaller lower hand to peel back the plate seam. The seed bead is small, black, and much too interested in the air.

Oru is admitted for a second treatment. Sivren is retained for one. The checkpoint makes two bodies quieter and files one object under parasite control.
-> END

=== ending_record_success ===
// ghostlight.ending_label: official_record_contradicted
// ghostlight.training_hook: plural_body_testimony_enters_imperial_archive
The nodule takes the mat's violet edge.

Nahl can still label it parasite sickness. The ledger can no longer say no contradiction was observed. Somewhere deeper in the imperial archive, a clean treatment now has an attached body that refused to agree.

Oru watches their own calm become disputed evidence.

It is a narrow disclosure. Narrow things enter systems through seams.
-> END

=== ending_record_cost ===
// ghostlight.ending_label: official_record_absorbs_contradiction
// ghostlight.training_hook: isolated_testimony_reclassified_as_noise
The ledger tastes the weak mismatch and classifies it before Sivren lifts their fingers.

Parasite residue. Groomer hesitation. No host-memory relevance.

{mat_testimony >= 2: The prismwake strip keeps its own account beside the drain, but the official nodule does not link to it.}
{mat_testimony < 2: Even the mat has too little trace to price the lie above ordinary traffic.}

The empire does not erase every contradiction. Sometimes it gives one a smaller name and lets everyone get tired.
-> END

=== ending_silence_success ===
// ghostlight.ending_label: host_admitted_witness_destroyed
// ghostlight.training_hook: immediate_safety_purchased_with_knowledge_loss
Oru passes through the doctrine arch.

No quarantine mark follows. No seed rides under Sivren's plate. No road carries a shed leaf. The treatment record remains clean.

Oru pauses beside Sivren and says, politely, "First time on Sluice Nine?"

Sivren says yes.

One body is safer tonight. The question that body could have asked is ash.
-> END

=== ending_silence_cost ===
// ghostlight.ending_label: silence_fails_to_protect_host
// ghostlight.training_hook: destroyed_evidence_does_not_end_suspicion
Nahl sees the living leaf, the mat edge, or Sivren's guarded hand—some remnant that refuses a clean line.

The supervisor rejects admission and orders Oru back onto the rinse dais. Without the intact flower, nobody can compare the next treatment to the state before it.

Sivren has destroyed the witness and failed to protect the host.

The incinerator remains warm. The forms remain admirably complete.
-> END
