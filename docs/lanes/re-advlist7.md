| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advlist7: the advice list control `cSC4WinAdviceList`, new spec advice-list.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-catview7) | **ready** (docs only). advice-list.md §1–§6: fields and the row record, the list interface (`AddEntry`, `RebuildHTML`, background, clear, HTML pass-throughs), the HTML page builder, the toggle and close links (messages `0x426841A1` and `0xCA09FA6C`), `Init`, `Shutdown`, `SetArea`, four conformance cases; AW1–AW3 of advisor-window.md answered; owner claimed: `cSC4WinAdviceList`: specified +3,351 bytes (+18 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 18 claimed functions were disassembled; both vtables and the HTML strings (`0x00AB564C`–`0x00AB5868`) were read
from the data section. Written on 2026-10-23. ui-windows.md only listed the class; no upstream lane covers this owner.

### What changed

- New docs/specs/advice-list.md.
- Read on the bytes:
  - the row record (five HTML strings, open byte, advice, static byte);
  - the nine `AddEntry` arguments (the last one replaces the background ID);
  - the page template and the per-row markup (toggle and close anchors, the width cell at the HTML width − 61);
  - the click sound;
  - the toggle path (open flip, `0x426841A1` with advisor ID and mood, advice flag `0x20`, rebuild keeping the
    scroll);
  - the close path (advice flag 8, `0xCA09FA6C` sent at once);
  - the `DoURL` fallback result.
- advisor-window.md: AW1, AW2 and AW3 answered (§5.2 and §6.2 updated, entries moved to
  questions/answered/advisor-window.md).
- news-ticker.md §7 now points to advice-list.md. QUESTIONS.md regenerated.

### Quirks found

- `QueryInterface` has no base-class fallback.
- The HTML control's pointer is kept without its own reference.
- The row index of a link is not bound-checked.
- Static rows still get a close link.

### Coverage

- tools/coverage-map.tsv: `cSC4WinAdviceList` → advice-list.md, before the `cSC4Win*` catch-all.
- Specified bytes 4,533,159 → 4,536,510 (+3,351, 81.2%, `cov_local.py`, on `5c0ff782` + the fifteen lanes before).

### Not covered

- The HTML control (`cGZWinHTML`, out of scope).
