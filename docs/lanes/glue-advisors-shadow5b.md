| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5b: advisor shadow round 2 | Codex / glue-advisors-shadow5b | in progress | Read AD answers and finish record shadow; validate through 2032. |

## Handoff
- Goal: default-off, record-only same-process advisors shadow exact through 2032.
- Round 1: glue-advisors-shadow5 reports 0/73, AD17/AD18 and invalid load date.
- Done: AD17 adapter in C ABI saved-object factory (no export change); fixed load date via simulator.
- Done: AD18 strict mismatches retained, separate transition classification; native graph and ABI fixtures added.
- AD18 explicitly forbids masking/restoration for exact credit; clean native advice cannot round-trip strictly.
- AD19 filed for a possible separate live-import contract; absent that, preserve strict Read/Write differences.
- Done: aa8ac388 implements AD17, AD18 classification/date correction and fixtures.
- Focused ABI fixture PASS 1/1; workspace qbuild all PASS 6536/0/0; clippy 0 diagnostics.
- Done: both native operations reached PASS, CTest 69/69 (including advisors_shadow_test).
- Explicit verify: 220/220 build edges, 233.0 s total; noted docs-only tree changes during run.
- All native job: 46/46 build edges, 77.8 s total; no changed-tree note. Final logs build-runs/fc4verify-{build,ctest}.log.
- Game adv-shadow5b-20261023-r1-2032 completed clean, ref_warnings=0, 2032-01-01.
- Strict result: adv-shadow5b-20261023-r1-2032 1/73; 1479 mismatches, all AD18 load transitions, zero other differences.
- Load date now 2026-01-27, 858 records exact; final month 861 records, 22 AD18 differences.
- Next: collect normal-speed watcher session 1491 for adv-shadow5b-20261023-r2-normal, then final WIP report/queue.
- Normal request to 2026-04-01 outstanding; advisors_shadow=1, disable=advisors,mysims.
- DLL SHA256 f4db142b37f7379725dc531249ecbed4ee0defd37ae7e99554a27aa7550ec80f.
- Formatting check exposed pre-existing advisors.rs weak-handle formatting; owned new test formatting fixed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-advisors-shadow5b-r1-manifest.json.
