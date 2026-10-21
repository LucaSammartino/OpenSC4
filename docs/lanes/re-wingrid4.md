| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-wingrid4: cGZWinGrid owner spec (198 functions, 32735 B) in docs/specs/win-grid.md | `re-wingrid4` / (RE lane) | in progress | 1. see Handoff |

## Handoff

- Goal: docs/specs/win-grid.md + questions/win-grid.md covering all 198 cGZWinGrid functions (inventory build-runs/evidence-run4/run4-wingrid-owner-inventory.tsv, 32735 B).
- Done: worktree, lane file (b83d0f26); spec part 1 9cfbd2e0: layout §2, slot table §3, flags §4, accessors §5.2-5.6, thunks/helpers §5.7; 53/198 fn, 1788/32735 B; WG1-WG3 filed.
- Next: §5.1 lifecycle (ctor/Shutdown/dtor/Init/Release/DoMessage), then selection, cell data, scroll bars, layout, sort, paint.
- Tools (gitignored, recreate if lost): build-runs/inv.py -> inv-named.tsv; vtdump.py 00add578 00add7b0 > vt0.txt; macmap.py -> slotmap.tsv; casm.py addr... (compact asm); status.tsv (addr<TAB>section) + appendix.py regenerates spec §A and prints coverage.
- REA: no app open (static fallback). Slot names from re/symport/vtables.pkl mac __ZTV10cGZWinGrid#0.
- Facts: decomp/cGZWinGrid.c (4914 lines); cell storage cRZSparseMatrix<WinGridCellData>, MatrixRow, CellFormatExtra.
