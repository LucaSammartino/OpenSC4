| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-comdirs7: the draw, graphic, Win32 graphic and Gimex COM directors and the disaster placement tool, com-directors-misc.md, plus rows for `cSC4Preferences` and `cSC4AutoGamePause` (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-simmisc7b) | **ready** (docs only). Seven owners claimed: specified +2,211 bytes (+29 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The five owners of the new spec were read in the decompile, the draw and graphic directors' registrations and
`OnStart` in the disassembly. Row-only: `cSC4Preferences` (its constructor is application.md's "Added 2026-10-22"
paragraph) and `cSC4AutoGamePause` (auto-game-pause.md is its dedicated contract). Left out: the toll-booth tool
(network tool UI, active upstream), `cGZScriptCOMDirector` (the Lua script server's director), `cWinSC4App`
(application, active upstream), `nsTexUtil` and `SC43DGridElement`. Written on 2026-10-24.

### What changed

- New docs/specs/com-directors-misc.md.

### Coverage

- tools/coverage-map.tsv: rows for the four directors and the disaster tool before their catch-alls; the
  `cSC4Preferences`, `cSC4AutoGamePause`, `cGZGraphicCOMDirector` and `cGXCOMDirector` rows filled in place.
- Specified bytes 4,953,902 → 4,956,113 (+2,211, 88.8%, `cov_local.py`).
