| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzcheats7: the cheat code manager `cGZCheatCodeManager`, new spec cheat-codes.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzclip7) | **ready** (docs only). cheat-codes.md §1–§6: fields, the 28-slot interface, matching, default processing, notifications, the enable/disable password lock, states and history, the save data layout, quirks, three cases; the existing row `cGZCheatCodeManager` now points to it; cheat-dialog.md's pointer updated: specified +3,811 bytes (+44 functions) | CCM1 (save-data callers) |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 44 functions were read in the decompile; the enable/disable notices, the password setter, the default processing
exits and both save-data paths were checked on the instructions. The vtable `0x00ACFCD0` and the strings
`0x00ACFC08`/`0x00ACFC00` were read. Written on 2026-10-24.

### What changed

- New docs/specs/cheat-codes.md and docs/specs/questions/cheat-codes.md (CCM1); cheat-dialog.md "not covered" line
  updated; QUESTIONS.md regenerated.

### Quirks found

- `SetCheatCodeSaveData` reads the saved state-ID count, stored as bytes, as a count, so it inserts four times too
  many IDs (CCM1).
- `SetCheatCodePassword` re-enables the cheats; restoring save data appends rather than replaces.
- A registered cheat matches any text that starts with it.
- `GetListOfAllCheatStrings(…, n = 0)` returns the count without writing `n`.

### Coverage

- tools/coverage-map.tsv: row `cGZCheatCodeManager` → cheat-codes.md (was unspecified).
- Specified bytes 4,732,993 → 4,736,804 (+3,811, 84.8%, `cov_local.py`).

### Not covered

- `cRZFastCompression2` and the callers of the save-data slots (CCM1).
