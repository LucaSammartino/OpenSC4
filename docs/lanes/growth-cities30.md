| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-cities30; engine; census growth shadows | Codex / growth-cities30 / detached HEAD; supervisor publishes branch | WIP; code df604caf2, cd3d49690; ABI exports unchanged; seam 0x444D0001; Rust 8358/0, clippy 0, CTest 142/142; final games corrupt 2/2; strict suite target unmet; TR-GC30-1 filed | 1. Repair reference ownership. 2. Resolve terrain and existing capture questions. 3. Rerun affected census cases and obtain full after counts before ready/enqueue. |

- Growth §5.2.1: publish each population word before the synchronous tract callback; bulk-grid regression previously observed -1 instead of 85/226 and now passes. No iterator ABI guessed.
- Added MS/DM/AU/GD-GC29-1 worked cases: mixed advisor counters, duplicate startup months, maximum-slot bits 0/512/90, preview-marker presence, lot-centre removal and direct landmark rebuild. Five focused tests pass; owned Rust fmt passes.
- Fixed the full-profile demolition ESP failure: lot slot 48's building interface is QI'd to the occupant interface before producing an occupant token; only that acquired reference is released. DLL regression tests distinct interface addresses, QI refusal and nulls.
- Final gate PASS after that fix: `build-runs/qbuild-test.log` (8358/0), `qbuild-clippy.log` (0), `fc4verify-ctest.log` (142/142), `build-results/qbuild-20261027-011634-build-ctest.verdict`. Earlier gate also passed; no incomplete check credited.
- E = `C:/Users/tlori/FreeCity4/build-runs/suite/`. Before below is E/sh28-census3/per_key.tsv. After is ONLY the two observed scenarios in E/growth-cities30-2/*.json; both runs are corrupt, one incomplete, so these are sample exactness counts without suite credit.
- development: before 165/176; latest observed 2/2 scenarios exact; E/growth-cities30-2/per_key.tsv and scenario logs. Full after unmeasured.
- mysims: before 169/179; latest 1/2 exact; Souther Bay 2/4 samples (April/May differ, log:5882/6646). Tag-1 gen28-empty-small improves 1/25 to 25/25, but its verdict is corrupt. Full after unmeasured.
- terrain: before 173/179; latest 2/2 exact; clean tag-1 Sydenham remains 33/35 (log:2800/2857: 637 differences each, normal 388/1024 and altitude 1023/1024 exact). Full after unmeasured.
- demand: before 172/176; latest 2/2 exact; tag-1 Sydenham also exact in this invocation. Existing NaN contract remains unresolved. Full after unmeasured.
- aura: before 173/176; latest 2/2 exact; timed-lot capture question remains unresolved. Full after unmeasured.
- tract: before 177/179; latest 2/2 exact; E/growth-cities30-2/per_key.tsv. Full after unmeasured.
- neighbours: before 171/173; latest 2/2 exact; E/growth-cities30-2/per_key.tsv. Full after unmeasured.
- pollution: before 174/176; latest 2/2 exact; E/growth-cities30-2/per_key.tsv. Full after unmeasured.
- Tag 1 (df604caf2): Sydenham clean/end/ref0; gen28-empty-small corrupt/end/ref0; Souther Bay and grown Lua exited before end with demolition ESP failures. Their authoritative *.verdict files supersede suite JSON/summary (which misreports gen28 clean).
- Tag 2 (cd3d49690): Souther Bay corrupt/end/ref1671, RTC0; grown Lua corrupt/no-end/ref2937, RTC0. Seven other keys are exact in emitted samples; neither run earns clean-run credit. See both *.verdict files, not CLI's clean-exit-crash wording.
- Remaining reference evidence: Souther Bay log:2783 over-release at Drop:263 and :6498 at FC4DemolitionHost::LotOccupants:808; grown Lua has the same ownership failures. Investigate ownership before interpreting further state drift.
- Filed TR-GC30-1 in docs/specs/questions/terrain.md: PC53 operation/float-store boundaries for GetAltitude/GetFaceNormal; Sydenham's two load checks fall between CW027F and CW007F tick logs. Retained live precision and every comparison.
- Existing blockers, unchanged: DM-GC29-2 (questions/demand.md), AU-GC29-2 (questions/aura.md), GD-GC29-2 (questions/growth-developer.md). No operand order, native signature or per-lot capture contract guessed.
- Acceptance unmet: full-census after counts, every-key exact/zero-resync suite evidence, clean/ref0 final runs and strict owner-credit flip. No coverage file changed; no ready enqueue. A WIP merge-queue entry records the actual detached HEAD.
- Next: fix Drop/LotOccupants ownership using interface/spec contracts; obtain the four answers; rerun the 23 baseline-inexact scenarios plus any newly affected cases, then report full before/after counts and enqueue kind engine only when ready.
