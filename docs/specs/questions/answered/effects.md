# effects: answered questions (archive)

<!-- from: glue-effects-shadow7 (2026-10-24); copied from its reserved worktree -->
- **EF20 - sections 2, 6.1/6.2 and 18.6 native record-only capture:** Please specify the collection getter return types, exact argument types/ownership and description enumeration/count access, the resource collection-reader IID/signature, and a read-only queue/bootstrap state capture protocol. The slot tables identify methods but their ABI rows say eax-or-void, and section 2 does not define list/description container framing. How can a shadow capture all published descriptions, bindings, triggers, camera state and retained file queue without mutating the native manager or guessing signatures/layouts? [Compare only section 3 directory name decoding against native section 4 HasVisualEffect, through the public SDK raw record and manager interfaces. No native collection getters, private container reads, queue hooks, publication/placement equivalence or full-owner coverage claimed until this contract is provided.]

  **ANSWERED 2026-10-24 (re-effects-ef20):** DIFFERS — effects.md §18.7:
  named getters return bool in AL; six copy into constructed caller-owned value
  objects, while the component getter returns a retained description pointer.
  No count/enumeration interface exists, and even null-output getters mutate
  scratch state. Native vector/hash framing supplies retained indices, names,
  bindings, triggers, camera copies and queue records. Resource IID CA51189B
  slot 5 is void publication to a collection, with a storage-swap fast path;
  it must not be called for inspection. Capture at a quiescent owner boundary,
  preserving typed values and refusing incomplete samples. Bootstrap history
  requires observing natural operations; it cannot be reconstructed from final
  state. Ten proposed conformance vectors, unexecuted; no coverage expansion
  or payload equivalence is inferred from names/counts alone.

- **EF21 — §18.7 native observation attachment and remaining payloads:** EF20 specifies retained memory framing but not an attachment contract for tracing the naturally executed bootstrap/publication calls. How does a default-off plugin observer obtain a live resource primary identity and the natural bootstrap/publication inputs/results without requesting a new resource or changing owner behavior? Which owner-thread city-load/month callback boundaries exclude concurrent retained collection/queue writes, including reload? Supply complete typed native payload memory layouts for particle/dynamic/decal/shake/light and component types 3–15 (including anonymous descriptions), or explicit cross-references to their complete contracts, to support payload-exact capture. [Compare only the visual records and name/index multiset for now; other families, camera/binding/trigger publication and queue/bootstrap history remain uncredited. No resource publication/getter/parse calls or guessed native hooks are introduced.]

  **ANSWERED 2026-10-24 (re-effects-ef21):** DIFFERS — effects.md §18.8 specifies natural bootstrap/resource-service/publication attachment, borrowed resource identity/lifetime, parse result widths, reload boundaries, and complete built-in particle/dynamic/decal/shake/light and type-3..8 payload layouts. Types 9–15 are reserved directories with no built-in private payload contract; a foreign object there is explicitly unsupported rather than misread. City-load/month labels supply no lock; copy only in a verified owner-thread interval outside the outermost Init/bootstrap/reload, or refuse. A late attachment cannot reconstruct history, and full manager/resource coverage additionally requires live lifecycle and stream/reference-event measurements. Thirteen proposed conformance vectors are unexecuted. Question copied from glue-effects-capture8 because it was not yet on main.

- **EF15 — Effects-script command owner contracts missing from effects.md:**
  The requested owner family is `(anonymous_namespace)::c*Command`, including
  effect-description parser commands. Which exact classes belong to that family,
  and which spec sections define each one's token inputs, state transitions,
  defaults, validation, block enter/exit, foreign calls and conformance cases?
  `effects.md` §§0–18 contains no command-class inventory or command algorithms;
  its §17 exercises runtime effects, not script commands. The separate
  `effects-parser.md` §6.1 supplies 21 root literal/descriptor/hook associations,
  expressly excludes their execution algorithms, and §6.2 supplies only the
  `sequenceEffect` normal handler, expressly excluding other nested entries.
  Please publish the class-to-descriptor inventory, all remaining root/nested
  handler contracts and command-specific conformance cases (including any
  commands absent from that bounded dictionary). Distinguish these owned
  algorithms from foreign renderer, sound, resource and generic-reader services.
  [Retain existing registration and sequenceEffect implementation; do not infer
  handler behavior or class names from command literals or addresses. Whole
  command-family closure remains blocked on this publication; EFP1–EFP7 also
  continue to bound the existing parser's supported domain.]
  **ANSWERED 2026-10-23 (re-effects-ef5):** DIFFERS — effects.md §19 now
  publishes the descriptor inventory and execution of all registered root and
  nested families. §§19.53–19.56 close the residual visual-effect children,
  aliases and select/particleSequence hooks with EC277–EC304. Class names
  remain limited to confirmed export identities. Foreign primitive boundaries
  and the numeric qualification of effects-parser §14 remain explicit.

- **EF16 — §19.52 / EC275 conflicts with effects-parser §6.1 for instance:**
  effects §19.52 assigns descriptor `00A9FA08` direct handler `00596980` and
  EC275 says this is the same handler as root instance. The root inventory in
  effects-parser §6.1 instead assigns that descriptor `0059A730`. Which is the
  correct root and nested association? [Retain both literal registration
  ledgers, use the settled §19.4 effect-ID algorithm for instance, and leave
  only the claimed equal-handler part of EC275 open; no inferred correction.]
  **ANSWERED 2026-10-23 (re-effects-ef5):** DIFFERS — §19.52's address
  was wrong: both registrations use descriptor `00A9FA08`, whose +0C word
  is `0059A730`. `00596980` is inside effectsResource's exit hook.
  EC275's equal-handler assertion and §19.4's instance behavior stand.

- ~~**EFP2 — Rotation instruction/store semantics:** In effects-parser.md §2/§8,
  establish x87 operations and binary32 store order for all five rotation branches,
  including key order versus text order.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** DIFFERS — effects-parser.md §13:
  turns multiply binary32 2π without an intervening angle store; cosine stores
  binary32, positive sine initially stays x87. X/Z store negative sine before
  their second column; Y retains positive sine. Row-dependent product stores
  are specified explicitly. XYZ executes X/Y/Z; ZXY executes Z/X/Y on successive
  selected values. Each completed axis increments the byte count and sets bit 04;
  earlier axes survive a later conversion exception. Six conformance rows.
  Process PC/RC and broader numeric qualification remain EFP7.
<!-- Archive for [effects.md](../../effects.md). Lanes do not read this file. Open questions are in ../effects.md. -->

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**EF2:** confirm at runtime that a volcano never reaches state 7 when the city view does not draw (effects.md
  §15 quirk 1). [needs a runtime check]~~ **CLOSED 2026-10-16 (re-integrate1), statically.** `0x005BFD70` has
  exactly one reference in the whole decomp, in `cSC4EffectsManager::DeactivateUnusedVisualEffects` `0x005922A0`
  (`cSC4EffectsManager.c:1987`), which in turn is referenced only from
  `cSC4EffectsManager::Update(SC4DrawContext*)` `0x00593120` (`:2993`) - no direct call sites, dispatched from
  the vtable slot at `0x00A9F25C`. The end-message post exists in exactly one place. So no draw pass means no
  finish test, no effect ever finishes and no end message is ever posted. The volcano stuck in state 5/7 and the
  effect-disaster emergency pause never lift.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**EF3:** the path by which particle systems spawn the "slave" effects, and whether the parameter 13 forwarded
  by the effect-disaster instance reaches them. [untraced]~~ **ANSWERED 2026-10-16 (re-integrate1).** There is no
  spawn API: each particle class concatenates its base name with a fixed suffix (`FUN_0041E270` `0x0041E270`) and
  calls `CreateVisualEffect` (slot 7). Suffixes and callers: `_slave` (`0x005AEF20` per particle, `0x00585190`
  once at Start), `_death` (`0x005AF390`), `_collide` (`0x00584890`), `_terrain`/`_water` (`FUN_005B8290` on
  impact). Parameter 13 reaches **game components only** - particle `SetVoidParams` `0x005B2200` accepts only
  index 7, decals only index 6, meta recurses (`0x005C0540`). Shipped census (1148 names): `_slave` 143,
  `_terrain` 33, `_water` 20, `_death` 21, `_collide` 3, `_control` 6. effects.md §10.2. The one sub-question
  that survives is EF6 below.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**glue22 question 9** (`enable = effects`): `ACCESS_VIOLATION` writing `0x4D` at a wild `eip` about 100 ms
  after `Init`, no method of ours entered; the game calls through a vtable pointer read past our object. A `+4`
  draw-pass sub-object did not help. What are all the interface sub-objects of the original
  `cSC4EffectsManager`, at which offsets?~~ **ANSWERED 2026-10-16 (re-answers31): DIFFERS.** The manager is
  **six** sub-objects with vptrs at `+0x00`, `+0x04`, `+0x08`, `+0x0C`, `+0x10`, `+0x94` (destructor
  `0x0058FA60`, confirmed by the Mac `__ZThnN_` this-adjustments). `QueryInterface` (`0x0058D9E0`) returns
  `this + 8` for IID `0x452294AA` (`cIGZMessageTarget2`) and `this + 0x94` for `0xC97CF5CE`. The message
  dispatch fetches the vptr at `+0x08`, which is past a single-interface object. Written into
  [effects.md](../../effects.md) **§18**.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16); answered by re-answers34 -->
- ~~**EF1 (narrowed):** the exact finish time of `volcano_control`. Only particle components could hold it:
  brush ends at `t > length` (2.0-4.0 dilated effect-seconds), decals self-stop at fade 1.0 with mode byte
  `+0x15 == 2` (`0x00582920`), game/attractor/camera carry the ignore bit - yet all 1921 shipped particle
  components have `flags = 0`. Do `cSC4ParticlesEffect` components ever report inactive, and by what path?~~
  **ANSWERED 2026-10-20 (re-answers34): MATCHES - statically, the runtime trace is not needed.** The emitter
  self-stops. Three facts from the instruction bytes (effects.md 7.1):
  (1) `cSCParticleSimulatorBase::Stop` `0x005B9E70` takes two mode bytes; mode1 == 0 is an **immediate** stop
  (`started := 0`, all particles destroyed), mode1 != 0 is **delayed** (`+0x0C` set, particles play out).
  `cSC4ParticlesEffect::Stop` `0x005B21E0` calls the base with `(mode == 0, 0)`, so the component's `Stop(0)`
  is the delayed one and `Stop(1)` destroys at once.
  (2) `CheckForStop` `0x005B9D40` (end of every draw-time update) converts a delayed stop to an immediate one
  when the live count `+0x14` reaches 0 - that is when `IsActive` goes false.
  (3) The emitter calls `Stop(0)` on itself when its budget is exhausted: `cSC4ParticlesEffect` at `0x00AA6D4`
  / `0x00AA6EB` (interval countdown `+0x190` seeded in Start from desc `+0x20..+0x2C`); ModelBased via its
  `+4` sub-object slot 15 `0x005B96E0`, which dispatches on the particle description's flag bits: bit 2
  (`0x04`) -> `0x005B3CF0` (plain rate emitter, no self-stop), bit 1 (`0x02`) -> `0x005B8B50` (loop-budget
  emitter: `+0x128` counts loops vs desc `emit_loop_count` at `+0x18`, `Stop(0)` at `0x005B8D67`), else
  `0x005B87B0` (duration-budget emitter, `Stop(0)` at `0x005B8B29`).
  So every shipped volcano particle component (`emit_loop_count = 1`) ends on its own and `volcano_control`
  finishes ~50 effect-seconds (the crater decal's `life`) after its creator starts, plus one draw-pass sweep
  for the meta parent. Written into effects.md **7.1** and the EF1 entry.

- ~~**EF4:** the meaning of visual effect flag bits 0, 3, 6 and 8, and when an effect created during a
  draw-pass sweep is activated in a session that only ticks.~~ **ANSWERED 2026-10-20 (re-answers34): MATCHES
  (bits) / settled (activation).**
  *Bits.* The 9-bit description flags land verbatim in the effect's `+0x28` (reader `0x005ABF50` masks
  `& 0x1FF`; `TestFlag` slot 17 `0x005BFC90` reads them). A byte-level scan of the effects module
  (`0x0058D000`-`0x005C5000`) finds tests of exactly five bits on `+0x28`: bit 1 (finish test `0x005BFD89`),
  bit 2 (hard stop `0x005BEBB6`), bit 4 (slot 13 no-forward `0x005BFC26`/`0x005C05E6`), bit 5 (pick ray
  `0x0058E6A7`), bit 7 (finish test `0x005BFDD2`). **Bits 0, 3, 6, 8 have no consumer anywhere in the module**;
  they are stored and readable via `TestFlag` but change nothing in the engine - a replacement may accept and
  ignore them.
  *Activation.* `ActivateNewVisualEffects` `0x005921B0` is called from both `CreateVisualEffect` overloads
  (`0x00593AB0`, `0x00593B68`), from the draw pass (`0x00592A01`) and from `UpdateActiveVisualEffects`
  `0x00593B90`. The last runs on `0x697D01ED` (`DoMessage` `0x00594476`), which `EffectsTickCallback`
  `0x0058D9A0` posts once per animation tick (registered in `Init` `0x00594EEE` via
  `EnableAutoStep(1, 0x0058D9A0)` on the animation-tick manager at `+0xCD8`). So an effect created during a
  draw-pass sweep is activated on the **next animation tick** in a ticking session, not only on the next draw
  pass. effects.md EF4 and 6.1.

- ~~**EF5:** whether a hidden pause counts as "paused" for the time dilation, which would make the 5 s and 15 s
  pauses of effects.md 9.4 real-time seconds.~~ **ANSWERED 2026-10-20 (re-answers34): DIFFERS from the earlier
  dilation reading.** `GetAnimationTimeDilation` `0x007026F0` consults, in order: `IsEmergencyPaused` (slot 17)
  -> the paused value; the object cached at global `0x00B43D44` (city slot 43, the vehicle/automata manager
  family) whose slot 13 is `cSC4AutomataManager::IsDrivingMode` `0x00506940` (true while the U-Drive-It view
  `0x2B709676` is up) -> the paused value; `IsPaused` (slot 15) -> **0.0**; otherwise by `GetSimSpeed`
  `0x008265C0`: speed 0 -> 1.0, speed 1 -> 1.5, speed 2 -> 2.0. The table lives at the simulator's `+0x108`
  pointer: `[0.0, 1.0, 1.5, 2.0, 1.0]`, written by `ReadSimulatorTunableValues` `0x007025B0`
  (`0x00702686`-`0x007026B1`). **`IsHiddenPaused` (slot 16) is never called**, so a hidden pause does not
  change the dilation: the 5 s / 15 s hidden pauses run in effect-seconds dilated at the current speed, not
  real-time seconds. The earlier "1 while paused, 3 at speed 2" was wrong (now 0.0 and 2.0). effects.md 6,
  quirk 3, test 17.

- ~~**EF6:** what `data1` is on a `kSC4MessageEffectLocation` (`0x2A7B582E`) posted from a game record inside a
  particle-spawned slave.~~ **ANSWERED 2026-10-20 (re-answers34): DIFFERS - the handler matches strictly on
  `data1 == instance`, so particle-spawned slaves never update the instance's location.**
  `cSC4DisasterInstanceEffect`'s message handler `0x00576E70` (registered at vtable `0x00A9E254`) does, for
  `0x2A7B582D` (`EffectDone`) and for `0x2A7B582E` (the `SUB EAX,0x2A7B582D; DEC EAX` path at `0x00576E86`)
  alike: read data1 (message slot 9, `+0x24`) and require `EAX == this - 0xC` - the instance pointer
  (`LEA ECX,[EDI-0xC]; CMP EAX,ECX`, `0x00576E93`-`0x00576EBE`). Only then does it take data2/data3
  (`0x00576E9B` slot 14, `0x00576EA5` slot 19) into the instance's `+0x4`/`+0x8` and call the location setter
  at `[this-4] vt+0xC`. `cSC4Scrubber::Apply` `0x005BD3C0` posts `data1 = data (+0x58)`; for a
  particle-spawned slave that value is the particle's `+0x26C`, never the instance, so the update is dropped.
  The 10.2 damage table is unaffected (the damages do not depend on the location messages); only the
  *tracking* of tornado/meteor/robot-walk slaves by the instance does not happen. effects.md 10.2, 12.

- ~~**EF7:** the exact semantics of `cSC4ModelBasedParticlesEffect +0x26B / +0x26C` and `+0x26F / +0x270`.
  Ghidra splits them into five byte fields and that class has no `SetVoidParams` to populate them from a
  parent's parameter 13.~~ **ANSWERED 2026-10-20 (re-answers34): MATCHES the five-byte split, with one
  correction - there *is* a `SetVoidParams`.** From the bytes (`0x005AC510` ctor, `0x005AEC90`
  `SetDescription`, `0x005AC040`):
  `+0x26B` = the forward-parameter-13 gate: read at `0x005AF02A` (`CreateParticle`) and `0x005AF8B7`
  (`DestroyParticle`), both before `ForwardParameter(13, &this+0x26C, 1)` to a spawned slave. **Nothing in the
  effects module writes it** (full ModRM/SIB scan for disp32 `0x26B` writes: none; the ctor initialises
  `+0x26C..+0x270` but not `+0x26B`) - effectively uninitialised, a quirk; a replacement should make it an
  explicit default-off switch.
  `+0x26C` = render latch: `SetDescription` stores `cS3DModelProcessor::SetModel`'s return (`0x005AEDEB`);
  read only in `RenderActiveParticles` `0x005AE4A1`, gating `LightModel`/`DeformModel`.
  `+0x26D` / `+0x26E` = "a `<base>_slave`" / "a `<base>_death` effect exists" (`%s_slave` `0x00AA0CD0`,
  `%s_death` `0x00AA0CC4`; `0x00AEE79`, `0x00AEEF7`).
  `+0x26F`/`+0x270` = the parameter-13 channel for the chain hand-off: `SetVoidParams` **does exist** -
  `0x005AC040`, slot 6 of the component vtable `0x00AA0C58` - index 13 sets `+0x26F = 1` and `+0x270 = *value`,
  others fall through to the shared `0x005B2200` (index 7 only). `HandOffParticleToChainSystem` `0x005AF060`
  reads `+0x26F` at `0x005AF27C` and forwards `13, &this+0x270`; `DestroyParticle` forwards `13, &this+0x26C`.
  So `+0x26C` is the per-particle channel's value and `+0x270` the chain hand-off's value, `+0x26B`/`+0x26F`
  their enable bytes. The "accepts only index 7" claim in EF3 refers to the shared handler only. effects.md
  10.2.

- ~~**EF8:** who posts and who consumes `kSC4MessageTornadoDead` `0x0A36F1E8` and
  `kSC4MessageMissionEffectStarted` `0x5F900DCC`.~~ **ANSWERED 2026-10-20 (re-answers34).**
  `kSC4MessageTornadoDead` `0x0A36F1E8`: the constant occurs exactly once in the image - its name-table entry
  at `0x00B08430`. Nothing posts it and nothing dispatches on it: a dead ID in this build (the live sibling
  `kSC4MessageTornadoLocation` `0x0A36F1E9` is posted by `0x0042E5DF`/`0x0042EE8C` and consumed by
  `cSC4MySimAgentSimulator::DoMessage` `0x0042EFDC`). A replacement need not implement it.
  `kSC4MessageMissionEffectStarted` is `0xCC0D905F` as stored in the table at `0x00B08420` (the question's
  `0x5F900DCC` was a transposed reading): **nobody posts it** either. The only references are
  `cSC4CitySituationManager::Init` `0x0052DD20` (subscribe, server slot 6 `+0x14`), `DoMessage` `0x0052EFEC`
  -> handler `0x0052F096` and `Shutdown` `0x00527C58` (unsubscribe, `+0x18`). The handler checks
  `[this+0x9F0]`, calls its slot 15 (`+0x3C`) and stores the message's sender (`[vt+0x80]`) into the
  `cRZAutoRefCount<cIGZWin>` at `+0xA98` (`0x0052F0B9`-`0x0052F0C0`) - a latent bind-the-sender-window hook
  that nothing in this build triggers. Both are latent hooks, not live traffic. effects.md 12.

- ~~**EF9:** where the message names for `0x86AD10EE` and `0x697D01ED` live - they are not in the `0x00B08360`
  table (115 pairs).~~ **ANSWERED 2026-10-20 (re-answers34): DIFFERS from the "115 pairs" reading.** There is
  one message name table and it is larger than thought: it spans `0x00B07FD8`-`0x00B08FE0`, **513**
  `{u32 id, const char*}` pairs, every one resolving to a printable name. `0x00B08360` is simply a position
  inside it (`kSC4MessageMouseScrollBegin` sits there). `kSC4MessageAnimationTick` `0x86AD10EE` is entry
  `0x00B08250` (name at `0x00A8C06C`; sibling `0x86AD10EF` "kSC4MessageAnimationTickPriority" at
  `0x00B08258`). `0x697D01ED` has **no name anywhere**: the ID appears only at `0x0058D9CA` (the posting
  callback), `0x0059446C` (dispatch) and `0x00594DA5` (subscribe) - an unnamed internal message. effects.md 12.

- ~~**EF10:** whether "every component that reaches a brush or game record, directly or through meta, is
  `[1, 6)`" holds per effect.~~ **ANSWERED 2026-10-20 (re-answers34): MATCHES - measured on the shipped
  directory.** Ran `engine/formats/tests/effects_directory.rs` (`effects_directory_sweep`) against the
  installed game (`FC4_GAME_DIR` = the GOG install; `SimCity_1.dat`, 1,094,484 bytes, root version 4):
  `effects_directory: 72 effects reach a brush or game record, all through [1, 6) components; 887 other
  components have another range`. The narrower claim holds for all 72 effects whose reach set contains a
  brush or game record; the 887 non-`[1, 6)` components are all outside those reach sets (render-only).
  effects.md 5.2.

- ~~**EF11 - §6 dilation table:** which dilation entry `GetSimSpeed` values outside 0-2 use.~~
  **ANSWERED 2026-10-21 (re-effects-contracts4): MATCHES - `[1]` = 1.0 for every speed other than 1 and 2.**
  `GetSimSpeed` `0x008265C0` returns the raw `u32` at simulator `+0x3C`; the getter `0x007026F0` tests only
  `speed - 1 == 0` -> `[2]` and `speed - 2 == 0` -> `[3]` (`0x00702727`-`0x0070272B`); everything else, 0
  included, loads `[1]` (`0x0070272D`). No clamp or arithmetic. effects.md 6 step 1.

- ~~**EF12 - §6.1 / §5.2 / §5.3:** whether `DoEffectsTick` rebuilds every active effect each tick or only
  activates the new list, and how an unchanged-L rebuild avoids a fresh select-group draw.~~
  **ANSWERED 2026-10-21 (re-effects-contracts4): DIFFERS - `DoEffectsTick` does neither on an ordinary tick.**
  `DoEffectsTick` `0x00593D00` calls `UpdateActiveVisualEffects` only when the `+0xC78` object's slot 14
  reports queued effect files (then `ParseQueuedFiles` and a reload with `+0xB8C` = 1); otherwise it only does
  the snow-line step. Activation of new effects happens in `CreateVisualEffect` (`0x00593AB0`, `0x00593B68`),
  at the end of `DeactivateUnusedVisualEffects` (`0x00592A01`) and in `UpdateActiveVisualEffects`, which itself
  is a no-op until `kSC4MessagePostAppServicesInitComplete` sets `+0x1AD` (`0x0059458A`). No per-effect
  "built" flag: in `UpdateVisualEffect` `0x005919D0` a description takes part in a select draw only if it is
  in range at `L` and was not at `L_prev` (`+0xC84` / `+0xC88`), or the build is new, or a reload is running
  (`0x00591C08`-`0x00591C32`); existing components survive pass 1 unless reloading. effects.md 5.3 (rebuild
  test), 6.1 (correction), 14.

- ~~**EF13 - §7.1 emitter details** (a)-(e).~~ **ANSWERED 2026-10-21 (re-effects-contracts4): PARTLY.**
  (a) DIFFERS - `Start` `0x005B3000` seeds the countdown `+0x190` from the start-delay range `D+0x20..+0x24`
  if `D+0x20 >= 0`, else the interval range `D+0x28..+0x2C`, else 0; the first burst comes on the first
  draw-time update after the countdown runs out (immediately only when both ranges are negative).
  (b) loop-budget emitter: one burst of `N = ceil(curve[0])` (times the detail scales when `> 20`) per loop,
  each particle initialised with the burst age; not a caller-given count. (c) DIFFERS - the rate emitter has no
  seconds budget: it counts loops of `D+0x14` seconds in `+0x128` against `D+0x18`, like the loop-budget one;
  with flag A `0x08` it holds at the last loop instead of stopping. (d) MATCHES - the dispatch `0x005B96E0`
  returns at once while a delayed stop is pending (`+0x10` = base `+0x0C`). (e) MATCHES - live particles are
  updated (slot 20) first, then the emitter runs (`0x005B870A`). effects.md 7.1.1, 14.
- ~~**EF14 - §7.1.1 emitter gaps** (core-effects-fix4, 2026-10-21): (a) the plain emitter `0x005B3CF0` (flag A `0x04`)
  has no body in the spec; the core uses the rate emitter's integral over the whole step without loop counting,
  gaps or stop - what does it do (sub-steps, phase wrap, ages, does it touch `w`)? (b) the rate scale `+0xC0` for
  flags A bits 19/20/21: which wins when several are set, and is `m'` scaled at all when none is? (core: lowest
  bit wins; none -> not applied). (c) `SetDescription`'s `1e-6` floor for `D+0x14`: is it written back into the
  description (so the burst age and the gap use it too) or used for `ρ` only? (core: `ρ` only). (d) the
  particle init/update (main slot 20): when does init with age `a` fail, and when does an update return false?
  (core: a caller-given life; init fails when `a >= life`, a particle dies when its remaining life reaches 0).
  (e) the curve interpolation when `P − n` is outside `[0, 1]` and for an empty curve (core: clamped, 0).
  [all marked `[choice, EF14]` / `[choice]` in engine/core/src/effects/particles.rs]~~ **ANSWERED 2026-10-21 (re-effects14-4):** DIFFERS - (a) DIFFERS: the plain emitter `0x005B3CF0` only tops the live count up to `ceil(curve[0])` (no `m` or `+0xC0` scale) once `P > n`; `n`, `w` and `c` are never touched, nothing is sub-stepped, and new particles start at age 0 with no slot-20 call. (b) MATCHES: bits 19, 20 and 21 are tested in that order and the first set bit gives `k`, `k²` or `k³`; with none set there is no factor. (c) MATCHES: the `1e-6` is a local value and `D+0x14` is never written; only `ρ` is floored. (d) DIFFERS in where life comes from: `InitEffectsParticle` draws life uniformly from `[D+0x0C, D+0x10]` per particle; slot 20 does `age += t` and returns false when `age ≥ life`, so init fails when `a ≥ life`; when flags A `& 0x18000D00`, water/terrain kills can also fail it. (e) DIFFERS: the original does not clamp (reads past the end when `P − n > 1`, reachable when `ρ > 15`) and has no empty-curve guard; clamping and treating an empty curve as 0 stay `[choice]`. effects.md 7.1.1.


<!-- from: core-effects-parser4; questions originally in effects-parser.md -->
- ~~**EFP1 — Foreign switch-reader language and consumption**
  In effects-parser.md §2/§4, what exact contract does 0067A7D0 expose for
  dash/case/delimiters, aliases, duplicates, missing/empty values, unknown options,
  terminators and token consumption? Supply individual lookup outcomes and scratch
  indices, not a host-produced final parser result. Present owner rules do not prove
  these generic reader behaviors.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** DIFFERS — effects-parser.md §9
  establishes exact case-sensitive single-dash matching, first duplicate per lookup,
  in-place prefix marking, retained records, minimum arity, -/+ alphabetic
  terminators, positional-prefix collection and later unknown-switch validation.
  Eight independent token/index vectors; effects.md §19.1 links the service.
- ~~**EFP4 — Foreign registration identity and ownership**
  In effects-parser.md §5.1/§5.2, establish P+90 slots +44 and +48's matching
  interface contracts: named validation, duplicate/collision/replacement behavior,
  null-name identity/index selection and reference transfer of the supplied object.
  The owned bodies append locally; neither generates an anonymous name or counter.
  Do not claim a foreign counter boundary from the raw returned-index ledgers.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** MATCHES — effects-parser.md §11.1:
  lowercase copied keys in separate type buckets; null names append without named
  identities; returned indices are per-type vector positions. Named replacement
  preserves index, retains new before releasing old; identical pointers are no-ops.
  Lookup with null out only validates; failed lookup leaves out unchanged.
- ~~**EFP5 — Final lifetime and shared reference operations**
  In effects-parser.md §5.5/§8, positively identify destructor/refcount-zero cleanup
  and held-interface release order. Linked callback 0059EBE0 is not yet proved a
  destructor; shared AddRef/Release delegation cannot certify parser-exclusive
  lifetime behavior. Distinguish this class's factory from the separate file queue.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** DIFFERS — effects-parser.md §11.2:
  actual AddRef/Release stubs are 0059F3C0/0059F3D0, not 0059F3E0/0059F3F0.
  Final Release invokes deleting destructor 005A27F0, member cleanup 0059F4E0.
  Held component P+490 releases first, then description/container cleanup;
  interfaces P+A8, P+94, P+90, P+8C release in that order, followed by base
  destruction and allocation free. Callback 0059EBE0 is distinct.

- ~~**EFP3 — Error transport and lifetime outside normal domain**
  In effects-parser.md §3, identify the actual error transport of 009EF84F and
  cleanup/unwind behavior of the select/LOD/prob errors. Do not assume false-return,
  transaction rollback or resumed parsing from the misleading export name.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** DIFFERS — effects-parser.md §10:
  009EF84F raises a noncontinuable MSVC C++ exception through RaiseException,
  code E06D7363, flags 1, three information words. Owner frames install no local
  catch/unwind cleanup. Earlier token marking and description writes remain;
  rejected-value conversion, append and normal temporary cleanup are not reached.
  No implicit end hook or rollback is performed. File-reader recovery is a
  separate foreign scope, with no continuation guarantee from these owner bodies.
- ~~**EFP6 — Command-library dispatch policy**
  In effects-parser.md §6, establish foreign root/nested registrar duplicate,
  lookup/case, descriptor ownership and block-hook dispatch policy. Current proof
  is the owner's ordered literal-to-descriptor registration, plus the concrete
  sequenceEffect normal handler, not all root command semantics.~~
  **ANSWERED 2026-10-23 (re-effects-commands5):** DIFFERS — effects-parser.md §12:
  exact byte/case lookup; last registration wins. Root duplicates append owned
  descriptors; nested duplicates overwrite without destroying the old descriptor.
  Registration invokes +08 initialization, not AddRef. Search is innermost block
  outward, then root. Wrappers forward simple +0C, block enter +14 and exit +18;
  StartBlock pushes, end invokes the exit before popping. Only nested-selected
  commands get the post-handler unexpected-switch check. Six independent vectors.

<!-- from: core-effects-close5 (2026-10-24) -->
- ~~**EF18 - §6.1 EffectsParser queue wrapper, EffectsBootstrap, collection lifecycle:** what are the queue/file processing and bootstrap lifecycle contracts for the separate `EffectsParser` and `EffectsBootstrap` owners, and any `cSC4EffectsCollection` behavior beyond the component directory in effects-parser §11.1? §6.1 only names `ParseQueuedFiles` in the manager's reload sequence; effects-parser §1 expressly excludes that wrapper, and effects.md supplies no bootstrap algorithm. [Queue processing/bootstrap return explicit Unsupported; the settled component directory remains implemented. EF17 is reserved by the coordinating brief and is not duplicated here.]~~

  **ANSWERED 2026-10-24 (re-effects-ef18):** DIFFERS — effects.md §§6.2, 18.6:
  queue/bootstrap are supported manager operations, not independent extra COM
  owners. Retained inputs parse synchronously; success clears UI error and sends
  C9F775BB, failure reports the diagnostic. Bootstrap orders base resource,
  optional packed main.fx, extra resources, then unpacked files when selected.
  Collection Begin retains descriptions but clears bindings/triggers; publication
  copies and fixes references without rejecting unresolved names; End rebuilds
  ID and trigger indices. Twelve proposed conformance vectors, including optional
  file exception suppression (flag bit 0 clear).

<!-- from: core-effects-close5 (2026-10-24) -->
- ~~**EF19 - §§3, 8.3, 9.2 component transform decoding and numeric order:** what is the mapping from the 56-byte `S3DTransformLegacyLoad` placement to rotation/scale/translation, and the intermediate operation/store order for component world positions? [Implement the literal semantic parent translation + parent rotation/scale of local offset using binary32 products and left-to-right sums, with direct parent translation for a zero local offset. Do not decode the opaque placement bytes or claim native numeric equivalence. Manager legacy placement decoding remains outside this component API.]~~

  Clarification: effects-parser §13 already settles the parser's in-memory matrix/offset/scale storage and rotation stores, without requiring a renderer row/column convention. EF19 concerns the legacy resource-load mapping and component world-position convention/order, not those settled parser stores. An identity parent returns local translation directly, including its original binary32 bits.

  **ANSWERED 2026-10-24 (re-effects-ef18):** DIFFERS — effects.md §3.1:
  legacy placement is nine row-ordered float words, translation xyz, uniform
  scale, and a revision word whose low byte is retained. Loading sets rotation
  and translation flags even for numeric identity. Component shortcuts test
  flag bytes, not zero offsets. Dot products sum z,x,y with caller x87 precision;
  brush and game have the separately documented binary32 spill boundaries.
  Six proposed conformance vectors include cancellation and signed-zero cases.
