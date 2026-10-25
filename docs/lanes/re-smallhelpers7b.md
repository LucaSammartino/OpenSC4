| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-smallhelpers7b: 68 small free functions (director getters, number and hex text, patterns, save-game records, window parameters, Gimex I/O callbacks), small-helpers2.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-fwmisc7c) | **ready** (docs only). 68 owners claimed: specified +5,327 bytes (+70 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 70 functions were read in the decompile; the save-game opener, the version record, the advisor notification, the
file-browser comparator and the director getters in the disassembly. Several are cited by other specs, which are
linked; the behaviour lines here are this lane's own reading. Left out: the text-encoding character helpers
(text-edit-encoding.md), the effects-parser switches, the Hit and radio factories and callbacks, and `GZCOM`.
Written on 2026-10-24.

### What changed

- New docs/specs/small-helpers2.md.

### Quirks found

- `ConvertBinaryDataToASCIIArray` ends its output with the character `'0'`, not a NUL.
- `SC4ReadAppLoadSaveVersion` reads a missing version record as 65 and an unreadable one as 0.

### Coverage

- tools/coverage-map.tsv: 68 `fn:` rows before the `fn:*` catch-all (`fn:operator[*]` escapes the literal star).
- Specified bytes 4,933,284 → 4,938,611 (+5,327, 88.4%, `cov_local.py`).
