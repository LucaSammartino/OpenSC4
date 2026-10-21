# Lane glue25fix3

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue25fix3 | `.claude/worktrees/glue25fix3` | in progress | finish `enable = occmgr`, gate, game run, regress |

## Handoff

Run4 round2 (2026-10-21): ROOT CAUSE FIXED in fdd2a5ef. FirstOut(fc4_occmgr_first_*(..., &occ), occ, out) took occ
by value; MSVC evaluates args right-to-left, so occ was 0 -> every slot 28-35 query missed (and leaked a ref).
g7 recheck proved the core hits. g8 (fdd2a5ef, enable=occmgr, Konradshohe to 2026-03): CLEAN, ref_warnings 0,
first 28315 calls / 12676 hits, lots with building 736/741, tax 4059 (= shadow g2), no shutdown AV.
51f91c0c: sampler off (kSampleFirstQueries), shutdown trace via FC4_LOGV. Outstanding: glue25fix3-g9 (g1 req to 2032).
Next: wait g9 verdict, then qbuild all, fmt, finish report + merge-queue json.

Run4 round1 diagnosis (2026-10-21):
- g1 log: the city was NOT killed by the exception first. From the first month budget tax income was 0 (g2: ~4059/month),
  funds went to -101730, the game ended the city itself at 2028-04-01 (bankruptcy), and the foreign AV (null read,
  SimCity 4.exe+0x1D0F94, called from FreeCity4.dll) happened inside "occmgr Shutdown: enter" (RemoveOccupants with
  shutdown, occupant callbacks). Same 742 lots in both runs, so lots exist but count no tax with occmgr.
- Added diagnostics (budget lot scan tallies: with building / livable / land value / building value; occmgr query
  tallies and a per-occupant shutdown trace) in 31efb035.
- g3 (enable=occmgr, turbo to 2026-05-01, corrupt): every budget lot scan has "with building 0" (745 lots, livable 723,
  land value>0 all, building value>0 0) and the lot count falls; first-occupant queries (slots 28-35) 42874 calls,
  0 hits (lists 70606 calls / 254212 items work). Shutdown AV is on the FIRST occupant's Shutdown() (trace #1: post
  Remove ok, then occ->Shutdown() of a construction occupant 0x096E763C crashes, null ecx) - likely a consequence.
- g5/g6 samples (cFC4OccMgr.cpp Sample(), every 397th first-query call): slot 33 (std cell 3,32 type 0x497F6D9D) MISSES
  though the cell holds a base texture 0x497F6D9D box [0.1,512.1..63.9,575.9]; the same cell with a null filter (slot 32)
  HITS. So the TYPE-filtered first query fails in game, while a core unit test with that exact box passes
  (tests.rs glue25fix3_first_by_standard_cell_typed_game_box, uncommitted? check git status).
- Uncommitted at the stop (commit it first): FC4BudgetHost.cpp tallies are committed; cFC4OccMgr.cpp Sample() recheck
  (logs "occmgr first-query recheck": typed fn, TYPE-kind filter struct, and first-all pointer) + the tests.rs test.
- Next: commit, quick build, queue g7 (same req as g6, label glue25fix3-g7), grep "first-query recheck". Suspects:
  includes() Filter::Type vs host occupant_type in game (HType GetType cast), or the null-filter hit being a different
  occupant (a prop) so the base texture's overlap test fails (then compare standard_box with the real box). Fix in core,
  then remove/keep diagnostics quiet, full gate, run to 2032 with enable=occmgr, then replace-union run.

- Current tip before this report f42c08b2. Baseline/full lane gate PASS: cargo3576/0, clippy clean, ctest62/62.
- glue25fix3-g1 replacement enable=occmgr is CORRUPT at 2028-04-01, foreign_exceptions=1, ref_warnings=0.
- glue25fix3-g2 is CLEAN to2032 with ref_warnings=0, but its request has ONLY occmgr_shadow=1 and no enable=occmgr. It is shadow evidence, not a replacement pass.
- NOT ready for merge; no replacement regress or coverage claim. Next: diagnose replacement g1 foreign exception, then replacement run and all-replacements regress including occmgr. Only after landing update main regress Replace and coverage-map replace key.
- The CLI background ceiling killed the round without a final result. Commits are preserved. No guard state was cleared.

## Prior Handoff

- Goal: finish the occupant manager GZCOM class (`enable = occmgr`) from saved glue25, gate, game run to 2032
  (ref_warnings=0), full regress with occmgr added to main's replace list, queue for the train.
- Done: rebased glue25 onto main ea097498 (conflicts only CMakeLists.txt / FC4Subsystems.cpp, union-resolved):
  6d10b053 (class), 0f8101f9 (2026-10-16 WIP, unverified).
- Baseline gate on 185e5ce0 (WIP included): PASS, cargo 3576/0, clippy 0, ctest 62/62.
- Next: game run to2032 with enable=occmgr (label glue25fix3-g1), then regress with occmgr added.
- Facts: submodule engine/third_party/gzcom-dll extracted by git archive, .git points at main's modules dir.
