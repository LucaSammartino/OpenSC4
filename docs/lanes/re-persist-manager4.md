| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-manager4: cGZPersistResourceManager complete-owner spec | `re-persist-manager4` / (RE spec lane) | in progress; export inventory pending; no ABI or game runs | 1. Inventory all manager functions; 2. audit bodies and native instructions; 3. publish contract and vectors |

## Handoff

- Goal: specify the complete cGZPersistResourceManager owner (selection: 8,437 B); docs only.
- Worktree reused at detached HEAD e517da60a6; status clean at entry.
- Read foundation lane first 25 lines, persistence sections 1/4/5, open PR1-PR3, lane and merge queue formats.
- Inventory: 109 attributed functions, 8,437 B, saved in build-runs/manager-inventory.tsv (size descending).
- Milestone cedc284d: initial dispatch/lookup contract committed; full contract now drafted in persistence-manager.md.
- All 109 selected bodies read, truncated bodies reread; companion §11 lists every function by role.
- Native: write returns open/create status even after write/commit failure; hash growth reverses duplicate selection.
- Native: service update returns uninitialized OR byte; filed PR4, no deterministic return claim.
- Validation: all 109 selection starts present in companion §11, 8,437 B; 30 proposed vectors; questions index regenerated.
- Native vtable audit corrected persistence §4.2's reversed factory record/type overloads and convenience slot 8.
- Exact-owner-only proposal: build-runs/coverage-map-re-persist-manager4.tsv; no ABI proposal or table edits.
- Next: final doc checks and review, commit full report, write merge queue; no engine/runtime gate needed.
- Draft includes 30 independent proposed vectors, not native executions. No game run needed for RE docs.
- Export only names constructor; all other bodies are in decomp/_unnamed/region_00970000.c (plus 0050/0089/0099 helpers).
- Main evidence root: C:/Users/tlori/FreeCity4; never read engine/ or edit main.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-manager4-r1-manifest.json.
- No subagents, no tools/ABI edits, no game requests, no map proposal before full audit.
