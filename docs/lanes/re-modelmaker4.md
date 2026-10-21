| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker4 (RE spec: exact cSC4ModelMaker owner, docs/specs/model-maker.md) | `re-modelmaker4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/model-maker.md specifying the whole cSC4ModelMaker owner (105 functions, 34705 bytes per tools/coverage.py --owners) with an explicit function inventory and empty remainder.
- Done: worktree; owner list in build-runs/mm-names.txt (scratch; regenerate: tools/coverage.py --owners). Spec docs/specs/model-maker.md §1-5 (identity, layout, lifecycle, save, DoMessage).
- Next: §6 filters (CreateOccupantFilters, OccupantFilterIterationFunction, 0x5ea370/550), §7 load rect/radius/unload, §8 occupant handlers, §9 state handlers, §10 view changes, §11 lighting/dynamic texture, §12 model loading, §13 shadow/terrain hiding, §20 inventory. Helpers build-runs/fn.py <addr> (decomp body), vt.py, ids.py.
- Facts: REA down (No app is open), fallback re_query+decomp. Read/Write see this+4. vtable 0x00A8EDE0, class 0xA9C741B3, GetGZCLSID 0x0048EA60, factory 0x004929B0.
