| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-uiwin-close13 (base-window settled contracts) | `Codex` / `core-uiwin-close13` (engine lane) | in progress | Implement WT5, WT10-WT12; gate and report |

## Handoff
Goal: close settled cGZWin and cGZFadeTmpWin contracts in engine/core/src/ui_windows/base.
Done: merged core-uiwin-close11 (77657872e) then re-widgettk-wt13 (af47e8a3e); lane reservation 47b6ef327.
Done: implemented WT5, WT10-WT12 lifetime, filters and dispatch; added section 10.1 vectors and updated owner closure (6c07efc0e).
Next: collect final-source focused cargo (build-runs/focused-final-console.log) to terminal, then run stable-source qbuild all and queued verify, report and enqueue actual detached HEAD.
Facts: no plugin/C ABI changes; no game requests outstanding. Native checks must use qbuild verify.
Questions: WT13 (unpublished fade destructor), WT14 (explicitly unsettled opaque graphics/slot 150) appended.
Evidence: q.py Bash wrapper unavailable (CreateFileMapping error); PowerShell/local cargo works. Initial fmt used wrong relative paths, corrected to absolute paths; formatting applied via apply_patch.
Formatting: explicit seven-file qbuild fmt passes with diff_lines=0.
Initial focused cargo session 12063 terminal PASS: 39/39, compile 16m22s; final source adds one drain-order test and is being checked separately. Expected should-panic sentinel-erasure vector passed.
