# Njörðr, the GameCult economy daemon

Status: specification, 2026-09-11. Nothing is built. The daemon is Njörðr;
no repository exists yet.

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
`mana_capacity`, `hex_resistance`. The daemon is genre-neutral because a
dimension is data. A world declares its dimensions once; nothing in the
daemon names one.

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
a functional requirement over dimensions (`conductor: thermal_conductivity ≥
200`, `spreader: thermal_conductivity ≥ 100 and density ≤ 3000`), a quantity,
and how the output's properties read the material filling the role. A
recipe also names a **process** role: the facility, tool, and practice that
run it, which carry their own dimensions (`precision`, `throughput`,
`contamination`) and are filled by the workshop's actual station and
operator. Substitution is therefore not a special case: any material meeting
the role's requirement fills it, and the trade-off is visible in the output's
derived properties and in the price paid. A heat spreader with a vapour
chamber and one without are two recipes for one product class, or one recipe
with an optional role, and the market prices the difference.

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

**Demand** enters three ways and is typed the same way: a Delvehold Hold's
ratified policy (mandates, reserves, rationing) as a demand profile on its
markets; a Ghostlight population's needs as pressure and dependency deltas
through the outbound batch; a player's or a workshop's standing orders. All
three are demand rows on a market at a revision. None is privileged.

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
Greathold, which is a strict boundary. The outside world's economy reaches
players as arrivals, prices, and news at the boundary subject, batched per
tick; a shipment outside the Greathold is abstract until it arrives, and no
player can meet it on the road. The integration note
(`delvehold-forced-ontology-integration.md`) already describes this shape,
and Njörðr serves it with one market mirror per Hold and the tick batch.

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
tables). Rust, matching Odin-class doctrine, with the market maker and the
property-derivation functions as pure, testable crates and the daemon as a
thin lifecycle shell. Clocks, transports, the Ghostlight port, and the game
host port are traits so a partial pipeline smoke can price a market from a
fixture ledger without the whole daemon.

## Verification

Unit: lot conservation under every command; provenance acyclic; a recipe
refuses a material that fails its role; output properties are a pure
function of input properties and process; the quote function is
deterministic and its loss is bounded; a stale quote is refused.

Pipeline: a fixture world with copper, aluminium, and gold conductors, one
spreader recipe with an optional vapour-chamber role, two markets on one
route; flooding one market moves both prices in the direction the route cost
predicts; closing the route decouples them. A shipment between the two
markets, seized mid-route against its revision, lands its lots on the seizer,
is refused a second time, and leaves the destination's next quote higher
than it would have been on arrival.

End to end: a Ghostlight world with a market mirror; a witnessed route
closure in Ghostlight arrives as a shock and moves a price; the price arrives
in Ghostlight as a fact witnessed at the market; a person's buy intent exits,
is executed, and returns as custody with a receipt; replay of the daemon's
journal reproduces every quote.

## Decisions

Taken 2026-09-11: the daemon is Njörðr; properties are dimensions, never a
scalar quality, because one axis cannot express a trade-off within a recipe;
process roles are dimensions like materials.

Still the operator's:

1. Whether Delvehold's civic economy (recipes, facilities, orders, policy)
   migrates into Njörðr or stays Delvehold-owned with Njörðr as its market
   and price authority only. The model works either way; the authority map
   assumes Njörðr owns lots and recipes for both games.
2. The market maker: logarithmic scoring rule versus constant-function curve.
   Recommend the scoring rule for its bounded loss and single liquidity
   parameter per market.
3. Who decides a shipment sails: a Ghostlight subject (a carrier institution
   with an operational agent, reading Njörðr's prices as facts) or an
   authored flow policy inside Njörðr. Recommend the Ghostlight subject, so a
   pirate's victim has a name, a route, and a grudge, and the daemon keeps
   owning no trader.
