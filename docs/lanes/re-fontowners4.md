| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fontowners4: complete cGZFontSys / cGZFont owner audit in drawing-fonts.md | `re-fontowners4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: spec cGZFontSys slots 0,2,4,12,17-19,21-24 + AddSubstitution 0x0089595D, then all 64 cGZFont slots (vtable 0x00AC55D8); publish inventory/remainder; credit only with empty remainder.
- Done: worktree from main bc88d498; drawing-fonts.md section 1 claim rewritten, section 12 slot numbers fixed (SetRenderer 6, SetMonochromeSizeRange 8), new 12.1 = all 25 cGZFontSys slots specified (commit 2).
- Next: add section 4.1 = 64-slot cGZFont inventory (build-runs/font-inv.txt; inv.py dumps vtables) + details for slots 0,1,5,9,10,12,13,27,28,37,47(TextArea(s,n,rect)),57; fix 8.2 slot 46 table and 8.3 ellipsis (ellipsis wider than rect -> text emptied).
