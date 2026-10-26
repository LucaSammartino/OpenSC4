| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tinyown8: one-function owners left in scope (effect records and class ids, effects-parser switches, audio director getters and factories, `cEaseSound`, `cGroup_SC4RH`, `GZCOM`, `D3DXMatrixIsIdentity`) and the framework's ordered-participant pass, tiny-owners-b9.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-smallown8) | **ready** (docs only). Twenty-eight owners claimed: specified +2,077 bytes (+28 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Each body was read in the decompile, those with branches or x87 code in the disassembly. The public SDK header
`cIGZCOMDirector.h` (nsgomez/gzcom-dll) named slot 12 `GZCOM()`. Written on 2026-10-25.

### What changed

- docs/specs/tiny-owners-b9.md (new): §1 effect records, class ids and the parser factory; §2 `ParseFloatRange` and
  `ParseRangedFloat` (a NaN passes the range test); §3 audio director getters, factories, `cEaseSound`,
  `cGroup_SC4RH::Clear`, the folded list constructor `cIGZSndUpdateTarget` and `EnumFunc_UpdateView`; §4 `GZCOM()`;
  §5 `D3DXMatrixIsIdentity`; §6 conformance tests.
- docs/specs/app-shell-window.md §2: the framework's slot 15 (`0x00473440`, symbol `SendFatalExceptionNotification`),
  the ordered-participant pass, was only named; its body is now given.
- docs/specs/audio.md §6.2: `0x004ECD60` is `GetSC4AudioCOMDirector`, not an automaton method (corrected).

### Coverage

- tools/coverage-map.tsv: rows for the 28 owners before their catch-alls (`cSC4*Particle*`, `cSC4*Description`,
  `fn:GZCOM_*`, `cIGZ*`, `fn:*`); `cEaseSound` and `cGroup_SC4RH` filled. The re-scrcmd8 row
  `fn:GZCOM_nSCRes_cFileParserCLSID` sat below `fn:GZCOM_*` and never matched; it is moved above it.
- Specified bytes 5,132,656 → 5,134,733 (+2,077, 92.0%, `cov_local.py` with owners-c1).
