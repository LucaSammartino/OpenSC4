| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwintext7: the text label `cGZWinText`, new spec widget-text.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; on main `bb641878` + re-batch7, after re-gzpie7) | **ready** (docs only). widget-text.md §1–§6: identity, fields, text flags, the 29-slot `cIGZWinText` table (the `<shared>` small slots included), lifecycle, message passing, font and colour handling, minimum text area, alignment and text position, painting, quirks, three cases; owner claimed: `cGZWinText`: specified +2,554 bytes (+21 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 21 functions were read in the decompile; the draw call's arguments and the single-line width rule were checked
on the instructions, and the interface's `<shared>` slots were disassembled. The vtables `0x00AE0118` and
`0x00ADFEB8` were read and compared with the `cGZWin` table. Written on 2026-10-24.

### What changed

- New docs/specs/widget-text.md.

### Quirks found

- `DoMessage` drops every non-key message unless text flag 1 passes it to the parent.
- `ResetColor` uses the font without a null check.

### Coverage

- tools/coverage-map.tsv: new row `cGZWinText` → widget-text.md, before the `cGZWin*` catch-all.
- Specified bytes 4,669,601 → 4,672,155 (+2,554, 83.6% → 83.7%, `cov_local.py`).

### Not covered

- The font slots, cited by number.
