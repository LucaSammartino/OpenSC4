| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-rzdirectory4 (complete cRZDirectory core owner) | `core-rzdirectory4` / detached reserved worktree (engine lane) | in progress | (1) Implement chapter 6 owner and independent tests; (2) focused checks; (3) full gate and report. |

## Handoff
- Goal: implement only cRZDirectory owner from framework-rz-files chapter 6, no ABI/glue/runtime edits.
- Done: chapter 6 owner implemented with byte-only primitive host; independent order/flags/failure/recursion/depth/path vectors added.
- Next: quick check running (exec session 2643); inspect result, fix failures, absolute-path fmt, then single final all gate.
- Baseline full build intentionally skipped per task-specific instruction; one final stable all gate required.
- Preserve native enumeration order, duplicate-dot quirks, caller append/front synthesis, global recursion depth, missing-separator child paths, outer-first rollback.
- OS host must expose first/next/close and create/remove primitives only; no computed lists/traversal.
- No game requests planned: core-only task has no shadow or runtime integration.
- Questions RZ9/RZ10/RZ11 appended: dot seen without flag 4; split length before/after separator; UNC with no third backslash.
- q.py Bash launch denied by environment; direct PowerShell capped commands work. fmt needs absolute paths (relative paths unresolved by wrapper).
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-rzdirectory4-r1-manifest.json.
