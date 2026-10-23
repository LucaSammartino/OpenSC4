| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-lh5: Lua thread and command-generator lifecycle | Codex / `re-lua-lh5` | in progress | Resolve LH4/LH5 from original export; doccheck and enqueue. |

## Handoff
- Goal: resolve all open Lua host questions using original export evidence; prose and conformance vectors only.
- Done: LH4 behavior written in §§6.2.1–6.2.3, from registry and wrapper instruction queries.
- Next: finish LH5 generator section; add conformance cases; move question entries and doccheck.
- Facts: main first ID 0; ID counter survives Close; GetThread borrows; CreateThread owns output and consumes one source value; flags are not nargs.
- Generator evidence: unnamed region `008b0000.c` lines 947–1825; generic Init `00648ef0` returns false; callbacks `008b5b70`; fallback result double 2.
- Main evidence root: `C:/Users/tlori/FreeCity4`; edits only in this worktree.
- No engine reads/edits, ABI changes, or game runs planned.
