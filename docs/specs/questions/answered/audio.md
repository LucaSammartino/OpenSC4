# audio: answered questions

**ANSWERED 2026-10-24 (re-audio-au40):** DIFFERS — AU41: audio.md §19.2 specifies
SEM/model defaults, binding, registration, retained-state failure and teardown, and all six
model slot 5/6 contracts with twelve vectors. Primary slot 4 calls the supplied object's
name slot; final volume uses a logical shift; effects pan uses absolute projected pixel X.
Registration does not roll back; failed model allocation and null-Hit teardown fault.

<!-- from: re-audio9 (2026-10-23) -->
- **AU41 — SEM and emanation arithmetic:** settle model slot 5/6 transformations (`0x004ED210`,
  `0x004ED2A0`, `0x004ED3F0`, `0x004ECDA0`, `0x004ECE70`, `0x004ECF90`) against instructions;
  numeric decompilation is corrupt. Recover SEM construction/defaults and registration rollback at
  `0x004EDAA0`; check caller ordering of teardown `0x004ED750` (misnamed Init in export).

**ANSWERED 2026-10-24 (re-audio-au40):** DIFFERS — AU44: audio.md §7.4.1 specifies all
five views, defaults, registration, filtering, dispatch, enumeration and lifetime. Failed Init
retains subscriptions; Shutdown releases scape but never removes its group. Insertion/removal
uses the acquired position-interface key, while enumeration uses the visited occupant key.

<!-- from: re-audio9 (2026-10-23) -->
- **AU44 — fire-handler complete owner contract:** audio §7.4 currently gives identity and corrects
  torch ownership, rather than specifying the full fire handler. Supplement §20.3 adds some lifecycle
  and scape IO facts. Recover construction/defaults, all interface views, message dispatch and group
  registration/removal contracts at `0x004F0240`–`0x004F132F` before calling this owner complete.

**ANSWERED 2026-10-24 (re-audio-au40):** DIFFERS — AU40: audio.md §18.4 and supplement
§18 specify camera-centered geometry, sea-plane correction, x87 rounding, tuning sizes and
literal failure lifetime. Notifications measure camera movement; absent server and null
message allocation fault; the cheat helper only queries/releases its payload view.

<!-- from: re-audio9 (2026-10-23) -->
- **AU40 — listener geometry and failure lifetime:** complete the camera/terrain-to-listener numerical
  contract at `0x004F14B0`, tuning fallback/array sizing at `0x004F1A80`, cheat helper `0x004F1470`,
  missing message-server handling and final message delivery/allocation path. Supplement §18 settles
  dispatch, payload shape, zoom multiply order and strict squared-distance threshold only.

**ANSWERED 2026-10-24 (re-audio-au50):** DIFFERS — AU48: audio.md §10.7.1 specifies AEMS
views/defaults, latched lifetime, backend calls/results, bank/file ownership and failure
quirks. The raw reader uses two unchecked sizes; resource success also depends on Close.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU48 — AEMS owner lifetime and missing event-system bodies:** §10.7 specifies raw bank slurp,
  resource TGI caching, successful AddBank linkage and registrar prerequisites, while §17 inventories
  24 event-system entries but gives no full QI, constructor fields, Init/Shutdown, StopAllSounds,
  event/service dispatch, bank removal/destruction, allocator and callback contracts. Please specify
  them, plus AddBank result convention, file reference ownership, allocation/read failures and
  the two GetSize calls disagreeing. §10.8 also lacks failed/short tuning read policy for AEMS.
  [Implement raw IO and successful linkage as explicit stages; no guessed full owner execution.
  Refuse a read larger than allocation; native unstable-size/error behavior stays unsupported.]

**ANSWERED 2026-10-24 (re-audio-au50):** DIFFERS — AU46: audio.md §8.3.1 supplies the
payload gates, reset/enumeration/volume/focus/stop-all calls, tick arithmetic, defaults and
nontransactional Init/Shutdown. Focus and stop-all DoEvent return false after their effects.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU46 — sound-manager missing owner bodies:** sections 8.1–8.3 inventory a focus flag,
  service pointers, Update callbacks and event routes, but do not define the payload gates,
  state transitions, return values or foreign slot calls for reset (`0x4A4D35E0`), enumeration
  (`0x4A4FE6F6`), volume (`0x4A451915`), focus (`0x6A95CBA2` and focus message), and the
  `0xCAA44CA7` stop-all helper. What are those algorithms, the per-sound update callback
  (`0x004F9690`), completion/registration lifetime, constructor flag defaults, and Init/Shutdown
  failure ordering? Section 10.6 supplies lookup/start facts, not these contracts.
  [Retain existing instance-table fixture; do not guess these missing owner bodies or claim
  full SoundManager closure. Implement settled routing/state/IO separately while awaiting answers.]

**ANSWERED 2026-10-24 (re-audio-au50):** DIFFERS — AU49: audio.md §8.3.1 specifies the
property-based x87 radius calculation, manager-volume play argument, completion lifetime,
allocation failure limits, hard/soft stop differences and foreign position writes.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU49 — start/position/stop instance completion:** §8.3 derives pause radius from the start
  parameter's low byte, but §10.6 step 4 assigns it from buffer property 0x38. What is the exact
  order and final value? Supply the tSound+14 constructor word passed to play(+14), buffer/QI and
  allocation failure paths, completion registration/unregistration calls, stop +20/+24 handle
  operations, and SetPosition's state fields/property calls/return semantics. The existing local
  fixture has no foreign position effect; the new resolve/play stages cannot close these bodies
  without those contracts. [Keep local fixture separate; implement known get-sound/QI/property/
  play operations as explicit stages, never assume missing word+14 or successful native dispatch.]

**ANSWERED 2026-10-24 (re-audio-au50):** DIFFERS — AU47: §11.3 specifies the nine named
registrations before all 83 leaves, unconditional five-child start sequence, cumulative
PostAppInit failure result without rollback, and distinct shutdown/destructor orders.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU47 — SC4 director complete lifetime and ordering:** §§1.1/1.3/11 give the five-director
  add/start order and 92 registrations, but not the constructor interleaving of the nine named
  classes with the 83 leaves, OnStart return/failure policy, or full PostAppInit service creation,
  initialization, failure unwind and shutdown/destruction ordering. What are those contracts?
  [Execute the settled add/start sequence; register the measured class set in literal table order.
  Leave unspecified application lifetime unsupported rather than invent service success.]

**ANSWERED 2026-10-24 (re-audio-au50):** MATCHES — AU45: §7.2's completed table is authoritative.
Conformance vector §16.12 now includes the fifth accepted, effect-free type `0x49E64801`;
the legacy four-operation fixture is a subset, not a dispatch whitelist.

<!-- from: core-audio-close11 (2026-10-24) -->
- **AU45 — completed dispatch versus vector 12:** section 7.2's completed dispatch table routes
  `0x49E64801` to HandleAutomataMessages as an accepted no-op, while section 16 vector 12 says only
  four other types may reach that helper. Should vector 12 include this fifth no-op type?
  [Implement the completed section 7.2 table literally; keep the legacy four-operation fixture list.]

Original questions and implementation-gap notes are retained below; their proposed fallbacks are
superseded by the cited spec sections, checked against 1.1.641 instructions.

**ANSWERED 2026-10-23 (re-audio-au5):** DIFFERS — AU13: audio.md §§7.2.1–7.2.4 and 7.3.2 define station selection, construction IDs, dispatch categories/capability checks and nested modal/mouse behavior. EC4 queues station 0. The completed dispatch table includes previously omitted routes; subscribed C53D10AA/E94671ED remain no-ops. Slot10 acquires nothing: it loads data1 as a raw pointer; the handler passes that pointer to radio then releases it, never the message. The mouse route can return false through main DoMessage.

<!-- from: core-audio-events5 (2026-10-23) -->
- **AU13 - sections 7.2 and 13, owner dispatch algorithms:** What are the complete sound-selection, state, argument and failure rules of HandleConstructionMessages (0x89F10866/0xA9F10E45), HandleRadioMessages (0x2A3AD653), the message built for 0x26D31EC4, LAB_004EFA91 (0xC9F1011A/0xE94671EB), HandleMouseMessagesForModalWindow and the modal target's 0x62BA7B42/43 notices? Are subscribed types absent from the dispatch table genuinely ignored? For 0xAA611B6E, does slot2 release the message or an interface obtained by slot10, and what reference does slot10 acquire? [Implement the listed routes and specified slot calls literally; report gaps for algorithms that have no specified steps. Unlisted types return 1 without effects. No guessed sound selection. These gaps prevent owner closure.]

<!-- from: core-audio-events5 (2026-10-23) -->
- **AU14 - sections 7.1-7.3, construction/lifetime/failure paths:** What initializes +0x18, the refcount/list/timer and any fields omitted from the layout? Which Init service supplies +0x18? What do constructor/Init/Shutdown do when each service lookup, registration or subscription fails, and on repeat Init/Shutdown? What are the exact return values for missing +0x18 or failed payload QI, and what happens if the radio service is null? [Keep COM services foreign; host supplies +0x18 availability. Failed Init service lookup reports a gap before registration, without inventing rollback. Missing capability/QI rejects with 0 and no unowned release. Default agrees with new; legacy word-only helpers retain their fixture assumptions. Full lifetime closure awaits these rules.]

  **ANSWERED 2026-10-23 (re-audio-au5):** DIFFERS — audio.md §7.1.1 supplies defaults and checked/unchecked failure boundaries. +0x18 is retained AudioScape service 49E50BA1/29E39DB4. Its lookup failure occurs after registration, returns false without rollback; registration/subscription results are ignored. Repeat successful Init/Shutdown is inert; failed Shutdown sound lookup preserves state. Successful Shutdown releases/nulls radio, which Init does not reacquire. Checked missing capability/payload paths return false with acquired-reference cleanup; null radio skips radio calls.

<!-- from: core-audio-events5 (2026-10-23) -->
- **AU15 - section 7.3, alternate automata and vector payload:** What selects the two +0x18 posts versus QI 0x26BB767E for 0xA9E3B6E3, what are their arguments and derived ID, and what exact calls/arguments/state changes does 0xEA3ADA33 make to SFX::SoundManager and GenerateNewInstanceId? What is the packed set-position event layout, its instance-key source, and the behavior on missing slot32/failed vector QI? [Alternate automata routes report gaps after balancing payload QI. Ordinary play forwards the vector and the slot14(1,0) instance to a foreign sound-system adapter, leaving wire packing foreign; failed vector lookup retains the supplied global seed as the literal seeded-vector reading. No guessed alternate sound events.]

  **ANSWERED 2026-10-23 (re-audio-au5):** DIFFERS — audio.md §§7.3–7.3.1 specifies both alternate selectors and all arguments. Ordinary position words are (data2,trunc(x),trunc(y),trunc(z)); failed position QI rejects without events and null void1 is unchecked. Slot14 takes no arguments. A9E3 uses signed data1 to choose paired nonpositional posts or one positional post keyed by void1, range 50.0f. EA3 calls SFX service access and a no-argument global ID generator, then AudioScape with the six-cell selector table; its failed QI leaves an indeterminate position, not a seed fallback.

<!-- from: core-audio-events5 (2026-10-23) -->
- **AU16 - sections 7 and 9.5, DoEvent, torch ownership and destruction:** The owner exposes DoEvent (0x004EF490) and a deleting destructor (0x004EFC60), but no callback event table, effects, or destruction algorithm is specified. What events does DoEvent accept, how do they trigger/stop sounds and modify handler state, what are its failure returns, and what cleanup does destruction require beyond Shutdown? Section 7.4 assigns torch entry points to cSC4AudioEventHandlerDisasterFire while section 9.5 names cSC4AudioEventHandler::_StartTorchSound/_StopTorchSound: which owner has them, and what selects their sounds, instances and failure behavior? [DoEvent reports a gap; explicit Shutdown implements the documented successful-path cleanup. Rust owns its collections; no foreign callbacks are guessed during Drop. Torch algorithms remain unspecified. Owner closure remains open.]

  **ANSWERED 2026-10-23 (re-audio-au5):** DIFFERS — audio.md §§7.4–7.6 gives the event table, true-only-on-missing-sound-system callback return, torch globals/numeric order, sound/key/volume table and destruction contract. Torch belongs to the main handler. Destruction does not call Shutdown or stop sounds; it destroys timer/list storage and releases remaining member services. Leftover raw modal pointers are not released by the native list destructor. Conformance groups 26–42 cover all four answers.

<!-- from: core-audio-close11 (2026-10-24) -->
- ~~**AU47 — SC4 director complete lifetime and ordering:** §§1.1/1.3/11 give the five-director
  add/start order and 92 registrations, but not the constructor interleaving of the nine named
  classes with the 83 leaves, OnStart return/failure policy, or full PostAppInit service creation,
  initialization, failure unwind and shutdown/destruction ordering. What are those contracts?
  [Execute the settled add/start sequence; register the measured class set in literal table order.
  Leave unspecified application lifetime unsupported rather than invent service success.]~~
  **ANSWERED 2026-10-25 (re-audiofilters8):** already answered in audio.md §11.3 (no interleaving, OnStart
  ignores child results and returns true, PostAppInit's ten steps with no unwind, PreAppShutdown order versus the
  destructor's release order); archived here. The 83 leaves' tests are audio-filters.md.
