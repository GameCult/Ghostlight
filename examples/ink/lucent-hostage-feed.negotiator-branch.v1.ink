VAR mara_trust = 0
VAR audience_heat = 2
VAR evidence_integrity = 0
VAR sol_stability = 2
VAR feed_control = 0
VAR influencer_noise = 2
VAR hostage_safety = 1
VAR security_pressure = 2
VAR recognition_given = false
VAR optics_mistrust = false
VAR lucent_reframe = false
VAR juno_gap = false
VAR pippa_spoke = false
VAR brant_spoke = false
VAR panels_muted = false
VAR safe_line = false
VAR security_receded = false
VAR handoff_attempted = false
VAR juno_distance_from_glass = 0

-> start

=== start ===
// ghostlight.fixture: lucent-hostage-feed.negotiator-branch.v1
// ghostlight.meta_coordinator_repair: Popper generated the branch structure; Codex repaired Ink syntax, validator action labels, and sidecar shape without changing the branch plan.
The Lucent tether station turns slowly enough for executives to call it graceful and fast enough for the floor to keep remembering physics.

Through the crisis feed, Mara Quell sees the transfer lounge at the media-eye end of the tether: glass walls, an elevator gate with a waist-high glass rail along its threshold, the city bubble far below like a sealed aquarium, every public surface bright with moderation overlays. Mara's private rail carries the controls; the lounge surfaces show enough status changes for Sol to know when the room has been edited. Sol Vant has one arm locked around Juno Ark. Juno is junior Reality Architecture, a feed-scaffold technician trained to shape overlays and packet routing, and is currently too close to Sol's hand to enjoy that fact.

Feed Ops fills Mara's left rail with prompts, sponsor-risk colors, breach estimates, audience heat, and three mutually exclusive suggestions labeled URGENT. Pippa Lux, an empathy influencer with a monetized tremble in her lower lip, waits in soft focus on a side panel. Brant Vee, an outrage streamer whose audience treats nuance as a food allergy, is already shouting without audio, a mercy granted by a tired god with excellent bandwidth.

Sol can hurt Juno before security can cross the lounge. Everyone can see everyone being seen.

-> first_contact

=== first_contact ===
Mara opens the live crisis channel. The delay indicator jitters. Sol stares into three cameras at once and manages to look personally betrayed by all of them.

// ghostlight.branch: branch-01-open-as-witness; action: speak; intent: recognize Sol before negotiating terms.
* [Speak to Sol as a witness before making demands.]
    "Sol Vant. I'm Mara. I am watching the same cut they gave everyone else, and I do not trust it to be complete. Talk to me before the room turns you into an ending."
    ~ mara_trust += 2
    ~ audience_heat += 1
    ~ recognition_given = true
    Sol blinks at the use of his whole name. The audience counter climbs because sincerity, like fire, is apparently also engagement.
    -> proof_or_face
// ghostlight.branch: branch-01-custody-demand; action: speak; intent: secure the arbitration proof before Lucent edits around it.
* [Freeze custody on the arbitration proof before anyone performs emotion.]
    "Before anyone performs remorse or outrage, I am freezing custody on the arbitration proof. Feed Ops, mark the collapsed packet as evidence, not content."
    ~ evidence_integrity += 2
    ~ sol_stability -= 1
    ~ security_pressure += 1
    Sol's jaw tightens. Evidence makes him real, but custody sounds like another office taking him away from himself.
    -> proof_or_face
// ghostlight.branch: branch-01-route-side-panels; action: use_object; intent: reduce influencer interference before first negotiation exchange.
* [Route Pippa and Brant into delayed side panels before opening the mic.]
    Mara drags Pippa and Brant into delayed side routing before opening her mic.
    ~ feed_control += 2
    ~ influencer_noise -= 2
    ~ sol_stability -= 1
    ~ optics_mistrust = true
    The lounge feed cleans up. Too clean. Sol sees the panel change and laughs once, sharp and ugly.

    "Oh, good. Private editing. My favorite public service."
    -> proof_or_face

=== proof_or_face ===
Feed Ops surfaces the collapsed arbitration packet: debt review denied, appeal window vanished, contract penalties rebundled into a creator wellness initiative with excellent fonts.

Juno's hand is half-hidden against the glass rail. She is breathing like a person trying to compress panic into technical documentation.

{evidence_integrity >= 2: The proof trail has a custody seal now. It glows in the feed corner, small and legally annoying.|The proof trail is still moving through Lucent's systems, which means it is wearing a paper hat and walking into a wood chipper.}
{optics_mistrust: Sol keeps checking the panel edges, hunting for the edit he cannot see.}

// ghostlight.branch: branch-02-freeze-edit-trail; action: use_object; intent: lock the proof trail through official evidence custody.
* [Authorize an evidence freeze and force the edit trail onto the public rail.]
    ~ evidence_integrity += 2
    ~ security_pressure += 1
    ~ feed_control += 1
    The packet stops refreshing. Feed Ops complains in amber. Legal complains in red. Sol watches the frozen trail like it might finally say his name without charging him rent.
    -> influencer_pressure
// ghostlight.branch: branch-02-create-gap-for-juno; action: use_object; intent: give Juno a narrow timing gap to signal without making her look like the operator.
* [Drop a two-second calibration banner that might hide Juno's hand.]
    ~ juno_gap = true
    ~ feed_control += 1
    {sol_stability <= 1 or optics_mistrust:
        ~ hostage_safety -= 1
        ~ sol_stability -= 1
        Sol notices Juno's shoulder move. Not the whole thing, not enough to prove anything, but fear is famously bad at evidentiary standards.

        "No little architect tricks," he says, pulling her tighter.
    - else:
        ~ evidence_integrity += 1
        ~ mara_trust += 1
        Juno taps once under the glare. A checksum appears on Mara's rail, ugly and useful.
    }
    -> influencer_pressure
// ghostlight.branch: branch-02-debt-review-for-movement; action: speak; intent: trade public recognition and review for moving Juno away from immediate harm.
* [Offer public debt review if Sol moves Juno away from the elevator glass.]
    "I can put the debt review on the main rail, live, with your packet attached. In exchange, you move Juno one full step from the elevator glass and keep both of her hands visible."
    ~ hostage_safety += 2
    ~ mara_trust += 1
    ~ evidence_integrity -= 1
    ~ recognition_given = true
    ~ lucent_reframe = true
    ~ juno_distance_from_glass = 1
    Sol swallows. It is not justice. It is a receipt with oxygen in it. Feed Ops immediately suggests the lower-third: CREATOR DEBT REVIEW BEGINS.
    -> influencer_pressure

=== influencer_pressure ===
Pippa's side panel pulses with permission requests. Brant's panel pulses harder, because outrage has never met a door it did not headbutt.

{feed_control >= 2: Mara has enough routing authority to make the next interruption surgical.|Mara's routing authority is thin. The side feeds push against the delay like dogs against a screen door.}
{audience_heat >= 3: Audience heat climbs into the red fringe. The room is becoming a sport.|Audience heat is noisy but not yet feral. A modest miracle, probably billable.}
{influencer_noise <= 0: The side panels are quiet for now, which on Lucent counts as paranormal activity.}

// ghostlight.branch: branch-03-pippa-tight-routing; action: wait; intent: allow limited empathy framing without surrendering control.
* [Let Pippa speak through tight routing and a delay kill-switch.]
    ~ pippa_spoke = true
    {feed_control >= 2:
        ~ audience_heat -= 1
        ~ mara_trust += 1
        Pippa says, "Sol, I don't know your whole story, but I can see someone taught the room to look away from it."

        It is almost clean. Sol hates that it lands.
    - else:
        ~ influencer_noise += 2
        ~ audience_heat += 1
        Pippa says, "If you're watching this and feeling activated, breathe with me."

        Sol laughs like a cable snapping. "Is she holding the knife or am I?"
    }
    -> movement_gate
// ghostlight.branch: branch-03-brant-targets-lucent; action: wait; intent: redirect outrage toward Lucent while avoiding direct humiliation of Sol.
* [Let Brant attack Lucent, with one instruction: not Sol.]
    ~ brant_spoke = true
    ~ audience_heat += 1
    {recognition_given:
        ~ mara_trust += 1
        Brant snarls, "This is what happens when a company turns debt into casting. Look at the contract, not the hostage's haircut."

        Sol flinches at the joke, then hears the first half.
    - else:
        ~ sol_stability -= 1
        Brant snarls, "Lucent built the trap and now this guy wants applause for bleeding in it."

        Correct target, wrong blade. Sol's mouth goes flat.
    }
    -> movement_gate
// ghostlight.branch: branch-03-hard-mute-panels; action: use_object; intent: remove influencer volatility and accept suppression optics.
* [Hard-mute both panels and pin the official crisis feed as the only live channel.]
    ~ panels_muted = true
    ~ feed_control += 2
    ~ influencer_noise -= 2
    ~ audience_heat += 1
    The feed becomes disciplined. The audience reads discipline as concealment, because sometimes the mob is not wrong, merely early and useless.
    -> movement_gate

=== movement_gate ===
Security waits beyond the lounge doors in Lucent gray, visible enough to threaten and distant enough to be late. Juno's eyes flick toward the safe-line painted on the floor: a white arc two careful steps from the elevator-side glass rail.

{hostage_safety >= 3: Juno has a little space now. Not freedom. A little space, which is the unit freedom uses when it is still negotiating.|Juno is still inside Sol's reach in every way that matters.}
{security_pressure >= 3: Breach pressure rises. Someone off-feed has found a heroic number and fallen in love with it.|Security pressure is ugly but not yet commanding the room.}
{sol_stability <= 0: Sol's performance is fraying into reflex. He is still reachable, but the channel is narrow.|Sol is frightened, angry, and still arranging himself into sentences. That is something.}

// ghostlight.branch: branch-04-safe-line-step; action: move; intent: convert negotiation trust into measurable hostage distance.
* [Ask Sol for one visible step to the safe line.]
    {juno_distance_from_glass >= 1:
        "One visible step to the safe line, Sol. Not surrender. Just proof that you still decide what happens before the breach team decides for you."
    - else:
        "Two careful steps to the safe line, Sol. Not surrender. Just proof that you still decide what happens before the breach team decides for you."
    }
    ~ safe_line = true
    {mara_trust >= 2 or recognition_given:
        ~ hostage_safety += 2
        ~ security_pressure -= 1
        Sol drags the moment out for the cameras, then moves. {juno_distance_from_glass >= 1: Juno crosses the white arc with one foot.|Juno takes the first step like the floor might invoice her for it, then crosses the white arc with one foot.} She does not thank anyone, which is the most technically sound response available.
        ~ juno_distance_from_glass = 2
    - else:
        ~ sol_stability -= 1
        Sol shakes his head. "Now it's choreography. Now I hit my mark and they call it progress."
    }
    -> contained_resolution
// ghostlight.branch: branch-04-face-saving-handoff; action: speak; intent: offer Sol a public exit that does not reward harm beyond recognition and review.
* [Offer a face-saving handoff script with the humiliating parts stripped out.]
    "You release Juno to the marked line. I keep your name on the evidence packet. You do not vanish into a breach clip."
    ~ handoff_attempted = true
    {recognition_given and audience_heat <= 3 and influencer_noise <= 2:
        ~ mara_trust += 2
        ~ sol_stability += 1
        ~ hostage_safety += 2
        ~ safe_line = true
        ~ juno_distance_from_glass = 2
        Sol looks at the cameras, then at the packet, then at Juno. He hates the ritual. He also understands rituals are doors when walls have been professionally monetized. He opens his arm enough for Juno to cross the white arc without making it look like he has been edited into obedience.
    - else:
        ~ sol_stability -= 2
        ~ audience_heat += 1
        The handoff script hits the room like a branded apology. Sol sees the shape of a clip forming around him and nearly breaks it with Juno's body.
    }
    -> contained_resolution
// ghostlight.branch: branch-04-security-recedes; action: use_object; intent: lower immediate breach pressure by visibly moving security back.
* [Authorize the corridor camera to show security taking three steps back.]
    ~ security_receded = true
    ~ security_pressure -= 2
    ~ sol_stability += 1
    ~ mara_trust += 1
    ~ lucent_reframe = true
    Sol exhales before he can turn it into a line. Feed Ops immediately flags CONTROL LOSS RISK, because mercy photographs badly unless sponsored.
    -> contained_resolution

=== contained_resolution ===
The next seconds do not become clean. They become countable.

{juno_gap and evidence_integrity >= 2: Juno's hidden checksum holds against the frozen packet. The proof has teeth now.}
{lucent_reframe: Lucent's lower-thirds keep trying to rename the event into something with less liability and more vowels.}
{panels_muted: Outside the official feed, muted influencers mime outrage through thumbnails. The silence becomes its own rumor.}
{pippa_spoke: Pippa's viewers flood the rail with soft-focus fury. Some of it lands on Sol as pity. Some of it lands on Mara as branding.}
{brant_spoke: Brant's clip is already moving faster than the crisis feed, dragging Lucent's contract machine into the center whether Mara likes his method or not.}
{safe_line: The white arc has become more than paint now. It is the room's cheapest border between negotiation and obituary.}
{security_receded: The corridor camera still shows security held back from the lounge doors, a visible absence doing more work than a threat.}
{handoff_attempted: The stripped-down handoff script waits on Mara's rail, useful only if Sol can touch it without feeling the audience's teeth.}

{hostage_safety >= 4 and evidence_integrity >= 3 and sol_stability >= 1:
    -> ending_safe_evidence
- else:
    {hostage_safety >= 4 and evidence_integrity < 3:
        -> ending_safe_compromised
    - else:
        {security_pressure >= 3 or sol_stability <= -1:
            -> ending_breach
        - else:
            {hostage_safety >= 3 and lucent_reframe:
                -> ending_devoured
            - else:
                -> ending_truce
            }
        }
    }
}

=== ending_safe_evidence ===
Juno steps clear on the safe line. Security does not surge. Sol lets the room see his empty hand before anyone can edit it into a weapon.

The arbitration packet remains frozen with checksum and custody intact. It is not justice yet. It is evidence with witnesses, which is justice's less glamorous cousin and sometimes the one that survives.

Mara keeps her mic open until Sol is contained, breathing, furious, and not dead.

-> END

=== ending_safe_compromised ===
Juno gets clear. That is the part Mara lets herself count first.

By the time Sol is contained, Lucent has already softened the packet into a debt-review narrative with generous lighting. The proof is not gone, but it has been dressed for a meeting where truth is expected to apologize for tone.

Juno looks at Mara through the feed and mouths, "I had worse drafts."

-> END

=== ending_breach ===
The breach order arrives through timing, not language: security bodies crossing from the lounge doors before the feed admits they have moved.

Mara hears herself say Sol's name. The moderation delay eats the edge off it. In the lounge, Juno drops toward the white arc, Sol twists between her and the elevator-side glass rail, and the cameras choose angles with obscene competence.

Everyone survives or almost does. The feed calls it decisive. Mara does not.

-> END

=== ending_devoured ===
Juno is released into security hands. Sol is contained without blood on the glass.

Then Lucent eats him live.

The lower-third becomes a moral diagram: unstable creator, brave platform, lessons learned. The debt packet remains visible just long enough to prove the system can display evidence while digesting it.

Mara signs off before her face can become the final kindness in the clip.

-> END

=== ending_truce ===
No one moves enough to call it release. No one breaks enough to call it breach.

Sol lowers his grip by an inch. Juno gets one hand visible. Mara keeps the debt packet on the rail and the breach team out of the frame. The audience hates the lack of climax with the moral force of people denied a purchased dessert.

The feed stays live. The truce is ugly, temporary, and real.

-> END
