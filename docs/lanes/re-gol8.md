| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gol8: the Game of Life tool's control parts game-of-life.md leaves out (construction, Init, Shutdown, keys, mouse, refresh), game-of-life-control.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-appshell8) | **ready** (docs only). Two owners claimed with game-of-life.md: specified +2,765 bytes (+18 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly; the cell colour was read in the PE. Written on 2026-10-25.

### What changed

- docs/specs/game-of-life-control.md (new): §1 the control's state and destruction; §2 Init (tick registration, the
  terrain, the marked-cell view, the simulator init and callbacks) and Shutdown; §3 keys and mouse (picking, the edit
  mode, capture); §4 the marked-cell refresh; §5 conformance tests. With game-of-life.md (the simulator bodies) the
  two owners are complete.

### Coverage

- tools/coverage-map.tsv: a `cSC4ViewInputControlGameOfLife` row before the `cSC4ViewInputControl*` catch-all and the
  `cSC4GameOfLifeSimulator` row filled.
- Specified bytes 5,108,976 → 5,111,741 (+2,765, 91.5%, `cov_local.py` with owners-c1).
