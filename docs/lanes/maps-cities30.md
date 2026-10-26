| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| maps-cities30: census protection/plumbing/power; kind engine | Codex; .claude/worktrees/maps-cities30; detached HEAD; reserved branch maps-cities30; code 5f3a56f52 | WIP; ABI unchanged; full gate PASS; three demolition scenarios clean; census acceptance incomplete | 1. Rerun 39 plumbing findings. 2. Resolve PR/PW questions and remaining shadows. 3. Verify all applicable census scenarios before ready enqueue. |

- Merged main first (c49f3dad8); code commits: eba3b7333 plumbing timing, 1b022d134 building-interface conversion, 5f3a56f52 query-list ownership.
- Changed: lot slot 48's building becomes an occupant through SDK AsOccupant; explicit releases clear owning list values, preventing destructor double releases; acquisitions are audited. Comparisons remain strict.
- Before census: protection 163/176, plumbing 137/176, power 175/176 exact (`C:/Users/tlori/FreeCity4/build-runs/suite/sh28-census3/per_key.tsv`).
- After full census: UNVERIFIED; strict suite-verified owner credit is NOT requested. The 39 plumbing / 13 protection / 1 power baseline findings still require complete affected-scenario verification.
- After targeted final runs: protection 1/3, plumbing 3/3, power 2/3 scenarios exact; E=`C:/Users/tlori/FreeCity4/build-runs/suite/maps-cities30-4/`, `per_key.tsv`.
- Darris: clean, end=1, ref_warnings=0, foreign_exceptions=0, rtc=0; 28 actions ok/1 refused (E/`maxisland-darris.verdict:2`, `summary.tsv`).
- Indereast: clean, end=1, ref_warnings=0, foreign_exceptions=0, rtc=0; 29 actions ok/0 refused (E/`maxisland-indereast.verdict:2`, `summary.tsv`).
- West Weburg: clean, end=1, ref_warnings=0, foreign_exceptions=0, rtc=0; 29 actions ok/0 refused (E/`maxisland-west-weburg.verdict:2`, `summary.tsv`).
- Reach: Darris 9085, Indereast 9369, West Weburg 9396 functions (E/`summary.tsv`); no new reached-byte credit calculated.
- Final game DLL SHA256: 263b1cbfb7fede92dd844f706191e1cf9f4bfcd6da4d2ae3d5f9ce6c83eb31bd (E/`summary.tsv`).
- Full gate after fixes: PASS; Rust 8076/0 failed (`build-runs/qbuild-test.log`); clippy 0 diagnostics (`build-runs/qbuild-clippy.log`).
- Native gate: build 37/37 edges PASS (`build-runs/fc4verify-build.log`); CTest 135/135 PASS (`build-runs/fc4verify-ctest.log`).
- Focused: qbuild quick PASS (680 capi tests, 228/228 native edges); actual plugin load test PASS, including distinct interfaces and exactly one query-reference release after list destruction (`engine/build-x86/FreeCity4.log`).
- Earlier failures kept separate: `build-runs/maps30-failed-checks.json`; suite `maps-cities30-2` was corrupt despite end=1 (before double-release fix); it supplies no acceptance evidence.
- Value-only repro per initial exit: `build-runs/repros/maps30-{darris,indereast,west-weburg}.json`; later invalid capture pointers are in the corresponding `*-capture.json` files. All six are ignored evidence, not game data.
- Prior Berlin evidence remains: clean, end=1, ref_warnings=0; plumbing 6/7 → 8/8 samples exact, resyncs=0 (`maps-cities30-1/berlin-konradshohe.verdict:273`). Final full-census zero-resync acceptance remains unverified.
- Open PR-MAPS30-01 (`docs/specs/questions/protection.md`): saved grids versus fresh rebuild / one-point cone differences; Darris and Indereast remain inexact in E.
- Open PW-MAPS30-01 (`docs/specs/questions/power.md`): West Weburg ageing arithmetic remains inexact in E. No unsupported arithmetic changes made; no new questions filed this round.
- Next: run `build-runs/rerun-plumbing.py` (39 named baseline failures, tag maps-cities30-3, NOT submitted); resolve PR/PW items, rerun affected scenarios with reach, establish all three census counts and zero resyncs, then enqueue ready. WIP queue only.
