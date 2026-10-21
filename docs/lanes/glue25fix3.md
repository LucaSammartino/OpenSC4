# Lane glue25fix3

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue25fix3 | `.claude/worktrees/glue25fix3` | in progress | finish `enable = occmgr`, gate, game run, regress |

## Handoff

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
