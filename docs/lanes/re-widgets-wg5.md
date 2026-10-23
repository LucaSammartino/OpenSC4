| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets-wg5 (widget control family specs) | Codex / `re-widgets-wg5` | in progress | Inspect owner exports and existing specs; write widget-controls.md and vectors; doccheck |

## Handoff

- Goal: specify Grid, TextEdit, ListBox, TreeView and FileBrowser without engine edits.
- Done: family reading map and Grid/TextEdit/FileBrowser summaries written with linked inventories, defaults, dispatch, drawing/IO and failure boundaries.
- Next: finish ListBox and TreeView sections from local decomp and native tables; add vectors/questions and doccheck.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export available via re_query.py. No REA needed or available binary.
- Tool note: q.py Bash wrapper fails with Windows CreateFileMapping error 5; use native PowerShell with capped output.
- No ABI edits, game requests or engine reads.
- Critical: ListBox export labels are shifted/misleading. Native secondary table 00AE1678 at W+0xD8 gives actual SetSelection slot 10=009C8CEE; window slot 158=009CA0DF removes an entry. Tree primary 00AE3240, window 00AE2FE0 at +4; unnamed ctor 009D2BF0.
- Ignored helper build-runs/inspect-widgets.py reads a function by address with 60-line cap or raw PE table words. Anonymous pseudocode is decomp/_unnamed/region_009c0000.c and region_009d0000.c.
