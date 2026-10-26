# automata: open questions
<!-- Questions about [automata.md](../automata.md). Append new entries at the END; when one is answered, move it to answered/automata.md. Rules: README.md. -->

## From the automata state behaviours and the effect-directory measurements (re-integrate1 lane, 2026-10-16)

Four RE reports, kept verbatim in `build-runs/re-integrate1-automata.md`, `re-integrate1-effects.md`,
`re-integrate1-vehicle-ticks.md` and `re-integrate1-air-water-ticks.md`. Their findings went into automata.md
§6.2, §6.5, §6.6, §8.1, §12-§14 and effects.md §3, §5.2, §7, §10.1, §10.2, §15-§17.

**Nothing needed striking here for the automata state questions**: automata.md's Q3-Q5 (filed above as AT3-AT5)
and AM1-AM12 were already answered by re-answers16 and re-answers20, and the effects questions EF1-EF5 lived in
effects.md §16 rather than in this file. Two of those are now closed or answered and are struck below so a
reader does not re-open them. AT6-AT19 continue automata.md §13's new Q7-Q20; EF1-EF10 continue effects.md §16.
The probe named is the one the report gives. **All of AT6-AT19 were answered on 2026-10-20 by re-answers33**
and moved to `answered/automata.md`.

## From the glue19 lane (2026-10-20)

**AT20.** Automata.md section 9 gives the U-Drive-It pause and resume hooks only from the manager's side
(slots 9 and 10) and says the city situation manager drives them, but the spec does not name the manager's
side: what city-situation slots or messages reach `fc4_automata_pause_all` / `_resume_all`, and when exactly
(countdown start, countdown end, mission end)? A replacement wiring this from the outside (glue19 wires them
to the manager's `MissionActivate` / countdown-end points through the city-situation interfaces) needs the
receiver names to check it against the game's own call sites.

## From the verify-automatarefs4 lane (2026-10-21)

**AT21.** Where does the original controller manager (city slot 47) drop the references it holds on its
controllers, and on the occupants and automata of its record, at city close? §3.4 says the manager keeps the
reference its Read returned, §7.1 (AM7 step 3) that a removed controller is shut down (slot 4) and released, and
§10 lists `0x99EF1143` RemoveOccupant as "drops the occupant's controllers [inferred]", but no section names the
manager's own `Shutdown` (slot 4) or destructor behaviour. In a city loaded and then saved (foundations §2.7 SV1:
the load's object table is gone), run sg4-c1 (2026-10-21) shows 214 generator (`0x4A1628C5`) and 24 attractor
(`0x6A1628BA`) controllers whose last reference is the manager's. Please state: (a) which of `PreCityShutdown`
`0x26D31EC2`, the RemoveOccupant handler during the occupant manager's shutdown, slot 4 `Shutdown` or the
destructor releases them, in which order relative to the occupant manager's shutdown; (b) whether each controller
is shut down (controller slot 4) before that release; (c) whether the queue entries (§3.3) and the 24-hour clock
reference (§3.1 field 2) go at the same point. verify-automatarefs4 keeps them to the destructor (the literal
reading of "the manager holds them") and declares that release the manager's own (FC4_RELEASE_AT_DESTRUCTION).

**Partial source answer 2026-10-23 (re-controller-manager-owner4):** See
[the dedicated owner supplement](../automata-controller-manager.md), §§2–9. Primary
Shutdown `0051B3E0` unregisters tick/messages, Releases script then clock, drains queue
occupant/automaton references, clears grid storage, then calls controller Shutdown
immediately before each controller-list Release and finally Releases occupant keys.
Final manager Release explicitly invokes Shutdown before storage destruction. The city
does this at `0045B2F2`, before occupant-manager Shutdown at `0045B833`; this manager
neither subscribes to nor handles PreCityShutdown as a cleanup trigger. Loaded/new
state does not branch in Shutdown. These are direct native call-site effects, not a
validation of the historical warning counts or destruction annotations.
**Remain open:** the full transitive controller Shutdown/reference ledger (AT22),
message/animation/debug registration services' internal retained-reference effects,
and whether earlier city callbacks emit RemoveOccupant before this explicit service
Shutdown. The supplement specifies the received RemoveOccupant path conditionally;
it does not prove every upstream producer or callbacks that mutate the live containers.
AT21 stays here rather than being archived as wholly answered by inference.

<!-- from: glue-automata-managers4 (2026-10-22) -->
- **AT22 - §§3.5, 7.1, 11; automata-support.md §2; automata-managers.md §1:** Please publish a complete
  inherited `cSC4AutomataController` lifecycle/interface contract: constructor/destructor defaults and
  ownership, Init/Shutdown and template assignment, activation, occupant attachment/movement, world bounds,
  grid-bound storage, tick/expiry and controlled-automata removal, with reconciled signatures/returns for
  every reachable slot. §3.5 says the saved position starts at +0x10 but the live getter at +0x14, and
  CheckFlag/SetFlag use +0x40 while saved flags use +0x3C and +0x40 also holds the list head. Resolve these
  before native layout calls or a live wrapper are implemented; a round-tripping record does not settle
  live accessors. The director lists only attractor (0x6A1628BA) and generator (0x4A1628C5) controller
  factories. Confirm the base has no separate factory and specify the derived overrides required on each
  concrete route. AT21 separately covers the controller manager's final releases. [blocked; keep record
  support only, no invented base CLSID or complete-owner claim]

**Partial source answer 2026-10-23 (re-automata-controller-lifecycle4):**
[The inherited base lifecycle contract](../automata-controller-lifecycle.md) settles the layout question:
the serializable interface is S = C + 4, so saved position S+0x10 is live C+0x14, saved flags S+0x3C are the
flags word C+0x40 and the list sentinel is C+0x44. It specifies construction defaults, QI/AddRef/Release,
template and occupant assignment, Init, Activate and the flag accessors, the sound/effect/view helpers,
Shutdown versus direct destruction, animation tick and expiry, position and bounds, the grid bytes,
controlled-automata removal, the lot-facing point and within-radius test, the state callbacks and behavior
selection, the secondary Read, and the attractor/generator override map. The director registers only the
two concrete factories; `4A18672B` is a serialization identity, not a base factory. Its conformance fixtures
are proposed and have not been run.
**Remain open:** the runtime value of global `00B227CC` (constructor position); slot 24/36 meanings and the
untyped extra arguments of state tick and DoAnimationTick; CreateTemplateInstance `0051BDF0`; whether
SC4ReadGZSerializable retains returned references; the secondary Write body; allocation-failure, exception,
reentrancy and non-finite numeric behaviour; the runtime x87 precision setting; the derived controllers' own
contracts. AT22 stays open.

<!-- from: glue-automata-managers4 (2026-10-22) -->
- **AT23 - §§6.1, 7.2, 11, 15; automata-managers.md §§1, 2.2:** Please publish the complete inherited
  `cSC4Automaton` construction/Init/Shutdown/destruction contract and its occupant/model/callback ownership,
  placement/unplacement, manager detach and deferred fade/removal lifecycle, with reconciled calling
  signatures, return conventions and this-pointer views for every reachable base slot. The current sparse
  plugin wrapper has void placeholders for Init/Shutdown and position/movement getters, returns null from
  AsOccupant, and keeps wrappers alive until manager destruction. These are explicitly partial record hooks,
  not a substitutable live owner. ABI rows for slots 5/6 still flag overload ordering as unconfirmed; §11
  leaves slot 70 untraced while automata-managers.md §2.2 describes slot 70 as placement. Resolve that
  surface and distinguish derived overrides before an owner replacement. The director has concrete
  vehicle/pedestrian/aircraft/watercraft routes (including a concrete aircraft using the automaton
  constructor), but no separate automaton factory: confirm which route can support the complete inherited
  owner without delegating its missing methods back to native code. [blocked; no new wrapper or factory]

<!-- from: automata-cities29 (2026-10-26) -->
- **AT24 — concrete carjacked/limo records:** Publish the complete ordered wire fields,
  leading version (if any), nesting, accepted versions and reference IIDs for
  `cSC4CarjackedVehicle` (`0xCBB32B0A`) and `cSC4MayorLimo` (`0x4C0DD224`).
  automata-managers.md §1 specifies their construction, not their records;
  docs/abi/vtables.tsv assigns both Write `0053ce00`, Read `00524490`, distinct
  from Vehicle. Census sh28-census3 reports unsupported records in 23 and 9
  scenarios respectively (e.g. maxisland-bevopolis, maxisland-derris-heights).
  Blocks replay of these two cases; no presumed alias or opaque-byte bypass.
- **AT25 — ambulance record:** Publish the complete ordered wire fields, leading
  version, nesting, accepted versions and reference IIDs for `cSC4Ambulance`
  (`0x6C1ACDE1`). automata-managers.md §1 identifies an emergency subclass but
  not its record; ABI assigns Write `00504bf0`, Read `00504c10` rather than the
  emergency base's methods. Unsupported in 12 census scenarios, e.g.
  lua-lots-grown and maxisland-east-weburg. Blocks this case's full replay.
- **AT26 — tank record framing:** vehicles.md §§1, 6, 12.13 say Vehicle plus
  two owner dwords, but do not specify whether Tank Write `0054aa30` / Read
  `0054a8e0` add a leading version wrapper, its value/acceptance, or the order
  and meanings of those dwords. Please specify the exact wire sequence (or
  explicitly confirm no wrapper and two raw dwords). Unsupported class
  `0xEBE0DCAD` in maxisland-norte-delano and paradis-zoe blocks those two cases.
