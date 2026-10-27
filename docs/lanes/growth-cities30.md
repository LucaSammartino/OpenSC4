| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-cities30; engine; census growth shadows | Codex / growth-cities30 / detached HEAD; supervisor publishes branch | WIP; main merged 385b079e9; code 5a7831644, 3d1b51150, 958974fcc, 800dc6ccd; ABI exports unchanged; seams 0x4D530001-2; Rust 8384/0, clippy 0, CTest 143/143; final games clean 2/2 but refs 0/1; strict suite target unmet | 1. Repair remaining saved-reference identity/lifetime defect. 2. Obtain arithmetic/capture answers. 3. Finish census and enqueue kind engine only when all acceptance criteria pass. |

- Merged main first; kept its borrowed building AsOccupant conversion and LotOccupants release/null fix, removing the superseded QI seam test. Retained df604caf2 population-before-callback fix and MS/DM/AU/GD-GC29-1 worked-case tests.
- Final gate PASS on code 800dc6ccd: `build-runs/qbuild-test.log` (8384/0), `qbuild-clippy.log` (0), `fc4verify-ctest.log` (143/143), `build-runs/build-results/qbuild-20261027-020119-build-ctest.verdict`.
- Owned Rust fmt: five files, exit 0/diff 0. Earlier quick failed a merge-created duplicate seam case; intermediate all failed the missing IsModal ABI annotation. Both corrected; only the final gate is credited.
- Fixed GetActiveMySims double Release: register each query-owned reference, release once, null SDK-vector entries. DLL tests cover distinct views, repeated pointers, QI refusal, nulls, empty vectors and actual SDK cleanup; also test IsModal slot 42 and use it for the shadow's daily/monthly gate.
- E = `C:/Users/tlori/FreeCity4/build-runs/suite/`. Before = E/sh28-census3/per_key.tsv. Latest below = ONLY E/growth-cities30-4/per_key.tsv (two scenarios); full-census after counts remain unmeasured, with no strict suite credit claimed.
- development: before 165/176; latest 2/2 scenarios exact, samples Sydenham 28/28 and Souther Bay 15/15; E/growth-cities30-4/*.json and *.log.
- mysims: before 169/179; latest 1/2 exact, samples 4/4 and 1/3; Souther Bay log:6069/6733 still differs in home_cell, commute_cell, jobless_days, home_position, job-null. EQ/HQ/rings/cursor differences are gone.
- terrain: before 173/179; latest 1/2 exact, samples 33/35 and 28/28; Sydenham log:2800/2857 retains the two PC53 load differences (TR-GC30-1).
- demand: before 172/176; latest 2/2 exact, samples 101/101 and 65/65; grown Lua precision and prior NaN cases remain open.
- aura: before 173/176; latest 2/2 exact, samples 3/3 and 3/3; prior intermittent timed-lot capture question remains open.
- tract: before 177/179; latest 2/2 exact, samples 7/7 and 5/5; E/growth-cities30-4/*.json and *.log.
- neighbours: before 171/173; latest 2/2 exact, samples 3/3 and 2/2; E/growth-cities30-4/*.json and *.log.
- pollution: before 174/176; latest 2/2 exact, samples 9/9 and 6/6; grown Lua double-accumulator case remains open.
- Final DLL SHA256 BFB974DE272CDB9A61942E96AE85B1D4F6B3F0CD5597E3793BA6E75E1AAABC1E. Tag 4 authoritative *.verdict: Sydenham clean/end/ref0/actions35/0; Souther Bay clean/end/ref1/actions29/0. Souther Bay fails the lane's zero-warning acceptance despite the clean label.
- Remaining ownership evidence: tag-4 Souther Bay log:7242, last building reference at MySims DropToken:285 during shutdown (class A9BD882D, serializable vtable +68721C). Investigate CaptureSims' raw serialized reference tokens versus CanonicalOccupant message tokens; do not suppress this warning.
- Tag 3 grown Lua: clean/end/ref0/actions293/115; development 15/15, mysims 3/3, terrain 27/28, demand 58/65, aura 3/3, tract 5/5, neighbours 2/2, pollution 5/12. Log:3755 demand differs under CW027F; :3829 recycled counter 5129.284889832139/5129.23388671875; :5077 normal queries 126/256 exact.
- Tag 3 gen28-empty-small: authoritative verdict corrupt/end/ref0/actions214/3; all eight keys exact (mysims 25/25, development 308/308). Log:23841 native access violation outside DLL. Suite CLI/JSON incorrectly calls it clean; no clean-run credit.
- Tag 3 Souther Bay: stalled/no-end in native dialog, log/verdict frozen at 2049-04-02; ref total unavailable. Tag 3 Sydenham clean/end/ref0. No stalled or corrupt sample is credited; tag 3 used the DLL before the modal fix.
- Filed DM-GC30-1 in docs/specs/questions/demand.md and PL-GC30-1 in docs/specs/questions/pollution.md: live-PC53 operation/store contracts and worked cases. Existing TR-GC30-1, DM-GC29-2, AU-GC29-2, GD-GC29-2 remain open on main; no arithmetic/signature guessed.
- Acceptance unmet: every-key exact/zero-resync full-census evidence, zero-warning final runs and strict owner-credit flip. Four of 23 baseline-inexact scenarios were revisited; 19 remain. No coverage files changed and no ready enqueue; WIP merge-queue tip records this handoff.
- Next: test/correct saved-reference occupant identity and lifetime, obtain the six arithmetic/capture answers, rerun remaining affected census cases and report full before/after counts, then enqueue kind engine only when all criteria pass.
