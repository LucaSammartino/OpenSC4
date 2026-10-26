| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| maps-cities30: census protection/plumbing/power; kind engine | Codex; .claude/worktrees/maps-cities30; detached HEAD; reserved branch maps-cities30; code eba3b7333 | WIP/blocked; ABI unchanged; full gate PASS; census acceptance incomplete | 1. Resolve demolition crash and PR/PW questions. 2. Finish remaining shadow timing fixes. 3. Rerun affected census scenarios with reach and enqueue only after all three keys are exact. |

- Changed: plumbing samples message-owned pipe/producer lists immediately before state 8, preserving computed pass grids/totals. Regression removes a pipe and producer after swap; comparisons remain strict.
- Before: protection 163/176, plumbing 137/176, power 175/176 exact (`C:/Users/tlori/FreeCity4/build-runs/suite/sh28-census3/per_key.tsv`). Full-census after counts remain UNVERIFIED.
- After, partial `maps-cities30-1/per_key.tsv`: protection 2/4, plumbing 2/2, power 3/3 scenarios with comparisons exact. Three runs ended early, so these are not full-scenario or suite-verified credit.
- Evidence root E: `C:/Users/tlori/FreeCity4/build-runs/suite/maps-cities30-1/`.
- Berlin: clean, end=1, ref_warnings=0; plumbing improved 6/7 to 8/8 exact samples, resyncs=0 (baseline `berlin-konradshohe.verdict:159`; E/`berlin-konradshohe.verdict:273`). Script: 18 ok/11 failed, as recorded in E/`summary.tsv`.
- Darris, Indereast, West Weburg: exited, end=0; reference warnings=0 (E/each `.verdict`; E/`summary.tsv`). No clean acceptance claimed.
- Integration blocker: all three exited reading pointer 0x5001 at DLL+0xF42198, symbolized with this DLL's PDB to `FC4DemolitionHost.cpp:406`, CanRemove; outside changed files. E/log lines: Darris 5282, Indereast 5328, West Weburg 5266.
- Focused Rust tests: 10/10 plumbing shadow tests passed, including the new regression. Owned-file fmt: 2 files, diff_lines=0.
- Required qbuild all: PASS; Rust 8012 passed/0 failed (`build-runs/qbuild-test.log`), clippy 0 diagnostics (`build-runs/qbuild-clippy.log`).
- Native gate: build PASS 328/328 edges (`build-runs/fc4verify-build.log`); CTest PASS 130/130 (`build-runs/fc4verify-ctest.log`). One all invocation; DLL SHA256 db8949090a079abe2c44d6ad4dfafd17db7606b6a5c609997a0f80c06bea3e36.
- PR-MAPS30-01: `docs/specs/questions/protection.md`; retained saved grids versus fresh cone rebuild, including one-point police differences. Fresh Darris/Indereast differences reproduced before their crashes.
- PW-MAPS30-01: `docs/specs/questions/power.md`; two one-ULP ageing differences in the original West Weburg census. Fresh run crashed before its failing May window, so unresolved.
- Missing: all three keys exact across every applicable census scenario, zero resyncs, clean end verdicts, reached-byte credit. No comparisons relaxed, scenarios excluded, ABI exports changed, or coverage records edited.
- Train: WIP hold only; no ready enqueue or suite-verified credit requested. Next: coordinator resolves the integration blocker/spec questions, then resume remaining implementation and complete census verification.
