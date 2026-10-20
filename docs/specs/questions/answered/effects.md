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
