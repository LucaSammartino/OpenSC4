| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-shadow7: effects record shadow | Codex / glue-effects-shadow7 (detached HEAD) | ready: measured name lookup scope; effects 73/73 exact, zero resyncs, ref_warnings=0; cargo 6918/0, Clippy clean, CTest 74/74; EF20 open | Publish reserved commits; review narrow manager lookup coverage; coordinator may append effects_shadow to the current shadows profile. EF20 blocks broader capture. |

## Result and measured scope

`effects_shadow = 1` now selects a record-only comparison when the native
effects manager is active. Default remains off. The existing replacement-call
mirror remains a separate path; do not set `enable = effects` for this native
measurement. No replacement is enabled by the new shadow.

At city load and monthly, the shadow opens the packed directory
`EA5118B0/EA5118B1/1` through the public SDK raw-record/file adapter. Rust parses
it with `core/src/effects/EffectDirectory`, then compares native manager
`HasVisualEffect` with core name membership for every name entry in original,
uppercase and lowercase form, plus empty, sentinel and absent-name probes.
Native answers never initialize or repair the core. Capture/parse failures are
counted as mismatches; empty captures cannot earn an exact line. Native mode
has no animation subscription, mirror ticking, effect creation or host writes.
Borrowed city/manager references are not released; acquired interfaces and
created adapters use the audited release paths.

The load date comes from the live simulator interface because the observer's
published Julian date is still zero at load. Dated lines use the single
lowercase `effects shadow` label and explicit mismatch/refusal/resync counts.

Coverage review should identify only **cSC4EffectsManager, section 4.2 slot 5
HasVisualEffect name membership**, including case handling and missing names.
The resource's name map is an input to that comparison, not a comparison of
native decoded resource fields. **Do not credit whole owner rows** for
cSC4EffectsManager or cSC4EffectsResource from this limited observation. No
cSC4EffectsCollection publication/copy/fixup/binding/trigger/camera state,
EffectsParser retained queue, EffectsBootstrap service order, EF19 placement,
brush/scrubber records, live effects or renderer state is compared. The EF18
and EF19 owner closures from core-effects-close6 remain unverified by this lane.
No coverage tables were read or edited.

**EF20** was appended to [questions/effects.md](../specs/questions/effects.md):
native getter argument/return/ownership signatures, enumeration/container
framing, collection-reader identity and a read-only queue/bootstrap capture
protocol are needed before those fields can be captured without guesses.
This is a ready partial measurement under the task's explicit gap policy.

Added C export `fc4_effects_verify_names`, documented at the end of `fc4.h`;
ABI changed, version unchanged. One focused ABI test checks exact case queries,
priority-zero membership, an injected native difference, malformed input and
missing callback refusals. Shared observer additions are grouped at the ends
of the record-shadow calls. No tools, game files or generated ABI entries changed.

## Final validation

Final engine source **edf633330**, unchanged during its full gate:

- `qbuild all`: **PASS**, cargo **6918 passed, 0 failed, 0 ignored** (all result
  lines summed), Clippy **0 diagnostics**, `-D warnings`.
- Its queued native operation **qbuild-20261024-050210-build-ctest**: **PASS**,
  build **47/47** (2.3 s), CTest **74/74** (73.9 s), overall **77.8 s**.
  This is the native build/CTest verification queue used by qbuild; no direct
  MSVC configure/build/ctest or duplicate outstanding verification was launched.
- Explicit owned-file fmt: two Rust files, **0 diff lines**. Focused ABI test:
  **1 passed, 0 failed**. Final logs: `build-runs/qbuild-test.log`,
  `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

Earlier evidence remains distinct: `gate-r1-failed/` preserves the missing
`cIGZCOM.h` build failure (CTest skipped); `gate-r2-failed/` preserves the
return-channel annotation failure (build passed, CTest 73/74). The reviewed
boolean call cites effects section 4.2 and the public SDK. `gate-r3-passed/`
preserves the passing gate before the subsequently diagnosed load-date fix.
The initial q.py Bash sandbox failure and incorrect relative fmt paths are
transport diagnostics, not source gate verdicts; no quick/baseline full build
was run.

## Game evidence

Final counted line:

```text
effects7-names-2032-20261024b 73/73
```

Harness: `effects 73 lines, 73 with every mismatch count 0`,
`effects resynced lines: 0`. **All 73 dated lines match the four-digit date
shape**, from `2026-01-27 load` through `2032-01-01 month`, each exact, no
capture refusal. Each captures **1,094,484 bytes**, compares **1,154 raw name
entries** with **3,465 queries**; total **252,945 native queries**. This counts
entries, not distinct folded names. No rendered frames or other owner state
is inferred from those counts.

Final run [411](../runs/effects7-names-2032-20261024b-411.md): **clean**,
`ref_warnings=0`, all problem counts zero, turbo to **2032-01-01**, **28.797 s**
city runtime, **40 s** wall, slot 2. DLL SHA256
`166CC8C4017909D81F0B6002413D2E556D66C8FAB91F93D7456D880289D55A3F`.
No normal-speed rerun is needed: no turbo-only difference exists.

Earlier run [410](../runs/effects7-names-2032-20261024a-410.md) was clean and
field-exact 73/73, but its load date used Julian zero. It is retained as
diagnostic evidence, not final coverage evidence. Its already-completed
verdict was accepted after a filesystem watcher missed the event; it was
never resent. Final B used a single blocking wait without qwait. Logs and
verdicts for both unique labels remain under `build-runs/game-results/`.
No game request or build operation remains outstanding.

## Landing and remaining work

Supervisor publishes the detached commits to the reserved lane branch; queue
entry is ready for the measured scope. House defaults and profiles are
unchanged. The coordinator must review scope before crediting coverage, and
can append `effects_shadow` to the current single shadows-profile string on
main. This clean-room lane cannot read/edit tools or merge main under its
transport restrictions. The coordinator also regenerates the questions index.
Answer EF20 before adding native collection/queue/bootstrap/placement capture;
keep that unmeasured scope uncredited and disabled.
