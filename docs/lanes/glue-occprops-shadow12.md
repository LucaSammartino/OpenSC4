| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-occprops-shadow12 (occupants and props shadows) | Codex / glue-occprops-shadow12 | ready, partial getter coverage: cargo 7294/0, clippy clean, CTest 81/81; run 512 clean, construction 73/73 | Integrate measured getter scope; append construction_shadow to current regress shadows profile after merge; lifecycle/render tracing and LD39 remain open |

## Report

Added `construction_shadow = 1` / `FC4_CONSTRUCTION_SHADOW`, default off, in source commit `2c2420ff7`. On city load and each month it uses the documented typed occupant iterator (occupant-manager §9.2/§9.4, slot 45). For each initialized construction it compares native position bitwise against production `ConstructionOccupant::position`, using only the native bounding box as input, and checks class `0xA97F909E` and occupant type `0x096E763C`. Bounds are inputs, not independently verified outputs. Non-finite inputs/outputs and missing bindings fail explicitly. Empty samples are labeled.

The manager holds each occupant throughout its callback. Serializable QI is independently acquired and released solely to read class identity. No native Write, Read, Init, Shutdown, Draw, tick or lifecycle method is invoked. In particular this avoids dereferencing construction field 13 after the teardown route documented in LR14. The private Rust static-link helper adds no public C ABI/DLL export or ABI-version change.

## Owner inventory and selection

The task-named coverage-map rows list these eleven owners:

| Owner | Mapped implementation |
|---|---|
| cSC4ConstructionOccupant | core/src/lot_records |
| cSC4PropOccupant | - |
| cSC4PropOccupantVisualEffect | - |
| cSC4PropOccupant* | - |
| cSC4SignpostOccupant | - |
| cSC4TextDecalOccupant | - |
| cSC4Occupant* | - |
| cSC4*OccupantFilter | - |
| cSC4*Occupant | - |
| cSC43DPlaceableObject | - |
| LinearRandomIterator | - |

Construction is the sole explicitly mapped implemented owner, so the mapped-owner byte ordering has only one candidate. The map supplies no owner byte column; the coverage-report subsystem total is 69,634 native bytes and is not claimed as construction-only bytes. Existing prop modules/fixtures and other shadows were left intact. No coverage map or shadows manifest was edited; whole-owner verification is not claimed.

## Validation

- Focused transport test: 1 passed; owned Rust file formatting passed. Evidence: `build-runs/focused-construction.log`. An initial relative fmt path was rejected; the absolute path passed. Git Bash q.py failed to start (CreateFileMapping access denied); the focused Python subprocess runner preserved that separate evidence in `build-runs/q/`.
- One final `qbuild.py --wt <WT> all` on stable source: **PASS**, workspace **7,294 passed / 0 failed** (48 result groups), clippy **0 diagnostics** with warnings denied.
- Its single queued native request `qbuild-20261024-160226-build-ctest`: build **263/263 edges**, CTest **81/81**, terminal verdict **pass**. No duplicate native verify was launched.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Both gate and game wait operations were collected to terminal results.

## Game evidence

[Run 512](../runs/occprops12-t79-20261024-a-524.md), label `occprops12-t79-20261024-a`, Konradshohe, turbo, requested end date 2032-01-01. Verdict **clean**, actual final date **2032-01-19**, `ref_warnings=0`, foreign exceptions/stalls/RTC/overruns all zero. DLL SHA256 prefix `52a20570`; 30.750 simulation seconds, 42 seconds including queue/load.

Exact summary lines:

```text
occprops12-t79-20261024-a 73/73
construction 73/73
```

There were **148 occupant observations, 740 field comparisons**, maximum 13 occupants in a sample, and no uninitialized instances. **39/73 samples were nonempty; 34/73 were explicitly empty**. These are repeated observations, not 148 distinct objects. Load 2026-01-27 had 7 occupants / 35 fields; 2026-02-01 had 12 / 60; 2031-12-01 had 3 / 15; 2032-01-01 was empty. Every dated line had `mismatches=0`; the harness reports `construction 73 lines, 73 with every mismatch count 0` and zero resyncs. No normal-speed repeat was needed because turbo had no differences.

Evidence: `build-runs/game-results/occprops12-t79-20261024-a.log` and `.verdict`. No game request remains outstanding.

## Questions and next steps

No new question filed: the getter contract is specified by lot-records §9.2/§9.3 and T79. Existing **LD39** in `docs/specs/questions/lot-developer.md` remains open for absent texture-scale properties; no duplicate was added. Record identity/lifetime and non-getter observation boundaries already have LR13/LR14 contracts. Full record, lifecycle, schedule and rendering comparison remains open and needs passive call/lifetime tracing, not active mutation probes or credit from getter equality.

This is a **ready partial closure**, not a claim that every method of the construction owner or the entire subsystem is verified. Coordinator: credit only the measured getter scope, and append `construction_shadow` to the current single `shadows = ...` regress profile after integration. The profile was not edited here: this reserved detached round forbids ref changes and the requested profile procedure requires merging main first. No main merge, branch switch, push or ABI bump was performed. At the two-thirds context wind-down point, no further implementation scope was started.
