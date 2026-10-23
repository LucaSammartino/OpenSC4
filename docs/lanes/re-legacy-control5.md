| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacy-control5 (legacy control behaviour spec) | `Codex` / `re-legacy-control5` (re) | in progress | (1) Inventory owner; (2) specify contracts and vectors; (3) doccheck and report. |

## Handoff
- Goal: specify cGZLegacyControl in docs/specs/legacy-control.md without engine edits.
- Worktree reused at detached HEAD as instructed; initial status clean.
- Done: initial scope commit ef6cbe97; discovered existing 45/57 spec and extended twelve interpreter contracts (§9.9–§9.20).
- Done: interpreter milestone 62354d10; added interface message/drawing/failure boundaries and 33 spec-side conformance vectors. LC4 answered; LC11/LC12 filed.
- Next: regenerate/check questions index, doccheck, diff review; finalize report and queue.
- Numeric facts: shared alpha multiplier binary32 0x3B808081; unsigned clamp 255; x87 multiplication stored as float argument.
- TextEdit flag table at 00AD56C8 decoded; OptGrp separator ':' at 00AA0424.
- Main evidence: C:/Users/tlori/FreeCity4/decomp/cGZLegacyControl.c and tools/re_query.py.
- No ABI edits, game requests, or engine reads authorized/needed.
