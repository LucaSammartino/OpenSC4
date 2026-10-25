| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzw32-8: the Windows graphic system `cGZGraphicSystemW32` and `cGZScreenBuffer::Init`, gz-graphics-w32.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-gol8) | **ready** (docs only). Two owners claimed: specified +2,409 bytes (+44 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly; the nine vtables and the DirectDraw IID were read in the PE. Written on
2026-10-25.

### What changed

- docs/specs/gz-graphics-w32.md (new): §1 identity, the nine faces and their thunks, QueryInterface; §2 the graphics
  face's overrides (screen size, video memory through DirectDraw, cursor position, canvas init/shutdown); §3 the
  helper and Windows faces (driver names, presence, mode selection); §4 the screen buffer's `Init`; §5 conformance
  tests.

### Coverage

- tools/coverage-map.tsv: rows for `cGZGraphicSystemW32` (before `cGZGraphicSystem*`) and `cGZScreenBuffer` (before
  `cGZ*`).
- Specified bytes 5,111,741 → 5,114,150 (+2,409, 91.6%, `cov_local.py` with owners-c1).
