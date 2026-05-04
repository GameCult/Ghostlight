VAR proof_integrity = 1
VAR sol_stability = 2
VAR juno_safety = 1
VAR security_pressure = 1
VAR audience_heat = 2
VAR influencer_noise = 2
VAR feed_control = 1
VAR receipt_exported = false
VAR pippa_centered = false
VAR brant_escalated = false
VAR juno_signaled = false
VAR bench_substitution_done = false
VAR demand_full_release = false

-> start

=== start ===
// ghostlight.fixture: lucent-hostage-feed.branch-and-fold.v0
// ghostlight.scene: lucent.transfer_atrium.crisis_entry
The Lucent media eye hangs above the city bubble like a corporate god with excellent lenses. Down-tether, the metropolis glitters inside its pressure dome. Up here, in the transfer atrium, every wall is a camera pretending to be architecture.

Sol Vant has one hand on Juno Ark's sleeve and one eye on the crisis feed. He is not calm. He is also not stupid. The feed that destroyed him is still trying to decide whether he is a monster, a product category, or tomorrow's most teachable clip.

Mara Quell enters as a voice first: warm, precise, and professionally allergic to any sentence that lets Lucent sell the ending too cleanly.

The official feed exposes an ugly EDIT TRAIL HOLD: source rows, splice marks, caption deltas, moderation touches, redacted operator IDs. It is the first object in the room Sol believes more than a person.

The first move is not morality. It is geometry.

-> first_maneuver

=== first_maneuver ===
// ghostlight.scene: lucent.branch.first_maneuver
Sol's fingers tighten when the influencer panels brighten. Juno's shoulders stay low. Security waits behind the service collars because the feed has not yet found a violence-shaped permission slip.

// ghostlight.branch: branch-01-center-edit-trail; action: speak; intent: Keep Sol's attention on verifiable evidence instead of audience humiliation.
* [Mara keeps the ugly edit trail centered and asks Sol for one smaller concession.]
    ~ proof_integrity += 1
    ~ feed_control += 1
    ~ sol_stability += 1
    ~ security_pressure -= 1
    -> center_edit_trail
// ghostlight.branch: branch-01-hard-mute-influencers; action: use_object; intent: Reduce side-panel provocation at the cost of making Lucent's control visible.
* [Feed ops hard-mutes Brant and Pippa before they can become negotiators by accident.]
    ~ influencer_noise -= 2
    ~ feed_control += 2
    ~ audience_heat += 1
    ~ security_pressure += 1
    -> hard_mute_influencers
// ghostlight.branch: branch-01-juno-visible-signal; action: gesture; intent: Let Juno become visible as a person without forcing her into testimony.
* [Juno makes a small visible signal that she is breathing and listening.]
    ~ juno_safety += 1
    ~ juno_signaled = true
    ~ audience_heat -= 1
    ~ sol_stability -= 1
    -> juno_visible_signal

=== center_edit_trail ===
// ghostlight.scene: lucent.branch.center_edit_trail
Mara does not tell Sol to be good. That would give him a wall to throw himself through.

"The receipt stays ugly," she says. "Named rows. Visible marks. No memorial rectangle wearing clean shoes. Give Juno more air and keep your demand where the room can inspect it."

For a moment Sol looks less like a man holding a hostage and more like a man realizing the hostage is the worst container for his proof.

-> physical_geometry

=== hard_mute_influencers ===
// ghostlight.scene: lucent.branch.hard_mute_influencers
The side panels dim. Brant's mouth keeps moving soundlessly. Pippa puts one hand to her chest with the tragic dignity of someone denied a market niche.

The room gets quieter. The audience gets angrier. Lucent's control has become visible, which is safer for Juno and worse for the brand.

Sol laughs once, too sharply. "Oh, so there is a button. Amazing what becomes technically possible when your pets start chewing the furniture."

-> physical_geometry

=== juno_visible_signal ===
// ghostlight.scene: lucent.branch.juno_visible_signal
Juno lifts one hand just enough for the cameras to see an open palm. Not escape. Not surrender. A living person reporting that she is still in the sentence.

The move helps the audience remember she is not an icon. It also reminds Sol that everyone can see exactly how close his hand is.

"Don't turn that into bravery," Juno says. "I am doing inventory on oxygen."

-> physical_geometry

=== physical_geometry ===
// ghostlight.scene: lucent.branch.physical_geometry
{sol_stability <= 1:
Sol's breath shortens. He is still listening, but listening has become work.
}
{sol_stability >= 3:
Sol watches the edit trail more than Juno now. The proof-object is doing some of the holding for him.
}
{influencer_noise <= 0:
The muted influencer panels hang in the corner like expensive aquarium fish denied pellets.
}
{influencer_noise > 0:
Brant and Pippa remain live enough to twitch the room whenever silence threatens to become useful.
}

Mara needs the next lever to be physical, boring, and survivable.

// ghostlight.branch: branch-02-bench-substitution; action: move; intent: Transfer Sol's grip from Juno's body to a nearby object without making him perform surrender.
* [Invite Sol to move his grip from Juno's sleeve to the bench edge beside her.]
    ~ bench_substitution_done = true
    ~ juno_safety += 2
    ~ sol_stability += 1
    ~ security_pressure -= 1
    -> bench_substitution
// ghostlight.branch: branch-02-demand-release; action: speak; intent: Push for immediate release and risk making Sol hear humiliation instead of safety.
* [Demand full release now, before Lucent can launder the evidence.]
    ~ demand_full_release = true
    ~ juno_safety += 1
    ~ sol_stability -= 2
    ~ security_pressure += 2
    -> demand_release
// ghostlight.branch: branch-02-let-pippa-appeal; action: wait; intent: Let an influencer humanize Sol, accepting the risk that she centers herself.
* [Let Pippa make one human appeal before cutting the side panel again.]
    ~ pippa_centered = true
    ~ influencer_noise += 1
    ~ audience_heat += 1
    ~ sol_stability += 1
    -> pippa_appeal

=== bench_substitution ===
// ghostlight.scene: lucent.branch.bench_substitution
Mara gives Sol an action small enough to survive being watched.

"Bench edge," she says. "Same room. Less body. Nobody has to discover mercy under lighting designed by a lawsuit."

Juno eases her sleeve toward the bench without pulling. Sol's fingers open one at a time and close on furniture instead of flesh.

The feed does not know what to do with brave furniture. This is its first decent quality.

-> evidence_decision

=== demand_release ===
// ghostlight.scene: lucent.branch.demand_release
"Let her go now," Mara says.

It is morally clean. That is the problem. Sol hears the shape of surrender before he hears the promise of safety.

His hand jerks away from Juno's throat but clamps harder at her sleeve. Security lights wake behind the collars, not entering yet, just reminding everyone that Lucent owns several kinds of ending.

-> evidence_decision

=== pippa_appeal ===
// ghostlight.scene: lucent.branch.pippa_appeal
Pippa's panel brightens. Her eyes are wet in a way that might be real and is definitely formatted.

"Sol," she says, intimate as a hand through glass, "you are still someone who can choose not to become this moment."

For half a breath, it works. Sol looks seen. Then the audience heat counter spikes because millions of people enjoy being present for forgiveness before the victim has been consulted.

Juno closes her eyes. Mara makes a note in the part of herself that still believes in hell.

-> evidence_decision

=== evidence_decision ===
// ghostlight.scene: lucent.branch.evidence_decision
{bench_substitution_done:
Juno is no longer the only thing anchoring Sol to the room. The bench edge is less poetic and much safer.
}
{demand_full_release:
The demand has made the right ending visible and the path to it narrower.
}
{pippa_centered:
The feed has tasted redemption content. It will ask for more unless someone breaks its little sugar teeth.
}

The evidence can now become a receipt, a spectacle, or a casualty.

// ghostlight.branch: branch-03-export-receipt; action: use_object; intent: Lock the edit trail to an external receipt before Lucent can repackage it.
* [Force feed ops to export the edit-trail receipt and name the external hold on air.]
    ~ receipt_exported = true
    ~ proof_integrity += 2
    ~ feed_control += 1
    ~ audience_heat += 1
    -> export_receipt
// ghostlight.branch: branch-03-safe-line-first; action: speak; intent: Prioritize Juno crossing the safe line before evidence escalation.
* [Keep the feed ugly, but move Juno to the marked safe line first.]
    ~ juno_safety += 2
    ~ sol_stability += 1
    ~ proof_integrity += 1
    -> safe_line_first
// ghostlight.branch: branch-03-brant-burns-lucent; action: wait; intent: Let Brant attack Lucent publicly, raising evidence pressure and human danger.
* [Let Brant spend one live minute attacking Lucent's edit machine.]
    ~ brant_escalated = true
    ~ audience_heat += 2
    ~ proof_integrity += 1
    ~ sol_stability -= 1
    ~ security_pressure += 1
    -> brant_burns_lucent

=== export_receipt ===
// ghostlight.scene: lucent.branch.export_receipt
Feed ops chooses the least beautiful button.

The receipt leaves Lucent's house archive and lands in an external hold whose name is too dull to trend. That is why it might survive.

Sol sees the export marker. The hand that is not holding anything trembles in public.

"Good," he says. "Now it has somewhere to live when you all start redecorating the corpse."

-> resolution

=== safe_line_first ===
// ghostlight.scene: lucent.branch.safe_line_first
Mara points the whole room at the safe-line marker.

"Juno walks. Slowly. No breach cue. No victory graphic. No one improves the moment until both her feet are on the line."

Juno moves as if every joint has been asked to submit a legal memo before bending. The room watches her become less useful as leverage.

-> resolution

=== brant_burns_lucent ===
// ghostlight.scene: lucent.branch.brant_burns_lucent
Brant's audio comes through like a chair thrown into a server rack.

"There it is. Emotion-contour optimization, sponsor-risk approval, grief clipping as an industrial process. Lucent didn't cover a tragedy. Lucent manufactured a handle and charged the audience admission to watch Sol bleed from it."

The words are not wrong. That makes them dangerous. Sol's face changes when his pain becomes someone else's weapon, even against the right target.

-> resolution

=== resolution ===
// ghostlight.scene: lucent.resolution.folded_outcome
{juno_safety >= 4 and sol_stability >= 2 and proof_integrity >= 3:
Juno reaches the safe line with both hands visible. Security remains behind the collars because the feed has named restraint more loudly than violence. Sol stands empty-handed in a box labeled NON-ADVANCING PARTY: SECURITY HANDOFF PENDING, which is not mercy, not justice, and not a clip anyone can sell without leaving the receipt in frame.

The export marker stays live. Lucent has not become honest. It has merely been forced, briefly, to keep one ugly thing visible.

-> END
}
{security_pressure >= 4 or sol_stability <= 0:
The service collars open three centimeters before the hold order catches up. That is enough. Everyone sees the almost-breach: drone apertures waking, Sol flinching, Juno freezing because bodies know geometry faster than law.

Mara forces the feed to show the near-miss twice. Not as drama. As evidence that even rescue can become a weapon when the room wants a clean ending too badly.

Juno survives the beat, but the safe line feels farther away now. The scene ends in a hard hold, with the receipt alive and every future option more expensive.

-> END
}
{receipt_exported:
The receipt survives before the room resolves. That changes the math. Lucent can still hurt people, but it cannot make the edit trail vanish quietly.

Juno remains under protected movement. Sol remains visible and not forgiven. Mara has won nothing except a documented problem the system now has to route around.

-> END
}
{juno_safety >= 3:
Juno reaches the safe line, but the evidence remains inside Lucent's custody. The feed calls it preservation. Mara hears packaging. Sol hears theft with better diction.

The hostage phase ends. The story does not. Somewhere behind the overlays, the edit machine is already learning how to describe what happened without confessing what it did.

-> END
}
{proof_integrity >= 3:
The edit trail stays visible, but Juno is still too close to the violence that made it visible. The room has protected the receipt better than the person.

Mara knows it. Sol knows it. Juno definitely knows it. The next move will have to choose which shame becomes survivable.

-> END
}
The feed folds the crisis into an ambiguous hold: no breach, no release, no exported receipt. The audience calls it tension. The people in the room call it being trapped inside someone else's format.

-> END


