| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzclip7: the clipboard service (`cGZClipboardManager`, `cGZClipboardManagerClipboard`, `cGZClipboardManagerClipboardData`), new spec clipboard.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzwinmgr7) | **ready** (docs only). clipboard.md §1–§6: the 21-slot manager (clipboards by ID, the −1/−2 conventions), the 21-slot clipboard (typed data map, text, change notices), the Windows bridge (viewer chain, read and write of `CF_TEXT`/`CF_UNICODETEXT`), the data object, quirks, three cases; the existing row `cGZClipboardManager*` now points to it: specified +6,279 bytes (+73 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 73 functions were read in the decompile; the Windows write path's branches and the stub slots were checked on the
instructions. The vtables `0x00ACFEE0`, `0x00ACFEAC`, `0x00ACFE94`, `0x00ACFE40` and `0x00ACFDCC` were read. Written
on 2026-10-24.

### What changed

- New docs/specs/clipboard.md.

### Quirks found

- The manager's "every clipboard" writes return false even when they succeed.
- The buffer forms of the text slots are stubs that return false.
- With framework code page 16 the Windows write sets each format twice.
- Change notices are off by default (the clipboard constructor clears the notify byte).

### Coverage

- tools/coverage-map.tsv: row `cGZClipboardManager*` → clipboard.md (was unspecified).
- Specified bytes 4,726,714 → 4,732,993 (+6,279, 84.6% → 84.8%, `cov_local.py`).

### Not covered

- `ConvertStringEncoding` and the code-page queries; the text edit's own clipboard use (text-edit.md).
