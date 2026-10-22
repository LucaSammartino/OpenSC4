| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-manager4: cGZPersistResourceManager complete-owner spec | `re-persist-manager4` / (RE spec lane) | in progress; export inventory pending; no ABI or game runs | 1. Inventory all manager functions; 2. audit bodies and native instructions; 3. publish contract and vectors |

## Handoff

- Goal: specify the complete cGZPersistResourceManager owner (selection: 8,437 B); docs only.
- Worktree reused at detached HEAD e517da60a6; status clean at entry.
- Read foundation lane first 25 lines, persistence sections 1/4/5, open PR1-PR3, lane and merge queue formats.
- Inventory: 109 attributed functions, 8,437 B, saved in build-runs/manager-inventory.tsv (size descending).
- Intermediate contract: docs/specs/persistence-manager.md; dispatch, segment lookup/enumeration, ownership asymmetry.
- Read ranks 0-29, but truncated bodies 0097C595, 00973E67, 00973FD6 need reread. Native write prefix checked.
- Next: read ranks 30 onward via build-runs/audit-manager.py rank; inspect 0xAD89D0 private slots and native quirks.
- Export only names constructor; all other bodies are in decomp/_unnamed/region_00970000.c (plus 0050/0089/0099 helpers).
- Main evidence root: C:/Users/tlori/FreeCity4; never read engine/ or edit main.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-manager4-r1-manifest.json.
- No subagents, no tools/ABI edits, no game requests, no map proposal before full audit.
