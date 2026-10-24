| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-uiwin-close13 (base-window settled contracts) | `Codex` / `core-uiwin-close13` (engine lane) | in progress | Implement WT5, WT10-WT12; gate and report |

## Handoff
Goal: close settled cGZWin and cGZFadeTmpWin contracts in engine/core/src/ui_windows/base.
Done: merged core-uiwin-close11 then re-widgettk-wt13 on reserved detached HEAD.
Next: read updated spec sections and existing base module; implement new conformance vectors.
Facts: no plugin/C ABI changes; no game requests outstanding. Native checks must use qbuild verify.
