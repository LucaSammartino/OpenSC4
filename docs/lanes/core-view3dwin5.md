| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view3dwin5: cSC4View3DWin owner closure | Codex / core-view3dwin5 | in progress | Audit existing ui_windows::view3d against owner specs, implement and validate. |

## Handoff
- Goal: close cSC4View3DWin core owner; foreign window/renderer/camera/simulator traits; no C ABI or plugin changes.
- Worktree reused at reserved detached HEAD; initial working tree clean.
- Existing module: engine/core/src/ui_windows/view3d.rs. Do not duplicate.
- Next: read existing implementation and assigned spec sections, then implement missing owner behavior and closure tests/table.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-view3dwin5-r1-manifest.json.
