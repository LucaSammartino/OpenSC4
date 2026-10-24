| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-ext5: remaining automata owner shadows | Codex / glue-automata-ext5 | in progress: round 3 main merge, additive header and regress profile conflicts resolved | Full gate, unique 2032 run, second main merge, final gate if merged, ready queue. |

## Handoff

- Goal: round 3 integration only; no new implementation scope.
- Done: merged main content; retained both automata construction and netrules C declarations. lib.rs merged automatically with strings_shadow; exactly one regress shadows line, main keys plus automata_owners_shadow.
- Next: commit merge, run qbuild all on stable source, game through 2032 with new unique label (no qwait), merge main again immediately before queue and repeat native gate if merged.
- Detached HEAD intentionally; do not switch branches or update refs. ABI true from round 1 export.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-automata-ext5-r3-manifest.json.
- No outstanding game/build request. Previous rounds' evidence below remains valid for those source revisions.

## Round 2 report

Merged main with `git merge --no-edit main` in merge commit `ee9db8e59`. Resolved FC4WriteCapture.cpp by retaining automata default independent native variant encoding and netocc opt-in property transport. Kept both sides of the shared capture layout, including `variants_` and object/reference bookkeeping, and main construction/dependency changes. Kept both config keys. Combined conflicting regress shadows entries into one entry containing all keys from both sides, including netocc and automataowners. No new implementation scope or C exports in round 2; round 1 adds one export, so queue ABI remains true. No ABI version bump.

Stable final source `ee9db8e59`: `qbuild all` PASS, **6804 Rust tests passed, 0 failed, 0 ignored**, clippy **0 diagnostics**. Its coordinator native verification request `qbuild-20261024-010820-build-ctest` completed with **build 207/207, CTest 70/70**. This is the full native verify path, including every plugin INI test; no duplicate check was launched. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; terminal verdict `build-runs/build-results/qbuild-20261024-010820-build-ctest.verdict`.

[Run 349](../runs/automata-ext5-r2-2032-20261024-a-349.md): Konradshohe, turbo through **2032-01-01**, **clean**, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, other WARNING lines=0. All three shadows enabled together. DLL SHA256 `3c1d8d7050131323febcac27293cafa7a42b48a39ff4b25bff86e75bc505128d`. Unique request collected by one blocking filesystem-watcher wait to terminal clean; no qwait or duplicate request. Evidence: `build-runs/game-results/automata-ext5-r2-2032-20261024-a.{log,verdict}`.

```text
automata-ext5-r2-2032-20261024-a automata 72/72
automata-ext5-r2-2032-20261024-a automataowners 72/72
automata-ext5-r2-2032-20261024-a netocc 72/72
automataowners shadow 2032-01-01 month: cSC4AutomataManager=4/4 bytes=5656 mismatches=0; cSC4TrafficAutomataManager=2/2 bytes=5384 mismatches=0; cSC4Automaton=633/633 bytes=203557 mismatches=0; cSC4AutomataController=297/297 bytes=27825 mismatches=0; cSC4ConstructionOccupant=1/1 bytes=262 mismatches=0; mismatches=0
netocc shadow month 2032-01-01: mismatches=0 fixups=857 records=1283 bytes=478251 network=857 prebuilt=0 bridge=0 tunnel=0 underground=0 pipe=403 subway=0 connection=0 pipe_connection=2 subway_connection=0 pole=3 line=18 pylon=0
automata shadow: city shutdown: 73 lines, 73 with mismatches=0; components compared 438, bytes compared 782015, mismatches 0
automataowners shadow: city shutdown: 73 lines, 73 with mismatches=0
netocc shadow summary: clean=73/73
```

Each shadow is **73/73 including load**. Across those samples, remaining-owner record comparisons were: AutomataManager 292/292, TrafficAutomataManager 146/146, Automaton 46277/46277, AutomataController 21758/21758, ConstructionOccupant 154/154. Fields remain exactly the round 1 table below; these are record-codec comparisons from the same inputs. Netocc compared 1283 records and 478251 bytes each sample, with monthly fixups=857 and load fixups=0.

Questions: none; no remaining differences. No outstanding builds or game requests. Coverage maps untouched. All commits remain on detached HEAD for supervisor publication; no branch/ref changes or pushes. Next: coordinator land and credit coverage.

## Round 1 report (preserved)

Added default-off `automata_owners_shadow` / `FC4_AUTOMATA_OWNERS_SHADOW` to FC4AutomataShadow. At load/month it compares inherited manager records, recursively captures referenced automata/controllers with cycle deduplication, and captures indexed constructions through occupant-manager Write references. Unsupported automaton/controller classes fail explicitly. Captured variants now use verified native getters and an independent encoder of foundations §9.2, rather than the former false-return stub.

| Owner | Compared fields | Exact record comparisons across 73 samples |
|---|---|---|
| cSC4AutomataManager | Version, max count, active, time-of-day flag, ordered automaton refs, source-building count pairs; embedded in all four concrete managers | 292/292 |
| cSC4TrafficAutomataManager | Version, embedded base, default path type, layer cursor, complete incident list; vehicle/pedestrian routes | 146/146 |
| cSC4Automaton | Complete supported derived records, including occupant base/properties, position/destination/orientation, radius/default speed, flags/dispatch, attractor/distance/strength/composite vector, leader, state stack, model keys/instance, callbacks, tweens, priority/group | 46256/46256 |
| cSC4AutomataController | Version, occupant, template, position, lifetime scalars, flags, controlled refs; generator periods/group/count or attractor tag/curve | 21863/21863 |
| cSC4ConstructionOccupant | Occupant base/properties, lot/building, transform, both bounds, crane presence/position, ground/roof/effect refs, all progress/crane scalars, crew tuples | 150/150 |

These validate record codecs from identical native inputs, not movement or lifecycle behavior. Manager byte counts include concrete extensions. Unsaved state such as hard limits is outside this comparison. Construction Read stores crane presence in a load-only flag; the shadow explicitly seeds live presence from that captured bit before Write, without creating a crane. One new C export, `fc4_automata_construction_roundtrip`, releases all input references on success/failure. ABI exports changed; ABI version unchanged.

## Validation

Final stable engine source `35981431d`: `qbuild all` PASS; Rust **6673 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**, native build **47/47**, CTest **69/69**. Native request `qbuild-20261024-000625-build-ctest` completed through the coordinator queue. Owned Rust fmt: 2 files, zero diff. Focused construction test passes, covering crane/scalar preservation and reference cleanup on truncated input. Logs: `build-runs/qbuild-{test,clippy}.log`, `fc4verify-{build,ctest}.log`.

[Corrected run record](../runs/automata-ext5-r1-2032-20261024-b.md): **automata-ext5-r1-2032-20261024-b 72/72**, `automataowners 72/72` monthly; **73/73 including load**. Konradshohe, turbo, reached 2032-01-01; clean, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, other WARNING lines=0. DLL SHA prefix `ffe49742`. Exact log lines:

```text
automataowners shadow ? load: cSC4AutomataManager=4/4 bytes=5784 mismatches=0; cSC4TrafficAutomataManager=2/2 bytes=5512 mismatches=0; cSC4Automaton=649/649 bytes=208728 mismatches=0; cSC4AutomataController=300/300 bytes=24040 mismatches=0; cSC4ConstructionOccupant=7/7 bytes=1810 mismatches=0; mismatches=0
automataowners shadow 2032-01-01 month: cSC4AutomataManager=4/4 bytes=5656 mismatches=0; cSC4TrafficAutomataManager=2/2 bytes=5384 mismatches=0; cSC4Automaton=633/633 bytes=203565 mismatches=0; cSC4AutomataController=297/297 bytes=27781 mismatches=0; cSC4ConstructionOccupant=0/0 bytes=0 mismatches=0; mismatches=0
automataowners shadow: city shutdown: 73 lines, 73 with mismatches=0
```

Construction sample sizes ranged 0–11; 150 actual records were compared. Existing `automata` stayed exact 72/72. Full evidence is in `build-runs/game-results/automata-ext5-r1-2032-20261024-b.{log,verdict}`. Added the new key at the end of the regress shadows profile only after this exact verdict; no coverage maps changed.

## Preserved diagnostics and next steps

[Initial run](../runs/automata-ext5-r1-2032-20261023-a.md) reached 2032 clean but was 0/72 for the new shadow: variant capture was a stub. Its separate log/verdict remain intact. Source `7207ed6dc` then built but failed CTest 68/69 on consuming GetValUnknownAs's uncertain return; full failed log is `build-runs/fc4verify-ctest-7207ed6dc-failed.log`. The final caller uses only the documented output pointer. Each repeat followed a diagnostic and source correction.

Questions: none; no remaining differences. No outstanding operations or game requests. Coordinator should review record-only coverage credit and land the detached-HEAD commits. No branch/ref changes or pushes were made.
