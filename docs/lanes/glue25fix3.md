# Lane glue25fix3

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue25fix3 | `.claude/worktrees/glue25fix3` | in progress | finish `enable = occmgr`, gate, game run, regress |

## Handoff

- Goal: finish the occupant manager GZCOM class (`enable = occmgr`) from saved glue25, gate, game run to 2032
  (ref_warnings=0), full regress with occmgr added to main's replace list, queue for the train.
- Done: rebased glue25 onto main ea097498 (conflicts only CMakeLists.txt / FC4Subsystems.cpp, union-resolved):
  6d10b053 (class), 0f8101f9 (2026-10-16 WIP, unverified).
- Baseline gate on 185e5ce0 (WIP included): PASS, cargo 3576/0, clippy 0, ctest 62/62.
- Next: game run to2032 with enable=occmgr (label glue25fix3-g1), then regress with occmgr added.
- Facts: submodule engine/third_party/gzcom-dll extracted by git archive, .git points at main's modules dir.
