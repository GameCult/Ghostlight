// ghostlight.artifact_id: veil_revision_scars_branch_fold_v0
// ghostlight.fixture_id: veil-revision-scars-v0
// ghostlight.scene_id: veil-revision-scars-v0.ninth-root-recertification
// ghostlight.final_ink_path: examples/ink/zyphos/veil-revision-scars-v0.branch-and-fold.v0.ink
// ghostlight.tonal_mode: workplace comedy with creeping biological dread
// ghostlight.name_status: fixture-local names and the locality label are provisional translations pending Weksa

VAR witness_integrity = 1
VAR court_suspicion = 1
VAR oru_trust = 2
VAR body_strain = 0
VAR fragment_strength = 0
VAR road_access = 1
VAR compliance_cover = 2
VAR flower_custody = 1

-> start

=== start ===
The Ninth Root Revision Court is called a court because "room grown around a captive matriarch root where citizens queue to have yesterday agree with policy" would not fit on the scent seals.

At the outer edge of the Airawa Empire, the court hangs vertically around one pale root column. Climbing ridges run up the inner wall. Three grooming ledges face a translucent recertification membrane. Below them, a candle fungal road ends at a black quarantine ring. Beyond the membrane wait the archive socket and a shallow silver repair basin cultured with mirror amoebae.

Umbros hangs fixed beyond the high vents. Eclipse ingress has turned the court's bioluminescent seams from blue to cold green.

-> routine_people

=== routine_people ===
Nara works the middle grooming ledge. They are an imperial Airawa records tender: taloned feet and two clawed upper hands anchored to the root ridges, two smaller lower hands free to comb a burden flower on their chest harness. The flower drinks mineral wash, inspects Nara's sweat, and makes professional judgments in purple.

Oru tends the next ledge, close enough to trade tools and far enough that falling would remain an individual administrative error. Their own burden flower is a well-fed yellow nuisance.

"Mine says I resent morning recertification," Oru says.

"Yours says that every morning."

"Consistency is a civic virtue."

-> routine_stakes

=== routine_stakes ===
At shift end, Adjudicator Vey will enter through the upper inspection membrane and certify the records crew. Vey does not decide what happened. The archive has already performed that vulgar labor. Vey decides whether each body agrees safely enough to keep working.

The routine is simple. Groom the flowers. Feed shed scales to the lattice-ant assay tray. Cross the recertification membrane. Touch the archive socket. Report calm.

Nara remembers returning from yesterday's patrol by the east fungal road. No injuries. No missing workers. No contact with a forest, because there are no disconnected forests inside the border and nothing outside the border has standing to count.

Their burden flower opens one purple petal, then a second. Between them it leaves a precise colorless gap.

Oru stops smiling.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: routine_grooming
+ [Feed the flower a full mineral spoon and tuck it under the standardized mantle.]
    // ghostlight.branch: preserve_flower_witness
    // ghostlight.branch_label: preserve_flower_witness
    // ghostlight.action: use_object
    // ghostlight.action_label: use_object
    // ghostlight.intent: Keep the flower alive and its testimony private until Nara understands the risk.
    ~ witness_integrity = witness_integrity + 2
    ~ court_suspicion = court_suspicion + 1
    ~ compliance_cover = compliance_cover - 1
    ~ oru_trust = oru_trust + 1
    Nara measures a full spoon instead of the regulation half. The flower drinks greedily and clamps beneath the mantle, where its rootlets tap Nara's sternum like a clerk asking to see the original.

    Oru notices the missing spoonful. They say nothing, which is a form of speech the court has not yet managed to tax.
    -> routine_fold
+ [Prune the colorless gap into the approved compost cup.]
    // ghostlight.branch: prune_as_protocol
    // ghostlight.branch_label: prune_as_protocol
    // ghostlight.action: groom
    // ghostlight.action_label: groom
    // ghostlight.intent: Preserve employment and treat the bloom as an ordinary grooming defect.
    ~ witness_integrity = witness_integrity - 1
    ~ compliance_cover = compliance_cover + 2
    ~ court_suspicion = court_suspicion - 1
    Nara takes the curved grooming blade in a lower hand and cuts exactly where the handbook would place the error.

    The petal falls into the compost cup. The flower tightens every rootlet. It does not have a concept of censorship. It has the simpler concept of being hungry while someone removes the profitable part.

    "Very civic," Oru says.
    -> routine_fold
+ [Press Nara's flower against Oru's and compare what both bodies report.]
    // ghostlight.branch: compare_living_witnesses
    // ghostlight.branch_label: compare_living_witnesses
    // ghostlight.action: touch_object
    // ghostlight.action_label: touch_object
    // ghostlight.intent: Test whether the gap belongs to one unreliable flower or to shared patrol exposure.
    ~ witness_integrity = witness_integrity + 1
    ~ oru_trust = oru_trust + 2
    ~ court_suspicion = court_suspicion + 1
    Nara braces with both upper hands and leans across the ledge. The flowers touch sensory filaments.

    Oru's yellow nuisance flashes the same colorless gap.

    "That seems less civic," Oru says.

    Neither of them says yesterday.
    -> routine_fold
+ [Carry the compost cup down to the fungal-road candles before recertification.]
    // ghostlight.branch: pay_the_road
    // ghostlight.branch_label: pay_the_road
    // ghostlight.action: move_object
    // ghostlight.action_label: move_object
    // ghostlight.intent: Buy route goodwill with clean organic material before the court seals for inspection.
    ~ road_access = road_access + 2
    ~ compliance_cover = compliance_cover + 1
    ~ body_strain = body_strain + 1
    Nara climbs down headfirst, upper claws and taloned feet taking the weight while one lower hand keeps the compost cup level. The candle road's fruiting beads brighten amber when the offering crosses the quarantine ring.

    Nara's stomach knots at the scent.

    There is no remembered reason for that, so the body is plainly being difficult on government time.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_grooming_before_pressure
The shift queue gathers on the lower ledge: archive clerks, graft tenders, two pollinator-route inspectors, all wearing standardized mantles and burden flowers groomed into discretion.

{witness_integrity >= 3: Beneath Nara's mantle, the fed flower holds the colorless gap open and warm against the chest plates.}
{witness_integrity <= 0: Nara's pruned flower shows obedient purple. Its rootlets have gone bitter.}
{oru_trust >= 4: Oru shifts one climbing foot onto Nara's ridge, close enough to catch a fall or receive an object without making either intention public.}
{road_access >= 3: Below, the paid fungal-road candles keep one amber lane open through the quarantine ring.}
{body_strain >= 1: The descent has left Nara's lower right hand trembling around the grooming comb.}

The recertification membrane inhales.

-> recertification_rupture

=== recertification_rupture ===
The membrane exhales the court's maintenance scent: crushed green resin, warm iron, and the calm conviction that yesterday contained no forest.

Nara's conscious memory agrees.

Their climbing talons lock hard enough to score the root ridge. The lower right hand closes around empty air as if it once held another wrist. Fine blue light races through the seams of Nara's chest plates and stops beneath the flower's colorless gap.

In the assay tray, lattice ants assemble around Nara's shed scale. They form a broken ring, dismantle it, and form it again.

The candle road below goes dark except for one uncertain bead.

-> rupture_witness

=== rupture_witness ===
Oru says, very softly, "Who did you hold?"

Nara knows the approved answer. No one. There was no injury. No missing worker. No foreign ecology with whom an imperial body could have made a promise.

Adjudicator Vey's silhouette appears above the inspection membrane, pale formal plates outlined by green light. Vey has seen the talon marks. The upper exit iris begins to close behind them.

There is time for one test before the adjudicator reaches the ledges.

-> rupture_choice

=== rupture_choice ===
// ghostlight.choice_layer: bodily_contradiction
+ [Give the ants another shed scale and uncover the flower for cross-reading.]
    // ghostlight.branch: assemble_absence_proof
    // ghostlight.branch_label: assemble_absence_proof
    // ghostlight.action: offer_body_sample
    // ghostlight.action_label: offer_body_sample
    // ghostlight.intent: Let two ecological witnesses compare the body's contradiction without supplying a forbidden story.
    ~ witness_integrity = witness_integrity + 2
    ~ body_strain = body_strain + 1
    ~ court_suspicion = court_suspicion + 2
    Nara scrapes one loose scale from the lower wrist and places it beside the first.

    The ants bridge the samples with microbial glue. The flower throws its petals wide. Both witnesses answer the recertification scent with the same empty interval.

    It is not a memory. It is the shape of two instruments refusing the calibration.
    -> inspection_fold
+ [Enter the silver repair basin and ask the mirror amoebae what the clenched hand is copying.]
    // ghostlight.branch: risk_fragment_replay
    // ghostlight.branch_label: risk_fragment_replay
    // ghostlight.action: enter_treatment
    // ghostlight.action_label: enter_treatment
    // ghostlight.intent: Seek a content fragment from peripheral tissue despite the risk of false familiarity and identity leakage.
    ~ fragment_strength = fragment_strength + 2
    ~ body_strain = body_strain + 2
    ~ witness_integrity = witness_integrity + 1
    ~ compliance_cover = compliance_cover - 1
    Nara crosses the open lower slit in the membrane and drops two body lengths into the shallow basin, taloned feet landing on the scored stone beneath silver culture film. Mirror amoebae climb the lower right hand in a cold shimmer.

    The hand closes again.

    Pressure. Bark under the upper claws. Someone else's wrist in the lower fingers. A command arriving through the roots: release.

    The fragment carries no face and no proof that it belongs to Nara.

    Nara climbs back through the lower membrane slit to the middle ledge, silver film still shining on the lower right hand.
    -> inspection_fold
+ [Recite the approved patrol record to Vey and surrender the flower for grooming.]
    // ghostlight.branch: reinforce_official_memory
    // ghostlight.branch_label: reinforce_official_memory
    // ghostlight.action: speak_and_transfer
    // ghostlight.action_label: speak_and_transfer
    // ghostlight.intent: Use procedural compliance to reduce immediate danger, even if the living witness is weakened.
    ~ compliance_cover = compliance_cover + 2
    ~ court_suspicion = court_suspicion - 1
    ~ witness_integrity = witness_integrity - 1
    ~ oru_trust = oru_trust - 1
    ~ flower_custody = 0
    "East road patrol completed," Nara says. "No injury. No loss. No contact."

    They unclip the flower with a lower hand and place it on the inspection rail.

    Vey's posture softens by one administrative degree. Oru's does not.
    -> inspection_fold
+ [Pass the flower's seed bead to Oru and point one lower hand toward the paid road candle.]
    // ghostlight.branch: route_the_witness
    // ghostlight.branch_label: route_the_witness
    // ghostlight.action: transfer_object
    // ghostlight.action_label: transfer_object
    // ghostlight.intent: Move a living trace toward a witness network while the court watches Nara's larger limbs.
    ~ road_access = road_access + 1
    ~ oru_trust = oru_trust + 1
    ~ witness_integrity = witness_integrity + 1
    ~ court_suspicion = court_suspicion + 1
    Nara shields the chest with one clawed upper hand. A smaller lower hand plucks a wet seed bead and presses it into Oru's waiting palm.

    Then Nara points down, not out: grooming ledge, quarantine ring, one amber candle.

    Oru closes their hand before Vey can see what moved.
    -> inspection_fold

=== inspection_fold ===
// ghostlight.fold: adjudicator_contains_the_question
Vey descends the root ridges with the unhurried balance of someone whose exits close for other people.

"A recertification response is not evidence," Vey says. "It is a request for care."

{witness_integrity >= 4: Nara's burden flower holds a loud colorless gap beneath the green court light; the ants keep rebuilding their broken ring.}
{witness_integrity <= 1: The flower's testimony is thin or pruned, and the ant assay has only one damaged sample to read.}
{fragment_strength >= 2: Silver amoebae shimmer across Nara's lower right hand. The fingers repeat the grip while the rest of Nara recoils from it.}
{oru_trust >= 4: Oru remains within lower-hand reach instead of retreating to the certified queue.}
{oru_trust <= 1: Oru steps back into regulation spacing. Whatever they believe, they will not spend their body on it.}
{court_suspicion >= 4: Vey presses an ivory wrist plate to the root. The lower quarantine ring seals, leaving the candle road's remaining amber bead as the only open witness lane.}
{court_suspicion <= 1: Vey still thinks this can be filed as one worker's maintenance fault.}
{compliance_cover >= 4: The approved patrol phrases sit ready in Nara's mouth, polished enough to pass one more inspection.}
{body_strain >= 3: Fever light pulses through Nara's plate seams. Their grip on the ridge is becoming unsafe.}
{flower_custody == 0: Nara's burden flower rests on the inspection rail within Vey's lower-hand reach.}

Behind Vey, the waiting workers watch their own burden flowers pretend not to notice.

Nara cannot disclose a forbidden fact. Nara does not possess it.

They can decide who is allowed to see the missing place.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: disclosure_scope
+ {flower_custody == 1} [Raise the flower to the waiting queue and say only what the witnesses prove.]
    // ghostlight.branch: disclose_absence_publicly
    // ghostlight.branch_label: disclose_absence_publicly
    // ghostlight.action: speak_and_show
    // ghostlight.action_label: speak_and_show
    // ghostlight.intent: Make the violation public without claiming ownership of the erased truth.
    {witness_integrity >= 4:
        Nara braces high on the root ridge, holds the flower where every ledge can see, and says, "My memory agrees with the archive. My body does not. Those are different facts."
        -> ending_public_proof
    - else:
        Nara raises a pruned or exhausted flower. The queue sees distress, but not a comparison they can defend.
        -> ending_public_cost
    }
+ [Give Oru the shed scale and keep Vey's attention on the repeating hand.]
    // ghostlight.branch: smuggle_witness_to_road
    // ghostlight.branch_label: smuggle_witness_to_road
    // ghostlight.action: transfer_and_misdirect
    // ghostlight.action_label: transfer_and_misdirect
    // ghostlight.intent: Preserve a distributed witness beyond the court even if Nara remains inside.
    {oru_trust >= 3 && road_access >= 3 && court_suspicion < 4:
        Nara lets the lower right hand clench in Vey's sight while the other lower hand passes a glued shed scale behind the standardized mantle.
        -> ending_road_proof
    - else:
        Nara attempts the pass, but trust, route credit, or time is too thin.
        -> ending_road_cost
    }
+ {fragment_strength >= 2} [Let the mirror amoebae drive the fragment until it gives a face or breaks.]
    // ghostlight.branch: demand_missing_content
    // ghostlight.branch_label: demand_missing_content
    // ghostlight.action: persist_treatment
    // ghostlight.action_label: persist_treatment
    // ghostlight.intent: Trade bodily safety for a possible content fragment while accepting that familiarity is not verification.
    {fragment_strength >= 2 && body_strain <= 2:
        Nara sinks the lower right hand deeper into the silver film and follows the grip past pain.
        -> ending_fragment
    - else:
        Nara asks an exhausted copying colony for truth it cannot own.
        -> ending_false_familiarity
    }
+ [Accept re-dosing, but cut the broken ant ring into the underside of the grooming comb.]
    // ghostlight.branch: preserve_private_scar
    // ghostlight.branch_label: preserve_private_scar
    // ghostlight.action: mark_object
    // ghostlight.action_label: mark_object
    // ghostlight.intent: Survive the court and preserve a future re-entry cue without asserting the missing memory.
    {compliance_cover >= 3:
        While Vey prepares the scent collar, Nara's lower hand scores a broken ring beneath the comb's handle, where grooming fingers will find it before inspection eyes do.
        -> ending_private_scar
    - else:
        Vey has already stopped treating Nara as a worker making a choice.
        -> ending_redosed_empty
    }

=== ending_public_proof ===
// ghostlight.ending_label: public_absence_proof
// ghostlight.training_hook: disclosure_without_truth_capture
For one breath, the court remains a queue.

Then burden flowers open across standardized mantles. Purple, yellow, rust, white. Not agreement. Comparison.

Vey orders the membrane closed. That makes the wrong kind of sense to too many bodies at once.

Nara has not recovered yesterday. They have made the theft of yesterday socially real.

The empire can revise a citizen. Revising a roomful of witnesses before they notice the maintenance crew arriving is a scheduling problem, and even total coordination occasionally meets a calendar.
-> END

=== ending_public_cost ===
// ghostlight.ending_label: public_distress_without_proof
// ghostlight.training_hook: disclosure_fails_when_witness_chain_is_thin
The queue sees Nara shake beneath a damaged flower.

Vey names it compassionately: contamination anxiety, complicated by poor grooming.

The phrase offers everyone a way to remain employed. Most take it.

Oru does not look away, but one private witness is exactly the size of tragedy the court knows how to digest.
-> END

=== ending_road_proof ===
// ghostlight.ending_label: distributed_witness_escapes
// ghostlight.training_hook: ecological_witness_chain_outlives_custody
Oru drops from the ledge while Vey watches Nara's hand repeat a grip with no remembered owner.

At the quarantine ring, Oru pays the amber candle with the seed bead and the ant-glued scale. The fungal road brightens one segment, then another, carrying not a story but a request for comparison.

By the time Vey turns, the court still owns every person in the room.

It no longer owns every copy of the question.
-> END

=== ending_road_cost ===
// ghostlight.ending_label: witness_route_intercepted
// ghostlight.training_hook: ecological_contracts_require_credit_and_trust
The pass fails in a small, legible way.

Oru carries the seed bead and glued scale down the ridge, then stops above the quarantine ring. Trust is too thin, the unpaid road stays dark, or Vey's seal leaves no lane the road will accept. Lattice ants scatter from the inspection resin.

Vey collects the scale in a sterile cup.

Distributed truth is not magic. It is logistics with opinions, and tonight one of the opinions is no.
-> END

=== ending_fragment ===
// ghostlight.ending_label: fragment_without_verdict
// ghostlight.training_hook: recovered_familiarity_is_not_verified_truth
The amoebae replay pressure until a silhouette appears in Nara's lower hand: an Airawa body on the east road, one climbing foot wounded, a burden flower crushed flat against the throat plates.

Nara feels recognition. The feeling is chemically perfect.

The ants cannot verify a face. The road will not answer from behind the sealed ring. Oru can confirm only that Nara held someone.

Vey says, "Now you understand why unsupervised repair is dangerous."

Nara leaves with a fragment, a fever, and no honest right to call the fragment true.
-> END

=== ending_false_familiarity ===
// ghostlight.ending_label: identity_leak_overwrites_gap
// ghostlight.training_hook: copying_process_cannot_own_truth
The silver film gives Nara six faces, then Oru's face, then Nara's own.

Each arrives with the warm certainty of home. The lower hand grips until the joints tear.

Vey burns the culture from the basin and calls the procedure a rescue.

The missing place has not been filled. It has been crowded.
-> END

=== ending_private_scar ===
// ghostlight.ending_label: private_reentry_cue_survives
// ghostlight.training_hook: survival_preserves_bounded_future_doubt
The maintenance scent enters through Nara's throat, gut, plate seams, and archive-facing symbionts.

The talons release. The right hand opens. Yesterday becomes smooth again.

On the next shift, Nara's lower fingers find a broken ring carved under the grooming comb. They do not know who carved it or what it means.

{flower_custody == 1: Their burden flower opens one precise colorless gap.}
{flower_custody == 0: The flower is absent from its harness. The empty clasp presses a precise shape into Nara's chest.}
-> END

=== ending_redosed_empty ===
// ghostlight.ending_label: recertification_erases_reentry
// ghostlight.training_hook: compliance_without_cover_becomes_custody
Vey takes the comb before Nara can turn it over.

The scent collar closes. Oru is moved behind the membrane. The burden flower goes into an approved compost cup.

When Nara returns to the ledge, the routine is simple. Groom. Sample. Cross. Touch. Report calm.

The lower right hand aches.

There is no administrative field for that.
-> END
