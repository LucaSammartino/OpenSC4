| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-uiwin-close13 (base-window settled contracts) | `Codex` / `core-uiwin-close13` (engine lane) | in progress | Implement WT5, WT10-WT12; gate and report |

## Handoff
Goal: close settled cGZWin and cGZFadeTmpWin contracts in engine/core/src/ui_windows/base.
Done: merged core-uiwin-close11 (77657872e) then re-widgettk-wt13 (af47e8a3e); lane reservation 47b6ef327.
Done: implemented WT5, WT10-WT12 lifetime, filters and dispatch; added section 10.1 vectors and updated owner closure (6c07efc0e).
Next: collect full qbuild all session 14900 to terminal (60-second write_stdin waits); DO NOT launch a duplicate. Its console is build-runs/all-console.log; workspace stage log build-runs/qbuild-test.log. Then collect queued native verify if all does not include it; report and enqueue actual detached HEAD.
Facts: no plugin/C ABI changes; no game requests outstanding. Native checks must use qbuild verify.
Questions: WT13 (unpublished fade destructor), WT14 (explicitly unsettled opaque graphics/slot 150) appended.
Evidence: q.py Bash wrapper unavailable (CreateFileMapping error); PowerShell/local cargo works. Initial fmt used wrong relative paths, corrected to absolute paths; formatting applied via apply_patch.
Formatting: explicit seven-file qbuild fmt passes with diff_lines=0.
Initial focused cargo session 12063 terminal PASS: 39/39, compile 16m22s. Expected should-panic sentinel-erasure vector passed.
Final focused session 30706 terminal PASS: 40/40, zero failures; full console preserved in build-runs/focused-final-console.log. Source is stable for the required full gate.
Full gate launched once after dcda2500b through build-runs/run-check.py all; workspace tests complete: 7180 passed, 0 failed, 0 ignored across 45 result groups. Clippy is running (build-runs/qbuild-clippy.log); full session 14900 still pending. Source unchanged since 6c07efc0e. Formatter and final focused result committed at dcda2500b.
