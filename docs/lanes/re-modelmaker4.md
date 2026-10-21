| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelmaker4 (RE spec: exact cSC4ModelMaker owner, docs/specs/model-maker.md) | `re-modelmaker4` / (RE spec lane) | in progress | see Handoff |

## Handoff
- Goal: docs/specs/model-maker.md specifying the whole cSC4ModelMaker owner (105 functions, 34705 bytes per tools/coverage.py --owners) with an explicit function inventory and empty remainder.
- Done: worktree; owner list in build-runs/mm-names.txt (gitignored scratch).
- Next: read decomp/cSC4ModelMaker.c (main checkout) by offset; write spec sections.
- Facts: vtable 0x00A8EDE0, class 0xA9C741B3, GetGZCLSID 0x0048EA60, factory 0x004929B0.
