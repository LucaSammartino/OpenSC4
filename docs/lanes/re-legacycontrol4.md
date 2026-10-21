| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacycontrol4: spec cGZLegacyControl (new docs/specs/legacy-control.md) | Claude / .claude/worktrees/re-legacycontrol4 | in progress | see Handoff |

## Handoff
- Goal: new docs/specs/legacy-control.md for owner cGZLegacyControl (59 fns, ~104 KB); docs only.
- Done: worktree, lane file.
- Next: read ctor 0095b8c4, SetParameters 00957a9b, small COM/vtable fns, Common vocab/interpret; write spec.
- Facts: decomp/cGZLegacyControl.c (9338 lines); vtables 00ad5360 (11), 00ad5324 (8); ImplementInterface ICF-folded at 00935ea0.
