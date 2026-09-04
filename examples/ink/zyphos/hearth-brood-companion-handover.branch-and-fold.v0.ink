// ghostlight.artifact_id: hearth_brood_companion_handover_branch_fold_v0
// ghostlight.fixture_id: hearth-brood-companion-handover-v0
// ghostlight.scene_id: hearth-brood-companion-handover-v0.first-route-packing-hollow
// ghostlight.final_ink_path: examples/ink/zyphos/hearth-brood-companion-handover.branch-and-fold.v0.ink
// ghostlight.tonal_mode: warm domestic comedy with quiet moral pressure

VAR child_trust = 2
VAR flower_credibility = 2
VAR care_capacity = 1
VAR privacy_boundary = 1
VAR route_credit = 2
VAR departure_time = 3
VAR family_cohesion = 2
VAR flower_attached = 1

-> start

=== start ===
The first-route packing hollow is one ramp below the breeding ground's arrival terrace, close enough to hear the candle road fruiting and far enough that nobody can pretend packing is a public ceremony.

The hollow curves around a low oval work cradle. Nursery nests open along its inner wall. On the outer wall, leaf-skin pockets hold portable archive membranes, route cords, folded shelter tissue, and three mineral spoons that have spent the morning becoming a constitutional problem.

Cold lantern knots hang between thick ceiling roots. Their blue light falls into a shallow mineral basin at the right side of the routeward ramp. Amber fungal candles mark the top of the ramp, where the mobile family waits to rejoin the continent.

-> handover_people

=== handover_people ===
Senn folds four running legs beneath a long rust-brown body at the work cradle. The smaller chest hands remain free to fit buckles, groom fibers, and lose arguments to spoons. Senn is the receiving family's route caretaker. Today that means receiving Oru, Oru's care record, and every consequence hidden inside the word receiving.

Oru is young enough for the facial fans to be too large for dignity and old enough to know it. The child sorts route cords by taste, then pretends this is not excitement.

Pale Mica grips Oru's bare left flank: a hand-sized burden flower with pale green leaves, copper-edged sensory filaments, and rootlets that have warned nursery caretakers through two fevers and one memorable attempt to eat archive paste.

Talar, the outgoing nursery caretaker, lays a flexible care archive across the cradle. Daro, Senn's repair-kin, tries to fit a shelter roll into the family flank frame without removing anything useful. This has become less engineering than a public accusation against volume.

"The child fits," Daro says.

"The child's obligations fit," Talar says. "The child was never in doubt."

Oru points a chest digit at the third mineral spoon. "That is the heavy one."

It is. The family had hoped morality would weigh less.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: ordinary_first_departure
+ [Fit a light-and-mineral sling for Pale Mica onto Senn's flank frame.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: fit_flower_sling
    ~ care_capacity = care_capacity + 2
    ~ flower_credibility = flower_credibility + 1
    ~ departure_time = departure_time - 1
    Senn unpacks a repair roll and threads its pale flexible ribs into a shallow flank sling. One side cups a mineral pad. The other angles a translucent leaf-skin hood toward the lantern knots.

    Pale Mica leans three leaves toward it.

    Daro looks at the repair roll, then at the shelter frame it was meant to mend. "Good. We can sleep under our principles when it rains."

    "Only the waterproof ones," Senn says.
    -> routine_fold
+ [Ask Daro to learn the flower's grooming sequence and take the second watch.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: distribute_flower_care
    ~ care_capacity = care_capacity + 2
    ~ family_cohesion = family_cohesion + 2
    ~ child_trust = child_trust + 1
    Talar shows Daro where Pale Mica's clasping rootlets collect salt and shed fiber. Daro repeats the sequence with two chest hands held open, never touching until the flower leans toward the offered mineral brush.

    "Morning wash, return-light hood, second watch," Daro recites.

    Oru corrects the angle of the brush. "It hates confidence."

    "Then it joins a well-supplied family."
    -> routine_fold
+ [Let Oru demonstrate Pale Mica's warning colors instead of translating for the child.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: child_demonstrates_signals
    ~ child_trust = child_trust + 2
    ~ privacy_boundary = privacy_boundary + 1
    ~ departure_time = departure_time - 1
    Oru taps a sequence against the work cradle: thirst, sharp scent, unfamiliar pressure. Pale Mica answers each cue with a different narrow band along its leaf edges.

    "And red?" Senn asks.

    Oru's facial fans draw in. "Red means ask. It does not mean announce an answer."

    Talar adds that sentence to the care archive.
    -> routine_fold
+ [Copy only care thresholds into the family archive and let Oru close the private layers.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: bound_care_archive
    ~ care_capacity = care_capacity + 1
    ~ privacy_boundary = privacy_boundary + 2
    ~ route_credit = route_credit + 1
    Senn opens the receiving archive beside Talar's membranes. Oru transfers fever thresholds, mineral intervals, and the color sequence for active danger. The child folds shut the layers holding dreams, embarrassing alarms, and the archive-paste incident.

    The two archives seal with a dull violet edge.

    Daro says, "A pity. I was hoping for the paste recipe."

    Pale Mica shows one copper line. Oru calls that contempt. Talar calls it insufficient evidence.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_care_before_pressure
Packing resumes under cold blue lantern light.

{care_capacity >= 3: The family flank frame now has a lit mineral sling and at least two adults who know why it exists.}
{care_capacity <= 2: Pale Mica has a place on the packing list, but not yet a convincing care system.}
{family_cohesion >= 4: Daro repeats the second-watch sequence without prompting. Care has begun spreading beyond the person holding the archive.}
{privacy_boundary >= 3: The receiving archive carries useful thresholds behind layers Oru was allowed to close.}
{privacy_boundary <= 1: Too much of Oru's nursery history lies open on the cradle, legible to anyone with the right chemistry and poor manners.}
{route_credit >= 3: At the ramp lip, the fungal candles hold a patient amber lane for the family's honest delay.}
{departure_time <= 2: The candle wave has begun thinning. The family can still leave, but not after indefinitely improving itself.}

Talar checks the transfer in the old order: child, archive, allied organism, named carers. Pale Mica must be shown the new sling or choose to remain attached for the first leg.

-> bloom_alarm

=== bloom_alarm ===
Talar slides a mineral brush beneath Pale Mica's lowest rootlet.

The flower clamps down.

Yellow strain runs across its leaves. Violet old-fever memory follows. Then red opens from the sensory cup to every edge, bright enough to paint Oru's bare flank and the routeward ramp.

The amber candles at the top of the ramp tighten into a narrow line. Lantern knots above the mineral basin pulse white. Neither ecology gives a verdict. Both have noticed.

Oru's four running legs fold until the child's belly nearly touches the floor. The facial fans close.

"It does that when everyone discusses where to put me," Oru says.

Talar keeps the brush still. Daro looks away from the open archive. Senn can smell fear, old immune heat, mineral hunger, and the terrible possibility that all four are present.

-> bloom_choice

=== bloom_choice ===
// ghostlight.choice_layer: exposed_private_alarm
+ [Leave Pale Mica attached and ask Oru what should remain private before asking what the red means.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: ask_child_first
    ~ child_trust = child_trust + 2
    ~ privacy_boundary = privacy_boundary + 2
    ~ flower_credibility = flower_credibility + 1
    ~ departure_time = departure_time - 1
    Senn folds lower until both sets of facial fans share the cradle's shadow.

    "What belongs in the care question?" Senn asks. "What belongs to you?"

    Oru thinks long enough for the adults to become uncomfortable, a useful educational service children provide for free.

    "The fever belongs in care. The leaving fear belongs to me. Pale Mica can say I need help. It cannot say why."

    The red remains. Its edges soften to yellow.
    -> exposed_alarm_fold
+ [Offer the mineral basin and let Pale Mica release Oru in its own time.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: offer_supported_release
    ~ care_capacity = care_capacity + 1
    ~ flower_credibility = flower_credibility + 2
    ~ child_trust = child_trust + 1
    ~ privacy_boundary = privacy_boundary + 1
    ~ flower_attached = 0
    ~ departure_time = departure_time - 1
    Senn brings the shallow basin from the right wall to the cradle's outer edge. Mineral water touches the flower's lowest rootlets. Talar holds the light hood open beside it.

    Pale Mica releases one grip, then another. Oru supports the plant body with both chest hands until the last rootlet settles into the basin.

    Red drains into a narrow copper pulse. The flower has accepted separation as care, at least for this moment.
    -> exposed_alarm_fold
+ [Block the ramp's sightline with Senn's long body while Talar compares the old-fever record.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: shelter_private_signal
    ~ privacy_boundary = privacy_boundary + 2
    ~ child_trust = child_trust + 1
    ~ route_credit = route_credit - 1
    Senn stands broadside between the cradle and routeward ramp. Four legs make a good privacy screen when arranged with conviction.

    Behind that living wall, Talar holds the violet archive layer near Pale Mica's leaf band. The colors resemble one another. Resemblance is evidence of resemblance, which is less satisfying than certainty and frequently more useful.

    At the ramp lip, one fungal candle dims. The road has seen concealment, even if it has not seen the child.
    -> exposed_alarm_fold
+ [Carry the visible alarm to the ramp lip and let the candle road witness it.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: disclose_to_road
    ~ route_credit = route_credit + 2
    ~ flower_credibility = flower_credibility + 1
    ~ privacy_boundary = privacy_boundary - 1
    ~ child_trust = child_trust - 1
    Senn asks Oru to stand at the work cradle's routeward side, where the red bloom can be seen from the ramp lip.

    The fungal candles brighten around shed fiber, foot pressure, and the flower's copper scent. A clean lane remains open. So does the memory of this alarm.

    Oru stands very still. Cooperation and consent sometimes share a body while remaining different facts.
    -> exposed_alarm_fold

=== exposed_alarm_fold ===
// ghostlight.fold: care_testimony_without_ownership
The packing hollow now contains a child who still wants to leave, a flower whose alarm has not become a diagnosis, and a family whose claim to kinship has acquired weight.

{child_trust >= 4: Oru's facial fans open toward Senn, fear still present but no longer solitary.}
{child_trust <= 1: Oru keeps the fans shut and answers only Talar. The receiving family has made itself another audience.}
{flower_credibility >= 4: Pale Mica holds a restrained copper-yellow pattern. Its warning is specific enough to deserve care and incomplete enough to forbid a verdict.}
{privacy_boundary >= 4: Talar folds the private archive layers closed and turns their chemical face toward the wall.}
{privacy_boundary <= 1: The red alarm reaches the ramp lip unobstructed, where the road can remember protection and exposure in the same trace.}
{route_credit >= 4: The fungal candles widen the departure lane and open a side pocket where the family may wait or reorganize.}
{route_credit <= 1: The road reduces its amber opening to a single-body lane. Private shelter has become a public route cost.}
{flower_attached == 0: Pale Mica rests in the mineral basin beside Oru, separated by consent and one handspan.}
{flower_attached == 1: Pale Mica remains clasped to Oru's flank, red fading but rootlets firm.}
{departure_time <= 1: Returning light is already touching the upper ramp. The current candle wave will close before another careful discussion can finish.}

Daro lifts the family flank frame. The shelter repair roll, the flower sling, the care archive, and the last three mineral spoons have reached an armed truce.

Senn must decide what shape the family will take onto the road.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: route_kinship_commitment
+ [Take Oru and Pale Mica together, with the negotiated care divided across the family.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: carry_child_and_companion
    {care_capacity >= 3 && child_trust >= 3 && flower_credibility >= 3 && family_cohesion >= 3 && route_credit >= 2:
        Senn fastens the care archive to the center of the family frame, where every watch must pass it.
        -> ending_shared_care_success
    - else:
        Senn names both bodies as family before the family has made enough room for either one's needs.
        -> ending_shared_care_cost
    }
+ [Split the first day's route so one part of the family travels at nursery pace.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: divide_route_for_care
    {family_cohesion >= 3 && care_capacity >= 2 && departure_time >= 1:
        Daro takes the trade frame. Senn takes the child, flower care, and the slower lantern route.
        -> ending_divided_route_success
    - else:
        Senn divides the route before the family has divided the obligation cleanly.
        -> ending_divided_route_cost
    }
+ [Arrange a supported separation: Pale Mica remains in nursery care while Oru takes the first route.]
    // ghostlight.action_label: negotiate
    // ghostlight.branch_label: support_temporary_separation
    {flower_attached == 0 && privacy_boundary >= 3 && child_trust >= 3:
        Senn asks Oru and Pale Mica separately, using words for one and mineral, light, grip, and time for the other.
        -> ending_separation_success
    - else:
        Senn proposes separation before the handover has made it legible as care.
        -> ending_separation_cost
    }
+ [Defer the whole departure until the next candle wave and keep the family in the hollow.]
    // ghostlight.action_label: refuse
    // ghostlight.branch_label: defer_first_departure
    {route_credit >= 3 && family_cohesion >= 2:
        Senn lowers the family frame back to the floor and returns the route cords to Oru.
        -> ending_defer_success
    - else:
        Senn stops the departure after the current route and family arrangements have already been drawn too tight.
        -> ending_defer_cost
    }

=== ending_shared_care_success ===
// ghostlight.ending_label: shared_care_success
// ghostlight.training_hook: kinship_as_distributed_cross_body_obligation
The family leaves through a lane wide enough for four adult bodies and one child who refuses to walk in the middle.

{flower_attached == 1: Pale Mica rides Oru's flank for the first slope, leaves angled toward Senn's new light hood.}
{flower_attached == 0: Pale Mica rides the mineral sling until Oru asks for the flower back; the transfer is treated as a question, not a correction.}

Daro holds the second watch. Senn holds the archive. Oru holds the right to say when an alarm explains danger and when it merely begins a conversation.

Behind them, Talar scratches four marks into the nursery's departure surface: child, companion, carers, route. The marks do not say who owns whom. They say who promised to notice.
-> END

=== ending_shared_care_cost ===
// ghostlight.ending_label: shared_care_cost
// ghostlight.training_hook: belonging_claim_without_material_capacity
They take both because the sentence sounds right.

By the second incline, the flower hood is shaded by trade rolls. Pale Mica blooms yellow with mineral hunger. Oru hides the color under a shelter flap. Senn discovers that declaring kinship is much faster than performing it, which is why declarations are so popular.

The road keeps them moving but narrows the next rest pocket. Talar's archive mark remains open: care accepted, care not yet demonstrated.
-> END

=== ending_divided_route_success ===
// ghostlight.ending_label: divided_route_success
// ghostlight.training_hook: family_shape_changes_to_preserve_care
The trading half of the family takes the bright main lane. Senn, Oru, Pale Mica, and the care frame turn onto the slower line of lantern knots along the grove edge.

The separation costs a market meeting and one intact repair roll. It also gives Oru time to learn the route without becoming cargo moving at adult speed.

Daro taps the second-watch promise into the split archive before leaving. Distance changes the family's shape for a day. It does not remove anyone from it.
-> END

=== ending_divided_route_cost ===
// ghostlight.ending_label: divided_route_cost
// ghostlight.training_hook: route_split_without_shared_obligation
Daro takes the trade frame and assumes Senn has the flower care. Senn assumes the mineral brushes stayed with Daro. Oru notices both errors and says nothing.

The two family lines can still smell one another on the first ridge. That makes the failure intimate rather than small.

Pale Mica's red bloom reaches the lantern grove before either adult admits which pack holds the wash salts.
-> END

=== ending_separation_success ===
// ghostlight.ending_label: supported_separation_success
// ghostlight.training_hook: negotiated_separation_without_disowned_kinship
Pale Mica remains in the mineral basin under Talar's care. Oru leaves a route cord looped around the basin rim and takes a matching loop on the family frame.

The child chooses a return interval. Talar copies it into both archives. Senn accepts responsibility for carrying mineral credit and route testimony back to the flower even while its body stays here.

At the ramp, Oru looks back once. Pale Mica shows a narrow copper line, the smallest signal in the hollow and the only one nobody translates aloud.
-> END

=== ending_separation_cost ===
// ghostlight.ending_label: supported_separation_cost
// ghostlight.training_hook: separation_imposed_as_logistics
Senn calls it temporary. Talar hears logistics. Oru hears that the family had room for the child only after subtracting something the child loved.

Pale Mica is moved to the basin while its rootlets are still tight. The red bloom splashes the closed archive layers. Oru walks up the ramp on command and does not correct another adult all day.

The family has gained pack space and lost the easiest path to knowing when the child needs help.
-> END

=== ending_defer_success ===
// ghostlight.ending_label: protective_delay_success
// ghostlight.training_hook: delay_as_collective_care_work
The family misses the current candle wave.

Nobody dies of this, which makes it socially expensive rather than heroic. Daro sends route testimony ahead through the fungal candles. Senn reopens the care archive. Oru eats the disputed third mineral spoon's ration with ceremonial malice.

By returning light, two adults can perform the grooming sequence and Pale Mica accepts the sling long enough to sleep. The first departure will happen later. It will still be first.
-> END

=== ending_defer_cost ===
// ghostlight.ending_label: protective_delay_cost
// ghostlight.training_hook: care_refusal_with_route_obligation_cost
Senn stops the departure. The road closes the main lane anyway, carrying the family's missed meeting and late notice into its next accounting.

Daro does not argue in front of Oru. The restraint is kind and therefore not free. Trade obligations will have to be repaired, and someone else will carry them while this hollow keeps five more bodies through returning light.

Oru stays beside Pale Mica. The child is safe. The family is not absolved. Care has protected one obligation by creating several others, exactly as advertised.
-> END
