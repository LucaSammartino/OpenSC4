| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzadvface7: the advisor's 3D face (`cSC4AdvisorUIImageWinProc`), advisor-face.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzstyles7) | **ready** (docs only). Object layout and interface slots, model loading (incl. DollyLlama), the idle/second animation cycle, `Init`/`Shutdown` and the 3D paint; owner claimed: specified +2,114 bytes (+6 functions); AW5 answered | AW4 (advice flag `0x20`) stays open |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 6 functions were read in the decompile; the loader, `DoPaint`, the constructor `0x00420B70`, the slots 6–9 and the
message target `0x00420100` were checked on the instructions; the vtables `0x00A83AC0`/`0x00A83AB0`, the mood tables
`0x00A83A8C`/`0x00A83A9C` and the constants `0x00A83AF8` (π) and `0x00A83B00` (0.85) were read. Written on 2026-10-24.

### What changed

- New docs/specs/advisor-face.md.
- advisor-window.md scope note points to it; AW5 moved to answered/advisor-window.md; QUESTIONS.md regenerated.

### Quirks found

- The camera is turned by a float π, leaving a ≈ −8.7·10⁻⁸ sine in its transform.
- `SetMood` remaps moods 0→3, 1→2, 3→0, 5→1 before choosing the models.
- The face's message target answers true for every message.
- The DollyLlama models only appear at the next model load.

### Coverage

- tools/coverage-map.tsv: new row `cSC4AdvisorUIImageWinProc` → advisor-face.md, before `cSC4Advisor*WinProc`.
- Specified bytes 4,766,509 → 4,768,623 (+2,114, 85.4%, `cov_local.py`).

### Not covered

- The model's own slots (advance, finished, draw) and `PlaceModel` (`0x00420C20`) beyond its arguments.
