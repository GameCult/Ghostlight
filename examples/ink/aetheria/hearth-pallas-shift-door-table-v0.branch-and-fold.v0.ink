// ghostlight.artifact_id: hearth_pallas_shift_door_table_branch_fold_v0
// ghostlight.fixture_id: hearth-pallas-shift-door-table-v0
// ghostlight.scene_id: hearth-pallas-shift-door-table-v0.outward-loop-clinic-handoff
// ghostlight.final_ink_path: examples/ink/aetheria/hearth-pallas-shift-door-table-v0.branch-and-fold.v0.ink

VAR table_reserve = 2
VAR neighborhood_trust = 2
VAR lea_claimshare_margin = 2
VAR care_clock = 3
VAR pavi_clinic_route = 0
VAR sori_harness_margin = 2
VAR kett_variance_risk = 0
VAR audit_visibility = 1
VAR shared_cost = 0
VAR table_location = 1
VAR task_clips_visible = 1

-> start

=== start ===
// ghostlight.scene: outward_loop_open
// ghostlight.visual_scene_id: pallas_corridor_establishing
Early in 2719, before the Pallas stoppages spread beyond one cavity yard, Lea Varek's housing loop wakes by shifts rather than mornings.

The loop lies outward in a half-commissioned Bloom, an asteroid rebuilt as a rotating habitat, where spin gives the grated floor a dependable down. Apartment doors line the curved outer wall. Utility panels and bundled pipes run along the inner wall. At the spinward end, a yellow pressure door opens toward the yard tram; an Aeronautics Unlimited shift scanner stands beside it, close enough to notice lateness and too far away to smell anybody's breakfast.

Between Lea's door and the scanner sits a dented composite table cut from a rejected radiator packing cover. It stays clear of the pressure-door arc. That is its only safety certification and, so far, its most successful career.

-> table_routine

=== table_routine ===
// ghostlight.scene: ordinary_care_handover
// ghostlight.visual_scene_id: pallas_table_routine
Lea sets down a lidded pot of bean broth before fastening her seal-rigger jacket. Her niece Pavi, eight and solemn about other people's errors, sorts reusable task clips by door number and clock. A blue clip asks someone passing the clinic to collect a sealed parcel. A green one asks for company through a child handover. A copper one asks for a tailored ration from the galley.

No clip names an employer or labor category. The people in the corridor already know who lives behind each door. AU's systems know three different kinds of liability and none of their birthdays.

Sori waits beside the table in a mobile dry-operation harness: a red-brown uplifted cephalopod supported by a flexible body loop, low rolling frame, humidity film, oxygenation tubes, padded pressure cuffs, and a small voice plate. The equipment keeps eight tentacles working in human-built corridors. Comfort is the portion the contract calls optional.

Kett-4 crouches beneath the table, a low six-limbed VitaForge maintenance biodrone with a slate-colored hide, four load-bearing limbs, and two finer front manipulators. Kett cannot use the shift scanner's speech prompts, but reads route colors quickly and moves the clips into the order of actual doors.

"Blue before copper," Pavi tells Kett.

Kett moves copper before blue.

"Because the galley is on the way," Lea says.

Pavi considers this betrayal by geography.

There are nine unbilled minutes before Lea should leave.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: morning_table_contribution
+ [Pour two extra meal jars and leave your own ration seal under the lids.]
    // ghostlight.branch: routine_stock_broth
    // ghostlight.action: use_object
    // ghostlight.intent: increase_shared_food_margin
    ~ table_reserve = table_reserve + 2
    ~ care_clock = care_clock - 1
    // ghostlight.visual_scene_id: pallas_routine_choice_result
    Lea fills two squat jars, locks both lids, and slides her own ration seal beneath the clamps.

    "That was breakfast," Pavi says.

    "That was accounting with steam."

    Sori's voice plate clicks. "Accounting usually takes the steam."

    Pavi writes LEA ATE on a scrap and puts it under the empty bowl. The table accepts this version of events.
    // ghostlight.consequence: food_reserve_up_personal_margin_down
    -> routine_fold
+ [Take Sori's blue clinic clip and fit it to the outside of your tool pouch.]
    // ghostlight.branch: routine_take_sori_clip
    // ghostlight.action: transfer_object
    // ghostlight.intent: make_neighbor_care_part_of_existing_route
    ~ neighborhood_trust = neighborhood_trust + 2
    ~ audit_visibility = audit_visibility + 1
    // ghostlight.visual_scene_id: pallas_routine_choice_result
    Lea snaps Sori's blue clip onto the pouch she is already taking past the clinic junction.

    The clip carries door, clock, and parcel seal. Nothing says why Sori cannot collect it during the harness service slot AU scheduled over the clinic window.

    "If the scanner asks," Lea says, "this is a very small wrench."

    Sori raises two tentacle tips. "Its repair function is emotional."

    Pavi changes the clip's route mark so nobody else wastes a trip.
    // ghostlight.consequence: trust_up_visibility_up
    -> routine_fold
+ [Let Kett reorder the whole morning's clips by reachable route.]
    // ghostlight.branch: routine_kett_route_sort
    // ghostlight.action: wait
    // ghostlight.intent: recognize_biodrone_route_judgment
    ~ neighborhood_trust = neighborhood_trust + 1
    ~ table_reserve = table_reserve + 1
    ~ kett_variance_risk = kett_variance_risk + 1
    // ghostlight.visual_scene_id: pallas_routine_choice_result
    Lea keeps her hands off the clips.

    Kett lays them in a clean route: galley, clinic, child bay, yard tram. One fine manipulator taps a faded service stripe beside the table; the stripe reaches the clinic junction without passing the spoken scanner.

    Pavi copies the order onto her lesson slate.

    Kett's limiter port blinks once. Initiative, the diagnostic will call it later, unless someone dislikes the result.
    // ghostlight.consequence: route_capacity_and_trust_up_variance_risk_up
    -> routine_fold
+ [Clock in early and leave the table to people whose records can survive generosity.]
    // ghostlight.branch: routine_clock_in_early
    // ghostlight.action: move
    // ghostlight.intent: protect_household_claimshare_margin
    ~ lea_claimshare_margin = lea_claimshare_margin + 2
    ~ neighborhood_trust = neighborhood_trust - 1
    ~ table_reserve = table_reserve - 1
    // ghostlight.visual_scene_id: pallas_routine_choice_result
    Lea touches her badge to the scanner seven minutes early.

    The display congratulates her household on protecting its claim trajectory.

    Behind her, Sori tightens a loose lid with three tentacles. Kett finishes the route sort. Pavi does not wave because she is busy learning what congratulations cost.
    // ghostlight.consequence: personal_margin_up_shared_reserve_and_trust_down
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: morning_contribution_into_shared_routine
// ghostlight.visual_scene_id: pallas_routine_fold
The loop settles into handover.

{table_reserve >= 4: Lidded jars and routed clips cover most of the scratched tabletop. The next mistake has somewhere soft to land.}
{table_reserve <= 1: One jar and two upright clips remain. The table can still help, provided reality develops modest requirements.}
{neighborhood_trust >= 4: Pavi moves among Sori, Kett, and Lea with the grave efficiency of a child who expects adults to share the obvious work.}
{neighborhood_trust <= 1: Pavi keeps Lea's door key in her fist. The table works, but it feels like a place other people are kind.}
{lea_claimshare_margin >= 4: Lea's early scan glows green beside her housing claim. For once, the household has room to lose a minute.}
{kett_variance_risk >= 1: Kett's route sort sits in plain view: useful judgment performed under a product code.}
{audit_visibility >= 2: Sori's blue clinic clip hangs from Lea's tool pouch where the shift scanner can read the handoff.}

Then every wall slate changes at once.

-> schedule_collision

=== schedule_collision ===
// ghostlight.scene: shift_and_clinic_collision
// ghostlight.visual_scene_id: pallas_schedule_collision
Cavity Twelve's seal inspection moves forty minutes forward. Lea's mandatory check-in now begins when Pavi's clinic parcel becomes available. Missing the inspection costs a claimshare mark. Missing the parcel means the next dispensary window is two days away.

The official child bay is closed behind a red pressure-isolation strip while its air filter is replaced. The notice offers apologies in six contract languages and help in none.

Sori's harness service slot opens at the same clock. Skip it and the humidity cartridge remains uncertified for another dry-yard shift.

Kett's service stripe reaches the clinic. VitaForge's route order does not include collecting medicine for a human child.

Pavi looks at the table, then at Lea. "Which rule is sick?"

"Several are showing symptoms."

The clinic clock drops to three blocks.

-> route_choice

=== route_choice ===
// ghostlight.choice_layer: clinic_handoff_route
+ [Take Pavi to the clinic yourself and accept the late shift mark.]
    // ghostlight.branch: route_lea_escort
    // ghostlight.action: move
    // ghostlight.intent: keep_guardian_care_inside_recognized_household
    ~ pavi_clinic_route = 1
    ~ lea_claimshare_margin = lea_claimshare_margin - 2
    ~ care_clock = care_clock - 1
    ~ shared_cost = shared_cost + 1
    // ghostlight.visual_scene_id: pallas_route_choice_result
    Lea turns away from the scanner and takes Pavi's hand.

    Sori lifts Lea's abandoned tool pouch onto the table. Kett slides the cavity checklist beneath it so the next rigger can see exactly what is missing.

    The clinic route is legal. The absence is legible. AU is very good at recognizing care after translating it into one worker's loss.
    // ghostlight.consequence: clinic_route_secured_claimshare_margin_spent
    -> clinic_fold
+ [Ask Sori to escort Pavi, knowing the harness service clock will keep running.]
    // ghostlight.branch: route_sori_escort
    // ghostlight.action: request_help
    // ghostlight.intent: trade_neighbor_harness_margin_for_clinic_continuity
    ~ pavi_clinic_route = 2
    ~ sori_harness_margin = sori_harness_margin - 2
    ~ neighborhood_trust = neighborhood_trust + 1
    ~ audit_visibility = audit_visibility + 1
    ~ shared_cost = shared_cost + 1
    ~ care_clock = care_clock - 1
    // ghostlight.visual_scene_id: pallas_route_choice_result
    Sori lowers the harness frame until Pavi can loop two fingers through a padded side grip.

    "Stay beside the blue wheel," the voice plate says. "It is the wheel with ambitions."

    The blue wheel squeaks toward the clinic junction. The harness service clock remains on the table slate, counting down an absence nobody will code as childcare.
    // ghostlight.consequence: clinic_route_secured_harness_margin_spent
    -> clinic_fold
+ [Give Kett the sealed pickup clip while Sori waits with Pavi at the table.]
    // ghostlight.branch: route_kett_pickup
    // ghostlight.action: transfer_object
    // ghostlight.intent: split_care_across_routes_and_bodies
    ~ pavi_clinic_route = 3
    ~ kett_variance_risk = kett_variance_risk + 2
    ~ neighborhood_trust = neighborhood_trust + 1
    ~ table_reserve = table_reserve - 1
    ~ shared_cost = shared_cost + 1
    ~ care_clock = care_clock - 1
    // ghostlight.visual_scene_id: pallas_route_choice_result
    Lea locks the clinic seal into the blue clip and places it between Kett's fine front manipulators.

    Kett checks the service stripe, the clinic light, and Pavi. Then the biodrone runs low along the inner wall, built for spaces that charge humans extra for knees.

    Sori settles beside Pavi and opens one broth jar. The clinic parcel will travel as cargo. The waiting will travel as friendship. Only one of those fits Kett's product record.
    // ghostlight.consequence: clinic_pickup_secured_biodrone_variance_risk_up
    -> clinic_fold
+ {table_reserve >= 3 || neighborhood_trust >= 3} [Split the route: Kett collects, Sori waits, and you trade half a shift with the next rigger.]
    // ghostlight.branch: route_shared_relay
    // ghostlight.action: mixed
    // ghostlight.intent: distribute_care_cost_across_existing_routes
    ~ pavi_clinic_route = 4
    ~ lea_claimshare_margin = lea_claimshare_margin - 1
    ~ sori_harness_margin = sori_harness_margin - 1
    ~ kett_variance_risk = kett_variance_risk + 1
    ~ neighborhood_trust = neighborhood_trust + 2
    ~ shared_cost = shared_cost + 2
    ~ audit_visibility = audit_visibility + 1
    // ghostlight.visual_scene_id: pallas_route_choice_result
    Kett takes the sealed pickup clip. Sori keeps Pavi at the table until the parcel returns. Lea swaps the first half of Cavity Twelve with the rigger behind door twenty-three and promises the second half after clinic handover.

    Nobody becomes free. Three bad overlaps become one workable route.

    Pavi eats half a jar of broth and leaves the rest for Kett, who cannot digest it. She is learning generosity before logistics, which is the usual order and rarely the useful one.
    // ghostlight.consequence: clinic_route_distributed_cost_and_trust_up
    -> clinic_fold

=== clinic_fold ===
// ghostlight.fold: clinic_route_into_shared_record_pressure
// ghostlight.visual_scene_id: pallas_clinic_fold
The blue clinic light changes from WAITING to RELEASED.

{pavi_clinic_route == 1: Pavi returns holding Lea's hand and the sealed parcel. Lea's shift slate now carries one clean late mark.}
{pavi_clinic_route == 2: Pavi returns beside Sori's blue harness wheel. The parcel is sealed; Sori's service countdown is nearly gone.}
{pavi_clinic_route == 3: Kett sets the parcel on the table while Sori finishes Pavi's broth. The VitaForge limiter port blinks amber.}
{pavi_clinic_route == 4: Kett returns with the parcel as Lea's replacement rigger arrives. Sori keeps one tentacle on Pavi's chair and another on the harness-service clock.}
{care_clock <= 1: The handoff finishes at the last useful block. Nobody has spare time left to make the story respectable.}
{sori_harness_margin <= 0: Sori's humidity cartridge remains physically working and administratively uncertified.}
{kett_variance_risk >= 2: Kett's limiter display has filed the clinic detour as route variance.}

Pavi has what she needs.

Then Imas Roe, the AU shift clerk, comes through the pressure door with the look of a person sent to discover why several correct systems have produced one impossible hallway.

-> clerk_arrival

=== clerk_arrival ===
// ghostlight.scene: table_liability_review
// ghostlight.visual_scene_id: pallas_clerk_arrival
Imas wears a clean gray shift coat and carries a slate full of separate questions.

{pavi_clinic_route == 1: Why did Cavity Twelve lose its registered rigger to a private household absence?}
{pavi_clinic_route == 2: Why did a BioElevate uplift harness miss service while carrying a human dependent?}
{pavi_clinic_route == 3: Why did a human medicine seal travel on a VitaForge route?}
{pavi_clinic_route == 4: Why did a VitaForge route, a BioElevate support clock, and an unscheduled rigger exchange all touch one human clinic parcel?}

And why is there furniture inside the shift scanner's observation field?

The table holds broth rings, the empty clinic slot, and clips for doors whose occupants are already at work.

"Who administers this?" Imas asks.

Pavi raises her hand.

Lea lowers it gently. "She is eight."

"Then she lacks table authority."

"That has been our strongest protection."

Imas looks tired enough to understand and employed enough to continue.

-> clerk_choice

=== clerk_choice ===
// ghostlight.choice_layer: table_liability_response
+ [Move the table inward behind Lea's housing door before Imas can inventory it.]
    // ghostlight.branch: clerk_move_table_inward
    // ghostlight.action: move_object
    // ghostlight.intent: preserve_care_capacity_by_narrowing_public_access
    ~ table_location = 2
    ~ audit_visibility = audit_visibility - 1
    ~ care_clock = care_clock - 1
    // ghostlight.visual_scene_id: pallas_clerk_choice_result
    Lea and Pavi lift one end. Sori hooks three tentacles beneath the other. Kett pushes low at the buckled middle.

    The table crosses Lea's threshold by less than a meter. It is now private furniture. It is also harder for doors twenty-four through thirty to reach before shift.

    Imas watches public care become a housing inconvenience and records a resolved obstruction.
    // ghostlight.consequence: table_preserved_access_narrowed_visibility_down
    -> liability_fold
+ [Call it your meal table and slide the task clips beneath the broth pot.]
    // ghostlight.branch: clerk_claim_furniture
    // ghostlight.action: speak
    // ghostlight.intent: place_public_practice_under_one_household_cover_story
    ~ lea_claimshare_margin = lea_claimshare_margin - 1
    ~ task_clips_visible = 0
    ~ audit_visibility = audit_visibility - 1
    // ghostlight.visual_scene_id: pallas_clerk_choice_result
    "Mine," Lea says. "Breakfast furniture."

    Pavi slips the clips under the warm pot with the solemnity of a junior conspirator and the subtlety of a dropped wrench.

    Imas records personal property near a controlled threshold. Lea's household gains the liability. The corridor keeps the surface.
    // ghostlight.consequence: table_and_clips_hidden_under_household_liability
    -> liability_fold
+ [Give Imas the clips but leave the food and chairs where they are.]
    // ghostlight.branch: clerk_surrender_clips
    // ghostlight.action: transfer_object
    // ghostlight.intent: preserve_visible_hospitality_by_sacrificing_coordination_record
    ~ task_clips_visible = 0
    ~ neighborhood_trust = neighborhood_trust - 1
    ~ audit_visibility = audit_visibility - 1
    ~ table_reserve = table_reserve - 1
    // ghostlight.visual_scene_id: pallas_clerk_choice_result
    Lea stacks the clips in Imas's open palm.

    Without them, the table becomes food and chairs. Kindness is acceptable when it cannot schedule itself.

    Kett taps the bare route stripe once. Sori coils two tentacle tips beneath the harness frame. Pavi puts a new blank clip in her pocket.
    // ghostlight.consequence: visible_table_survives_coordination_and_trust_thin
    -> liability_fold
+ [Set Pavi's sealed clinic parcel on the slate and ask which contract should have left it behind.]
    // ghostlight.branch: clerk_show_completed_care
    // ghostlight.action: show_object
    // ghostlight.intent: force_the_review_to_name_material_dependency
    ~ audit_visibility = audit_visibility + 2
    ~ neighborhood_trust = neighborhood_trust + 1
    ~ shared_cost = shared_cost + 1
    // ghostlight.visual_scene_id: pallas_clerk_choice_result
    Lea places the parcel across Imas's list of infractions.

    "Pick one," she says. "My shift, Sori's harness slot, Kett's route order, or the parcel."

    Imas does not choose. The slate opens three incident channels and a household review.

    The parcel remains on top, small and already delivered. Bureaucracy arrives late to the only fact that mattered this morning.
    // ghostlight.consequence: care_made_visible_cross_category_exposure_up
    -> liability_fold

=== liability_fold ===
// ghostlight.fold: table_response_into_cost_ownership
// ghostlight.visual_scene_id: pallas_liability_threshold
The shift horn sounds through the pressure door. The cavity yard wants bodies. The clinic wants the parcel seal acknowledged. The housing system wants one household to own anything left in the corridor.

{table_location == 2: The table stands just inside Lea's open door, close enough to serve the loop and far enough inside to make every visit look personal.}
{table_location == 1: The table remains beside the shift scanner, dented and publicly useful.}
{task_clips_visible == 0: The task clips are hidden or surrendered. People will have to remember the routes aloud.}
{task_clips_visible == 1: Door numbers and clocks remain visible on the tabletop, a tiny map of work no contract assigned.}
{audit_visibility >= 4: Imas's slate has opened separate labor, supplier, and housing reviews around the same clinic parcel.}
{audit_visibility <= 1: Imas can close the hallway obstruction without describing how Pavi reached the clinic.}
{shared_cost >= 3: More than one person has already spent time, margin, or classification safety on the handoff.}
{lea_claimshare_margin <= 0: Lea's household has no clean mark left between this morning and housing review.}
{neighborhood_trust >= 4: Nobody around the table pretends Lea made the route alone.}

Pavi tucks the clinic parcel against her chest.

Lea can put the cost on one household, let each system see the piece it is already prepared to punish, or keep the table ownerless by taking it farther inside.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: care_cost_ownership
+ [Put every unbilled minute on Lea's private absence and keep the neighbors out of the report.]
    // ghostlight.branch: decide_private_absence
    // ghostlight.action: commit_record
    // ghostlight.intent: protect_cross_category_neighbors_with_one_household_margin
    ~ lea_claimshare_margin = lea_claimshare_margin - 1
    {lea_claimshare_margin >= 1:
        -> ending_private_foundation
    - else:
        -> ending_private_cost
    }
+ [Let each helper's actual part remain visible in the separate incident channels.]
    // ghostlight.branch: decide_shared_record
    // ghostlight.action: commit_record
    // ghostlight.intent: refuse_to_make_care_look_like_one_household_failure
    {neighborhood_trust >= 4 && shared_cost >= 2 && care_clock >= 1:
        -> ending_shared_foundation
    - else:
        -> ending_shared_cost
    }
+ [Keep no account at all; move the table and its next blank clips farther inside.]
    // ghostlight.branch: decide_ownerless_table
    // ghostlight.action: withhold_record
    // ghostlight.intent: preserve_informal_care_by_reducing_legibility
    ~ table_location = 2
    {audit_visibility <= 2 && table_reserve >= 2:
        -> ending_inward_foundation
    - else:
        -> ending_inward_cost
    }

=== ending_private_foundation ===
// ghostlight.ending_label: one_household_absorbs_cost_with_margin
// ghostlight.training_hook: private_care_shields_neighbors_but_concentrates_debt
// ghostlight.visual_scene_id: pallas_ending_private
Lea signs the absence.

Pavi's parcel closes its clinic clock. The table remains where the loop can reach it.

{pavi_clinic_route == 1: The clinic trip adds nothing to Sori's harness or Kett's route records because Lea carried the whole recognized route.}
{pavi_clinic_route == 2: Sori reaches the remaining harness-service minutes; Kett gains no clinic detour.}
{pavi_clinic_route == 3: Kett's detour loses the human medicine seal that made it easy to punish; Sori reaches service after waiting with Pavi.}
{pavi_clinic_route == 4: Sori reaches the remaining service minutes, and Kett's sealed parcel transfer stays attached to a completed shared route.}

Lea's household loses a claimshare mark and keeps enough standing to survive the next review. That is not justice. It is one person having just enough margin to spend on people who will spend theirs later.

Pavi returns the blank clip to the table.
-> END

=== ending_private_cost ===
// ghostlight.ending_label: one_household_absorbs_cost_without_margin
// ghostlight.training_hook: care_succeeds_while_housing_risk_concentrates
// ghostlight.visual_scene_id: pallas_ending_private
Lea signs the absence because the clinic parcel is already in Pavi's arms and cannot be turned back into an innocent schedule.

The household slate changes from green to housing review.

{pavi_clinic_route == 1: The clinic trip adds nothing to Sori's harness or Kett's route records while Lea's household carries the whole route.}
{pavi_clinic_route == 2: Sori's late service remains visible, but Lea's signature keeps it out of the child-care report.}
{pavi_clinic_route == 3: Kett's detour returns to the supplier queue as unexplained variance.}
{pavi_clinic_route == 4: Sori's shortened service and Kett's detour remain visible as separate small faults beneath Lea's signed absence.}

The table survives under Lea's name.

That evening, three doors send food to Lea's apartment without clips. Mutual aid is less photogenic after it has eaten the rent.
-> END

=== ending_shared_foundation ===
// ghostlight.ending_label: distributed_care_record_holds
// ghostlight.training_hook: cross_category_dependency_becomes_usable_testimony
// ghostlight.visual_scene_id: pallas_ending_shared
Lea leaves each act where it happened.

{pavi_clinic_route == 1: Lea escorted Pavi; Sori and Kett preserved the table and exposed the missed cavity work.}
{pavi_clinic_route == 2: Sori escorted Pavi; Lea reached Cavity Twelve; Kett kept the table route ordered.}
{pavi_clinic_route == 3: Kett collected the parcel; Sori kept Pavi safe; Lea held the shift.}
{pavi_clinic_route == 4: Kett collected the parcel; Sori kept Pavi safe; another rigger covered Cavity Twelve; Lea returned the hours.}

The systems issue three small warnings because they do not possess one language in which to issue the large one. No household loses the loop today. Kett's variance remains attached to a successful delivery. Sori's missed service minute remains beside a working harness. Imas cannot make the records agree, but cannot make the care disappear either.

At the next handover the table has two new jars and one fresh blue clip.

They still call it the table.
-> END

=== ending_shared_cost ===
// ghostlight.ending_label: distributed_record_exposes_separate_workers
// ghostlight.training_hook: truthful_care_becomes_cross_category_liability
// ghostlight.visual_scene_id: pallas_ending_shared
Lea leaves the parts visible.

{pavi_clinic_route == 1: AU marks Lea's household absence and the exposed cavity handoff as separate irregularities.}
{pavi_clinic_route == 2: BioElevate queues Sori's missed harness service as adaptation variance while AU records Lea's part as an unauthorized dependent handoff.}
{pavi_clinic_route == 3: VitaForge opens a product-interference check on Kett's clinic route while AU records Sori's waiting time as unassigned corridor use.}
{pavi_clinic_route == 4: AU marks the shift exchange irregular, BioElevate queues Sori's shortened harness service as adaptation variance, and VitaForge opens a product-interference check on Kett's clinic route.}

Each entry is locally tidy. Together they describe a neighborhood in the syntax of malfunction.

Pavi still receives the parcel on time. Before the next shift, someone bolts the dented table to the floor just beyond the scanner's clean view. Nobody signs the work.
-> END

=== ending_inward_foundation ===
// ghostlight.ending_label: ownerless_table_survives_inward
// ghostlight.training_hook: informal_care_preserved_by_narrower_access
// ghostlight.visual_scene_id: pallas_ending_inward
The table moves inside Lea's threshold.

Imas closes the obstruction. The clinic closes Pavi's parcel. The shift system keeps the lateness it already recorded. Behind the open door, clips for six households gather beside the broth.

The table now depends on Lea being home or trusting someone with the key. It is smaller than the corridor arrangement and safer than losing it entirely. Hope, on an AU ramp, is often an access compromise with soup on it.
-> END

=== ending_inward_cost ===
// ghostlight.ending_label: ownerless_table_survives_as_thin_memory
// ghostlight.training_hook: hidden_care_loses_public_capacity
// ghostlight.visual_scene_id: pallas_ending_inward
They carry the table inward, but the jars are nearly empty and half the clips are gone.

Pavi's parcel arrived. That is the morning's real success. The next shift will rely on remembered doors, spoken favors, and whoever can afford to be late.

Kett scratches the galley-clinic-child-bay route into the underside with one fine manipulator. Sori feels the marks with a tentacle tip. Lea opens her door wide enough for the next person to see there is still somewhere to put a need.
-> END
