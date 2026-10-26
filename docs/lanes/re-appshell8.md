| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-appshell8: the application's top window `cWinSC4App` and the SC4 framework and message-server subclasses, app-shell-window.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-budgetdlg8) | **ready** (docs only). Four owners claimed: specified +3,776 bytes (+26 functions); budget-dialogs.md wording corrected | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly (most of `cWinSC4App` does not decompile); strings, constants and
vtables were read in the PE. Written on 2026-10-25.

### What changed

- docs/specs/app-shell-window.md (new): §1 `cWinSC4App` (state, Init with the key accelerator, the intro switch,
  the cheat registration and the time window, the four screens, messages, painting rate, shutdown); §2 the SC4
  framework's tick pass with its exception guard; §3 the SC4 message servers' sends; §4 quirks; §5 conformance tests.
- docs/specs/budget-dialogs.md: the dialog switch messages are sent (message server 2 slot 3, `MessageSend`), not
  posted.

### Coverage

- tools/coverage-map.tsv: `cWinSC4App` row filled; rows for `cSC4Framework`, `cSC4MessageServer` and
  `cSC4MessageServer2` (before the `cSC4*` catch-all).
- Specified bytes 5,105,200 → 5,108,976 (+3,776, 91.5%, `cov_local.py` with owners-c1).
