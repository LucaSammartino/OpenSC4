# effects: answered questions (archive)
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
