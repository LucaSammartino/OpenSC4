| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| histclock (history + clock glue) | `ac7e10a873454fec7` / `agent-ac7e10a873454fec7` | Reserved ABI 55, seam op 54. **merged `efac89a`** (ABI 52, taken as 55; seam op 54; gate histclock: cargo 1990/0, clippy clean, ctest 51/51; regress histclock run 113 PASS): `cFC4HistoryWarehouse` (`enable = history`) and `cFC4Clock24` (`enable = clock24`), both off by default and clean together in game (`hc-final-replace-2`); `history_shadow = 1` and `clock24_shadow = 1` exact in every lane run (224/224 and 207/207 at normal speed to 2043); save sweep record mode (`cargo test -p fc4-formats --release --test save_records -- --nocapture`): 417 saves, 0 failures | done. Next: add `history`/`clock24` to the regress replace list and both shadows to the shadows list (exact in every lane run; the clock shadow needs normal speed to see steps, CK22); an in-game save/load run with `enable = history, clock24` (script `save` action, then reload); a combined run with the regress replacement list plus both. Questions HW13-HW17, CK16-CK22 filed |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| histclock: history warehouse and 24-hour clock glue (`enable = history`, `clock24`), shadows, save sweep for the corpus | `ac7e10a873454fec7` | worktree | ABI 55, seam op 54 | merged `efac89a` (ABI 52); HW13-HW17, CK16-CK22 |
