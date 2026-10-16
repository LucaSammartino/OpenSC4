# Lane glue22: the effects layer glue

Status: **engine lane**, branch `glue22`, worktree `.claude/worktrees/glue22`.
Behaviour source: [docs/specs/effects.md](../specs/effects.md) (`cSC4EffectsManager`).

The effects **core** landed from `cores19` (`engine/core/src/effects/`, capi `fc4_*`, the `ef17_*` conformance
vectors). This lane is its plugin side: the host table on the game, the GZCOM classes, and the same-process shadow.

## What is wired

| Piece | Where | State |
|---|---|---|
| `fc4_effects_host` on the game | `engine/plugin/FC4EffectsHost.{h,cpp}` | live and shadow modes |
| `cSC4EffectsManager` (CLSID `0x49822F75`, city slot 85) | `engine/plugin/cFC4Effects.{h,cpp}` | `enable = effects` |
| `cISC4VisualEffect` + the effect component at `+4` | `engine/plugin/cFC4Effects.{h,cpp}` | `enable = effects` |
| the same-process mirror | `engine/plugin/FC4EffectsShadow.{h,cpp}` | `effects_shadow = 1` |
| the compiled slot tables | `engine/plugin/FC4Interfaces.h` | `cFC4ISC4EffectsManager`, `cFC4ISC4VisualEffect`, `cFC4ISC4EffectComponent`, `cFC4ISC4DisasterManagerEffect` |
| the animation tick | `cFC4EffectsManager::DoMessage` | `kSC4MessageAnimationTick` `0x86AD10EE` → `fc4_effects_tick` then `fc4_effects_finish_hook` |

The host reaches, per `EffectsHost`:

- the simulator's animation time dilation (slot 32) and its Pause / HiddenPause / Resume / HiddenResume
  (slots 9, 10, 12, 13) — §6 and §9.2;
- the 24-hour clock's slot 26 with 1 / 0 — §9.2;
- the city's cells and `LocationIsInBounds` (§8.3, §9.3 step 1);
- terrain `ApplyBrush` (terrain slot 57) and the long terrain operation's open / close — §8.3;
- the fire disaster manager's `IgniteCellRadius`, `SetCause` and `ExtinguishCell` (§9.3 steps 1 and 4), reached
  as the spec states: city → disaster layer slot 7 `GetDisasterManager(0x621CDA33)` → QI `0x49B43F57`;
- the toxic-spill manager's `AttemptDisasterStart(x, z)` (`GetDisasterManager(0xEBFB0404)`) — §9.3 step 2;
- `DemolishRegion` on the demolition utility over a full `SC4CellRegion` — §9.3 step 3.

## The draw-pass finish decision (EF2)

**Effects finish outside the draw pass.** The glue does **not** synthesise the end message.

The original's only route to the finish test is `DeactivateUnusedVisualEffects` (`0x005922A0`), called from
`Update(SC4DrawContext)` (`0x00593120`), which has no other caller (effects.md §7, EF2 — proven). A session that
never draws the city therefore never finishes an effect and never posts its end message, so the volcano stays in
state 5 or 7 and the effect-disaster emergency pause never lifts.

This implementation runs the **same §7 rule** from a deterministic point instead: the manager calls
`fc4_effects_finish_hook` once per animation tick, after `fc4_effects_tick`. That is the core's
`FinishMode::Hook`. Nothing is faked — the sweep, the finish test, `Stop(1)` and the end post are the §7 code; only
the *call site* moved, which is exactly the escape the spec offers ("run the same sweep from somewhere else, or
synthesise the end message").

The original's behaviour is still there for comparison:

| ini | effect |
|---|---|
| `effects_finish = 0` (default) | `FinishMode::Hook`: the sweep runs after each animation tick |
| `effects_finish = 1` | `FinishMode::DrawPass`: `fc4_effects_draw_pass` only — the original's quirk 1, and a session that never draws never finishes an effect |

## Animations are stubbed

The core's render components (particles, decals, sequences, sounds, cameras, automata attractors) are
`Component::Render` placeholders. Nothing animates them in FreeCity4, in the core or in this glue.

Consequently `EffectsHost::render_component_active` answers **false always**: a stubbed render component does not
hold an effect. That is the core's documented `[choice, EF1]`, and it is a **difference from the original**, not
a finish test: the real game waits for the shipped decals' `life` (45 s lava, 50 s crater, effects.md §7), so
`volcano_control` there finishes about 50 effect-seconds after its creator starts plus one further sweep for the
meta parent. Here it finishes as soon as its brush, game and meta work is done.

This is deliberate and stated rather than papered over: faking the particle and decal lifetimes would be inventing
an animation system and pretending it was the finish test. See question 2.

## The shadow

`effects_shadow = 1` runs a second `fc4_effects` in the same process, over an `EffectsHostContext` in shadow mode:
its reads come from the game and **every write is recorded in the context instead of being made** (FC4EffectsHost.h),
so the mirror never changes the city. It is ticked with the same microseconds as the live manager and finishes
through the same sweep. The live side's create / start / stop / end-message calls are mirrored in
(`EffectsShadowObserveCreate` / `...StartStop` / `...EndMessage`).

At shutdown it logs one line:

```
effects shadow: ticks N, created N, end posts N, select draws N, brush applies N, sweeps N, host writes N, differences N
```

`differences` counts the times the mirror could not make an effect the live side made. The run's shadow summary
line goes into `docs/runs/<tag>.md` per run.

What the shadow does **not** do: it does not hook the original's vtables. So when `enable = effects` is **off**,
the comparison is of the mirror's own state and its recorded writes only. The game runs use `enable = effects`
together with `effects_shadow = 1`, which is the mode where the two cores are driven call for call.

## The disaster layer's effect path

Untouched and still working: `cISC4DisasterManagerEffect` (IID `0xCA7B3F3C`, vtable `0x00A9E190`) has its compiled
declaration in `FC4Interfaces.h` (slots 3-5: `StartDisasterEffect`, `AttractDisasterEffect`, `CancelAttract`), and
the disaster layer's own effect path (`fc4_disasters_start_effect`, `core/src/disasters/effect.rs`) is unchanged.
`cFC4Residential.cpp` and `cFC4Pollution.cpp`, the two live callers of the effects manager, keep working: they
reach whichever manager the city runs.

## Ini keys

| Key | Default | Meaning |
|---|---|---|
| `enable = effects` | off | replaces `cSC4EffectsManager` (CLSID `0x49822F75`) with ours |
| `effects_shadow` | 0 | `1`: the same-process mirror (FC4EffectsShadow.h) |
| `effects_finish` | 0 | `0`: finish outside the draw pass (EF2); `1`: the original's draw-pass compat mode |

`disable` still wins. `FC4_ABI_VERSION` is unchanged at `69u`; this lane adds no `fc4_*` entry point (it consumes
the ones `cores19` added).

## Questions

1. **The effect directory is not read from `SimCity_1.dat`.** The core parses the real resource format (25 brush,
   64 attractor, 57 game records, 1154 effects, effects.md §3) and the `ef17_*` vectors cover it. The plugin's
   manager loads the core's synthetic volcano / meteor-crater directory (`fc4_effects_directory_volcano`) because
   the plugin has no DBPF reader yet: the game's `cIGZPersistResourceManager` hands out `cIGZPersistBufferResource`
   whose buffer is a pixel buffer (`cIGZBuffer`), not raw bytes, and wiring the raw resource path is its own piece
   of work. The synthetic directory holds the records §10 needs (`volcano_control` and its nine creators, the
   meteor-crater game and brush records), so the disasters work; the other 1100-odd effects are not reachable by
   name in game yet. **Should the loader go through `cIGZPersistResourceManager::GetPrivateResource` with a
   `cIGZPersistBufferResource`, or is there a raw-byte resource interface the original uses?**
2. **The finish test's render components.** Animations are stubbed, so `render_component_active` is false and
   particles / decals / sequences / sounds never hold an effect. `volcano_control` therefore finishes when its
   brush, game and meta work is done, not after the shipped crater decal's 50 effect-seconds. This is a measured
   difference from the original, kept and reported rather than faked. **Is a stubbed-inactive render component the
   right default, or should a render component hold the effect until something else stops it (which would keep the
   original's "never finishes without a draw" behaviour and break the disasters)?**
3. **The 24-hour clock's pause.** effects.md §9.2 says "the 24-hour clock (`0x00B43DBC`) slot 26 with 1". The
   compiled declaration in `FC4Interfaces.h` has `HiddenPause(bool)` at slot 26 and `Pause(bool)` at slot 24, so
   the glue calls `HiddenPause(on)`. **Is slot 26 the clock's hidden pause, or its ordinary pause?**
4. **The long terrain operation.** effects.md §8.3 gives terrain slots 87 (`BeginLongTerrainOperation`) and 88
   (`EndLongTerrainOperation`). `cISTETerrain` in the SDK has no such pair; the glue tracks the stroke's long
   operation open/closed in the context and logs the write once. **What are the exact names / slots of the long
   terrain operation on the terrain interface?**
5. **The demolition filter.** §9.3 step 3 passes a per-occupant filter (`fc4_effects_demolish_filter_skips`:
   signposts and text decals always, networks and flora by the flags, size limits). `cISC4Demolition::DemolishRegion`
   takes an `cISC4OccupantFilter*`, which is a different mechanism; the glue passes no filter and demolishes the
   whole rectangle. **How is the effects filter expressed as an occupant filter?**
6. **The queued posts.** §12 requires the end message and the game records' messages to be posted queued through
   the message server (`0x00B43CCC`) with the effect as the `IGZUnknown`. The host records every post (type, data1,
   cell) and the shadow's counts include them, but the live mode does not yet make the post. **Is
   `cIGZMessageServer2::MessagePost` with a `cFC4Message2Standard` the right transport, and what goes in the
   message's object field?**
7. **`CreateVisualEffect` by ID pair** (slot 6, §4.2). The core's directory lookup is by name; the pair's
   `HasEffectDesc` semantics are not modelled, so the ID-pair form returns false and logs. Pollution
   (pollution.md §4.10) is the caller. **What do the two IDs of the pair select on?**
8. **The two `CreateVisualEffect` overloads' slot order.** effects.md §4.1/§4.2 and docs/abi rows 2274-2275 both
   put the **name** form at slot 7 (`0x005939B0`, 2 dwords, pop 8) and the **ID-pair** form at slot 6
   (`0x00593AE0`, 3 dwords, pop 12). The SDK's `cISC4EffectsManager.h` declares them name-first, which is the
   opposite order. The compiled declaration in `FC4Interfaces.h` follows the measured order. **Is the SDK header's
   declaration order wrong (MSVC lays overloads out in reverse declaration order — see the note in
   `cIGZPersistBufferResource.h`), and if so should `cFC4Residential.cpp`'s `CreateVisualEffect(name, &e)` call
   change?**

## Files

New: `engine/plugin/cFC4Effects.{h,cpp}`, `engine/plugin/FC4EffectsHost.{h,cpp}`,
`engine/plugin/FC4EffectsShadow.{h,cpp}`.

Shared, edited additively: `engine/plugin/FC4Interfaces.h` (one new section), `engine/plugin/FC4Subsystems.cpp`
(`kEffectsClasses`, the `effects` subsystem entry, `EffectsShadowStartup`), `engine/plugin/FC4Config.cpp`
(`effects_shadow`, `effects_finish`), `engine/plugin/FC4Observer.cpp` (the city hooks),
`engine/CMakeLists.txt` (the six sources), `engine/capi/include/fc4.h` (the ABI 69 changelog entry, merged from
`cores19`), `engine/conformance/README.md`, `engine/conformance/src/runner.rs`, `engine/tests/capi_smoke.cpp`.
