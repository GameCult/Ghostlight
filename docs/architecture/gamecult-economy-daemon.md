# Njörðr, the GameCult economy daemon

Status: specification, 2026-09-11. Nothing is built. The daemon is Njörðr,
repository `GameCult/Njordr` (ASCII spelling for the repo; the name keeps its
letters in prose); no repository exists yet.

## Objective

One economy organ serving Aetheria and Delvehold, in which every item is the
product of a supply chain, materials are interchangeable within a role at
different trade-offs, prices are a pure function of committed state, and the
economy both produces from player choices and reacts to them. Consequences
authored in Ghostlight (a closed route, a defaulted promise, a witnessed
collapse) reach players as prices; player choices (what was crafted from
what, where it was sold) reach Ghostlight's people as facts and pressures.

## What the two games already hold

Aetheria's shared model (`Assets/Scripts/ServerShared/ItemData.cs`,
`ItemInstance.cs`, `ItemManager.cs`) carries a scalar `Quality` on every
crafted instance, a `Price` integer on the item datum, and stats evaluated as
`lerp(min, max, quality^exponent)`. The legacy model kept the better idea and
lost it to a breach: a crafted instance remembered every ingredient instance,
and a stat named the ingredient whose quality it read
(`Aetheria-legacy/Assets/Scripts/Item.cs` line 385). Delvehold's design
(`Game Design/HOLD/Production and Logistics.md`) already states the target
rule: "Every transformation records recipe or method, consumed and produced
lots, operator or automation, facility, time, quality-relevant conditions,
and provenance," and "players specialize sites, trade, commission, use
institutional services, or accept less efficient substitutes." The daemon is
the single owner of that rule for both games.

Ghostlight keeps a unitless conserved ledger and no prices, by construction
(`Quantity` has no unit "because a unit is where prices enter"). That stance
does not change. The daemon is where units, properties, and prices live, and
it reaches Ghostlight only through the consumer contract
(`ghostlight-world-consumer-api.md`) as one more consumer that owns market
mirrors.

## The model

Six nouns. Everything else is derived.

**Dimension.** A named property axis with a unit tag, authored per world:
`thermal_conductivity (W/m·K)`, `density (kg/m³)`, `tensile_strength`,
`corrosion_resistance`, and in other worlds `aether_conductance`,
`mana_capacity`, `hex_resistance`. Some dimensions are **operating
conditions** rather than properties: `temperature`, `pressure`, `aether
density`. A property value is either a scalar or a curve over one condition
dimension, so a vapour chamber's `heat_transfer_capacity` is a curve over
`temperature` that falls to nothing outside its working band, while a copper
block's is a flat line. Curves are the same piecewise shape Aetheria's
shared model already uses for heat performance. The daemon is genre-neutral
because a dimension is data. A world declares its dimensions once; nothing
in the daemon names one.

**Material.** A class of stuff with a property vector over the world's
dimensions and a base extraction or synthesis source. Copper, aluminium, gold
are three materials sharing most dimensions with different values. A
material may itself be the product of a recipe (an alloy, a refined ore, a
grown culture), in which case its property vector is derived from its lot,
not authored.

**Lot.** A quantity of one material at one place with one provenance:
extracted at a site, or produced by one recipe run from named input lots. A
lot's property vector is its material's for an authored material, or the
recipe's output function applied to its inputs for a produced one. Lots are
conserved: a recipe run consumes input lots and produces output lots, a
transfer moves a lot between holders and places, and nothing else creates or
destroys quantity except an extraction with a source and a consumption with a
sink. Provenance is the DAG of lot ids through recipe runs, acyclic by
construction because a run's outputs are minted after its inputs are
consumed. This is the legacy "remembers every ingredient," made durable and
queryable rather than carried on the instance.

**Recipe.** A transformation stated as roles, not ingredients. Each role names
a functional requirement over dimensions, a quantity, and how the output's
properties read the lot filling the role. A role is filled by any lot, raw
or produced: `spreader: heat_transfer_capacity ≥ 100 over the recipe's
temperature envelope, density ≤ 3000` is filled by a block of copper, by a
produced vapour-chamber part, or by whatever the world's physics offers,
and the recipe never names which. A requirement over a curve-valued property
states the envelope it must hold across, so a vapour chamber that fails hot
fills a cool recipe and not a hot one. A recipe also names a **process**
role: the facility, tool, and practice that run it, which carry their own
dimensions (`precision`, `throughput`, `contamination`) and are filled by
the workshop's actual station and operator. Substitution is therefore not a
special case and needs no optional roles or alternate recipes: any lot
meeting the role's requirement fills it, the output's derived properties
carry the difference (a thruster's cooling curve is its spreader's capacity
curve through the recipe's output function), and the market prices it.

**Product.** A produced lot with a property vector, a product class, and its
provenance. There is no scalar quality on the product. Game stats read the
product's properties by dimension the way the legacy stat read a named
ingredient's quality: a thruster's output reads its conductor's
`thermal_conductivity`, its housing's `density`, and its process's
`precision`. Rarity tiers, star ratings, and "quality" are projections a game
computes for display from the vector, never inputs. A scalar quality on the
instance is what the breach left; it does not come back.

**Shipment.** A lot in motion: one or more lots, a carrier subject, a route,
a departure revision, and an arrival derived from the route's cost and the
carrier's speed. While a shipment is in transit its lots are held by the
shipment, not by the sender or the receiver, so a seized cargo is a custody
change on the shipment and nothing else has to be repaired. A shipment is
how economic activity becomes concrete: a price gap across a route is not
closed by an arithmetic transfer but by a carrier deciding to move a lot,
and that carrier, its cargo, and its position along the route are world
facts a player can meet. The daemon owns the shipment ledger; it owns no
carrier's decision to sail.

Two consequences of the model that the games asked for:

- An exceptional item is a product of a well-managed supply chain by
  construction. Its properties are its inputs' properties through the
  recipe's output function, and its provenance names every choice that made
  it so.
- "Functionally interchangeable with different trade-offs" is the recipe's
  role requirement plus the market's price. The daemon never ranks materials;
  the dimensions and the prices do.

## Markets and prices

A **market** is a place-bound institution with a liquidity reserve per
material class and a demand profile. In Ghostlight terms a market is an
externally controlled mirror subject the daemon owns: Ghostlight's people may
stand in it, transact toward it, witness facts in it, and never mutate it.
Several markets exist per world, one configured mirror each.

A **price** is a pure function of committed daemon state at a revision: the
market's reserve of the class, its demand profile, the route cost to the
nearest markets holding the class, and a liquidity parameter. Use the
standard mechanism rather than a bespoke one: a per-market, per-class
automated market maker (a logarithmic scoring rule or constant-function
curve, chosen at build for its bounded loss) quoting against the reserve,
with tatonnement between markets carried by shipments, not by the daemon: a
price gap across a route is an opportunity for a Ghostlight subject or a
player to move a lot, the move is a shipment, and its arrival is what closes
the gap. The daemon owns no trader and never moves a lot on its own.

**Demand** is a rate, not a wish. A demand row on a market is a consumption
rate per class per unit of simulation time plus a reserve target, and
consumption is a real lot sink, so a market that is oversupplied has its
reserve drawn down at the rate its people use the class and its price finds
a floor there rather than at zero. Rows enter three ways and are typed the
same way: a Delvehold Hold's ratified policy (mandates, reserves, rationing);
a Ghostlight population's needs as pressure and dependency deltas through
the outbound batch; a player's or a workshop's standing orders. None is
privileged. **Intermediate demand is derived, never authored**: a producer
policy (below) that would run a recipe at current quotes contributes its
input needs as demand rows on the markets it buys from, so a crash in a
product's price stops its producers, withdraws their demand for inputs,
lowers the inputs' quotes, and cheapens everything else made from them. That
propagation through the recipe graph, in both directions, is how a player
who floods one class moves the prices of classes they never touched.

**Admission** is a market policy over provenance. Every lot's provenance is
permanent and a seizure is in it forever, so a market may refuse, accept, or
discount a lot by what its provenance contains and how deep it looks. A
legitimate market refuses a seizure at any depth; a black market accepts it
at a discount; a lax market looks one level deep, so a re-smelted seized
ingot passes there and nowhere stricter. A black market is therefore an
ordinary market with a permissive admission policy at its own place, and a
diverted shipment is still in play: its lots re-enter supply wherever they
are admitted, and the shortage they caused at their destination is answered
by whoever the destination's quote draws.

**Cornering** is possible and expensive by construction. Buying a reserve out
walks the quote up the scoring rule's curve, and the maker's loss is bounded,
so a market cannot be bankrupted by a corner. What answers a corner is
supply: price gaps pull shipments, and producers make more. A market that no
carrier's routes reach and no producer serves is a shortage nobody answers,
which is why seeding carries the rule below.

**Quotes.** A proposed trade is quoted against a named revision and honoured
only against that revision; a stale quote is refused, never repriced
silently. This is the same idempotency and staleness rule the Ghostlight
consumer contract uses, and it is what makes a price replayable: replaying
the committed lot and demand events reproduces every price ever quoted.

## The two directions

**Ghostlight to prices.** The outbound consumer batch (next on Ghostlight's
map) is designed as this daemon's signal surface: custody deltas by holder,
resource, and place; pressure and dependency deltas by subject; route open,
close, and cost changes; witnessed events by place subtree. The daemon lowers
custody deltas onto its lots for Ghostlight-owned holders, pressure and
dependency deltas onto demand profiles, route changes onto its route-cost
table, and events onto shocks. A collapsed grate above a basin town is a
route closure and a witnessed event; its price consequence is derived, not
authored.

**Prices to Ghostlight.** The daemon writes back through consumer patches: a
price at a market is a fact witnessed at the market place, landing as
knowledge on everyone standing there and reaching a Persona as prose; a
shortage is a pressure on the holders who depend on the class; a realized
trade is a custody change on the Ghostlight subject, with the daemon's
receipt as evidence. A Ghostlight person's intent to buy exits as an
attributed proposal, the daemon quotes and executes it against its revision,
and the realized custody returns through the receipt. One tick of latency
between wanting and holding is the design, and the Projector renders a
pending intent as "asked, not yet answered".

**Players to prices.** A game host submits lot events as typed CultMesh
commands with receipts: extract, run recipe, transfer, consume, sell, buy.
The daemon validates against the recipe catalog and the lot ledger, commits
atomically, and the market maker's next quote reflects the new reserve. A
player who floods a market with aluminium spreaders lowers their price and
raises copper's relative standing; a workshop that buys every lot of a scarce
conductor raises its price in every market a route reaches.

## Two consumer profiles

The model is one; the granularity at which a game consumes it is not.

**Delvehold, the boundary profile.** All player agency occurs inside the
Greathold, which is a strict boundary. Njörðr owns the interior economy as
much as the exterior: a Hold's workshops, facilities, inventories, and
orders are its lots, recipes, and markets, and a Hold's ratified policy
reaches it as demand rows and carrier policies. What the boundary bounds is
Ghostlight's reach, not Njörðr's: the outside world reaches players as
arrivals, prices, and news at the boundary subject, batched per tick; a
shipment outside the Greathold is abstract until it arrives, and no player
can meet it on the road. The integration note
(`delvehold-forced-ontology-integration.md`) describes the Ghostlight side
of this shape; its "Delvehold owns its own quantitative economy" sentence is
superseded by this document.

**Aetheria, the interspersed profile.** Player actors are spread through the
whole world and get to meddle with all of it. Nothing economic may stay
abstract where a player is: a shipment on a route a player is flying is a
ship with a hold full of lots, and piracy is the canonical case. This needs
three things the boundary profile does not:

- Materialization. The Aetheria host asks Njörðr for the shipments whose
  derived position lies inside a zone a player occupies, and instantiates
  each as an entity whose cargo is the shipment's lots by id. The entity is
  a rendering of the shipment, bound to its id and revision; it is not a
  second truth about where the cargo is. When no player is present the
  shipment progresses abstractly by route cost and time, and the two
  descriptions must agree at the boundary: a materialized ship's position is
  the derived one at the moment of materialization, and its arrival is
  whichever comes first, the derived arrival or the host's committed docking.
- Interception. A seizure is a command from the host carrying the shipment
  id, the revision it was materialized against, the seizing holder, and the
  combat receipt as evidence. Njörðr moves the lots from the shipment to the
  seizer, marks the shipment lost, and refuses the command if the shipment's
  revision moved (it arrived, or was already taken) exactly as a stale quote
  is refused. The same command shape serves a legitimate boarding, a
  customs seizure, a salvage, and a wreck.
- Cadence. The interspersed profile emits and consumes events per zone as
  they happen, not per world tick. A seizure reaches Ghostlight at once as a
  custody change on the carrier's owner, a witnessed event over the route's
  places, and a pressure on whoever was owed the cargo; the destination
  market's next quote reflects the lot that will not arrive. Consequence is
  expressed as price, as news, and as a promise now in default, and all
  three are derived from the one committed seizure.

Both profiles use the one lot ledger, the one shipment ledger, and the one
quote function. The difference is which events a host subscribes to and at
what grain, which is subscription configuration, not a second model.

## Authority map

- Owner: Njörðr owns dimensions, materials, lots, recipes, products,
  shipments, markets, demand rows, and prices for every world it serves. One
  process, one CultCache store per world, one revision counter per world.
- Inputs: authored catalogs (dimensions, materials, recipes, facilities) per
  world; lot and trade commands from game hosts; the Ghostlight outbound
  batch; policy demand documents from Delvehold's civic organs.
- Outputs: price and reserve projections per market as CultMesh state
  lowered to Eve; quotes and receipts; consumer patches to Ghostlight;
  provenance and property queries for any lot.
- Derived state: every price, every product property vector, every rarity
  projection, every "quality". None is stored as truth; all are recomputed
  from lots, recipes, demand, and routes at a revision.
- Forbidden writers: a game host writes no price and moves no lot except by
  a command with a receipt; a materialized entity is never the position of
  record; Ghostlight writes no lot;
  the daemon writes no Ghostlight component except through a consumer patch
  the kernel validates; no client, projection, cache, or Eve surface commits
  anything; no material carries a scalar quality; no recipe names a material
  where a role will do.
- Shared paths: player trades, Ghostlight-subject trades, institutional
  procurement, and replay all pass through the one lot ledger and the one
  quote function. There is no second price path for NPCs.
- Deletion line: Aetheria's `ItemData.Price`, `CraftedItemInstance.Quality`,
  and `ItemManager.GetPrice` become projections read from the daemon or are
  cut; the commented-out `Ingredients` and `Blueprint` fields are not
  restored, because provenance lives on the lot.

## Engine and shell

Njörðr is two things, and the split is load-bearing. The **engine** is a
pure, deterministic step function over a state document, a stream of typed
events, and a clock: it owns the lot and shipment ledgers, the recipe and
property derivation, the quote function, carrier policy execution, and
replay, and it performs no I/O, reads no environment, and holds no port. The
**shell** is the daemon: CultMesh discovery, the Eve composition graph, the
Ghostlight consumer port, the game host command port, the CultCache store,
and lifecycle. The shell feeds the engine events and persists what it
returns. Nothing in the engine knows whether a shell exists.

Two consequences:

- Every input reaches the engine as an **exogenous event** of one typed
  shape, whatever its origin: a Ghostlight outbound batch lowered by the
  shell, a game host's lot command, a Delvehold policy document, or an
  authored script. The engine cannot tell where an event came from, and that
  is the invariant that makes the single-player mode below the same machine
  rather than a second one.
- Nobody decides inside the engine. Carriers execute **carrier policies**
  and producers execute **producer policies**, and both are data. A carrier
  policy is a home market, a set of known routes, a capacity, the classes it
  carries, a minimum margin after route cost, and a risk term per route that
  rises with seizures on it and decays with time; each step the carrier
  sails the best known route above its margin. A producer policy is a
  facility, the recipes it can run, and a margin; each step it runs the
  recipe whose output quote clears its input quotes plus margin, and its
  planned inputs are the derived demand above. Online, a Ghostlight subject
  amends its own policy through a consumer command, and its name, route,
  and grudge stay Ghostlight's. Offline, the seeded policies run unchanged.
  The engine owns no trader and no maker in either mode; it owns the
  executor of authored policy. Seeding rule: every market is on at least one
  carrier's route set or is marked unserved, and unserved is a fact
  Ghostlight receives, because a shortage nobody can answer is a story, not a
  bug.

## Modes

**Online.** The shell runs as a daemon beside Ghostlight and the game hosts.
Ghostlight's outbound batch is lowered to exogenous events; prices and
seizures return through consumer patches; both games' hosts submit lot
commands with receipts. This is the multiplayer body.

**Seeded single-player, Aetheria.** The full machine runs once to make a
world: Ghostlight and Njörðr together seed the initial state, and a separate
branching-narrative pass, not yet begun, authors what the story needs to
happen. The handover artifact is exactly two documents plus the engine:

1. the engine's state document at the seed revision, as CultCache; and
2. an **exogenous event script**: authored demand rows, shocks, route
   changes, and carrier policy amendments keyed to simulation time, produced
   by the seeding run and the narrative pass.

The client then runs the engine locally with a deterministic clock and a
seeded `CultMath.Random`, replays the script as time passes, and applies
the player's own lot commands into the same stream. "Precalculated narrative
price movements" are therefore precalculated **events**, never precalculated
prices. A script that pinned prices would make the player's choices inert
against the story; a script of shocks and demand keeps every price a pure
function of committed state, so the player who floods a market during a
scripted shortage moves the outcome, and the story's pressure still lands.
Save and load are engine checkpoints; a checkpoint plus the script replays
to the same state.

The engine must therefore be embeddable in the Aetheria client. Rust with a
C ABI as a Unity native plugin is the default under Odin-class doctrine; a
second implementation of the engine in C# is refused, because two
implementations of a deterministic step function are two truths the moment
one is patched.

## Why a daemon

An organ earns a process only when independent lifecycle, privilege,
resource, or failure isolation protects a named invariant. The invariants
here are lot conservation and price determinism across two game hosts that
start, stop, and fail independently of each other and of Ghostlight's world
ticks. A price computed inside either game host would fork the ledger the
moment the other host wrote; a price computed inside Ghostlight would put
units into a kernel built to have none. The daemon is the one place both
hosts and Ghostlight can agree a lot moved.

## Substrate

CultCache `.cc` state, one store per world; CultNet typed documents for
commands, receipts, and projections; CultMesh for discovery and the Eve
composition graph (price boards, reserve gauges, provenance views, TUI
tables). Rust, matching Odin-class doctrine: the engine as pure crates (the
ledgers, the market maker, property derivation, carrier policy execution,
replay) with no I/O and a C ABI for embedding, and the daemon as a thin
lifecycle shell around them. Clocks, transports, the Ghostlight port, and the
game host port are traits on the shell so a partial pipeline smoke can price
a market from a fixture ledger without the whole daemon, and the engine can
be driven from a script with no shell at all.

## Verification

Unit: lot conservation under every command; provenance acyclic; a recipe
refuses a material that fails its role; output properties are a pure
function of input properties and process; the quote function is
deterministic and its loss is bounded; a stale quote is refused; the same
state, script, and seed replay to the same state on two machines; an event
lowered from a Ghostlight batch and the same event read from a script
produce identical state.

Pipeline: a fixture world with copper, aluminium, and gold conductors, a
vapour-chamber part recipe whose capacity is a curve over temperature, one
thruster recipe with a spreader role fillable by the copper block or the
part, two markets on one route; the hot variant of the thruster recipe
refuses the vapour chamber and the cool one accepts it; flooding one market moves both prices in the direction the route cost
predicts; closing the route decouples them. A shipment between the two
markets, seized mid-route against its revision, lands its lots on the seizer,
is refused a second time, and leaves the destination's next quote higher
than it would have been on arrival; the seized lots are refused by the strict
market and admitted by the black market at a discount, and the destination's
shortage resolves through the carrier whose margin the gap now clears. A
producer flooding the spreader class walks its quote down the curve to the
consumption floor, withdraws the producer's conductor demand, and lowers the
conductor quote at the market it bought from; buying the gold reserve out
walks its quote up, and the far market's shipment answers it.

End to end: a Ghostlight world with a market mirror; a witnessed route
closure in Ghostlight arrives as a shock and moves a price; the price arrives
in Ghostlight as a fact witnessed at the market; a person's buy intent exits,
is executed, and returns as custody with a receipt; replay of the daemon's
journal reproduces every quote.

## Decisions

Taken 2026-09-11: the daemon is Njörðr; properties are dimensions, never a
scalar quality, because one axis cannot express a trade-off within a recipe;
process roles are dimensions like materials; Njörðr owns the economy whole,
Delvehold's interior included, so Greathold recipes, facilities, capacity,
inventories, orders, and prices are Njörðr lots, recipes, markets, and
quotes, and Delvehold's civic organs author policy as demand rows and
carrier policies rather than running a second ledger; the Unity embedding is
the Rust engine as a native plugin over a C ABI; the market maker is the
logarithmic market scoring rule, one liquidity parameter per market and
class, chosen for its bounded loss.

Delvehold has no existing economy surface to cut against: its world host
names a commons market id and nothing else. The shape is therefore
constrained only by the invariants above and by the verification fixtures
below, which is why those fixtures are written before the engine is.

Still the operator's:

1. Resolved by the engine split, and widened to producers: a shipment sails because a carrier policy
   fires, and the policy is data. Online, a Ghostlight carrier subject owns
   and amends its policy, so a pirate's victim has a name, a route, and a
   grudge; offline, the seeded policies run. What remains the operator's is
   the policy vocabulary's first cut: which thresholds and preferences the
   seeding run may author.
2. The policy vocabulary's first cut: recommend the route-bounded,
   gap-seeking carrier and the margin-driven producer described under
   "Engine and shell", with the risk term decaying in time and the margin
   per subject rather than global, so a desperate carrier sails what a
   cautious one refuses.
