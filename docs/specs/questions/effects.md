# effects: open questions
<!-- Questions about [effects.md](../effects.md). Append new entries at the END; when one is answered, move it to answered/effects.md. Rules: README.md. -->

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF1 (narrowed):** the exact finish time of `volcano_control`. It is now known that only **particle**
  components can hold it: brush ends at `t > length` (2.0-4.0 dilated effect-seconds), decals self-stop when
  their fade product passes 1.0 and mode byte `+0x15 == 2` (`0x00582920`), and game/attractor/camera all carry
  the ignore bit. The open part is sharp: *do `cSC4ParticlesEffect` components ever report inactive, and by what
  path?* The bytes show no self-stop (`started` `+0x05` is written only by `Start` `0x005B9CF0` and `Stop`
  `0x005B9E70`; `CheckForStop` `0x005B9D40` fires only when a stop was already requested and the live count
  `+0x14` hits 0), yet all 1921 shipped particle components have `flags = 0`, so the finish test waits on them.
  **Probe:** one runtime trace of `0x005BFD70` on a live `volcanocreator*` logging per component `comp+0x40` and
  each `IsActive()` return - this also gives the wall-clock end. Static alternative: the emission path around
  `cSC4ParticlesEffect::CreateParticle` `0x005B7C20`, looking for a terminal `Stop(0)`. Static bound: at least
  the child's brush length plus one draw-pass sweep.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF4:** the meaning of visual effect flag bits 0, 3, 6 and 8, and when an effect created during a draw-pass
  sweep (`+0x1B4`, effects.md §5.1) is activated in a session that only ticks. **Probe:** untested at runtime.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF5:** whether a hidden pause counts as "paused" for the time dilation, which would make the 5 s and 15 s
  pauses of effects.md §9.4 real-time seconds. **Probe:** compare `GetAnimationTimeDilation` across a hidden pause.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF6:** what `data1` is on a `kSC4MessageEffectLocation` (`0x2A7B582E`) posted from a game record inside a
  **particle-spawned** slave. Statically it is the game component's `data` field (`+0x58`), which for such a
  slave is the particle's `+0x26C` value and **not** the disaster instance, so effects.md §10.2's earlier
  "data1 = the instance" is wrong for that case. **Probe:** break on `0x005BD3C0` when `+0x30 == 0x2A7B582E` and
  compare `data1` with the instance pointer; if they match there is a parameter-13 path not yet found, or
  `0x00576E70` matches on something else.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF7:** the exact semantics of `cSC4ModelBasedParticlesEffect +0x26B / +0x26C` and `+0x26F / +0x270`. Ghidra
  splits them into five byte fields and that class has no `SetVoidParams` to populate them from a parent's
  parameter 13. **Probe:** the class layout from `0x005AEC90` in full, or a runtime watch.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF8:** who posts and who consumes `kSC4MessageTornadoDead` `0x0A36F1E8` and
  `kSC4MessageMissionEffectStarted` `0x5F900DCC`, both present in the message name table
  `0x00B08360`-`0x00B086F8` and neither traced. **Probe:** xref on each constant.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF9:** where the message names for `0x86AD10EE` and `0x697D01ED` live - they are not in the `0x00B08360`
  table (115 `{u32 id, const char* name}` pairs). **Probe:** scan for another name table.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- **EF10:** whether "every component that reaches a brush or game record, directly or through meta, is `[1, 6)`"
  holds per effect - the narrower claim that survives the measurement that 840 of 3373 components are not
  `[1, 6)`. **Probe:** the assertion already made by `engine/formats/tests/effects_directory.rs`.
