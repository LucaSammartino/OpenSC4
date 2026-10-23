| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin-vw5: VW6–VW18 owner contracts | Codex / re-view3dwin-vw5 | in progress | Verify export behavior, extend owner spec and vectors, close questions, doccheck. |

## Handoff
- Goal: answer VW6–VW18 in view3d-window.md; spec text only, no engine reads/edits.
- Worktree reused at detached HEAD as instructed; initial status clean.
- Read rules, lane format, question inventory and core-view3dwin5 lane handoff. Owner closure README is under forbidden engine/; use its filed VW questions as closure requirements.
- q.py cannot start Bash (CreateFileMapping error 5); use bounded native PowerShell commands instead.
- Done: VW10/VW11/VW13/VW15/VW16 owner rules and VW-T19–T25; corrected view3d.md paint-tail boundary.
- Numeric evidence: paint default 0 at 007F0AE1; strict elapsed > period at 007F6254; fade uses float32 0.17 and 0.13333334, not 150/1500.
- Next: VW6/7/8 lifetime facts, VW9 full command inventory, VW12 hash/parser, VW14 failures, VW17 menus, VW18 remaining helpers.
- Evidence sources: C:/Users/tlori/FreeCity4/tools/re_query.py, ghidra/export/windows, decomp/.
- No game runs or ABI rows planned. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-view3dwin-vw5-r1-manifest.json.
