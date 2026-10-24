| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gameprefs7: the game preferences dialog `cSC4GamePreferencesWinProc`, new spec game-preferences.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-introvid7) | **ready** (docs only). game-preferences.md §1–§5: fields and the five checkbox-to-preference bytes, `ShowDialog` (table, sizing, pages, notification targets), the modal run and close, the window procedure and the six buttons (accept, cancel, credits, two pages, tutorial-city reset), quirks, four conformance cases; owner claimed: `cSC4GamePreferencesWinProc`: specified +2,810 bytes (+8 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 8 claimed functions were disassembled (one is the credits dialog's constructor, assigned here by reach); the
dialog table (`0x00AB4B54`) and the vtable (`0x00AB4B64`) were read from the data section. Written on 2026-10-23.
ui-windows.md only listed the class; no upstream lane covers it. The preference bytes are named from
disasters.md, foundations.md, advisors.md and application.md.

### What changed

- New docs/specs/game-preferences.md.
- Read on the bytes:
  - the five checkboxes and their bytes (`+0xF08`, `+0xF09`, `+0x4EC`, `+0xF0B`, `+0xF0A`);
  - the one-entry dialog table (script `0x0A7DF315`, window `0x2A57DB82`);
  - the parent-filling size (640 × 480 without a parent) and the two pages;
  - the in-city disabling of `0xEA5E99D9`;
  - accept (write, PreferencesChanged, `SavePreferences`), cancel, credits;
  - the tutorial-city reset message `0xAC3B3FA6`;
  - the modal run with the location store, focus restore and the modal-flag close.

### Quirks found

- `ShowDialog` returns false on success.
- Accept posts and saves even without a preferences block.

### Coverage

- tools/coverage-map.tsv: `cSC4GamePreferencesWinProc` → game-preferences.md, before the `cSC4*WinProc` catch-all.
- Specified bytes 4,539,550 → 4,542,360 (+2,810, 81.3%, `cov_local.py`, on `5c0ff782` + the seventeen lanes before).

### Not covered

- `cSC4CreditsWinProc` (its own owner).
